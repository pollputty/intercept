use std::io::{Error, ErrorKind, Result};

use tracing::{error, info, warn};

use crate::{
    recorder::PIDRecord,
    tracer::{OperationResult, Tracee},
    SysNum,
};

pub struct PIDManager {
    pid: Option<u32>,
}

impl PIDManager {
    pub fn new(pid: Option<u32>) -> Self {
        if pid.is_some() {
            warn!("Caution: PID overriding is still experimental");
        }
        PIDManager { pid }
    }

    pub fn process(&self, tracee: &mut Tracee, num: SysNum) -> Result<PIDRecord> {
        let result = match tracee.get_result()? {
            OperationResult::Success(pid) => {
                info!("getpid({}, syscall={:?})", pid, num);
                pid
            }
            OperationResult::Error(errno) => {
                // This should never happen
                error!("getpid returned an error: {}", errno);
                return Err(Error::new(ErrorKind::Other, "getpid returned an error"));
            }
        };

        // If overwriting result is active
        if let Some(pid) = self.pid {
            match num {
                SysNum::GetPID => {
                    info!("overriding PID");
                    tracee.set_result(pid as u64)?;
                }
                _ => {
                    warn!("cannot overwrite pid for syscall {:?}", num);
                }
            }
        }

        Ok(PIDRecord { pid: result as u32 })
    }
}
