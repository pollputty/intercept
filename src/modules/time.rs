use std::{
    io::Result,
    time::{self, SystemTime},
};

use tracing::info;

use crate::{
    recorder::TimeRecord,
    syscall::Clock,
    tracer::{OperationResult, Tracee},
    SysNum,
};

pub struct TimeManager {
    active: bool,
}

impl TimeManager {
    pub fn new(active: bool) -> Self {
        TimeManager { active }
    }

    pub fn process(
        &self,
        tracee: &mut Tracee,
        num: SysNum,
        clock: Clock,
        addr: u64,
    ) -> Result<TimeRecord> {
        if self.active {
            todo!("redirect time syscall");
        }
        let time = match tracee.get_result()? {
            OperationResult::Success(code) => {
                assert_eq!(code, 0);
                let result = self.get_result(tracee, num, addr)?;
                info!("time({:?}, {:?})", clock, result);
                Some(result)
            }
            OperationResult::Error(errno) => {
                info!("time({:?}, {})", clock, errno);
                None
            }
        };

        Ok(TimeRecord { clock, time })
    }

    fn get_result(&self, tracee: &mut Tracee, num: SysNum, addr: u64) -> Result<SystemTime> {
        match num {
            SysNum::ClockGetTime => {
                let data = tracee.read_memory(addr, 12)?;
                let secs = u64::from_ne_bytes(data[0..8].try_into().unwrap());
                let nanos = u32::from_ne_bytes(data[8..12].try_into().unwrap());
                let time = SystemTime::UNIX_EPOCH
                    + time::Duration::from_nanos(secs * 1_000_000_000 + nanos as u64);
                Ok(time)
            }
            _ => unreachable!("unexpected time syscall {:?}", num),
        }
    }
}
