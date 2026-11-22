use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::config::Config;
use crate::content::content_renderer::{BreakTag, MaxLineCount, PreviewOptions};
use crate::post_list::{PostList, PostListType};
use crate::query_string::QueryString;
use anyhow::Result;
use chrono::{Datelike, NaiveDate, Utc};
use ntex::web::HttpRequest;
use ramhorns::Template;

#[derive(ramhorns::Content)]
struct IndexPage {
    years_developing: i64,
    post_count: i64,
    days_since_started: i64,
}

#[derive(Debug, PartialEq)]
pub struct PostLink {
    pub post_name: String,
    pub post_path: PathBuf,
}

pub fn list_post_files(root_dir: &Path, post_file: &PostListType) -> Result<Vec<PostLink>> {
    let root_dir = root_dir.to_path_buf();
    let post_list = PostList {
        root_dir,
        post_file: post_file.clone(),
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

pub fn read_template(tpl_dir: &Path, file_name: &str) -> io::Result<String> {
    let full_path = tpl_dir.join(file_name);
    fs::read_to_string(full_path)
}

pub fn render_index(
    req: HttpRequest,
    num_of_posts: usize,
    tpl_dir: &Path,
    activity_start_year: i32,
    blog_start_date: NaiveDate,
) -> io::Result<String> {
    let index_tpl_src: String = match read_template(tpl_dir, "index.tpl") {
        Ok(s) => s,
        Err(e) => {
            return Err(io::Error::new(
                ErrorKind::InvalidInput,
                format!("Error loading index template: {}", e),
            ));
        }
    };

    let index_tpl = match Template::new(index_tpl_src) {
        Ok(x) => x,
        Err(e) => {
            return Err(io::Error::new(
                ErrorKind::InvalidInput,
                format!("Error parsing index template: {}", e),
            ));
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

pub fn get_cur_page(req: HttpRequest) -> u32 {
    if let Some(query_str) = req.uri().query() {
        let qs = QueryString::from(query_str);
        qs.get_page()
    } else {
        1
    }
}

pub fn get_preview_option(config: &Config) -> PreviewOptions {
    let mut max_line_count = None;
    let mut tag = "<!-- more -->";

    if let Some(ref line_tag) = config.defaults.summary_line_tag {
        tag = line_tag;
    }
    if let Some(ref line_count) = config.defaults.summary_line_count {
        max_line_count = Some(MaxLineCount(*line_count));
    }

    PreviewOptions {
        max_line_count,
        tag_based: BreakTag(tag.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_last() {
        let list_type = PostListType::IndexBaseName("index".to_string());
        let mut posts = list_post_files(&PathBuf::from("res/posts"), &list_type).unwrap();
        posts.sort_by(|a, b| a.post_name.cmp(&b.post_name));

        let mut expected = vec![
            PostLink {
                post_name: "html_post_with_image".to_string(),
                post_path: PathBuf::from("res/posts/html_post_with_image/index.html"),
            },
            PostLink {
                post_name: "post_with_image".to_string(),
                post_path: PathBuf::from("res/posts/post_with_image/index.md"),
            },
            PostLink {
                post_name: "html_post".to_string(),
                post_path: PathBuf::from("res/posts/html_post.htm"),
            },
            PostLink {
                post_name: "post_without_images".to_string(),
                post_path: PathBuf::from("res/posts/post_without_images.md"),
            },
            PostLink {
                post_name: "new_post".to_string(),
                post_path: PathBuf::from("res/posts/new_post.md"),
            },
        ];
        expected.sort_by(|a, b| a.post_name.cmp(&b.post_name));
        assert_eq!(expected, posts);
    }
}
