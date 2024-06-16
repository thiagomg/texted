use std::collections::HashMap;
use std::ops::DerefMut;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use chrono::Duration;
use ntex::web;
use ntex::web::HttpRequest;
use ntex_files::NamedFile;
use spdlog::{debug, error, info};
use tokio::sync::mpsc::Sender;

use crate::config::Config;
use crate::content::Content;
use crate::content_cache::{ContentCache, Expire};
use crate::metrics::{MetricEvent, MetricHandler, MetricWriter};
use crate::post_list::PostListType;
use crate::post_processor::*;
use crate::util::toml_date::TomlDate;

struct AppState {
    /// Links of posts. E.g. my-blog.ca/view/my_post_url
    post_links: HashMap<String, PathBuf>,
    /// Links of posts. E.g. my-blog.ca/page/my_bio
    page_links: HashMap<String, PathBuf>,
    /// Texted configuration
    config: Config,
    /// Cache for post and page contents
    post_cache: ContentCache<String>,
    /// Cache for post and page summary, used in listing
    summary_cache: ContentCache<Content>,
    /// Sender to generate access metrics
    metric_sender: Option<Sender<MetricEvent>>,
}

// Begin: Redirect region --------
#[web::get("/view/{post}")]
async fn view_wo_slash(path: web::types::Path<String>) -> web::HttpResponse {
    web::HttpResponse::TemporaryRedirect()
        .header("Location", format!("{}/", path.into_inner()))
        .content_type("text/html; charset=utf-8")
        .finish()
}

#[web::get("/page/{post}")]
async fn page_wo_slash(path: web::types::Path<String>) -> web::HttpResponse {
    web::HttpResponse::TemporaryRedirect()
        .header("Location", format!("{}/", path.into_inner()))
        .content_type("text/html; charset=utf-8")
        .finish()
}
// End: Redirect region --------

#[web::get("/page/{page}/")]
async fn page(
    page_name: web::types::Path<String>,
    state: web::types::State<Arc<Mutex<AppState>>>,
) -> web::HttpResponse {
    let page_name = page_name.into_inner();

    let mut state_g = state.lock().unwrap();
    let state = state_g.deref_mut();
    let cache = &mut state.post_cache; //.lock().unwrap();
    let config = &state.config;
    let page_links = state.page_links.clone();

    let rendered_page = match cache.get_page_or(&page_name, Expire::Never, || {
        info!("Rendering page {} from file", page_name);
        open_content(&config, &page_links, "page.tpl", &page_name)
    }) {
        Ok(rendered_post) => {
            debug!("Retrieving page {} from cache", page_name);
            rendered_post
        }
        Err(e) => {
            return web::HttpResponse::BadRequest()
                .body(format!("Error loading page {}: {}", page_name, e));
        }
    }
        .to_string();

    web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(rendered_page)
}

#[web::get("/view/{post}/")]
async fn view(
    req: HttpRequest,
    post_name: web::types::Path<String>,
    state: web::types::State<Arc<Mutex<AppState>>>,
) -> web::HttpResponse {
    let post_name = post_name.into_inner();

    let mut state_g = state.lock().unwrap();
    let state = state_g.deref_mut();

    let origin: String = get_origin(&req);
    if let Some(ref metric) = state.metric_sender {
        match metric
            .send(MetricEvent {
                post_name: post_name.clone(),
                origin,
            })
            .await
        {
            Ok(_) => {}
            Err(e) => error!("Error writing metrics: {}", e),
        };
    }

    let cache = &mut state.post_cache; //.lock().unwrap();
    let config = &state.config;
    let post_links = state.post_links.clone();

    let rendered_post = match cache.get_post_or(&post_name, Expire::Never, || {
        info!("Rendering post {} from file", post_name);
        open_content(&config, &post_links, "view.tpl", &post_name)
    }) {
        Ok(rendered_post) => {
            debug!("Retrieving post {} from cache", post_name);
            rendered_post
        }
        Err(e) => {
            return web::HttpResponse::BadRequest()
                .body(format!("Error loading post {}: {}", post_name, e));
        }
    }
        .to_string();

    web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(rendered_post)
}

#[web::get("/list")]
async fn list(
    req: HttpRequest,
    state: web::types::State<Arc<Mutex<AppState>>>,
) -> web::HttpResponse {
    let mut state_g = state.lock().unwrap();
    let state = state_g.deref_mut();
    let mut cache = &mut state.summary_cache; //.lock().unwrap();
    let config = &state.config;
    let post_links = state.post_links.clone();

    let preview_opt = get_preview_option(&config);
    let rendered_posts = match retrieve_post_list(&mut cache, &post_links, None, &preview_opt) {
        Ok(posts) => posts,
        Err(e) => {
            return web::HttpResponse::InternalServerError()
                .body(format!("Error listing posts: {}", e))
        }
    };

    let cur_page: u32 = get_cur_page(req);
    let post_list = match render_list(&config, rendered_posts, cur_page) {
        Ok(posts) => posts,
        Err(e) => {
            return web::HttpResponse::InternalServerError()
                .body(format!("Error rendering post list: {}", e))
        }
    };

    web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(post_list)
}

#[web::get("/list/{tag}/")]
async fn list_with_tags(
    req: HttpRequest,
    path: web::types::Path<String>,
    state: web::types::State<Arc<Mutex<AppState>>>,
) -> web::HttpResponse {
    let tag = path.into_inner();

    let mut state_g = state.lock().unwrap();
    let state = state_g.deref_mut();
    let mut cache = &mut state.summary_cache; //.lock().unwrap();
    let config = &state.config;
    let post_links = state.post_links.clone();

    let preview_opt = get_preview_option(&config);
    let rendered_posts = match retrieve_post_list(&mut cache, &post_links, Some(tag), &preview_opt)
    {
        Ok(posts) => posts,
        Err(e) => {
            return web::HttpResponse::InternalServerError()
                .body(format!("Error listing posts: {}", e))
        }
    };

    let cur_page: u32 = get_cur_page(req);
    let post_list = match render_list(&config, rendered_posts, cur_page) {
        Ok(posts) => posts,
        Err(e) => {
            return web::HttpResponse::InternalServerError()
                .body(format!("Error rendering post list: {}", e))
        }
    };

    web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(post_list)
}

#[web::get("/view/{post}/{file}")]
async fn post_files(
    path: web::types::Path<(String, String)>,
    state: web::types::State<Arc<Mutex<AppState>>>,
) -> Result<NamedFile, web::Error> {
    let (post, file) = path.into_inner();
    let state = state.lock().unwrap();
    get_file(&state.config.paths.posts_dir, post, file)
}

#[web::get("/page/{post}/{file}")]
async fn page_files(
    path: web::types::Path<(String, String)>,
    state: web::types::State<Arc<Mutex<AppState>>>,
) -> Result<NamedFile, web::Error> {
    let (post, file) = path.into_inner();
    let state = state.lock().unwrap();
    get_file(&state.config.paths.pages_dir, post, file)
}

#[web::get("/public/{file_name}")]
async fn public_files(
    path: web::types::Path<String>,
    state: web::types::State<Arc<Mutex<AppState>>>,
) -> Result<NamedFile, web::Error> {
    if path.contains("../") {
        return Err(web::error::ErrorUnauthorized("Access forbidden").into());
    }

    let state = state.lock().unwrap();
    let file_path = state.config.paths.public_dir.join(path.into_inner());

    Ok(NamedFile::open(file_path)?)
}

#[web::get("/")]
async fn index(
    req: web::HttpRequest,
    state: web::types::State<Arc<Mutex<AppState>>>,
) -> web::HttpResponse {
    let mut state_g = state.lock().unwrap();
    let state = state_g.deref_mut();

    let cache = &mut state.post_cache; //.lock().unwrap();
    let config = &state.config;
    let page_name = "-index-page";

    let rendered_page =
        match cache.get_page_or(&page_name, Expire::After(Duration::days(1)), || {
            info!("Rendering page {} from file", page_name);
            let TomlDate(blog_start_date) = state.config.personal.blog_start_date;
            let activity_start_year = state.config.personal.activity_start_year;
            render_index(
                req,
                state.post_links.len(),
                &config.paths.template_dir,
                activity_start_year,
                blog_start_date,
            )
        }) {
            Ok(rendered_post) => {
                debug!("Retrieving page {} from cache", page_name);
                rendered_post
            }
            Err(e) => {
                return web::HttpResponse::BadRequest()
                    .body(format!("Error loading index page {}: {}", page_name, e));
            }
        }
            .to_string();

    web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(rendered_page)
}

fn get_origin(req: &web::HttpRequest) -> String {
    if let Some(header) = req.headers().get("X-Forwarded-For") {
        if let Ok(addr) = header.to_str() {
            return addr.to_string();
        }
    }

    req.peer_addr().map_or("".to_string(), |x| format!("{}", x))
}

pub async fn server_run(config: Config) -> Result<()> {
    let index_base_name = match config.defaults.index_base_name {
        None => PostListType::AnyContentFile,
        Some(ref base_name) => PostListType::IndexBaseName(base_name.clone()),
    };

    // List post files and generate list of link -> post file
    let post_link_vec: Vec<PostLink> = list_post_files(&config.paths.posts_dir, &index_base_name)?;
    for file in post_link_vec.iter() {
        info!("Post added to listing: {:?}", file.post_name);
    }

    let page_link_vec: Vec<PostLink> = list_post_files(&config.paths.pages_dir, &index_base_name)?;
    for file in page_link_vec.iter() {
        info!("Page found: {:?}", file.post_name);
    }

    let post_links: HashMap<_, _> = post_link_vec
        .into_iter()
        .map(|link| (link.post_name, link.post_path))
        .collect();

    let page_links: HashMap<_, _> = page_link_vec
        .into_iter()
        .map(|link| (link.post_name, link.post_path))
        .collect();
    
    let (post_cache, content_cache) = match config.defaults.rendering_cache_enabled {
        true => (
            ContentCache::new(),
            ContentCache::new(),
        ),
        false => (
            ContentCache::non_caching(),
            ContentCache::non_caching(),
        ),
    };

    let (metric_sender, _metrics) = if let Some(ref metrics_cfg) = config.metrics {
        // When configuration is loaded, we already set a location if the metrics section is defined
        let location = metrics_cfg.location.as_ref().unwrap();
        let metrics = MetricWriter::new(&location)?;
        let metric_handler = MetricHandler::new(metrics);
        let sender = metric_handler.new_sender();
        (Some(sender), Some(metric_handler))
    } else {
        (None, None)
    };

    let bind_addr = config.server.address.clone();
    let bind_port = config.server.port;
    let app_state = Arc::new(Mutex::new(AppState {
        post_links,
        page_links,
        config,
        post_cache,
        summary_cache: content_cache,
        metric_sender,
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
        .map_err(anyhow::Error::from)
}
