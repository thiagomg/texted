use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use spdlog::{info, warn};

use texted::logger::configure_logger;
use texted::server::server_run;

use crate::config::open_config;

mod config;

const CFG_FILE_NAME: &str = "texted.toml";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Config path
    #[arg(short, long)]
    config_path: Option<String>,
}

#[ntex::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config_path = args.config_path.map(PathBuf::from);

    let config = match open_config(config_path) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{}", err);
            eprintln!("Please run texted --help");
            return Ok(());
        }
    };

    if let Err(err) = configure_logger(&config) {
        warn!("Error creating logger sinks. Using console instead. Desc={}", err);
    }

    info!("Starting Texted =-=-=-=-=-=-=-=-=-=-=-=-=-=-=-");
    info!("Listening on {}:{}", config.server.address, config.server.port);

    server_run(config).await
}
