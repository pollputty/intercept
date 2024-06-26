mod operation;
mod tracee;

use crate::{
    config::{Config, SpawnOptions},
    modules::{FileManager, PIDManager, RandomManager, TimeManager},
    Record, Recorder,
};
use nix::{errno::Errno, sys::ptrace, unistd::Pid};
use operation::Operation;
pub use operation::OperationResult;
use std::{collections::HashMap, io::Result};
pub use tracee::Tracee;
use tracing::debug;

pub struct Tracer {
    pid: Pid,
}

impl Tracer {
    pub fn spawn<I, S>(cmd: &str, args: I, options: SpawnOptions) -> Result<Tracer>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        Ok(Tracer {
            pid: Tracee::spawn(cmd, args, options)?,
        })
    }

    pub fn run(&self, cfg: &Config) -> Result<()> {
        debug!("run");
        let mut files_redirect: HashMap<String, String> = cfg
            .redirect
            .files
            .iter()
            .map(|redirect| (redirect.from.clone(), redirect.to.clone()))
            .collect();

        // Add default redirection for /dev/(u)random if randomness is redirected.
        if cfg.redirect.random {
            for key in ["/dev/urandom", "/dev/random"] {
                if !files_redirect.contains_key(key) {
                    debug!("redirecting {} to /dev/zero", key);
                    files_redirect.insert(key.to_string(), "/dev/zero".to_string());
                }
            }
        }

        let mut recorder = Recorder::new(&cfg.record)?;
        let mut random_mgr = RandomManager::new(cfg.redirect.random);
        let time_mgr = TimeManager::new(cfg.redirect.time);
        let file_mgr = FileManager::new(files_redirect);
        let pid_mgr = PIDManager::new(cfg.redirect.pid);
        let disable_vdso = cfg.record.time || cfg.redirect.time.is_some();

        loop {
            match Tracee::wait(self.pid, disable_vdso) {
                Ok(None) => {
                    debug!("command exited");
                    return Ok(());
                }
                Ok(Some((ref mut tracee, operation))) => {
                    let record: Record = match operation {
                        Operation::Open { ref path, num, read, write } => {
                            file_mgr.process(tracee, path, num, read, write)?.into()
                        }
                        Operation::Rand { len, addr } => {
                            random_mgr.process(tracee, len, addr)?.into()
                        }
                        Operation::Time { num, clock, addr } => {
                            time_mgr.process(tracee, num, clock, addr)?.into()
                        }
                        Operation::Pid { num } => pid_mgr.process(tracee, num)?.into(),
                        op @ (Operation::Fork { .. } | Operation::Wait | Operation::Exit) => {
                            unreachable!(
                                "this operation type should not be returned here: {:?}",
                                op
                            )
                        }
                    };
                    recorder.record(record)?;
                }
                Err(e) => panic!("unexpected error: {:?}", e),
            }
        }
    }
}

impl Drop for Tracer {
    fn drop(&mut self) {
        match ptrace::detach(self.pid, None) {
            Ok(_) => (),
            Err(Errno::ESRCH) => (),
            Err(e) => panic!("Error detaching the command: {:?}", e),
        }
    }
}
