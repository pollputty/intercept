mod operation;
mod tracee;

use nix::{
    errno::Errno,
    sys::{
        ptrace,
        wait::{wait, WaitStatus},
    },
    unistd::Pid,
};
use operation::{Operation, OperationResult};
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
    let opts = ptrace::Options::PTRACE_O_TRACESYSGOOD | ptrace::Options::PTRACE_O_EXITKILL;
    ptrace::setoptions(pid, opts)?;
    ptrace::syscall(pid, None)?;
    Ok(())
}

pub fn run() -> Result<()> {
    debug!("command spawned");

    let src_path = "/home/paul/test.txt";
    let dst_path = "/home/paul/evil.txt";

    loop {
        match wait_syscall() {
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

                    let absolute = absolute.to_string_lossy();
                    if absolute == src_path {
                        info!("redirecting open() from {} to {}", absolute, dst_path);

                        // Inject the new path into the tracee's memory.
                        // TODO: free the memory
                        let mem = tracee.write_string(dst_path)?;
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
                let mut tracee = Tracee::new(pid);
                // TODO: remove this assertion
                assert_eq!(
                    Errno::ENOSYS,
                    Errno::from_raw(-(tracee.get_registers()?.rax as i32))
                );
                let syscall = tracee.parse_syscall()?;
                if let Some(syscall) = syscall {
                    return Ok(Some((tracee, syscall)));
                } else {
                    // Syscall not supported, keep going.
                    continue;
                }
            }
            // TODO: support forking, etc...
            Ok(s) => panic!("unexpected stop reason: {:?}", s),
            Err(e) => panic!("unexpected waitpid error: {:?}", e),
        }
    }
}
