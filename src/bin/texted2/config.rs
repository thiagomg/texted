use std::env;
use std::path::PathBuf;
use texted2::config::{Config, read_config};
use crate::CFG_FILE_NAME;
use crate::config_data::write_sample_cfg;

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
        None => return Err("Could not find Texted2 configuration".to_string()),
        Some(x) => x,
    });
    println!("Current dir: {}", env::current_dir().unwrap().to_str().unwrap());
    println!("Reading config from {}", config_path.to_str().unwrap());
    let config = read_config(&config_path).unwrap();

    println!("Listening on {}:{}", config.server.address, config.server.port);

    Ok(config)
}

pub(crate) fn generate_cfg(config_path: &Option<PathBuf>) -> PathBuf {
    let path: PathBuf = if let Some(ref path) = config_path {
        path.clone()
    } else {
        let cfg_dir = dirs::config_dir().expect("Could not find user config dir");
        cfg_dir.join(CFG_FILE_NAME)
    };

    println!("Writing sample config to {}", path.to_str().unwrap());
    write_sample_cfg(&path);

    path
}