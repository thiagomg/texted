use crate::content::content_header::ContentHeader;

pub mod content_file;
pub mod content_renderer;
pub mod content_header;
pub mod parsing_utils;
pub mod html_renderer;
pub mod texted_renderer;

pub struct Content {
    pub header: ContentHeader,
    pub link: String,
    pub title: String,
    pub rendered: String,
}
