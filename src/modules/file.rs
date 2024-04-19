use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
    path::Path,
};

use tracing::info;

use crate::{
    recorder::FileRecord,
    tracer::{OperationResult, Tracee},
    SysNum,
};

pub struct FileManager {
    redirects: HashMap<String, String>,
}

impl FileManager {
    pub fn new(redirects: HashMap<String, String>) -> Self {
        FileManager { redirects }
    }

    pub fn process(
        &self,
        tracee: &mut Tracee,
        path: &Path,
        num: SysNum,
        read: bool,
        write: bool,
    ) -> Result<FileRecord> {
        // Maybe redirect the open syscall to a different file.
        let absolute = match path.canonicalize() {
            Ok(absolute) => absolute,
            Err(_) => {
                // TODO: use other function to normalize paths
                // debug!("failed to canonicalize path: {:?}", path);
                path.to_path_buf()
            }
        };

        let absolute = absolute.to_string_lossy().to_string();
        if let Some(dest) = self.redirects.get(&absolute) {
            info!("redirecting open() from {} to {}", absolute, dest);

            // Inject the new path into the tracee's memory.
            self.redirect(tracee, dest, num)?;
        }

        let result = tracee.get_result()?;
        let success = match result {
            OperationResult::Success(_) => {
                info!("open({})", path.display());
                true
            }
            OperationResult::Error(errno) => {
                info!("open({}): {}", path.display(), errno);
                false
            }
        };

        // Let the syscall run.
        let record = FileRecord {
            path: path.to_string_lossy().to_string(),
            success,
            read,
            write,
        };
        Ok(record)
    }

    fn redirect(&self, tracee: &mut Tracee, dest: &str, num: SysNum) -> Result<()> {
        let mem = tracee.write_string(dest)?;

        let arg = match num {
            SysNum::Open => 1,
            SysNum::OpenAt => 2,
            _ => {
                return Err(Error::new(
                    ErrorKind::Other,
                    "invalid sysnum in Open operation",
                ))
            }
        };
        tracee.set_arg(arg, mem)?;
        Ok(())
    }
}
