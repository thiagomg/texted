// STOPPED - https://ntex.rs/docs/handlers
// https://ntex.rs/docs/application

// TODO order:
// 1. Render html of the posts
// 2. Render markdown of the posts
// 3. Add comments and keep session state
// 4. Post list
// 5. Cache result of the markdown (into HTML)



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
