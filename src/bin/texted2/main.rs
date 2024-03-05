mod config_data;
mod config;

use std::path::PathBuf;
use clap::Parser;
use texted2::server::server_run;
use crate::config::{generate_cfg, open_config};

const CFG_FILE_NAME: &str = "texted2.toml";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Config path
    #[arg(short, long)]
    config_path: Option<String>,

    /// Generates texted2 config. Use with -c to specify the location
    #[arg(long)]
    generate_cfg: bool,
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let config_path = args.config_path.and_then(|x| Some(PathBuf::from(x)));

    if args.generate_cfg {
        let _ = generate_cfg(&config_path);
        return Ok(());
    }

    let config = match open_config(config_path) {
        Ok(config) => config,
        Err(err) => {
            println!("{}", err);
            println!("Please run texted2 --help");
            return Ok(());
        }
    };
    server_run(config).await
}
