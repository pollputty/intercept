use std::{io::Result, path::PathBuf};

use nix::errno::Errno;
use tracing::{debug, warn};

use super::tracee::Tracee;
use crate::syscall::{Clock, SysNum};

#[derive(Debug)]
pub enum Operation {
    Fork {
        num: SysNum,
    },
    Open {
        num: SysNum,
        path: PathBuf,
        read: bool,
        write: bool,
    },
    Rand {
        len: usize,
        addr: u64,
    },
    Time {
        num: SysNum,
        clock: Clock,
        addr: u64,
    },
    Pid {
        num: SysNum,
    },
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
        // Parse the syscall.
        let registers = tracee.registers();
        match registers.orig_rax.into() {
            // Open
            SysNum::Open => {
                let path = tracee.read_string(registers.rdi)?;
                let path = PathBuf::from(path);
                let rw_flags = registers.rsi & 0b11;
                Ok(Some(Operation::Open {
                    path,
                    num: SysNum::Open,
                    read: rw_flags != 1,
                    write: rw_flags != 0,
                }))
            }
            SysNum::OpenAt => {
                // For now we only handle the case where the first argument is AT_FDCWD.
                assert_eq!(registers.rdi as i32, -100);
                let path = tracee.read_string(registers.rsi)?;
                let path = PathBuf::from(path);
                let rw_flags = registers.rdx & 0b11;
                Ok(Some(Operation::Open {
                    path,
                    num: SysNum::OpenAt,
                    read: rw_flags != 1,
                    write: rw_flags != 0,
                }))
            }
            // Rand
            SysNum::GetRandom => {
                let len = registers.rsi as usize;
                let addr = registers.rdi;
                Ok(Some(Operation::Rand { len, addr }))
            }
            // Time
            SysNum::ClockGetTime => {
                let num = SysNum::ClockGetTime;
                Ok(Some(Operation::Time {
                    num,
                    addr: registers.rsi,
                    clock: registers.rdi.into(),
                }))
            }
            SysNum::Time => {
                let num = SysNum::Time;
                Ok(Some(Operation::Time {
                    num,
                    addr: registers.rdi,
                    clock: Clock::Realtime(0),
                }))
            }
            num @ (SysNum::GetPID
            | SysNum::GetPPID
            | SysNum::GetGID
            | SysNum::GetEGID
            | SysNum::GetUID
            | SysNum::GetEUID) => Ok(Some(Operation::Pid { num })),
            // Fork
            num @ (SysNum::Clone | SysNum::Fork | SysNum::VFork) => {
                debug!("fork-like operation");
                Ok(Some(Operation::Fork { num }))
            }
            // Wait
            SysNum::Wait => {
                debug!("process waits for child");
                Ok(Some(Operation::Wait))
            }
            // Exit
            SysNum::ExitGroup | SysNum::Exit => Ok(Some(Operation::Exit)),
            // Unknown syscall
            SysNum::Other(num) => {
                warn!(syscall = num, "unsupported");
                Ok(None)
            }
            // The rest is identified, and there is nothing to do
            num => {
                debug!(syscall = ?num, "ignored");
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
