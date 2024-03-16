use std::{
    io::{Error, ErrorKind, Result},
    path::PathBuf,
};

use nix::errno::Errno;

use crate::syscall::SysNum;

pub enum Operation {
    Open { num: SysNum, path: PathBuf },
    Exit,
}

pub enum OperationResult {
    FileDescriptor(i32),
    Error(Errno),
}

impl Operation {
    pub fn intercept(
        &self,
        tracee: &mut crate::tracer::tracee::Tracee,
        address: u64,
    ) -> Result<()> {
        match self {
            Operation::Open { num, .. } => {
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

                tracee.set_arg(arg, address)?;
                Ok(())
            }
            Operation::Exit => Err(Error::new(
                ErrorKind::Other,
                "cannot intercept an exit syscall",
            )),
        }
    }
}
