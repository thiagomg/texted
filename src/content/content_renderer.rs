use std::io;

use crate::content::Content;
use crate::content::content_file::ContentFile;

#[derive(Clone)]
pub struct ImagePrefix(pub String);

#[derive(Clone)]
pub enum RenderOptions {
    PreviewOnly(ImagePrefix),
    FullContent,
}

pub trait ContentRenderer {
    fn render(content_file: &ContentFile, render_options: RenderOptions) -> io::Result<Content>;
}
