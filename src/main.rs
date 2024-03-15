use std::os::unix::process::CommandExt;

use clap::Parser;
use nix::{
    errno::Errno,
    sys::{
        ptrace,
        wait::{wait, WaitStatus},
    },
    unistd::Pid,
};
use tracing::{info, span, Level};

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long)]
    debug: bool,
    #[arg(last = true)]
    cmd: Vec<String>,
}

fn main() {
    let args = Args::parse();

    // Init logging
    tracing_subscriber::fmt()
        .with_max_level(if args.debug {
            Level::DEBUG
        } else {
            Level::INFO
        })
        .init();

    let _span = span!(Level::DEBUG, "main").entered();
    info!(cmd = args.cmd.join(" "), "Will run command");

    if let Some(program) = args.cmd.first() {
        let mut cmd = std::process::Command::new(program);
        cmd.args(args.cmd.iter().skip(1));
        unsafe {
            cmd.pre_exec(|| {
                ptrace::traceme().expect("failed to ptrace traceme");
                Ok(())
            });
        }
        let child = cmd.spawn().expect("failed to execute process");
        let pid = Pid::from_raw(child.id() as i32);
        ptrace::setoptions(pid, ptrace::Options::PTRACE_O_TRACESYSGOOD).unwrap();
        info!(pid = pid.as_raw(), "spawned process");
        let mut is_return = false;
        let mut arg1 = 0;
        // let mut arg2 = 0;
        let mut path = String::new();
        // let mut arg3 = 0;
        // let mut arg4 = 0;
        // let mut arg5 = 0;
        // let mut arg6 = 0;

        loop {
            ptrace::syscall(pid, None).unwrap();

            match wait() {
                Ok(WaitStatus::Exited(_, code)) => {
                    assert_eq!(code, 0);
                    break;
                }
                Ok(WaitStatus::PtraceSyscall(_)) => {
                    let registers = ptrace::getregs(pid).unwrap();
                    match registers.orig_rax {
                        // open, openat, creat
                        2 | 257 | 85 => {
                            if is_return {
                                let retval = registers.rax as i64;
                                if retval < 0 {
                                    println!(
                                        "opened a file: {} / {} ({})",
                                        arg1 as i32,
                                        path,
                                        Errno::from_raw(-retval as i32)
                                    );
                                } else {
                                    println!(
                                        "opened a file: {} / {} (fd {})",
                                        arg1 as i32, path, retval
                                    );
                                }
                            } else {
                                arg1 = registers.rdi;
                                // arg2 = registers.rsi;
                                // arg3 = registers.rdx;
                                // arg4 = registers.r10;
                                // arg5 = registers.r8;
                                // arg6 = registers.r9;
                                path = read_string(pid, registers.rsi as u64).unwrap();
                            }
                            is_return = !is_return;
                        }
                        _ => {}
                    }
                }
                // TODO: support forking, etc...
                Ok(s) => panic!("unexpected stop reason: {:?}", s),
                Err(e) => panic!("unexpected waitpid error: {:?}", e),
            }
        }
        info!("command exited")
    }
}

fn _read_memory(pid: Pid, addr: u64, len: usize) -> std::io::Result<Vec<u8>> {
    use std::os::unix::fs::FileExt;

    let path = format!("/proc/{}/mem", pid.as_raw() as u32);

    let mut data = vec![0u8; len];
    let mem = std::fs::File::open(path)?;
    let len_read = mem.read_at(&mut data, addr)?;

    data.truncate(len_read);
    Ok(data)
}

fn read_string(pid: Pid, addr: u64) -> std::io::Result<String> {
    use std::os::unix::fs::FileExt;

    let path = format!("/proc/{}/mem", pid.as_raw() as u32);
    let mem = std::fs::File::open(path)?;
    let mut result = String::new();
    let mut cont = true;

    while cont {
        let mut data = vec![0u8; 4096];
        let len_read = mem.read_at(&mut data, addr)?;
        if len_read == 0 {
            break;
        }
        let null_byte = data.iter().position(|&x| x == 0);
        if let Some(null_byte) = null_byte {
            data.truncate(null_byte);
            cont = false;
        }
        result.push_str(
            String::from_utf8(data)
                .map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "read non-utf8 data")
                })?
                .as_str(),
        );
    }
    Ok(result)
}
