use intercept::tracer;

use clap::Parser;
use tracing::{error, info, span, Level};

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long)]
    debug: bool,
    #[arg(last = true)]
    cmd: Vec<String>,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // Init logging
    tracing_subscriber::fmt()
        .with_max_level(if args.debug {
            Level::DEBUG
        } else {
            Level::WARN
        })
        .init();

    let _span = span!(Level::DEBUG, "main").entered();
    info!(cmd = args.cmd.join(" "), "Will run command");

    if let Some(program) = args.cmd.first() {
        tracer::spawn(program, args.cmd.iter().skip(1)).inspect_err(|e| error!("{}", e))?;
        info!("command spawned");
        tracer::run().inspect_err(|e| error!("Error: {}", e))?;
        info!("command exited")
    }
    Ok(())
}
