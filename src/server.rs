use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use chrono::{Datelike, Utc};

use ntex::web;
use ntex_files::NamedFile;
use ramhorns::{Content, Template};
use crate::config::Config;
use crate::post::Post;
use crate::post_cache::PostCache;
use crate::post_list::PostList;
use crate::post_render::render_post;
use crate::text_utils::format_date_time;

// TODO: MISSING
// 1. Caching rendered pages - final html and no comments?

#[derive(Content)]
struct IndexPage {
    years_developing: i64,
    post_count: i64,
    days_since_started: i64,
}

#[derive(Content)]
struct ListPage {
    post_list: Vec<PostItem>,
}

#[derive(Content)]
struct PostItem {
    date: String,
    time: String,
    link: String,
    title: String,
    summary: String,
}

fn get_posts(root_dir: &PathBuf) -> io::Result<Vec<Post>> {
    // TODO: Posts location should be configurable
    let post_file = "index.md".to_string(); // TODO: index.md should be configurable
    let root_dir = root_dir.clone();
    let post_list = PostList { root_dir, post_file };

    let dirs = post_list.retrieve_dirs()?;
    let mut posts = vec![];
    for dir in dirs.as_slice() {
        let p = dir.join(&post_list.post_file);
        let post = Post::from(&p, true)?;
        posts.push(post);
    }

    Ok(posts)
}

#[web::get("/list")]
async fn list(state: web::types::State<Arc<Mutex<AppState>>>) -> web::HttpResponse {
    let state = &state.lock().unwrap();

    let tpl_dir = &state.config.paths.template_dir;
    let list_tpl_src: String = match read_template(tpl_dir, "postlist.tpl") {
        Ok(s) => s,
        Err(e) => {
            return web::HttpResponse::InternalServerError()
                .body(format!("Error loading postlist template: {}", e));
        }
    };

    let list_tpl = match Template::new(list_tpl_src) {
        Ok(x) => x,
        Err(e) => {
            return web::HttpResponse::InternalServerError()
                .body(format!("Error parsing postlist template: {}", e));
        }
    };

    // TODO: Implement multiple readers, single writer or remove lock
    let mut post_list = vec![];
    {
        let cache = &state.posts;

        for (_, uuid) in cache.post_list.iter() {
            let post_item = cache.posts.get(uuid.as_str()).unwrap();
            let post_link = format!("view/{}/", post_item.link);
            let post = &post_item.post;
            let html = match render_post(post.content.as_str(), Some(post_link.as_str())) {
                Ok(html) => html,
                Err(e) => return web::HttpResponse::InternalServerError()
                    .body(format!("Error rendering post: {}", e)),
            };

            let (date, time) = format_date_time(&post.header.date);
            let post_item = PostItem {
                date: date.to_string(),
                time: time.to_string(),
                link: post_link,
                title: post.title.clone(),
                summary: html,
            };
            post_list.push(post_item);
        }
    }

    let rendered = list_tpl.render(&ListPage {
        post_list
    });

    web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(rendered)
}

#[derive(Content)]
struct ViewItem {
    errors: Vec<String>,
    id: String,
    author: String,
    date: String,
    time: String,
    post_title: String,
    post_content: String,
}

#[web::get("/view/{post}/{file}")]
async fn post_files(path: web::types::Path<(String, String)>,
                    state: web::types::State<Arc<Mutex<AppState>>>,
) -> Result<NamedFile, web::Error> {
    let (post, file) = path.into_inner();
    if post.contains("../") || file.contains("../") {
        return Err(web::error::ErrorUnauthorized("Access forbidden").into());
    }

    let state = state.lock().unwrap();
    let post_location = &state.config.paths.posts_dir;
    let file_path = post_location.join(post).join(file);

    Ok(NamedFile::open(file_path)?)
}

#[web::get("/view/{post}")]
async fn view_wo_slash(path: web::types::Path<String>) -> web::HttpResponse {
    web::HttpResponse::TemporaryRedirect()
        .header("Location", path.into_inner() + "/")
        .content_type("text/html; charset=utf-8")
        .finish()
}

#[web::get("/view/{post}/")]
async fn view(path: web::types::Path<String>,
              state: web::types::State<Arc<Mutex<AppState>>>,
) -> web::HttpResponse {
    let state = state.lock().unwrap();
    let view_tpl_src: String = match read_template(&state.config.paths.template_dir, "view.tpl") {
        Ok(s) => s,
        Err(e) => {
            return web::HttpResponse::InternalServerError()
                .body(format!("Error loading post view template: {}", e));
        }
    };

    // TODO: Cache renderer?
    let view_tpl = match Template::new(view_tpl_src) {
        Ok(x) => x,
        Err(e) => {
            return web::HttpResponse::InternalServerError()
                .body(format!("Error parsing post view template: {}", e));
        }
    };

    let posts: &PostCache = &state.posts;
    let path = path.into_inner();
    let post_summary = match posts.from_link(&path) {
        Some(post) => post,
        None => return web::HttpResponse::InternalServerError()
            .body(format!("Error loading post with link: {}", &path)),
    };

    let post = match Post::from(&post_summary.header.file_name, false) {
        Ok(post) => post,
        Err(e) => {
            return web::HttpResponse::InternalServerError()
                .body(format!("Error loading post content: {}", e));
        }
    };

    let html = match render_post(&post.content, None) {
        Ok(post) => post,
        Err(e) => {
            return web::HttpResponse::InternalServerError()
                .body(format!("Error rendering post content: {}", e));
        }
    };

    let (date, time) = format_date_time(&post.header.date);

    // TODO: Ref instead of clone
    let rendered = view_tpl.render(&ViewItem {
        errors: vec![],
        id: post.header.id.clone(),
        author: post.header.author.clone(),
        date: date.to_string(),
        time: time.to_string(),
        post_title: post.title.clone(),
        post_content: html,
    });

    web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(&rendered)
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
    let index_tpl_src: String = match read_template(&state.config.paths.template_dir, "index.tpl") {
        Ok(s) => s,
        Err(e) => {
            return web::HttpResponse::InternalServerError()
                .body(format!("Error loading index template: {}", e));
        }
    };

    let index_tpl = match Template::new(index_tpl_src) {
        Ok(x) => x,
        Err(e) => {
            return web::HttpResponse::InternalServerError()
                .body(format!("Error parsing index template: {}", e));
        }
    };

    let cache = &state.posts;
    let days_since_first_post = if cache.post_list.is_empty() {
        0
    } else {
        let (date, _) = cache.post_list.first().unwrap();
        let res = Utc::now().naive_utc().signed_duration_since(date.clone());
        res.num_days()
    };
    let years_developing = (Utc::now().year() - state.programming_start_year) as i64;

    // TODO: Calculate numbers
    let rendered = index_tpl.render(&IndexPage {
        years_developing,
        post_count: cache.posts.len() as i64,
        days_since_started: days_since_first_post,
    });

    let mut referer: String = match req.headers().get("referer") {
        Some(v) => v.to_str().unwrap().to_string(),
        None => "http://sei-la/".to_string(),
    };

    if !referer.ends_with("/") {
        referer += "/";
    }

    web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(rendered)
}

fn read_template(tpl_dir: &PathBuf, file_name: &str) -> Result<String, io::Error> {
    let full_path = tpl_dir.join(file_name);
    std::fs::read_to_string(full_path)
}

struct AppState {
    programming_start_year: i32,
    posts: PostCache,
    config: Config,
}

pub async fn server_run(config: Config) -> std::io::Result<()> {
    let md_posts = match get_posts(&config.paths.posts_dir) {
        Ok(posts) => posts,
        Err(err) => {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Error retrieving post list template: {}. Dir={}", err, config.paths.posts_dir.to_str().unwrap())));
        }
    };

    let app_state = Arc::new(Mutex::new(AppState {
        programming_start_year: 2000, // TODO: Make it configurable
        posts: PostCache::new(),
        config,
    }));

    {
        let cache = &mut app_state.lock().unwrap().posts;
        for post in md_posts {
            cache.add(post)?;
        }
        cache.sort();
    }

    web::HttpServer::new(move || {
        web::App::new()
            .state(app_state.clone())
            .service(index)
            .service(public_files)
            .service(list)
            .service(view)
            .service(view_wo_slash)
            .service(post_files)
    })
        .bind(("0.0.0.0", 8001))? // TODO: Address and port should be configurable
        .run()
        .await
}
