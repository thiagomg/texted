use std::env;
use std::path::PathBuf;

use texted::config::{Config, read_config};

use crate::CFG_FILE_NAME;

fn get_config_path() -> Option<PathBuf> {
    let exe_path = env::current_exe().unwrap();
    let exe_dir = exe_path.parent().unwrap();
    let cur_dir = env::current_dir().unwrap();

    if exe_dir.join(CFG_FILE_NAME).exists() {
        return Some(exe_dir.join(CFG_FILE_NAME));
    }

    if cur_dir.join(CFG_FILE_NAME).exists() {
        return Some(cur_dir.join(CFG_FILE_NAME));
    }

    let cfg_dir = dirs::config_dir().expect("Could not find user config dir");
    if cfg_dir.join(CFG_FILE_NAME).exists() {
        return Some(cfg_dir.join(CFG_FILE_NAME));
    }

    None
}

pub(crate) fn open_config(cfg_path: Option<PathBuf>) -> Result<Config, String> {
    let config_path = cfg_path.unwrap_or(match get_config_path() {
        None => return Err("Could not find Texted configuration".to_string()),
        Some(x) => x,
    });

    println!("Current dir: {}", env::current_dir().unwrap().to_str().unwrap());
    println!("Reading config from {}", config_path.to_str().unwrap());
    let mut config = match read_config(&config_path) {
        Ok(config) => config,
        Err(e) => return Err(e.to_string()),
    };

    if let Some(mut log) = config.log {
        let location = log.location.unwrap_or_else(|| {
            dirs::cache_dir().unwrap().join("Texted").join("log").join("server.log")
        });
        log.location = Some(location);
        println!("Log enabled. Files will be written in {}", log.location.as_ref().unwrap().to_str().unwrap());
        config.log = Some(log);
    } else {
        println!("Log disabled. Using stdout");
    }

    Ok(config)
}
