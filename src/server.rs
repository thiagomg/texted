
use std::{io, str::FromStr};
use std::path::PathBuf;

use ntex::web;
use ntex_files::NamedFile;
use ramhorns::{Content, Template};
use crate::post::Post;
use crate::post_list::PostList;

#[web::get("/list")]
async fn list() -> Result<String, web::Error> {
    let mut page_buf = String::new();
    let root_dir = PathBuf::from("/home/thiago/src/texted2/posts");
    let post_file = "post.md".to_string();
    let post_list = PostList { root_dir, post_file };

    let dirs = post_list.retrieve_dirs()?;
    for dir in dirs.as_slice() {
        let p = dir.join(&post_list.post_file);
        let post = Post::from(&p, true)?;
        page_buf.push_str(format!("{}\n{}\n=-=-=-=-=-=-=-=-=-=-\n", p.to_str().unwrap(), post).as_str());
    }

    Ok(page_buf)
}

#[web::get("/view/{post}")]
async fn view(path: web::types::Path<String>) -> Result<String, web::Error> {
    let post_name = path.into_inner();
    Ok(format!("Received {:?}", post_name))
}

#[derive(Content)]
struct IndexPage {
    years_developing: i64,
    post_count: i64,
    days_since_started: i64,
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

#[derive(Clone)]
struct AppState {
    app_name: String,
}

pub async fn server_run() -> std::io::Result<()> {
    let app_state = AppState {
        app_name: "Thiago Cafe".to_string(),
    };

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
