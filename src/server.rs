
use std::{io, str::FromStr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use ntex::web;
use ntex_files::NamedFile;
use ramhorns::{Content, Template};
use crate::post::Post;
use crate::post_cache::PostCache;
use crate::post_list::PostList;
use crate::render_post::render_post;

// TODO: MISSING
// 1. Caching rendered pages - final html and no comments?
// 2. When markdown tries to access an image, we need to retrieve the image
//    E.g. http://127.0.0.1:8001/view/game_of_life.gif
// 3. Fill stats in the main page
// 4. Parse date/time from the header
// 5. Change templates to "tell me your opinion" using twitter 

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

fn get_posts() -> io::Result<Vec<Post>> {
    // TODO: Move it to post_list
    // TODO: Posts location should be configurable
    let root_dir = PathBuf::from("/home/thiago/src/texted2/posts");
    let post_file = "index.md".to_string();
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

// TODO: un-mut this when caching at the start
#[web::get("/list")]
async fn list(mut state: web::types::State<Arc<Mutex<AppState>>>) -> web::HttpResponse {

    // TODO: Make templates location and names configurable
    let list_tpl_src: String = match read_template("postlist.tpl") {
        Ok(s) => s,
        Err(e) => {
            return web::HttpResponse::InternalServerError()
                .body(format!("Error loading postlist template: {}", e))
        }
    };

    // TODO: Cache renderer?
    let list_tpl = match Template::new(list_tpl_src) {
        Ok(x) => x,
        Err(e) => {
            return web::HttpResponse::InternalServerError()
                .body(format!("Error parsing postlist template: {}", e))
        }
    };

    // TODO: Implement multiple readers, single writer
    let mut post_list = vec![];
    {
        let cache = &state.lock().unwrap().posts;

        for (link, uuid) in cache.link_to_uuid.iter() {
            // TODO: Implement From trait. Do we need to clone?
            let post = cache.posts.get(uuid).unwrap();
            let html = match render_post(post.content.as_str()) {
                Ok(html) => html,
                Err(e) => return web::HttpResponse::InternalServerError()
                    .body(format!("Error rendering post: {}", e)),
            };

            let post_item = PostItem {
                date: post.header.date.clone(),
                time: "TO_PARSE".to_string(), // TODO: Parse date time
                link: "view/".to_string() + link,
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
    post_content: String,
}

#[web::get("/view/{post}")]
async fn view(path: web::types::Path<String>, 
    state: web::types::State<Arc<Mutex<AppState>>>
) -> web::HttpResponse {

    let view_tpl_src: String = match read_template("view.tpl") {
        Ok(s) => s,
        Err(e) => {
            return web::HttpResponse::InternalServerError()
                .body(format!("Error loading post view template: {}", e))
        }
    };

    // TODO: Cache renderer?
    let view_tpl = match Template::new(view_tpl_src) {
        Ok(x) => x,
        Err(e) => {
            return web::HttpResponse::InternalServerError()
                .body(format!("Error parsing post view template: {}", e))
        }
    };

    let posts : &PostCache = &state.lock().unwrap().posts;
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
                .body(format!("Error loading post content: {}", e))
        }
    };

    let html = match render_post(&post.content) {
        Ok(post) => post,
        Err(e) => {
            return web::HttpResponse::InternalServerError()
                .body(format!("Error rendering post content: {}", e))
        }
    };

    // TODO: Ref instead of clone
    let rendered = view_tpl.render(&ViewItem { 
        errors: vec![], 
        id: post.header.id.clone(), 
        author: post.header.author.clone(), 
        date: post.header.date.clone(), 
        time: "to-fill".to_string(), 
        post_content: html

    });

    web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(&rendered)
}

#[web::get("/public/{file_name}")]
async fn public_files(path: web::types::Path<String>) -> Result<NamedFile, web::Error> {
    if path.starts_with("..") {
        return Err(web::error::ErrorUnauthorized("Access forbidden").into());
    }

    // TODO: Make it configurable
    let mut file_name = "/home/thiago/src/texted2/res/public/".to_string();
    file_name.push_str(path.into_inner().as_str());

    let file_path = std::path::PathBuf::from_str(file_name.as_str()).unwrap();

    Ok(NamedFile::open(file_path)?)
}

#[web::get("/")]
async fn index(req: web::HttpRequest) -> web::HttpResponse {
    let index_tpl_src: String = match read_template("index.tpl") {
        Ok(s) => s,
        Err(e) => {
            return web::HttpResponse::InternalServerError()
                .body(format!("Error loading index template: {}", e))
        }
    };

    let index_tpl = match Template::new(index_tpl_src) {
        Ok(x) => x,
        Err(e) => {
            return web::HttpResponse::InternalServerError()
                .body(format!("Error parsing index template: {}", e))
        }
    };

    // TODO: Calculate numbers
    let rendered = index_tpl.render(&IndexPage {
        years_developing: 23,
        post_count: 12,
        days_since_started: 821,
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

fn read_template(file_name: &str) -> Result<String, io::Error> {
    // TODO: Make it configurable
    let tpl_path = std::path::PathBuf::from("/home/thiago/src/texted2/res/template");
    let full_path = tpl_path.join(file_name);

    std::fs::read_to_string(full_path)
}

struct AppState {
    app_name: String,
    posts: PostCache,
}

pub async fn server_run() -> std::io::Result<()> {
    let app_state = Arc::new(Mutex::new(AppState {
        app_name: "Thiago Cafe".to_string(),
        posts: PostCache::new(),
    }));

        // TODO: Retrieve from cache first
    let md_posts = match get_posts() {
        Ok(posts) => posts,
        Err(err) => {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Error retrieving post list template: {}", err)));
        }
    };

    // TODO: If update cache
    {
        let cache = &mut app_state.lock().unwrap().posts;
        for post in md_posts {
            cache.add(post)?;
        }
    }

    web::HttpServer::new(move || {
        web::App::new()
            .state(app_state.clone())
            .service(index)
            .service(public_files)
            .service(list)
            .service(view)
    })
    .bind(("0.0.0.0", 8001))?
    .run()
    .await
}
