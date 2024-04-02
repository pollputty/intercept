use std::{
    io::Result,
    time::{Duration, SystemTime},
};

use tracing::info;

use crate::{
    recorder::TimeRecord,
    syscall::Clock,
    tracer::{OperationResult, Tracee},
    SysNum,
};

pub struct TimeManager {
    time: Option<SystemTime>,
    // real_ref: SystemTime,
    // mono_ref: Instant,
}

impl TimeManager {
    pub fn new(timestamp: Option<u64>) -> Self {
        let time = timestamp.map(|secs| (SystemTime::UNIX_EPOCH + Duration::from_secs(secs)));
        TimeManager {
            time,
            // real_ref: SystemTime::now(),
            // mono_ref: Instant::now(),
        }
    }

    pub fn process(
        &self,
        tracee: &mut Tracee,
        num: SysNum,
        clock: Clock,
        addr: u64,
    ) -> Result<TimeRecord> {
        let true_time = match tracee.get_result()? {
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

        if let (Some(time), Some(true_time)) = (self.time, true_time) {
            info!("overriding time");
            let new_time = match clock {
                Clock::Realtime(_) => time,
                _ => true_time,
            };
            let secs = new_time
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let nanos = new_time
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos();
            let mut bytes = Vec::from(secs.to_ne_bytes());
            bytes.append(&mut Vec::from(nanos.to_ne_bytes()));
            tracee.write_bytes(addr, &bytes)?;
        }

        Ok(TimeRecord {
            clock,
            time: true_time,
        })
    }

    fn get_result(&self, tracee: &mut Tracee, num: SysNum, addr: u64) -> Result<SystemTime> {
        match num {
            SysNum::ClockGetTime => {
                let data = tracee.read_memory(addr, 12)?;
                let secs = u64::from_ne_bytes(data[0..8].try_into().unwrap());
                let nanos = u32::from_ne_bytes(data[8..12].try_into().unwrap());
                let time = SystemTime::UNIX_EPOCH
                    + Duration::from_nanos(secs * 1_000_000_000 + nanos as u64);
                Ok(time)
            }
            _ => unreachable!("unexpected time syscall {:?}", num),
        }
    }
}
