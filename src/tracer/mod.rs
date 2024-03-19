mod operation;
mod tracee;

use crate::config::Config;
use operation::{Operation, OperationResult};
use std::{collections::HashMap, io::Result};
use tracee::Tracee;
use tracing::{debug, info};

pub fn spawn<I, S>(cmd: &str, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    Tracee::spawn(cmd, args)
}

pub fn run(cfg: &Config) -> Result<()> {
    debug!("command spawned");

    let files_redirect: HashMap<String, String> = cfg
        .redirect
        .files
        .iter()
        .map(|redirect| (redirect.from.clone(), redirect.to.clone()))
        .collect();

    loop {
        match Tracee::wait() {
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
                            info!("opened {} (fd {})", path.to_string_lossy(), fd);
                        }
                        OperationResult::Error(errno) => {
                            info!("opened {} ({})", path.to_string_lossy(), errno);
                        }
                    }
                }
                Operation::Exit => {
                    return Ok(());
                }
            },
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }
}
