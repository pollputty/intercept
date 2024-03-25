use std::io::Result;

use rand::{rngs::StdRng, RngCore, SeedableRng};
use tracing::info;

use crate::{
    recorder::RandomRecord,
    tracer::{OperationResult, Tracee},
};

const SEED: u64 = 0xdeadbeef;

pub struct RandomManager {
    rng: StdRng,
    active: bool,
}

impl RandomManager {
    pub fn new(active: bool) -> Self {
        RandomManager {
            active,
            rng: StdRng::seed_from_u64(SEED),
        }
    }

    pub fn getrandom(&mut self, len: usize) -> Vec<u8> {
        let mut buf = vec![0; len];
        self.rng.fill_bytes(&mut buf);
        buf
    }

    pub fn process(&mut self, tracee: &mut Tracee, len: usize, addr: u64) -> Result<RandomRecord> {
        if self.active {
            // TODO: skip the syscall
            tracee.get_result()?;
            // Overwrite result with 0s.
            let data = self.getrandom(len);
            tracee.write_bytes(addr, &data)?;
            tracee.set_result(len as u64)?;
        }

        let result = tracee.get_result()?;
        match result {
            OperationResult::Success(num_bytes) => {
                info!("getrandom({})", num_bytes);
            }
            OperationResult::Error(errno) => {
                info!("getrandom({})", errno);
            }
        }
        Ok(RandomRecord { length: len })
    }
}
