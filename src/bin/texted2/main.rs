use std::env;
use texted2::config::{Config, read_config};
use texted2::server::server_run;


fn open_config() -> Config {
    let exe_path = env::current_exe().unwrap();
    let exe_dir = exe_path.parent().unwrap();
    println!("Current dir: {}", env::current_dir().unwrap().to_str().unwrap());
    let config = read_config(&exe_dir.join("../../texted2.toml")).unwrap();

    println!("Listening on {}:{}", config.server.address, config.server.port);

    config
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    let config = open_config();
    server_run(config).await
}
