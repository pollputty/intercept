use intercept::config;
use intercept::tracer::Tracer;

use clap::Parser;
use tracing::{debug, error, info, span, Level};

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short('f'), long, default_value("intercept.yaml"))]
    config_file: String,
    #[arg(last = true)]
    cmd: Vec<String>,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let conf = config::load(&args.config_file)?;

    // Init logging
    let level: Level = (&conf.log.level).into();
    tracing_subscriber::fmt().with_max_level(level).init();

    debug!("Configuration: {:#?}", conf);

    let _span = span!(Level::DEBUG, "main").entered();
    info!(cmd = args.cmd.join(" "), "Will run command");

    if let Some(program) = args.cmd.first() {
        let tracer = Tracer::spawn(program, args.cmd.iter().skip(1)).inspect_err(|e| error!("{}", e))?;
        info!("command spawned");
        tracer.run(&conf).inspect_err(|e| error!("Error: {}", e))?;
        info!("command exited")
    }
    Ok(())
}
