use std::io;
use std::path::PathBuf;
use chrono::{Datelike, Utc};
use ntex::http::Response;
use ntex::web;
use ntex::web::{Error, HttpRequest};
use ntex_files::NamedFile;
use ramhorns::{Content, Template};
use crate::paginator::Paginator;
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
struct ListPage<'a> {
    post_list: Vec<PostItem>,
    tags: Vec<ViewTag<'a>>,
    page_list: Vec<ViewPagination>,
    show_pagination: bool,
}

#[derive(Content)]
struct PostItem {
    date: String,
    time: String,
    link: String,
    title: String,
    summary: String,
}

#[derive(Content)]
struct ViewItem<'a> {
    errors: Vec<String>,
    id: &'a str,
    author: &'a str,
    tags: &'a Vec<ViewTag<'a>>,
    date: &'a str,
    time: &'a str,
    post_title: &'a str,
    post_content: &'a str,
}

#[derive(Content)]
struct ViewTag<'a> {
    tag: &'a str,
}

#[derive(Content)]
struct ViewPagination {
    current: bool,
    number: u32,
}

pub fn get_posts(root_dir: &PathBuf, post_file: &str) -> io::Result<Vec<Post>> {
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

pub fn process_post(path: String, template_dir: &PathBuf, template_name: &str, posts: &PostCache) -> Result<Response, Response> {
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

    let post_summary = match posts.with_link(&path) {
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

    let ref tags: Vec<ViewTag> = post.header.tags.iter().map(|t| ViewTag { tag: t.as_str() }).collect();

    let rendered = view_tpl.render(&ViewItem {
        errors: vec![],
        id: post.header.id.as_str(),
        author: post.header.author.as_str(),
        tags,
        date: date.as_str(),
        time: time.as_str(),
        post_title: post.title.as_str(),
        post_content: html.as_str(),
    });

    Ok(web::HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(&rendered))
}

pub fn list_posts(tpl_dir: &PathBuf, cache: &PostCache, tag: Option<String>, cur_page: u32, page_size: u32) -> Result<String, String> {
    let list_tpl_src: String = match read_template(tpl_dir, "postlist.tpl") {
        Ok(s) => s,
        Err(e) => {
            return Err(format!("Error loading postlist template: {}", e));
        }
    };

    let list_tpl = match Template::new(list_tpl_src) {
        Ok(x) => x,
        Err(e) => {
            return Err(format!("Error parsing postlist template: {}", e));
        }
    };

    // TODO: Implement multiple readers, single writer or remove lock
    let mut post_list = vec![];
    let paginator = Paginator::from(cache.post_list(), page_size);
    let cur_page = match cur_page {
        0 => 1,
        x if x > paginator.page_count() => 1,
        x => x,
    };

    // TODO: Implement pagination for tags or remove if tag is selected

    {
        for (_, uuid) in paginator.get_page(cur_page)? {
            let post_item = cache.posts().get(uuid.as_str()).unwrap();
            let post_link = format!("/view/{}/", post_item.link);
            let post = &post_item.post;

            // TODO: This is not efficient when we have a long list of items. Needs improvement in the future
            if let Some(ref tag) = tag {
                if !post.header.tags.contains(tag) {
                    continue;
                }
            }

            let html = match render_post(post.content.as_str(), Some(post_link.as_str())) {
                Ok(html) => html,
                Err(e) => return Err(format!("Error rendering post: {}", e)),
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

    let tags: Vec<_> = cache.tags().iter().map(|t| ViewTag { tag: t.as_str() }).collect();
    let mut page_list: Vec<ViewPagination> = Vec::with_capacity(paginator.page_count() as usize);
    for i in 1..=paginator.page_count() {
        let current = if i == cur_page { true } else { false };
        page_list.push(ViewPagination {
            current,
            number: i,
        })
    }

    let show_pagination = page_list.len() > 1 && tag.is_none();
    let rendered = list_tpl.render(&ListPage {
        post_list,
        tags,
        page_list,
        show_pagination,
    });
    Ok(rendered)
}

pub fn read_template(tpl_dir: &PathBuf, file_name: &str) -> Result<String, io::Error> {
    let full_path = tpl_dir.join(file_name);
    std::fs::read_to_string(full_path)
}

pub fn get_file(root_dir: &PathBuf, post: String, file: String) -> Result<NamedFile, Error> {
    if post.contains("../") || file.contains("../") {
        return Err(web::error::ErrorUnauthorized("Access forbidden").into());
    }

    let file_path = root_dir.join(post).join(file);
    Ok(NamedFile::open(file_path)?)
}

pub fn render_index(req: HttpRequest, cache: &&PostCache, tpl_dir: &PathBuf, activity_start_year: i32) -> Result<String, String> {
    let index_tpl_src: String = match read_template(tpl_dir, "index.tpl") {
        Ok(s) => s,
        Err(e) => {
            return Err(format!("Error loading index template: {}", e));
        }
    };

    let index_tpl = match Template::new(index_tpl_src) {
        Ok(x) => x,
        Err(e) => {
            return Err(format!("Error parsing index template: {}", e));
        }
    };


    let days_since_first_post = if cache.post_list().is_empty() {
        0
    } else {
        let (date, _) = cache.post_list().last().unwrap();
        let res = Utc::now().naive_utc().signed_duration_since(date.clone());
        res.num_days()
    };
    let years_developing = (Utc::now().year() - activity_start_year) as i64;

    let rendered = index_tpl.render(&IndexPage {
        years_developing,
        post_count: cache.posts().len() as i64,
        days_since_started: days_since_first_post,
    });

    let mut referer: String = match req.headers().get("referer") {
        Some(v) => v.to_str().unwrap().to_string(),
        None => "http://sei-la/".to_string(),
    };

    if !referer.ends_with("/") {
        referer += "/";
    }
    Ok(rendered)
}