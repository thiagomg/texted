use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::config::Config;
use crate::content::Content;
use crate::content_cache::{ContentCache, Expire};
use crate::metrics::metric_handler::MetricHandler;
use crate::metrics::metric_sender::MetricSender;
use crate::metrics::metric_writer::MetricWriter;
use crate::post_list::PostListType;
use crate::post_processor::*;
use crate::util::toml_date::TomlDate;
use anyhow::Result;
use chrono::Duration;
use ntex::web;
use ntex::web::HttpRequest;
use ntex_files::NamedFile;
use spdlog::{debug, info};

struct AppState {
    /// Links of posts. E.g. my-blog.ca/view/my_post_url
    post_links: RwLock<HashMap<String, PathBuf>>,
    /// Links of posts. E.g. my-blog.ca/page/my_bio
    page_links: RwLock<HashMap<String, PathBuf>>,
    /// Texted configuration
    config: RwLock<Config>,
    /// Cache for post and page contents
    post_cache: RwLock<ContentCache<String>>,
    /// Cache for post and page summary, used in listing
    summary_cache: RwLock<ContentCache<Content>>,
    /// Sender to generate access metrics
    metric_sender: MetricSender,
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
    req: HttpRequest,
    page_name: web::types::Path<String>,
    app_state: web::types::State<Arc<AppState>>,
) -> web::HttpResponse {
    let page_name = page_name.into_inner();
    let origin: String = get_origin(&req);
    app_state
        .metric_sender
        .page(page_name.clone(), origin)
        .await;

    let read_cache = app_state.post_cache.read().unwrap();
    let rendered_page = match read_cache.get_page(&page_name) {
        None => {
            // Let's load and update the cache
            drop(read_cache);
            let mut write_cache = app_state.post_cache.write().unwrap();
            info!("Rendering page {} from file", page_name);
            let config = &app_state.config.read().unwrap();
            let page_links = &app_state.page_links.read().unwrap();
            let content = match open_content(config, page_links, "page.tpl", &page_name) {
                Ok(content) => content,
                Err(e) => {
                    return web::HttpResponse::BadRequest()
                        .body(format!("Error loading page {}: {}", &page_name, e));
                }
            };
            write_cache.add_page(&page_name, content, Expire::Never)
        }
        Some(content) => {
            debug!("Returning cached page for {}", &page_name);
            content
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
    app_state: web::types::State<Arc<AppState>>,
) -> web::HttpResponse {
    let post_name = post_name.into_inner();

    let origin: String = get_origin(&req);
    app_state
        .metric_sender
        .view(post_name.clone(), origin)
        .await;

    let read_cache = app_state.post_cache.read().unwrap();
    let rendered_post = match read_cache.get_post(&post_name) {
        None => {
            // Let's load and update the cache
            drop(read_cache);
            let mut write_cache = app_state.post_cache.write().unwrap();
            info!("Rendering post {} from file", post_name);
            let config = &app_state.config.read().unwrap();
            let post_links = &app_state.post_links.read().unwrap();
            let content = match open_content(config, post_links, "view.tpl", &post_name) {
                Ok(content) => content,
                Err(e) => {
                    return web::HttpResponse::BadRequest()
                        .body(format!("Error loading post {}: {}", &post_name, e));
                }
            };
            write_cache.add_post(&post_name, content, Expire::Never)
        }
        Some(content) => {
            debug!("Returning cached post for {}", &post_name);
            content
        }
    }
    .to_string();

    web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(rendered_post)
}

#[web::get("/list")]
async fn list(req: HttpRequest, app_state: web::types::State<Arc<AppState>>) -> web::HttpResponse {
    let origin: String = get_origin(&req);
    app_state.metric_sender.list(None, origin).await;

    let config = app_state.config.read().unwrap();
    let preview_opt = get_preview_option(&config);
    let post_links = app_state.post_links.read().unwrap();

    let rendered_posts =
        match retrieve_post_list(&app_state.summary_cache, &post_links, None, &preview_opt) {
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
    app_state: web::types::State<Arc<AppState>>,
) -> web::HttpResponse {
    let tag = path.into_inner();

    let origin: String = get_origin(&req);
    app_state
        .metric_sender
        .list(Some(tag.clone()), origin)
        .await;

    let config = app_state.config.read().unwrap();
    let preview_opt = get_preview_option(&config);
    let post_links = app_state.post_links.read().unwrap();

    let rendered_posts = match retrieve_post_list(
        &app_state.summary_cache,
        &post_links,
        Some(tag),
        &preview_opt,
    ) {
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

#[web::get("/rss")]
async fn rss(req: HttpRequest, app_state: web::types::State<Arc<AppState>>) -> web::HttpResponse {
    let origin: String = get_origin(&req);
    app_state.metric_sender.rss(origin).await;

    let config = app_state.config.read().unwrap();
    let post_links = &app_state.post_links.read().unwrap();
    if let Some(ref rss_feed) = config.rss_feed {
        let preview_opt = get_preview_option(&config);
        let rendered_posts =
            match retrieve_post_list(&app_state.summary_cache, post_links, None, &preview_opt) {
                Ok(posts) => posts,
                Err(e) => {
                    return web::HttpResponse::InternalServerError()
                        .body(format!("Error listing posts: {}", e))
                }
            };

        let post_list = match render_rss(rss_feed, rendered_posts) {
            Ok(posts) => posts,
            Err(e) => {
                return web::HttpResponse::InternalServerError()
                    .body(format!("Error rendering post list: {}", e))
            }
        };

        web::HttpResponse::Ok()
            .content_type("application/rss+xml; charset=UTF-8")
            .body(post_list)
    } else {
        web::HttpResponse::BadRequest().body("RSS feed is not available.")
    }
}

#[web::get("/view/{post}/{file}")]
async fn post_files(
    path: web::types::Path<(String, String)>,
    app_state: web::types::State<Arc<AppState>>,
) -> Result<NamedFile, web::Error> {
    let (post, file) = path.into_inner();
    let posts_dir = &app_state.config.read().unwrap().paths.posts_dir;
    get_file(posts_dir, post, file)
}

#[web::get("/page/{post}/{file}")]
async fn page_files(
    path: web::types::Path<(String, String)>,
    app_state: web::types::State<Arc<AppState>>,
) -> Result<NamedFile, web::Error> {
    let (post, file) = path.into_inner();
    let pages_dir = &app_state.config.read().unwrap().paths.pages_dir;
    get_file(pages_dir, post, file)
}

#[web::get("/public/{file_name}")]
async fn public_files(
    path: web::types::Path<String>,
    app_state: web::types::State<Arc<AppState>>,
) -> Result<NamedFile, web::Error> {
    if path.contains("../") {
        return Err(web::error::ErrorUnauthorized("Access forbidden").into());
    }

    let config = app_state.config.read().unwrap();
    let file_path = config.paths.public_dir.join(path.into_inner());

    Ok(NamedFile::open(file_path)?)
}

#[web::get("/")]
async fn index(req: HttpRequest, app_state: web::types::State<Arc<AppState>>) -> web::HttpResponse {
    let origin: String = get_origin(&req);
    app_state.metric_sender.index(origin).await;

    let read_cache = app_state.post_cache.read().unwrap();
    let page_name = "-index-page";

    let rendered_page = match read_cache.get_page(page_name) {
        None => {
            // Let's load from the file and update the cache
            info!("Rendering page {} from file", page_name);
            let config = app_state.config.read().unwrap();
            let num_of_posts = app_state.post_links.read().unwrap().len();

            let TomlDate(blog_start_date) = config.personal.blog_start_date;
            let activity_start_year = config.personal.activity_start_year;

            let rendered_post = match render_index(
                req,
                num_of_posts,
                &config.paths.template_dir,
                activity_start_year,
                blog_start_date,
            ) {
                Ok(rendered_post) => rendered_post,
                Err(e) => {
                    return web::HttpResponse::BadRequest()
                        .body(format!("Error loading index page {}: {}", page_name, e));
                }
            };

            drop(read_cache);

            let mut rw_cache = app_state.post_cache.write().unwrap();
            rw_cache.add_page(page_name, rendered_post, Expire::After(Duration::days(1)))
        }
        Some(rendered) => rendered,
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

    let (post_cache, summary_cache) = match config.defaults.rendering_cache_enabled {
        true => (ContentCache::new(), ContentCache::new()),
        false => (ContentCache::non_caching(), ContentCache::non_caching()),
    };

    let (metric_sender, _metrics) = if let Some(ref metrics_cfg) = config.metrics {
        // When configuration is loaded, we already set a location if the metrics section is defined
        let location = metrics_cfg.location.as_ref().unwrap();
        let time_slot = Duration::seconds(metrics_cfg.time_slot_secs.unwrap());
        let metrics = MetricWriter::new(location, time_slot)?;
        let metric_handler = MetricHandler::new(metrics);
        let sender = metric_handler.new_sender();
        (sender, Some(metric_handler))
    } else {
        (MetricHandler::no_op(), None)
    };

    let post_links = RwLock::new(post_links);
    let page_links = RwLock::new(page_links);
    let bind_addr = config.server.address.clone();
    let bind_port = config.server.port;
    let config = RwLock::new(config);
    let post_cache = RwLock::new(post_cache);
    let summary_cache = RwLock::new(summary_cache);

    let app_state = Arc::new(AppState {
        post_links,
        page_links,
        config,
        post_cache,
        summary_cache,
        metric_sender,
    });

    web::HttpServer::new(move || {
        web::App::new()
            .state(app_state.clone())
            .service(index)
            .service(public_files)
            .service(list)
            .service(list_with_tags)
            .service(rss)
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
