use std::io;
use std::io::ErrorKind;

use regex::Regex;

use crate::content::Content;
use crate::content::content_file::ContentFile;
use crate::content::content_format::ContentFormat;
use crate::content::content_renderer::{ContentRenderer, ImagePrefix, RenderOptions};
use crate::content::parsing_utils::{extract_content, parse_texted_header, parse_title_html};

pub struct HtmlRenderer {}

impl ContentRenderer for HtmlRenderer {
    fn render(content_file: &ContentFile, render_options: RenderOptions) -> io::Result<Content> {
        if content_file.format != ContentFormat::Html {
            return Err(io::Error::new(ErrorKind::InvalidData, format!("Unsupported format: {:?}", content_file.format)));
        }

        let link = content_file.link.clone();
        // The header is the same for HTML, but always living in an HTML comment block in the top of the file
        let (header, lines, maybe_line) = parse_texted_header(&content_file.file_path, content_file.raw_content.lines())?;
        let (title, lines, _title_line) = parse_title_html(lines, maybe_line);
        let content = extract_content(lines, &render_options);

        let rendered = match render_options {
            RenderOptions::PreviewOnly(ImagePrefix(prefix)) => Self::change_images(&prefix, &content),
            RenderOptions::FullContent => content,
        };

        Ok(Content {
            header,
            link,
            title,
            rendered,
        })
    }
}

impl HtmlRenderer {
    fn change_images(prefix: &str, html: &str) -> String {
        let prefix = if prefix.ends_with("/") {
            prefix.to_string()
        } else {
            format!("{}/", prefix)
        };

        // Regex pattern to match img tags
        let img_regex = Regex::new(r#"<img[^>]*src="([^"]*)"[^>]*>"#).unwrap();

        // Replace img tags with prefixed src attribute
        let result = img_regex.replace_all(html, |captures: &regex::Captures| {
            let src = captures.get(1).unwrap().as_str();
            let prefixed_src = if src.contains("://") {
                format!(r#"{}"#, src)
            } else {
                format!(r#"{}{}"#, prefix, src)
            };
            captures.get(0).unwrap().as_str().replace(src, &prefixed_src)
        });

        result.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::test_data::POST_DATA_HTML;

    use super::*;

    #[test]
    fn test_full_content_html() {
        let file_name = PathBuf::from("posts/20200522_how_to_write_a_code_reviewindex.md");
        let content = ContentFile {
            link: "".to_string(),
            file_path: file_name,
            format: ContentFormat::Html,
            raw_content: POST_DATA_HTML.to_string(),
        };
        let content = HtmlRenderer::render(&content, RenderOptions::FullContent).unwrap();
        assert_eq!(content.rendered, r##"<p>How to be a great software engineer?</p>
<p>Someone asked me this question today and I didn’t have an answer. After thinking for a while, I came up with a list of what I try to do myself.</p>
<p>Disclaimer: I don't think I am a great engineer, but I would love to have listened to that myself when I started my career, over 20 years ago.</p>
<p>I will divide this in parts, non-technical and technical</p>

<!-- more -->

<h2>Non technical</h2>
<h3>Have a honest image of yourself</h3>
<p>You finished university and learned a lot. You solved many hard problems. It's common to think you are awesome and the smartest person in the planet. Some day in your life, you will find that you are not and that there are many developers much better than you. Not in capacity, but in wisdom and knowledge. <strong>The earlier you find that, the better.</strong> This will drive you to improve yourself as you now recognize better your weakest points.</p>
<h3>The awesome thing you learned doesn't solve all the problems</h3>
<p>The less knowledge you have, the more you will feel that something awesome you learned is the solution for everything. <strong>There is no Saint Graal</strong>. Always search for alternatives, even if they don't look good. The more you know, the more you will see the problems of new trends and concepts and you will be able to choose the best solution for the problem you need to solve</p>
"##)
    }

    #[test]
    fn test_change_images() {
        let html = r#"<html>
<body>
    <img src="image1.jpg">
    <img some="212" src="image2.jpg">
    <img style="asd" src="image3.jpg" type="ddd">
    <img style="asd" src="http://not-change/image4.jpg" type="ddd">
    <img style="asd" src="https://not-change/image5.jpg" type="ddd">
    <img src="ftp://not-change/image5.jpg">
</body>
</html>"#;

        let prefixed_html = HtmlRenderer::change_images("view/post_name", html);
        assert_eq!(prefixed_html, r#"<html>
<body>
    <img src="view/post_name/image1.jpg">
    <img some="212" src="view/post_name/image2.jpg">
    <img style="asd" src="view/post_name/image3.jpg" type="ddd">
    <img style="asd" src="http://not-change/image4.jpg" type="ddd">
    <img style="asd" src="https://not-change/image5.jpg" type="ddd">
    <img src="ftp://not-change/image5.jpg">
</body>
</html>"#);
    }

    #[test]
    fn test_change_images_no_image() {
        let html = r#"<html>
<body>
    <span>some text</span>
</body>
</html>"#;

        let prefixed_html = HtmlRenderer::change_images("view/post_name/", html);
        assert_eq!(prefixed_html, r#"<html>
<body>
    <span>some text</span>
</body>
</html>"#);
    }
}
