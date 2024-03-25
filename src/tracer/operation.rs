use std::{io::Result, path::PathBuf};

use nix::errno::Errno;
use tracing::{debug, warn};

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
    Success(i32),
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
                debug!("fork-like operation");
                Ok(Some(Operation::Fork { num }))
            }
            SysNum::Wait => {
                debug!("process waits for child");
                Ok(Some(Operation::Wait))
            }
            // TODO: handle more syscalls
            SysNum::Other(num) => {
                warn!(syscall = num, "received an unsupported syscall");
                Ok(None)
            }
            // The process will exit
            SysNum::ExitGroup | SysNum::Exit => Ok(Some(Operation::Exit)),
            // The rest is identified, and there is nothing to do
            num => {
                debug!(syscall = ?num, "received ignored syscall");
                Ok(None)
            }
        }
    }

    pub fn result(retval: i64) -> OperationResult {
        if retval < 0 {
            OperationResult::Error(Errno::from_raw(-retval as i32))
        } else {
            OperationResult::Success(retval as i32)
        }
    }
}
