mod operation;
mod tracee;

use crate::config::Config;
use nix::{errno::Errno, sys::ptrace, unistd::Pid};
use operation::{Operation, OperationResult};
use std::{collections::HashMap, io::Result};
use tracee::Tracee;
use tracing::{debug, error, info};

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
        debug!("command spawned");

        let files_redirect: HashMap<String, String> = cfg
            .redirect
            .files
            .iter()
            .map(|redirect| (redirect.from.clone(), redirect.to.clone()))
            .collect();

        loop {
            match Tracee::wait(self.pid) {
                Ok(None) => {
                    debug!("tracee exited");
                    return Ok(());
                }
                Ok(Some((ref mut tracee, operation))) => match operation {
                    Operation::Open { ref path, .. } => {
                        // Maybe redirect the open syscall to a different file.
                        let absolute = match path.canonicalize() {
                            Ok(absolute) => absolute,
                            Err(_) => {
                                // TODO: use other function to normalize paths
                                // debug!("failed to canonicalize path: {:?}", path);
                                path.clone()
                            }
                        };

                        let absolute = absolute.to_string_lossy().to_string();
                        if let Some(dest) = files_redirect.get(&absolute) {
                            info!("redirecting open() from {} to {}", absolute, dest);

                            // Inject the new path into the tracee's memory.
                            // TODO: free the memory
                            let mem = tracee.write_string(dest)?;
                            operation.intercept(tracee, mem)?;
                        }

                        let result = tracee.get_result(&operation)?;

                        // Let the syscall run.
                        match result {
                            OperationResult::FileDescriptor(fd) => {
                                info!("open({}) = {}", path.to_string_lossy(), fd);
                            }
                            OperationResult::Error(errno) => {
                                info!("open({}) = {}", path.to_string_lossy(), errno);
                            }
                            e => error!(result = ?e, "unexpected result for open operation"),
                        }
                    }
                    Operation::Rand { len, .. } => {
                        if cfg.redirect.random {
                            info!("redirecting getrandom({})", len);
                            operation.intercept(tracee, 0)?;
                        }

                        let result = tracee.get_result(&operation)?;
                        match result {
                            OperationResult::NumBytes(num_bytes) => {
                                info!("getrandom({})", num_bytes);
                            }
                            OperationResult::Error(errno) => {
                                info!("getrandom({})", errno);
                            }
                            e => error!(result = ?e, "unexpected result for rand operation"),
                        }
                    }
                    op @ (Operation::Fork { .. } | Operation::Wait) => {
                        panic!("this operation type should not be returned here: {:?}", op);
                    }
                    Operation::Exit => {
                        return Ok(());
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
