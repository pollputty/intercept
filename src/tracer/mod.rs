mod tracee;

use crate::syscall::{Operation, OperationResult};
use nix::{
    sys::{
        ptrace,
        wait::{wait, WaitStatus},
    },
    unistd::Pid,
};
use std::io::Result;
use std::os::unix::process::CommandExt;
use tracee::Tracee;
use tracing::{debug, info};

pub fn spawn<I, S>(cmd: &str, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    // Parse the command line and spawn the child process in a stopped state.
    let mut cmd = std::process::Command::new(cmd);
    cmd.args(args);
    unsafe {
        cmd.pre_exec(|| {
            ptrace::traceme()?;
            Ok(())
        });
    }
    let child = cmd.spawn()?;

    // Configure the child process to stop on system calls and resume it.
    let pid = Pid::from_raw(child.id() as i32);
    ptrace::setoptions(pid, ptrace::Options::PTRACE_O_TRACESYSGOOD)?;
    ptrace::syscall(pid, None)?;
    Ok(())
}

pub fn run() -> Result<()> {
    debug!("command spawned");
    loop {
        match wait_syscall() {
            Ok(None) => {
                return Ok(());
            }
            Ok(Some((ref mut tracee, syscall))) => match syscall {
                Operation::Open { ref path } => {
                    let result = tracee.get_result(&syscall)?;
                    match result {
                        OperationResult::FileDescriptor(fd) => {
                            info!("opened {} (fd {})", path, fd);
                        }
                        OperationResult::Error(errno) => {
                            info!("opened {} ({})", path, errno);
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

fn wait_syscall() -> Result<Option<(Tracee, Operation)>> {
    loop {
        match wait() {
            Ok(WaitStatus::Exited(_, code)) => {
                // TODO: remove this assertion
                assert_eq!(code, 0);
                return Ok(None);
            }
            Ok(WaitStatus::PtraceSyscall(pid)) => {
                // A tracee is ready.
                let tracee = Tracee::new(pid);
                let syscall = tracee.parse_syscall()?;
                if let Some(syscall) = syscall {
                    return Ok(Some((tracee, syscall)));
                } else {
                    // Not supported syscall, keep going.
                    continue;
                }
            }
            // TODO: support forking, etc...
            Ok(s) => panic!("unexpected stop reason: {:?}", s),
            Err(e) => panic!("unexpected waitpid error: {:?}", e),
        }
    }
}
