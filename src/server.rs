use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use ntex::web;
use ntex::web::HttpRequest;
use ntex_files::NamedFile;

use crate::config::Config;
use crate::content_cache::ContentCache;
use crate::post_processor::*;
use crate::util::toml_date::TomlDate;

struct AppState {
    post_links: Arc<HashMap<String, PathBuf>>,
    page_links: Arc<HashMap<String, PathBuf>>,
    config: Arc<Config>,
    cache: Arc<Mutex<ContentCache>>,
}

// Begin: Redirect region --------
#[web::get("/view/{post}")]
async fn view_wo_slash(path: web::types::Path<String>) -> web::HttpResponse {
    web::HttpResponse::TemporaryRedirect()
        .header("Location", path.into_inner() + "/")
        .content_type("text/html; charset=utf-8")
        .finish()
}

#[web::get("/page/{post}")]
async fn page_wo_slash(path: web::types::Path<String>) -> web::HttpResponse {
    web::HttpResponse::TemporaryRedirect()
        .header("Location", path.into_inner() + "/")
        .content_type("text/html; charset=utf-8")
        .finish()
}
// End: Redirect region --------

#[web::get("/page/{page}/")]
async fn page(page_name: web::types::Path<String>, state: web::types::State<Arc<Mutex<AppState>>>) -> web::HttpResponse {
    let page_name = page_name.into_inner();

    let state = state.lock().unwrap();
    let mut cache = state.cache.lock().unwrap();
    let config = state.config.clone();
    let page_links = state.page_links.clone();

    let rendered_page = match cache.get_page_or(&page_name, || {
        println!("Rendering page {} from cache", page_name);
        open_content(&config, &page_links, "page.tpl", &page_name)
    }) {
        Ok(rendered_post) => {
            println!("Retrieving page {} from cache", page_name);
            rendered_post
        }
        Err(e) => {
            return web::HttpResponse::BadRequest()
                .body(format!("Error loading page {}: {}", page_name, e));
        }
    }.to_string();

    web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(rendered_page)
}

#[web::get("/view/{post}/")]
async fn view(post_name: web::types::Path<String>, state: web::types::State<Arc<Mutex<AppState>>>) -> web::HttpResponse {
    let post_name = post_name.into_inner();

    let state = state.lock().unwrap();
    let mut cache = state.cache.lock().unwrap();
    let config = state.config.clone();
    let post_links = state.post_links.clone();

    let rendered_post = match cache.get_post_or(&post_name, || {
        println!("Rendering post {} from cache", post_name);
        open_content(&config, &post_links, "view.tpl", &post_name)
    }) {
        Ok(rendered_post) => {
            println!("Retrieving post {} from cache", post_name);
            rendered_post
        }
        Err(e) => {
            return web::HttpResponse::BadRequest()
                .body(format!("Error loading post {}: {}", post_name, e));
        }
    }.to_string();

    web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(rendered_post)
}

#[web::get("/list")]
async fn list(req: HttpRequest, state: web::types::State<Arc<Mutex<AppState>>>) -> web::HttpResponse {
    let state = &state.lock().unwrap();
    let config = &state.config;

    let cur_page: u32 = get_cur_page(req);
    let post_list = match render_list(&state.post_links, &config, cur_page, None) {
        Ok(posts) => posts,
        Err(e) => return web::HttpResponse::InternalServerError()
            .body(format!("Error listing posts: {}", e)),
    };

    web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(post_list)
}

#[web::get("/list/{tag}/")]
async fn list_with_tags(req: HttpRequest, path: web::types::Path<String>, state: web::types::State<Arc<Mutex<AppState>>>) -> web::HttpResponse {
    let state = &state.lock().unwrap();
    let config = &state.config;
    let tag = path.into_inner();

    let cur_page: u32 = get_cur_page(req);
    let post_list = match render_list(&state.post_links, &config, cur_page, Some(tag)) {
        Ok(posts) => posts,
        Err(e) => return web::HttpResponse::InternalServerError()
            .body(format!("Error listing posts: {}", e)),
    };

    web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(post_list)
}

#[web::get("/view/{post}/{file}")]
async fn post_files(path: web::types::Path<(String, String)>, state: web::types::State<Arc<Mutex<AppState>>>) -> Result<NamedFile, web::Error> {
    let (post, file) = path.into_inner();
    let state = state.lock().unwrap();
    get_file(&state.config.paths.posts_dir, post, file)
}

#[web::get("/page/{post}/{file}")]
async fn page_files(path: web::types::Path<(String, String)>, state: web::types::State<Arc<Mutex<AppState>>>) -> Result<NamedFile, web::Error> {
    let (post, file) = path.into_inner();
    let state = state.lock().unwrap();
    get_file(&state.config.paths.pages_dir, post, file)
}

#[web::get("/public/{file_name}")]
async fn public_files(path: web::types::Path<String>, state: web::types::State<Arc<Mutex<AppState>>>) -> Result<NamedFile, web::Error> {
    if path.contains("../") {
        return Err(web::error::ErrorUnauthorized("Access forbidden").into());
    }

    let state = state.lock().unwrap();
    let file_path = state.config.paths.public_dir.join(path.into_inner());

    Ok(NamedFile::open(file_path)?)
}

#[web::get("/")]
async fn index(req: web::HttpRequest, state: web::types::State<Arc<Mutex<AppState>>>) -> web::HttpResponse {
    let state = state.lock().unwrap();
    let mut cache = state.cache.lock().unwrap();
    let config = state.config.clone();
    let page_name = "-index-page";

    let rendered_page = match cache.get_page_or(&page_name, || {
        println!("Rendering page {} from cache", page_name);
        let TomlDate(blog_start_date) = state.config.personal.blog_start_date;
        let activity_start_year = state.config.personal.activity_start_year;
        render_index(req, state.post_links.len(), &config.paths.template_dir, activity_start_year, blog_start_date)
    }) {
        Ok(rendered_post) => {
            println!("Retrieving page {} from cache", page_name);
            rendered_post
        }
        Err(e) => {
            return web::HttpResponse::BadRequest()
                .body(format!("Error loading index page {}: {}", page_name, e));
        }
    }.to_string();

    web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(rendered_page)
}

pub async fn server_run(config: Config) -> io::Result<()> {
    // List post files and generate list of link -> post file
    let post_link_vec: Vec<PostLink> = list_post_files(&config.paths.posts_dir, config.defaults.index_base_name.as_str())?;
    for file in post_link_vec.iter() {
        println!("Post: {:?}", file.post_name);
    }

    let page_link_vec: Vec<PostLink> = list_post_files(&config.paths.pages_dir, config.defaults.index_base_name.as_str())?;
    for file in page_link_vec.iter() {
        println!("Page: {:?}", file.post_name);
    }

    let post_links: HashMap<_, _> = post_link_vec.into_iter()
        .map(|link| { (link.post_name, link.post_path) })
        .collect();
    let post_links = Arc::new(post_links);

    let page_links: HashMap<_, _> = page_link_vec.into_iter()
        .map(|link| { (link.post_name, link.post_path) })
        .collect();
    let page_links = Arc::new(page_links);

    let config = Arc::new(config);
    let cache = Arc::new(Mutex::new(ContentCache::new()));

    // TODO: Remove this get_posts
    let mut index_file_name = config.defaults.index_base_name.as_str().to_string();
    index_file_name += ".md";

    let bind_addr = config.server.address.clone();
    let bind_port = config.server.port;
    let app_state = Arc::new(Mutex::new(AppState {
        post_links,
        page_links,
        config,
        cache,
    }));

    web::HttpServer::new(move || {
        web::App::new()
            .state(app_state.clone())
            .service(index)
            .service(public_files)
            .service(list)
            .service(list_with_tags)
            .service(view)
            .service(view_wo_slash)
            .service(post_files)
            .service(page)
            .service(page_wo_slash)
            .service(page_files)
    })
        .bind((bind_addr, bind_port))?
        .run()
        .await
}
