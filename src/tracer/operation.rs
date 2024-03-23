use std::{
    io::{Error, ErrorKind, Result},
    path::PathBuf,
};

use nix::errno::Errno;
use tracing::info;

use super::tracee::Tracee;
use crate::syscall::SysNum;

#[derive(Debug)]
pub enum Operation {
    Open { num: SysNum, path: PathBuf },
    Rand { len: usize, addr: u64 },
    Fork { num: SysNum },
    Wait,
    Exit,
}

#[derive(Debug)]
pub enum OperationResult {
    FileDescriptor(i32),
    NumBytes(usize),
    Pid(i32),
    Error(Errno),
}

impl Operation {
    pub fn parse(tracee: &mut Tracee) -> Result<Option<Operation>> {
        let _span = tracing::span!(tracing::Level::INFO, "parse", pid = tracee.pid()).entered();
        // Parse the syscall.
        let registers = tracee.registers();
        match registers.orig_rax.into() {
            SysNum::Open => {
                let path = tracee.read_string(registers.rdi)?;
                let path = PathBuf::from(path);
                Ok(Some(Operation::Open {
                    path,
                    num: SysNum::Open,
                }))
            }
            SysNum::OpenAt => {
                // For now we only handle the case where the first argument is AT_FDCWD.
                assert_eq!(registers.rdi as i32, -100);
                let path = tracee.read_string(registers.rsi)?;
                let path = PathBuf::from(path);
                Ok(Some(Operation::Open {
                    path,
                    num: SysNum::OpenAt,
                }))
            }
            SysNum::GetRandom => {
                let len = registers.rsi as usize;
                let addr = registers.rdi;
                Ok(Some(Operation::Rand { len, addr }))
            }
            num @ (SysNum::Clone | SysNum::Fork | SysNum::VFork) => {
                Ok(Some(Operation::Fork { num }))
            }
            SysNum::Wait => Ok(Some(Operation::Wait)),
            // TODO: handle more syscalls
            SysNum::Other(num) => {
                info!(syscall = num, "received an unsupported syscall");
                Ok(None)
            }
            // The process will exit
            SysNum::ExitGroup => Ok(Some(Operation::Exit)),
            // The rest is identified, and there is nothing to do
            _ => Ok(None),
        }
    }

    pub fn result(&self, retval: i64) -> Result<OperationResult> {
        if retval < 0 {
            Ok(OperationResult::Error(Errno::from_raw(-retval as i32)))
        } else {
            match self {
                Operation::Open { .. } => Ok(OperationResult::FileDescriptor(retval as i32)),
                Operation::Rand { .. } => Ok(OperationResult::NumBytes(retval as usize)),
                Operation::Fork { .. } => Ok(OperationResult::Pid(retval as i32)),
                Operation::Wait => Ok(OperationResult::Pid(retval as i32)),
                Operation::Exit => Err(Error::new(
                    ErrorKind::Other,
                    "result not available for exited process",
                )),
            }
        }
    }

    pub fn intercept(&self, tracee: &mut Tracee, address: u64) -> Result<()> {
        match self {
            Operation::Open { num, .. } => {
                let arg = match num {
                    SysNum::Open => 1,
                    SysNum::OpenAt => 2,
                    _ => {
                        return Err(Error::new(
                            ErrorKind::Other,
                            "invalid sysnum in Open operation",
                        ))
                    }
                };

                tracee.set_arg(arg, address)?;
                Ok(())
            }
            Operation::Rand { len, addr } => {
                // TODO: skip the syscall instead
                tracee.get_result(self)?;
                // Overwrite result with 0s.
                let data = vec![0u8; *len];
                tracee.write_bytes(*addr, &data)?;
                tracee.set_result(*len as u64)?;
                Ok(())
            }
            Operation::Exit | Operation::Fork { .. } | Operation::Wait => Err(Error::new(
                ErrorKind::Other,
                "cannot intercept an exit syscall",
            )),
        }
    }
}
