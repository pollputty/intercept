use super::{Operation, OperationResult};
use crate::syscall::SysNum;
use nix::{
    errno::Errno,
    libc::{c_void, user_regs_struct, MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE},
    sys::{
        ptrace,
        signal::Signal,
        wait::{waitpid, WaitStatus},
    },
    unistd::Pid,
};
use std::{
    io::{Error, ErrorKind, Result},
    path::PathBuf,
};
use tracing::{debug, warn};

enum State {
    BeforeSyscall,
    AfterSyscall,
    Exited,
}

pub struct Tracee {
    pid: Pid,
    state: State,
    registers: Option<user_regs_struct>,
}

impl Tracee {
    pub fn new(pid: Pid) -> Self {
        Self {
            pid,
            state: State::BeforeSyscall,
            registers: None,
        }
    }

    fn resume(&mut self) -> std::io::Result<()> {
        if let State::BeforeSyscall = self.state {
            // Step over the syscall instruction.
            self.step_over_syscall()?;
        }
        if let State::AfterSyscall = self.state {
            ptrace::syscall(self.pid, None)?;
            self.registers.take();
        }
        Ok(())
    }

    pub fn get_registers(&mut self) -> std::io::Result<user_regs_struct> {
        if let Some(registers) = self.registers {
            Ok(registers)
        } else {
            let registers = ptrace::getregs(self.pid)?;
            self.registers = Some(registers);
            Ok(registers)
        }
    }

    fn set_registers(&mut self, registers: user_regs_struct) -> std::io::Result<()> {
        ptrace::setregs(self.pid, registers)?;
        self.registers = Some(registers);
        Ok(())
    }

    pub fn set_arg(&mut self, index: u8, value: u64) -> std::io::Result<()> {
        debug!(index, value, "overwriting syscall argument");
        let mut registers = self.get_registers()?;
        match index {
            1 => registers.rdi = value,
            2 => registers.rsi = value,
            3 => registers.rdx = value,
            4 => registers.r10 = value,
            5 => registers.r8 = value,
            6 => registers.r9 = value,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "invalid argument index",
                ))
            }
        }
        self.set_registers(registers)?;

        Ok(())
    }

    pub fn parse_syscall(&mut self) -> Result<Option<Operation>> {
        // Make sure we are in the proper state.
        match self.state {
            State::BeforeSyscall => {}
            _ => return Ok(None),
        }

        // Parse the syscall.
        let registers = self.get_registers()?;
        match registers.orig_rax.into() {
            SysNum::Open => {
                let path = self.read_string(registers.rdi as u64)?;
                let path = PathBuf::from(path);
                Ok(Some(Operation::Open {
                    path,
                    num: SysNum::Open,
                }))
            }
            SysNum::OpenAt => {
                // For now we only handle the case where the first argument is AT_FDCWD.
                assert_eq!(registers.rdi as i32, -100);
                let path = self.read_string(registers.rsi as u64)?;
                let path = PathBuf::from(path);
                Ok(Some(Operation::Open {
                    path,
                    num: SysNum::OpenAt,
                }))
            }
            // TODO: handle more syscalls
            SysNum::Other(num) => {
                debug!(syscall = num, "received an unsupported syscall");
                Ok(None)
            }
            // The process will exit
            SysNum::ExitGroup => Ok(Some(Operation::Exit)),
            // The rest is identified, and there is nothing to do
            _ => Ok(None),
        }
    }

    pub fn get_result(&mut self, syscall: &Operation) -> Result<OperationResult> {
        // Make sure we are in the proper state.
        match self.state {
            State::BeforeSyscall => {
                // Step over the syscall instruction.
                self.step_over_syscall()?;
            }
            State::AfterSyscall => {}
            State::Exited => {
                return Err(Error::new(
                    ErrorKind::Other,
                    "invalid state: process has exited",
                ))
            }
        }

        // Read the syscall result.
        let retval = self.get_registers()?.rax as i64;
        match syscall {
            Operation::Open { .. } => {
                if retval < 0 {
                    Ok(OperationResult::Error(Errno::from_raw(-retval as i32)))
                } else {
                    Ok(OperationResult::FileDescriptor(retval as i32))
                }
            }
            Operation::Exit => Err(Error::new(
                ErrorKind::Other,
                "result not available for exited process",
            )),
        }
    }

    fn step_over_syscall(&mut self) -> Result<()> {
        // Make sure we are in the proper state.
        match self.state {
            State::BeforeSyscall => {
                // Step over the syscall instruction.
                ptrace::syscall(self.pid, None)?;
                self.registers.take();
                match waitpid(self.pid, None)? {
                    WaitStatus::PtraceSyscall(_) => {
                        assert_ne!(
                            Errno::ENOSYS,
                            Errno::from_raw(-(self.get_registers()?.rax as i32))
                        );
                        self.state = State::AfterSyscall;
                    }
                    WaitStatus::Exited(_, _) => {
                        self.state = State::Exited;
                    }
                    _ => {
                        return Err(Error::new(
                            ErrorKind::Other,
                            "expected to be at syscall-exit-stop",
                        ))
                    }
                }

                Ok(())
            }
            State::AfterSyscall | State::Exited => {
                Err(Error::new(ErrorKind::Other, "invalid state"))
            }
        }
    }

    fn step_to_syscall(&mut self) -> Result<()> {
        // Make sure we are in the proper state.
        self.resume()?;
        match waitpid(self.pid, None)? {
            WaitStatus::PtraceSyscall(_) => {
                // Sanity check that we are at the syscall-enter-stop.
                assert_eq!(
                    Errno::ENOSYS,
                    Errno::from_raw(-(self.get_registers()?.rax as i32))
                );
                self.state = State::BeforeSyscall;
            }
            WaitStatus::Exited(_, _) => {
                self.state = State::Exited;
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::Other,
                    "expected to be at syscall-enter-stop",
                ))
            }
        }

        Ok(())
    }

    fn _step(&mut self) -> Result<()> {
        ptrace::step(self.pid, None)?;
        self.registers.take();

        match waitpid(self.pid, None)? {
            WaitStatus::Stopped(_, Signal::SIGTRAP) => Ok(()),
            WaitStatus::Exited(_, _) => {
                self.state = State::Exited;
                Ok(())
            }
            _ => Err(Error::new(ErrorKind::Other, "expected sigtrap")),
        }
    }

    pub fn send_syscall(
        &mut self,
        syscall: SysNum,
        arg1: u64,
        arg2: u64,
        arg3: u64,
        arg4: u64,
        arg5: u64,
        arg6: u64,
    ) -> Result<u64> {
        debug!("sending syscall");
        if let State::BeforeSyscall = self.state {
        } else {
            return Err(Error::new(ErrorKind::Other, "invalid state"));
        }

        let old_registers = self.get_registers()?;
        // info!(old_opcodes = ?old_opcodes, "old opcodes");

        // // Write the syscall instruction
        // unsafe {
        //     ptrace::write(self.pid, rip, syscall_opcodes)?;
        // }

        // Update registers
        let mut new_registers = old_registers;
        new_registers.orig_rax = syscall.into();
        new_registers.rdi = arg1;
        new_registers.rsi = arg2;
        new_registers.rdx = arg3;
        new_registers.r10 = arg4;
        new_registers.r8 = arg5;
        new_registers.r9 = arg6;
        self.set_registers(new_registers)?;

        // Do the syscall
        self.step_over_syscall()?;

        let result = self.get_registers()?.rax as i64;

        // Restore registers and force a new syscall to be in the same state as before
        self.set_registers(old_registers)?;
        let rip = old_registers.rip as *mut c_void;
        let old_opcodes = ptrace::read(self.pid, rip)? as *mut c_void;
        let syscall_opcodes =
            u64::from_le_bytes([0x0F, 0x05, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90]) as *mut c_void;
        unsafe {
            ptrace::write(self.pid, rip, syscall_opcodes)?;
        }

        // Return in syscall-enter
        self.step_to_syscall()?;

        // Restore previous opcodes and registers (mostly for rip)
        unsafe {
            ptrace::write(self.pid, rip, old_opcodes)?;
        }
        self.set_registers(old_registers)?;

        if result < 0 {
            let err = Errno::from_raw(-result as i32);
            warn!(?err, "syscall error");
            return Err(Error::new(ErrorKind::Other, err));
        }

        Ok(result as u64)
    }

    fn reserve_memory(&mut self, len: usize) -> Result<u64> {
        self.send_syscall(
            SysNum::Mmap,
            0,
            len as u64,
            (PROT_READ | PROT_WRITE) as u64,
            (MAP_ANONYMOUS | MAP_PRIVATE) as u64,
            0,
            0,
        )
    }

    fn write_memory(&self, addr: u64, data: &[u8]) -> Result<()> {
        use std::os::unix::fs::FileExt;
        let path = format!("/proc/{}/mem", self.pid.as_raw() as u32);
        let mem = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)?;
        mem.write_all_at(data, addr)
    }

    pub fn write_string(&mut self, string: &str) -> Result<u64> {
        let addr = self.reserve_memory(string.len() + 1)?;
        let mut data = Vec::from(string.as_bytes());
        data.push(0);
        self.write_memory(addr, &data)?;
        debug!(addr, "wrote string in tracee");
        Ok(addr)
    }

    pub fn _read_memory(&self, addr: u64, len: usize) -> std::io::Result<Vec<u8>> {
        use std::os::unix::fs::FileExt;

        let path = format!("/proc/{}/mem", self.pid.as_raw() as u32);

        let mut data = vec![0u8; len];
        let mem = std::fs::File::open(path)?;
        let len_read = mem.read_at(&mut data, addr)?;

        data.truncate(len_read);
        Ok(data)
    }

    // TODO: what if not UTF-8?
    pub fn read_string(&self, addr: u64) -> std::io::Result<String> {
        debug!(addr, "reading string from tracee's memory");
        use std::os::unix::fs::FileExt;

        let path = format!("/proc/{}/mem", self.pid.as_raw() as u32);
        let mem = std::fs::File::open(path)?;
        let mut result = String::new();
        let mut cont = true;

        while cont {
            let mut data = vec![0u8; 4096];
            let len_read = mem.read_at(&mut data, addr)?;
            if len_read == 0 {
                break;
            }
            let null_byte = data.iter().position(|&x| x == 0);
            if let Some(null_byte) = null_byte {
                data.truncate(null_byte);
                cont = false;
            }
            result.push_str(
                String::from_utf8(data)
                    .map_err(|_| {
                        std::io::Error::new(std::io::ErrorKind::InvalidData, "read non-utf8 data")
                    })?
                    .as_str(),
            );
        }
        Ok(result)
    }
}

impl Drop for Tracee {
    fn drop(&mut self) {
        self.resume().unwrap()
    }
}
