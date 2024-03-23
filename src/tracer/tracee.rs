use super::{Operation, OperationResult};
use crate::syscall::SysNum;
use nix::{
    errno::Errno,
    libc::{c_void, user_regs_struct, MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE, PTRACE_EVENT_CLONE, PTRACE_EVENT_FORK, PTRACE_EVENT_VFORK, PTRACE_EVENT_VFORK_DONE, PTRACE_EVENT_EXEC},
    sys::{
        ptrace,
        wait::{waitpid, WaitPidFlag, WaitStatus},
    },
    unistd::Pid,
};
use std::io::{Error, ErrorKind, Result};
use std::os::unix::process::CommandExt;
use tracing::{debug, warn, info};

#[derive(Copy, Clone, Debug)]
enum State {
    BeforeSyscall,
    AfterSyscall,
    Exited,
}

#[derive(Debug)]
pub struct Tracee {
    pid: Pid,
    state: State,
    registers: user_regs_struct,
    allocations: Option<Vec<Memory>>,
}

#[derive(Debug)]
struct Memory {
    addr: u64,
    len: usize,
}

impl Tracee {
    pub fn spawn<I, S>(cmd: &str, args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        // Parse the command line and spawn the child process in a stopped state.
        let mut cmd = std::process::Command::new(cmd);
        cmd.args(args);
        unsafe {
            cmd.pre_exec(|| {
                ptrace::traceme()?;
                Ok(())
            });
        }
        let child = cmd.spawn()?;

        // Configure the child process to stop on system calls and resume it.
        let pid = Pid::from_raw(child.id() as i32);
        ptrace::setoptions(pid, ptrace::Options::all())?;
        ptrace::syscall(pid, None)?;
        Ok(())
    }

    fn new(pid: Pid, registers: user_regs_struct) -> Self {
        Self {
            pid,
            registers,
            state: if Errno::from_raw(-(registers.rax as i32)) == Errno::ENOSYS {
                State::BeforeSyscall
            } else {
                State::AfterSyscall
            },
            allocations: None,
        }
    }

    pub fn pid(&self) -> i32 {
        self.pid.as_raw()
    }

    pub fn registers(&self) -> user_regs_struct {
        self.registers
    }

    fn update_registers(&mut self) -> Result<()> {
        self.registers = ptrace::getregs(self.pid)?;
        Ok(())
    }

    fn set_registers(&mut self, registers: user_regs_struct) -> std::io::Result<()> {
        ptrace::setregs(self.pid, registers)?;
        self.registers = registers;
        Ok(())
    }

    pub fn set_arg(&mut self, index: u8, value: u64) -> Result<()> {
        debug!(index, value, "overwriting syscall argument");
        let mut registers = self.registers();
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

    pub fn set_result(&mut self, result: u64) -> Result<()> {
        debug!(result, "overwriting syscall result");
        let mut registers = self.registers();
        registers.rax = result;
        self.set_registers(registers)?;

        Ok(())
    }

    pub fn get_result(&mut self, operation: &Operation) -> Result<OperationResult> {
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
        let retval = self.registers().rax as i64;
        operation.result(retval)
    }

    // Helper methods to step the tracee to syscal-{enter,exit}-stop.
    fn step_syscall_and_wait(&mut self) -> Result<()> {
        loop {
            ptrace::syscall(self.pid, None)?;
            match waitpid(self.pid, None)? {
                WaitStatus::PtraceSyscall(_) => {
                    self.update_registers()?;
                    if Errno::from_raw(-(self.registers().rax as i32)) == Errno::ENOSYS {
                        self.state = State::BeforeSyscall;
                    } else {
                        self.state = State::AfterSyscall;
                    }
                    return Ok(());
                }
                WaitStatus::Exited(_, _) => {
                    self.state = State::Exited;
                    return Ok(());
                }
                WaitStatus::PtraceEvent(pid, _, _) => {
                    debug!(?pid, "ptrace event received while waiting for syscall");
                    continue;
                }
                e => {
                    return Err(Error::new(
                        ErrorKind::Other,
                        format!("unexpected wait status: {:?}", e),
                    ))
                }
            }
        }
    }

    fn step_over_syscall(&mut self) -> Result<()> {
        // Make sure we are in the proper state.
        match self.state {
            State::BeforeSyscall => {
                // Step over the syscall instruction.
                self.step_syscall_and_wait()?;
                assert!(matches!(self.state, State::AfterSyscall | State::Exited));
                Ok(())
            }
            State::AfterSyscall | State::Exited => {
                Err(Error::new(ErrorKind::Other, "invalid state"))
            }
        }
    }

    fn step_to_syscall(&mut self) -> Result<()> {
        // Make sure we are in the proper state.
        match self.state {
            State::AfterSyscall => {
                // Step over the syscall instruction.
                self.step_syscall_and_wait()?;
                assert!(matches!(self.state, State::BeforeSyscall));
                Ok(())
            }
            State::BeforeSyscall | State::Exited => {
                Err(Error::new(ErrorKind::Other, "invalid state"))
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
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

        // Copy old values.
        let old_registers = self.registers();
        let rip = old_registers.rip as *mut c_void;
        let old_opcodes = ptrace::read(self.pid, rip)? as *mut c_void;

        // Prepare new values for syscall
        let mut new_registers = old_registers;
        let sysno: u64 = syscall.into();
        new_registers.rax = sysno;
        new_registers.orig_rax = sysno;
        new_registers.rdi = arg1;
        new_registers.rsi = arg2;
        new_registers.rdx = arg3;
        new_registers.r10 = arg4;
        new_registers.r8 = arg5;
        new_registers.r9 = arg6;
        let syscall_opcodes =
            u64::from_le_bytes([0x0F, 0x05, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90]) as *mut c_void;

        let result = match self.state {
            State::BeforeSyscall => {
                // Setup syscall
                self.set_registers(new_registers)?;

                // Do the syscall.
                self.step_over_syscall()?;
                let result = self.registers().rax as i64;

                // Restore registers and force a new syscall so that we end up in the same state as before.
                self.set_registers(old_registers)?;
                unsafe {
                    ptrace::write(self.pid, rip, syscall_opcodes)?;
                }

                // Return in syscall-enter.
                self.step_to_syscall()?;

                // Restore previous opcodes and registers (mostly for rip).
                unsafe {
                    ptrace::write(self.pid, rip, old_opcodes)?;
                }
                self.set_registers(old_registers)?;

                result
            }
            State::AfterSyscall => {
                // Setup syscall
                self.set_registers(new_registers)?;
                unsafe {
                    ptrace::write(self.pid, rip, syscall_opcodes)?;
                }

                // Do the syscall.
                self.step_to_syscall()?;
                self.step_over_syscall()?;
                let result = self.registers().rax as i64;

                // Restore registers and opcodes.
                self.set_registers(old_registers)?;
                unsafe {
                    ptrace::write(self.pid, rip, old_opcodes)?;
                }

                result
            }
            State::Exited => return Err(Error::new(ErrorKind::Other, "invalid state")),
        };

        if result < 0 {
            let err = Errno::from_raw(-result as i32);
            warn!(?err, "syscall error");
            return Err(Error::new(ErrorKind::Other, err));
        }
        Ok(result as u64)
    }

    fn reserve_memory(&mut self, len: usize) -> Result<u64> {
        let addr = self.send_syscall(
            SysNum::Mmap,
            0,
            len as u64,
            (PROT_READ | PROT_WRITE) as u64,
            (MAP_ANONYMOUS | MAP_PRIVATE) as u64,
            0,
            0,
        )?;
        let mem = Memory { addr, len };
        match self.allocations {
            Some(ref mut vec) => vec.push(mem),
            None => self.allocations = Some(vec![mem]),
        }
        Ok(addr)
    }

    fn free_memory(&mut self, mem: &Memory) -> Result<()> {
        self.send_syscall(SysNum::Munmap, mem.addr, mem.len as u64, 0, 0, 0, 0)?;
        Ok(())
    }

    pub fn write_bytes(&self, addr: u64, data: &[u8]) -> Result<()> {
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
        data.push(0); // TODO: not necessary because memory is 0-initialized
        self.write_bytes(addr, &data)?;
        debug!(addr, "wrote string in tracee");
        Ok(addr)
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

    pub fn wait() -> Result<Option<(Tracee, Operation)>> {
        loop {
            match waitpid(None, Some(WaitPidFlag::__WALL)) {
                Ok(WaitStatus::Exited(pid, code)) => {
                    // TODO: remove this assertion and remember all children
                    warn!(?pid, ?code, "tracee exited");
                    return Ok(None);
                }
                Ok(WaitStatus::PtraceSyscall(pid)) => {
                    // A tracee is ready.
                    let registers = ptrace::getregs(pid)?;
                    let mut tracee = Tracee::new(pid, registers);
                    if let State::AfterSyscall = tracee.state {
                        // We get the result of a syscall we didn't bother checking
                        debug!(?pid, "received ignored result for syscall");
                        continue
                    }
                    let operation = Operation::parse(&mut tracee)?;
                    if let Some(operation) = operation {
                        // Some operations could block the tracee until the new process does something.
                        // For now these operations never reach the client.
                        if let Operation::Fork { .. } | Operation::Wait = operation {
                            // So that drop doesn't block.
                            tracee.state = State::AfterSyscall;
                            continue;
                        }
                        return Ok(Some((tracee, operation)));
                    } else {
                        // Syscall not supported, keep going.
                        continue;
                    }
                }
                Ok(WaitStatus::PtraceEvent(pid, _, event)) => {
                    // TODO: more interesting logging
                    match event {
                        PTRACE_EVENT_CLONE | PTRACE_EVENT_FORK | PTRACE_EVENT_VFORK => info!(?pid, "process is forking"),
                        PTRACE_EVENT_VFORK_DONE => info!(?pid, "vfork done"),
                        PTRACE_EVENT_EXEC => info!(?pid, "execing"),
                        _ => 
                            warn!(event, "unsupported ptrace event"),
                        
                    }
                    debug!(?pid, event, "ptrace event");
                    ptrace::syscall(pid, None)?;
                    continue;
                }
                Ok(WaitStatus::Stopped(pid, _)) => {
                    // TODO: more interesting logging
                    info!(?pid, "process was stopped");
                    ptrace::syscall(pid, None)?;
                    continue;
                }

                Ok(s) => panic!("unexpected stop reason: {:?}", s),
                Err(e) => panic!("unexpected waitpid error: {:?}", e),
            }
        }
    }
}

impl Drop for Tracee {
    fn drop(&mut self) {
        // free reserved memory
        if let Some(allocations) = self.allocations.take() {
            for mem in allocations {
                self.free_memory(&mem).unwrap();
            }
        }
        // resume the tracee
        ptrace::syscall(self.pid, None).unwrap();
    }
}
