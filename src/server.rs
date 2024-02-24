use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use chrono::{Datelike, Utc};
use ntex::http::Response;

use ntex::web;
use ntex::web::Error;
use ntex::web::types::Path;
use ntex_files::NamedFile;
use ramhorns::{Content, Template};
use crate::config::Config;
use crate::post::Post;
use crate::post_cache::PostCache;
use crate::post_list::PostList;
use crate::post_render::render_post;
use crate::text_utils::format_date_time;

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

struct AppState {
    activity_start_year: i32,
    posts: PostCache,
    pages: PostCache,
    config: Config,
}

fn get_posts(root_dir: &PathBuf, post_file: &str) -> io::Result<Vec<Post>> {
    let root_dir = root_dir.clone();
    let post_list = PostList {
        root_dir,
        post_file: post_file.to_string(),
    };

    let dirs = post_list.retrieve_dirs()?;
    let mut posts = vec![];
    for dir in dirs.as_slice() {
        let p = dir.join(&post_list.post_file);
        let post = Post::from(&p, true)?;
        posts.push(post);
    }

    // Retrieve files in post directory
    let md_posts: Vec<PathBuf> = post_list.retrieve_files()?;
    for post_file in md_posts {
        let post = Post::from(&post_file, true)?;
        posts.push(post);
    }

    Ok(posts)
}

fn process_post(path: Path<String>, template_dir: &PathBuf, template_name: &str, posts: &PostCache) -> Result<Response, Response> {
    let view_tpl_src: String = match read_template(template_dir, template_name) {
        Ok(s) => s,
        Err(e) => {
            return Err(web::HttpResponse::InternalServerError()
                .body(format!("Error loading post view template: {}", e)));
        }
    };

    // TODO: Cache renderer?
    let view_tpl = match Template::new(view_tpl_src) {
        Ok(x) => x,
        Err(e) => {
            return Err(web::HttpResponse::InternalServerError()
                .body(format!("Error parsing post view template: {}", e)));
        }
    };

    let path = path.into_inner();
    let post_summary = match posts.from_link(&path) {
        Some(post) => post,
        None => return Err(web::HttpResponse::InternalServerError()
            .body(format!("Error loading post with link: {}", &path))),
    };

    let post = match Post::from(&post_summary.header.file_name, false) {
        Ok(post) => post,
        Err(e) => {
            return Err(web::HttpResponse::InternalServerError()
                .body(format!("Error loading post content: {}", e)));
        }
    };

    let html = match render_post(&post.content, None) {
        Ok(post) => post,
        Err(e) => {
            return Err(web::HttpResponse::InternalServerError()
                .body(format!("Error rendering post content: {}", e)));
        }
    };

    let (date, time) = format_date_time(&post.header.date);

    let rendered = view_tpl.render(&ViewItem {
        errors: vec![],
        id: post.header.id.as_str(),
        author: post.header.author.as_str(),
        date: date.as_str(),
        time: time.as_str(),
        post_title: post.title.as_str(),
        post_content: html.as_str(),
    });

    Ok(web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(&rendered))
}

fn read_template(tpl_dir: &PathBuf, file_name: &str) -> Result<String, io::Error> {
    let full_path = tpl_dir.join(file_name);
    std::fs::read_to_string(full_path)
}

fn get_file(root_dir: &PathBuf, post: String, file: String) -> Result<NamedFile, Error> {
    if post.contains("../") || file.contains("../") {
        return Err(web::error::ErrorUnauthorized("Access forbidden").into());
    }

    let file_path = root_dir.join(post).join(file);
    Ok(NamedFile::open(file_path)?)
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
async fn page(path: web::types::Path<String>,
              state: web::types::State<Arc<Mutex<AppState>>>,
) -> web::HttpResponse {
    let state = state.lock().unwrap();
    let template_dir = &state.config.paths.template_dir;
    let posts: &PostCache = &state.pages;

    match process_post(path, template_dir, "page.tpl", posts) {
        Ok(value) => value,
        Err(value) => return value,
    }
}

#[web::get("/view/{post}/")]
async fn view(path: web::types::Path<String>,
              state: web::types::State<Arc<Mutex<AppState>>>,
) -> web::HttpResponse {
    let state = state.lock().unwrap();
    let template_dir = &state.config.paths.template_dir;
    let posts: &PostCache = &state.posts;

    match process_post(path, template_dir, "view.tpl", posts) {
        Ok(value) => value,
        Err(value) => return value,
    }
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
struct ViewItem<'a> {
    errors: Vec<String>,
    id: &'a str,
    author: &'a str,
    date: &'a str,
    time: &'a str,
    post_title: &'a str,
    post_content: &'a str,
}

#[web::get("/view/{post}/{file}")]
async fn post_files(path: web::types::Path<(String, String)>,
                    state: web::types::State<Arc<Mutex<AppState>>>,
) -> Result<NamedFile, web::Error> {
    let (post, file) = path.into_inner();
    let state = state.lock().unwrap();
    get_file(&state.config.paths.posts_dir, post, file)
}

#[web::get("/page/{post}/{file}")]
async fn page_files(path: web::types::Path<(String, String)>,
                    state: web::types::State<Arc<Mutex<AppState>>>,
) -> Result<NamedFile, web::Error> {
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
        let (date, _) = cache.post_list.last().unwrap();
        let res = Utc::now().naive_utc().signed_duration_since(date.clone());
        res.num_days()
    };
    let years_developing = (Utc::now().year() - state.activity_start_year) as i64;

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
