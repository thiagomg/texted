use std::io;

use crate::content::Content;
use crate::content::content_file::ContentFile;

#[derive(Clone)]
pub struct ImagePrefix(pub String);

#[derive(Clone)]
pub struct MaxLineCount(pub i32);

#[derive(Clone)]
pub struct BreakTag(pub String);

#[derive(Clone)]
pub struct PreviewOptions {
    pub max_line_count: Option<MaxLineCount>,
    pub tag_based: BreakTag,
}

#[derive(Clone)]
pub enum RenderOptions {
    PreviewOnly(PreviewOptions, ImagePrefix),
    FullContent,
}

pub trait ContentRenderer {
    fn render(content_file: &ContentFile, render_options: RenderOptions) -> io::Result<Content>;
}
