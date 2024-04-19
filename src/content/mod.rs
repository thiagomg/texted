use std::path::PathBuf;

use chrono::NaiveDateTime;

pub mod content_file;
pub mod content_renderer;
pub mod parsing_utils;
pub mod html_renderer;
pub mod texted_renderer;
pub mod content_format;

pub struct Content {
    pub header: ContentHeader,
    pub link: String,
    pub title: String,
    pub rendered: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContentHeader {
    pub file_name: PathBuf,
    pub id: PostId,
    pub date: NaiveDateTime,
    pub author: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct PostId(pub String);
