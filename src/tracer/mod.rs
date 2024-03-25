mod operation;
mod tracee;

use crate::{
    config::Config,
    modules::{FileManager, RandomManager},
    Recorder,
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
    pub fn spawn<I, S>(cmd: &str, args: I) -> Result<Tracer>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        Ok(Tracer {
            pid: Tracee::spawn(cmd, args)?,
        })
    }

    pub fn run(&self, cfg: &Config) -> Result<()> {
        debug!("run");
        let files_redirect: HashMap<String, String> = cfg
            .redirect
            .files
            .iter()
            .map(|redirect| (redirect.from.clone(), redirect.to.clone()))
            .collect();

        let mut recorder = Recorder::new("record.json")?;
        let mut random_mgr = RandomManager::new(cfg.redirect.random);
        let file_mgr = FileManager::new(files_redirect);

        loop {
            match Tracee::wait(self.pid) {
                Ok(None) => {
                    debug!("command exited");
                    return Ok(());
                }
                Ok(Some((ref mut tracee, operation))) => match operation {
                    Operation::Open { ref path, num } => {
                        let record = file_mgr.process(tracee, path, num)?;
                        recorder.record(record)?;
                    }
                    Operation::Rand { len, addr } => {
                        random_mgr.process(tracee, len, addr)?;
                    }
                    op @ (Operation::Fork { .. } | Operation::Wait | Operation::Exit) => {
                        panic!("this operation type should not be returned here: {:?}", op);
                    }
                },
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
