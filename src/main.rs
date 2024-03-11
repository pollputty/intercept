use clap::Parser;
use tracing::{debug, span, Level};

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
    debug!(cmd = args.cmd.join(" "), "Will run command");

    if let Some(program) = args.cmd.first() {
        let mut cmd = std::process::Command::new(program);
        cmd.args(args.cmd.iter().skip(1));
        cmd.status().expect("failed to execute process");
    }
}
