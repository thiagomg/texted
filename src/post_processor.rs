use std::{fs, io};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use chrono::{Datelike, NaiveDate, Utc};
use ntex::web;
use ntex::web::{Error, HttpRequest};
use ntex_files::NamedFile;
use ramhorns::Template;
use spdlog::info;

use crate::config::Config;
use crate::content::Content;
use crate::content::content_file::ContentFile;
use crate::content::content_format::ContentFormat;
use crate::content::content_renderer::{ContentRenderer, ImagePrefix, RenderOptions};
use crate::content::html_renderer::HtmlRenderer;
use crate::content::texted_renderer::TextedRenderer;
use crate::content_cache::{ContentCache, Expire};
use crate::paginator::Paginator;
use crate::post_list::PostList;
use crate::query_string::QueryString;
use crate::view::list_renderer::ListRenderer;
use crate::view::post_renderer::PostRenderer;

#[derive(ramhorns::Content)]
struct IndexPage {
    years_developing: i64,
    post_count: i64,
    days_since_started: i64,
}

#[derive(ramhorns::Content)]
struct ListPage<'a> {
    post_list: Vec<PostItem>,
    tags: Vec<ViewTag<'a>>,
    page_list: Vec<ViewPagination>,
    show_pagination: bool,
}

#[derive(ramhorns::Content)]
struct PostItem {
    date: String,
    time: String,
    link: String,
    title: String,
    summary: String,
}

#[derive(ramhorns::Content)]
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

#[derive(ramhorns::Content)]
struct ViewTag<'a> {
    tag: &'a str,
}

#[derive(ramhorns::Content)]
struct ViewPagination {
    current: bool,
    number: u32,
}

#[derive(Debug)]
pub struct PostLink {
    pub post_name: String,
    pub post_path: PathBuf,
}

pub fn list_post_files(root_dir: &PathBuf, post_file: &str) -> Result<Vec<PostLink>> {
    let root_dir = root_dir.clone();
    let post_list = PostList {
        root_dir,
        post_file: post_file.to_string(),
    };

    let dirs = post_list.retrieve_dirs()?;
    let mut posts = vec![];
    for (dir, file_name) in dirs {
        // Adding default file to directory posts
        let post_name = dir.iter().last().unwrap().to_str().unwrap().to_string();
        let post_path = dir.join(file_name);

        posts.push(PostLink {
            post_name,
            post_path,
        });
    }

    // Retrieve files in post directory
    let md_posts: Vec<PathBuf> = post_list.retrieve_files()?;
    for post_file in md_posts {
        let post_name = post_file.file_stem().unwrap().to_str().unwrap().to_string();
        let post_path = post_file;
        posts.push(PostLink {
            post_name,
            post_path,
        });
    }

    Ok(posts)
}

pub fn read_template(tpl_dir: &PathBuf, file_name: &str) -> io::Result<String> {
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

pub fn render_index(req: HttpRequest, num_of_posts: usize, tpl_dir: &PathBuf,
                    activity_start_year: i32, blog_start_date: NaiveDate) -> io::Result<String> {
    let index_tpl_src: String = match read_template(tpl_dir, "index.tpl") {
        Ok(s) => s,
        Err(e) => {
            return Err(io::Error::new(ErrorKind::InvalidInput, format!("Error loading index template: {}", e)));
        }
    };

    let index_tpl = match Template::new(index_tpl_src) {
        Ok(x) => x,
        Err(e) => {
            return Err(io::Error::new(ErrorKind::InvalidInput, format!("Error parsing index template: {}", e)));
        }
    };

    let days_since_first_post = (Utc::now().date_naive() - blog_start_date).num_days();
    let years_developing = (Utc::now().year() - activity_start_year) as i64;

    let rendered = index_tpl.render(&IndexPage {
        years_developing,
        post_count: num_of_posts as i64,
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

pub fn open_content(config: &Config, link_to_files: &HashMap<String, PathBuf>, template_filename: &str, link: &str) -> io::Result<String> {
    let content_path = match link_to_files.get(link) {
        None => return Err(io::Error::new(io::ErrorKind::NotFound, "Could not find post")),
        Some(path) => path,
    }.clone();

    let content_file = ContentFile::from_file(link.to_string(), content_path)?;
    let content = match content_file.format {
        ContentFormat::Texted => TextedRenderer::render(&content_file, RenderOptions::FullContent),
        ContentFormat::Html => HtmlRenderer::render(&content_file, RenderOptions::FullContent),
    }?;

    let template_dir = &config.paths.template_dir;
    let template_path = template_dir.join(template_filename);
    let template_src = fs::read_to_string(&template_path)?;

    let post_renderer = PostRenderer::new(&template_src)?;
    Ok(post_renderer.render(&content))
}

pub fn get_cur_page(req: HttpRequest) -> u32 {
    if let Some(query_str) = req.uri().query() {
        let qs = QueryString::from(query_str);
        qs.get_page()
    } else {
        1
    }
}

pub struct PostListWithTags {
    contents: Vec<Arc<Content>>,
    tag_map: HashMap<String, i32>,
}

pub fn retrieve_post_list(cache: &mut ContentCache<Content>, link_to_files: &HashMap<String, PathBuf>, tag_to_filter: Option<String>) -> io::Result<PostListWithTags> {
    let mut contents = vec![];
    let mut tag_map = HashMap::new();

    for (post_link, content_path) in link_to_files.iter() {
        let content = cache.get_post_or(post_link, Expire::Never, || {
            info!("Rendering post preview from file for {}", post_link);
            let content_file = ContentFile::from_file(post_link.clone(), content_path.clone())?;
            let img_prefix = ImagePrefix(format!("/view/{}", post_link));
            match content_file.format {
                ContentFormat::Texted => TextedRenderer::render(&content_file, RenderOptions::PreviewOnly(img_prefix)),
                ContentFormat::Html => HtmlRenderer::render(&content_file, RenderOptions::PreviewOnly(img_prefix)),
            }
        })?;

        for post_tag in content.header.tags.iter() {
            *tag_map.entry(post_tag.clone()).or_insert(0) += 1;
        }

        match tag_to_filter {
            None => contents.push(content),
            Some(ref s_tag) => {
                if content.header.tags.contains(s_tag) {
                    contents.push(content);
                }
            }
        };
    }

    Ok(PostListWithTags {
        contents,
        tag_map,
    })
}

pub fn render_list(config: &Config, posts: PostListWithTags, cur_page: u32) -> io::Result<String> {
    let tag_map = posts.tag_map;
    let mut contents = posts.contents;

    // Sort tags by frequency reversed
    let mut tag_list: Vec<(String, i32)> = tag_map.into_iter().map(|(k, v)| { (k, v) }).collect();
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_last() {
        let posts = list_post_files(&PathBuf::from("res/posts"), "index.md").unwrap();
        for post in posts {
            println!("{:?}", &post);
        }
    }
}