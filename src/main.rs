use crate::server::server_run;

mod server;
mod post_list;
mod post;
mod test_data;
mod post_cache;
mod post_render;
mod text_utils;

#[ntex::main]
async fn main() -> std::io::Result<()> {
    server_run().await
}
