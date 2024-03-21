use std::io;
use std::sync::{Arc, Mutex};

use ntex::web;
use ntex::web::HttpRequest;
use ntex_files::NamedFile;
use crate::config::Config;
use crate::post_cache::PostCache;
use crate::post_processor::*;
use crate::query_string::QueryString;

struct AppState {
    activity_start_year: i32,
    posts: PostCache,
    pages: PostCache,
    config: Config,
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
async fn page(path: web::types::Path<String>, state: web::types::State<Arc<Mutex<AppState>>>) -> web::HttpResponse {
    let state = state.lock().unwrap();
    let template_dir = &state.config.paths.template_dir;
    let posts: &PostCache = &state.pages;
    let path = path.into_inner();

    match process_post(path, template_dir, "page.tpl", posts) {
        Ok(value) => value,
        Err(value) => return value,
    }
}

#[web::get("/view/{post}/")]
async fn view(path: web::types::Path<String>, state: web::types::State<Arc<Mutex<AppState>>>) -> web::HttpResponse {
    let state = state.lock().unwrap();
    let template_dir = &state.config.paths.template_dir;
    let posts: &PostCache = &state.posts;
    let path = path.into_inner();

    match process_post(path, template_dir, "view.tpl", posts) {
        Ok(value) => value,
        Err(value) => return value,
    }
}

#[web::get("/list")]
async fn list(req: HttpRequest, state: web::types::State<Arc<Mutex<AppState>>>) -> web::HttpResponse {
    let state = &state.lock().unwrap();
    let tpl_dir = &state.config.paths.template_dir;
    let cache = &state.posts;

    // TODO - Thiago: Make page_size configurable
    let page_size: u32 = 10;
    let cur_page: u32 = if let Some(query_str) = req.uri().query() {
        let qs = QueryString::from(query_str);
        qs.get_page()
    } else {
        1
    };

    let response = match list_posts(tpl_dir, cache, None, cur_page, page_size) {
        Ok(body) => web::HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(body),
        Err(error) => return web::HttpResponse::InternalServerError()
            .body(error),
    };

    response
}

#[web::get("/list/{tag}/")]
async fn list_with_tags(path: web::types::Path<String>, state: web::types::State<Arc<Mutex<AppState>>>) -> web::HttpResponse {
    let state = &state.lock().unwrap();
    let tpl_dir = &state.config.paths.template_dir;
    let cache = &state.posts;
    let tag = path.into_inner();

    // For now, we don't support pagination when tags are used
    let response = match list_posts(tpl_dir, cache, Some(tag), 1, cache.posts().len() as u32) {
        Ok(body) => web::HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(body),
        Err(error) => return web::HttpResponse::InternalServerError()
            .body(error),
    };

    response
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
    let cache = &state.posts;
    let tpl_dir = &state.config.paths.template_dir;
    let activity_start_year = state.activity_start_year;


    let response = match render_index(req, &cache, tpl_dir, activity_start_year) {
        Ok(rendered) => web::HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(rendered),
        Err(error) => return web::HttpResponse::InternalServerError()
            .body(error),
    };

    response
}

pub async fn server_run(config: Config) -> io::Result<()> {
    let md_posts = match get_posts(&config.paths.posts_dir, config.defaults.index_file_name.as_str()) {
        Ok(posts) => posts,
        Err(err) => {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Error retrieving post list template: {}. Dir={}", err, config.paths.posts_dir.to_str().unwrap())));
        }
    };

    let md_pages = match get_posts(&config.paths.pages_dir, config.defaults.index_file_name.as_str()) {
        Ok(posts) => posts,
        Err(err) => {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Error retrieving post list template: {}. Dir={}", err, config.paths.posts_dir.to_str().unwrap())));
        }
    };


    let bind_addr = config.server.address.clone();
    let bind_port = config.server.port;
    let app_state = Arc::new(Mutex::new(AppState {
        activity_start_year: config.personal.activity_start_year,
        posts: PostCache::new(config.defaults.index_file_name.as_str()),
        pages: PostCache::new(config.defaults.index_file_name.as_str()),
        config,
    }));

    {
        let state = &mut app_state.lock().unwrap();
        let post_cache = &mut state.posts;
        for post in md_posts {
            post_cache.add(post)?;
        }
        post_cache.sort();

        let page_cache = &mut state.pages;
        for post in md_pages {
            page_cache.add(post)?;
        }
        page_cache.sort();
    }

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
