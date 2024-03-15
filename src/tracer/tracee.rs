use crate::syscall::{Operation, OperationResult, SysNum};
use nix::{
    errno::Errno,
    sys::{
        ptrace,
        wait::{waitpid, WaitStatus},
    },
    unistd::Pid,
};
use std::io::{Error, ErrorKind, Result};
use tracing::debug;

enum State {
    BeforeSyscall,
    AfterSyscall,
    Exited,
}

pub struct Tracee {
    pid: Pid,
    state: State,
}

impl Tracee {
    pub fn new(pid: Pid) -> Self {
        Self {
            pid,
            state: State::BeforeSyscall,
        }
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

    pub fn read_string(&self, addr: u64) -> std::io::Result<String> {
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

    fn resume(&mut self) -> std::io::Result<()> {
        if let State::BeforeSyscall = self.state {
            // Step over the syscall instruction.
            self.step_over_syscall()?;
        }
        if let State::AfterSyscall = self.state {
            ptrace::syscall(self.pid, None)?;
        }
        Ok(())
    }

    pub fn parse_syscall(&self) -> Result<Option<Operation>> {
        // Make sure we are in the proper state.
        match self.state {
            State::BeforeSyscall => {}
            _ => return Ok(None),
        }

        // Reminder:
        // arg1 = registers.rdi;
        // arg2 = registers.rsi;
        // arg3 = registers.rdx;
        // arg4 = registers.r10;
        // arg5 = registers.r8;
        // arg6 = registers.r9;

        // Parse the syscall.
        let registers = ptrace::getregs(self.pid).unwrap();
        match registers.orig_rax.into() {
            SysNum::Open | SysNum::OpenAt | SysNum::Creat => {
                let path = self.read_string(registers.rsi as u64)?;
                Ok(Some(Operation::Open { path }))
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
        let retval = ptrace::getregs(self.pid)?.rax as i64;
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
                match waitpid(self.pid, None) {
                    Ok(WaitStatus::PtraceSyscall(_)) => {
                        self.state = State::AfterSyscall;
                    }
                    Ok(WaitStatus::Exited(_, _)) => {
                        self.state = State::Exited;
                    }
                    _ => {
                        return Err(Error::new(
                            ErrorKind::Other,
                            "expected to return after syscall",
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
}

impl Drop for Tracee {
    fn drop(&mut self) {
        self.resume().unwrap()
    }
}
