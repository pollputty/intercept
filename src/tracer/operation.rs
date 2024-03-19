use std::{
    io::{Error, ErrorKind, Result},
    path::PathBuf,
};

use nix::errno::Errno;
use tracing::info;

use super::tracee::{State, Tracee};
use crate::syscall::SysNum;

pub enum Operation {
    Open { num: SysNum, path: PathBuf },
    Exit,
}

pub enum OperationResult {
    FileDescriptor(i32),
    Error(Errno),
}

impl Operation {
    pub fn parse(tracee: &mut Tracee) -> Result<Option<Operation>> {
        // Make sure we are in the proper state.
        match tracee.state() {
            State::BeforeSyscall => {}
            _ => return Err(Error::new(ErrorKind::Other, "invalid state")),
        }

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
            Operation::Exit => Err(Error::new(
                ErrorKind::Other,
                "cannot intercept an exit syscall",
            )),
        }
    }
}
