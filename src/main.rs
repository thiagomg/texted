use std::env;
use crate::config::{Config, read_config};
use crate::server::server_run;

mod server;
mod post_list;
mod post;
mod test_data;
mod post_cache;
mod post_render;
mod text_utils;
mod config;

fn open_config() -> Config {
    let exe_path = env::current_exe().unwrap();
    let exe_dir = exe_path.parent().unwrap();
    println!("cur_dir: {}", env::current_dir().unwrap().to_str().unwrap());
    read_config(&exe_dir.join("texted.toml")).unwrap()
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    let config = open_config();
    server_run(config).await
}
