use std::{fs, io};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use ntex::web;
use ntex::web::HttpRequest;
use ntex_files::NamedFile;

use crate::config::Config;
use crate::content::content_file::ContentFile;
use crate::content::content_renderer::{ContentRenderer, ImagePrefix, RenderOptions};
use crate::content::html_renderer::HtmlRenderer;
use crate::content::texted_renderer::TextedRenderer;
use crate::paginator::Paginator;
use crate::post::ContentFormat;
use crate::post_processor::*;
use crate::query_string::QueryString;
use crate::util::toml_date::TomlDate;
use crate::view::list_renderer::ListRenderer;

struct AppState {
    post_links: HashMap<String, PathBuf>,
    page_links: HashMap<String, PathBuf>,
    // posts: PostCache,
    // pages: PostCache,
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
async fn page(page_name: web::types::Path<String>, state: web::types::State<Arc<Mutex<AppState>>>) -> web::HttpResponse {
    let state = state.lock().unwrap();
    let page_name = page_name.into_inner();

    let rendered_post = match open_content(&state.config, &state.page_links, "page.tpl", &page_name) {
        Ok(post) => post,
        Err(e) => {
            return web::HttpResponse::BadRequest()
                .body(format!("Error loading post {}: {}", page_name, e));
        }
    };

    // TODO: Add Cache here with the FINAL result of the render
    // Something like post_name -> rendered_post

    web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(rendered_post)
}

#[web::get("/view/{post}/")]
async fn view(post_name: web::types::Path<String>, state: web::types::State<Arc<Mutex<AppState>>>) -> web::HttpResponse {
    let state = state.lock().unwrap();
    let post_name = post_name.into_inner();

    let rendered_post = match open_content(&state.config, &state.post_links, "view.tpl", &post_name) {
        Ok(post) => post,
        Err(e) => {
            return web::HttpResponse::BadRequest()
                .body(format!("Error loading post {}: {}", post_name, e));
        }
    };

    // TODO: Add Cache here with the FINAL result of the render
    // Something like post_name -> rendered_post

    web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(rendered_post)
}

fn get_cur_page(req: HttpRequest) -> u32 {
    if let Some(query_str) = req.uri().query() {
        let qs = QueryString::from(query_str);
        qs.get_page()
    } else {
        1
    }
}

fn render_list(link_to_files: &HashMap<String, PathBuf>, config: &Config, cur_page: u32, tag: Option<String>) -> io::Result<String> {
    let mut contents = vec![];
    let mut tag_map = HashMap::new();

    for (post_link, content_path) in link_to_files.iter() {
        let content_file = ContentFile::from_file(post_link.clone(), content_path.clone())?;
        let img_prefix = ImagePrefix(format!("/view/{}", post_link));
        let content = match content_file.format {
            ContentFormat::Texted => TextedRenderer::render(&content_file, RenderOptions::PreviewOnly(img_prefix)),
            ContentFormat::Html => HtmlRenderer::render(&content_file, RenderOptions::PreviewOnly(img_prefix)),
        }?;

        for post_tag in content.header.tags.iter() {
            *tag_map.entry(post_tag.clone()).or_insert(0) += 1;
        }

        match tag {
            None => contents.push(content),
            Some(ref s_tag) => {
                if content.header.tags.contains(s_tag) {
                    contents.push(content);
                }
            }
        };
    }

    // Sort tags by frequency reversed
    let mut tag_list: Vec<(String, u32)> = tag_map.into_iter().map(|(k, v)| { (k, v) }).collect();
    tag_list.sort_by(|a, b| {
        let (_, va) = a;
        let (_, vb) = b;
        vb.cmp(va)
    });
    let tags = tag_list.into_iter().map(|(k, _v)| { k }).collect();

    // sort contents by date reversed
    contents.sort_by(|a, b| {
        b.header.date.cmp(&a.header.date)
    });

    let page_size = config.defaults.page_size;
    let paginator = Paginator::from(&contents, page_size);
    let cur_page = match cur_page { // Sanity check for current page
        0 => 1,
        x if x > paginator.page_count() => 1,
        x => x,
    };

    let template_dir = &config.paths.template_dir;
    let template_path = template_dir.join("postlist.tpl");
    let template_src = fs::read_to_string(&template_path)?;
    let list_posts = ListRenderer::new(&template_src, paginator.page_count());
    let list_posts = list_posts.unwrap();

    let content_page = match paginator.get_page(cur_page) {
        Ok(content) => content,
        Err(err_desc) => return Err(io::Error::new(ErrorKind::InvalidInput, err_desc)),
    };

    let res = list_posts.render(content_page, cur_page, tags);
    Ok(res)
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
async fn list_with_tags(
    req: HttpRequest,
    path: web::types::Path<String>,
    state: web::types::State<Arc<Mutex<AppState>>>) -> web::HttpResponse {
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
    // let cache = &state.posts;
    let tpl_dir = &state.config.paths.template_dir;

    let TomlDate(blog_start_date) = state.config.personal.blog_start_date;
    let activity_start_year = state.config.personal.activity_start_year;
    let response = match render_index(req, state.post_links.len(), tpl_dir, activity_start_year, blog_start_date) {
        Ok(rendered) => web::HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(rendered),
        Err(error) => return web::HttpResponse::InternalServerError()
            .body(error),
    };

    response
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

    let page_links: HashMap<_, _> = page_link_vec.into_iter()
        .map(|link| { (link.post_name, link.post_path) })
        .collect();

    // TODO: Remove this get_posts
    let mut index_file_name = config.defaults.index_base_name.as_str().to_string();
    index_file_name += ".md";
    // let md_posts = match get_posts(&config.paths.posts_dir, &index_file_name) {
    //     Ok(posts) => posts,
    //     Err(err) => {
    //         return Err(io::Error::new(
    //             io::ErrorKind::InvalidData,
    //             format!("Error retrieving post list template: {}. Dir={}. CurDir={}",
    //                     err,
    //                     config.paths.posts_dir.to_str().unwrap(),
    //                     env::current_dir().unwrap().to_str().unwrap()
    //             )));
    //     }
    // };
    //
    // let md_pages = match get_posts(&config.paths.pages_dir, &index_file_name) {
    //     Ok(posts) => posts,
    //     Err(err) => {
    //         return Err(io::Error::new(
    //             io::ErrorKind::InvalidData,
    //             format!("Error retrieving post list template: {}. Dir={}. CurDir={}",
    //                     err,
    //                     config.paths.posts_dir.to_str().unwrap(),
    //                     env::current_dir().unwrap().to_str().unwrap()
    //             )));
    //     }
    // };

    let bind_addr = config.server.address.clone();
    let bind_port = config.server.port;
    let app_state = Arc::new(Mutex::new(AppState {
        post_links,
        page_links,
        // posts: PostCache::new(config.defaults.index_base_name.as_str()),
        // pages: PostCache::new(config.defaults.index_base_name.as_str()),
        config,
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
