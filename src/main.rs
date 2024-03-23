use std::process::exit;

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

fn main() {
    let args = Args::parse();
    let conf = match config::load(&args.config_file) {
        Ok(conf) => conf,
        Err(e) => {
            println!("Error in configuration: {}", e);
            exit(1);
        }
    };
    // Init logging
    let level: Level = (&conf.log.level).into();
    tracing_subscriber::fmt().with_max_level(level).init();

    debug!("Configuration: {:#?}", conf);

    let _span = span!(Level::DEBUG, "main").entered();
    info!(cmd = args.cmd.join(" "), "Will run command");

    if let Some(program) = args.cmd.first() {
        let tracer = match Tracer::spawn(program, args.cmd.iter().skip(1)) {
            Ok(tracer) => tracer,
            Err(e) => {
                error!("couldn't spawn command: {}", e);
                exit(1)
            }
        };
        info!("command spawned");
        if let Err(e) = tracer.run(&conf) {
            error!("error during command execution: {}", e);
            exit(1)
        }
        info!("command exited")
    }
}
