use std::io;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::str::Lines;

use markdown::Options;

use crate::content::content_file::ContentFile;
use crate::content::content_format::ContentFormat;
use crate::content::content_renderer::RenderOptions;
use crate::content::parsing_utils::{extract_content, generate_header_from_file, parse_texted_header, parse_title_markdown, remove_comments};
use crate::content::{Content, ContentHeader};

pub struct TextedRenderer {}

impl TextedRenderer {
    pub fn render(content_file: &ContentFile, render_options: RenderOptions) -> io::Result<Content> {
        if content_file.format != ContentFormat::Texted {
            return Err(io::Error::new(ErrorKind::InvalidData, format!("Unsupported format: {:?}", content_file.format)));
        }

        let link = content_file.link.clone();
        let (header, lines, maybe_line) = Self::parse_markdown_header(&content_file.file_path, content_file.raw_content.lines())?;
        let (title, lines, _title_line) = parse_title_markdown(lines, maybe_line);
        let content = extract_content(lines, &render_options);

        let prefix: Option<&str> = match render_options {
            RenderOptions::PreviewOnly(ref _preview_opt, ref img_prefix) => Some(img_prefix.0.as_str()),
            RenderOptions::FullContent => None,
        };
        let rendered = Self::render_markdown(&content, prefix)?;

        Ok(Content {
            header,
            link,
            title,
            rendered,
        })
    }

    pub fn parse_markdown_header<'a>(file_name: &PathBuf, lines: Lines<'a>) -> io::Result<(ContentHeader, Lines<'a>, Option<&'a str>)> {
        let lines_clone = lines.clone();
        match parse_texted_header(file_name, lines) {
            Ok((header, lines, maybe_line)) => {
                Ok((header, lines, maybe_line))
            }
            Err(_) => {
                // Let's try generating from the file, if no header is available
                let header = generate_header_from_file(file_name)?;
                Ok((header, lines_clone, Some("")))
            }
        }
    }
    // parse_texted_header

    fn render_markdown(md_text: &str, img_prefix: Option<&str>) -> io::Result<String> {
        let buf = remove_comments(md_text)?;
        let buf = if let Some(img_prefix) = img_prefix {
            Self::change_images(img_prefix, buf.as_str())
        } else {
            buf
        };
        match markdown::to_html_with_options(buf.as_str(), &Options::gfm()) {
            Ok(x) => Ok(x),
            Err(e) => Err(io::Error::new(ErrorKind::InvalidInput, e.reason.as_str())),
        }
    }

    fn change_images(post_name: &str, md_post: &str) -> String {
        let mut parsed_string = String::new();
        let mut remaining_input = md_post;

        while let Some(text_start) = remaining_input.find("![") {
            let text_end = text_start + 2;

            // Append the text before the ![ pattern
            parsed_string.push_str(&remaining_input[0..text_end]);

            // Update the remaining input to start after the current ![ pattern
            remaining_input = &remaining_input[text_end..];

            // Look for the closing bracket of the link text
            if let Some(link_end) = remaining_input.find("](") {
                let link_text = &remaining_input[..link_end];
                let url_start = link_end + 2; // For ](

                let url_start_slice = &remaining_input[url_start..];
                if let Some(url_end) = url_start_slice.find(')') {
                    let url = &remaining_input[url_start..url_end + url_start];
                    let prefixed_url = if post_name.ends_with("/") {
                        format!("{}{}", post_name, url)
                    } else {
                        format!("{}/{}", post_name, url)
                    };


                    // Append the modified link to the parsed string
                    parsed_string.push_str(link_text);
                    parsed_string.push_str("](");
                    parsed_string.push_str(&prefixed_url);
                    parsed_string.push(')');

                    // Update the remaining input to start after the current URL
                    let remaining = &url_start_slice[url_end + 1..];
                    remaining_input = remaining;
                }
            }
        }

        // Append any remaining text after the last pattern
        parsed_string.push_str(remaining_input);

        parsed_string
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::content::content_renderer::{BreakTag, ImagePrefix, PreviewOptions};
    use crate::test_data::POST_DATA_MD;

    use super::*;

    #[test]
    fn test_header_only() {
        let file_name = PathBuf::from("posts/20200522_how_to_write_a_code_review/index.md");
        let content = ContentFile {
            link: "".to_string(),
            file_path: file_name,
            format: ContentFormat::Texted,
            raw_content: POST_DATA_MD.to_string(),
        };

        let prefix = ImagePrefix { 0: "image/".to_string() };
        let preview_opt = PreviewOptions { max_line_count: None, tag_based: BreakTag("<!-- more -->".to_string()) };
        let content = TextedRenderer::render(&content, RenderOptions::PreviewOnly(preview_opt, prefix)).unwrap();
        assert_eq!(content.rendered, r##"<p>How to be a great software engineer?</p>
<p>Someone asked me this question today and I didn’t have an answer. After thinking for a while, I came up with a list of what I try to do myself.</p>
<p>Disclaimer: I don't think I am a great engineer, but I would love to have listened to that myself when I started my career, over 20 years ago.</p>
<p>I will divide this in parts, non-technical and technical</p>
"##);
    }

    #[test]
    fn test_full_content() {
        let file_name = PathBuf::from("posts/20200522_how_to_write_a_code_review/index.md");
        let content = ContentFile {
            link: "".to_string(),
            file_path: file_name,
            format: ContentFormat::Texted,
            raw_content: POST_DATA_MD.to_string(),
        };
        let content = TextedRenderer::render(&content, RenderOptions::FullContent).unwrap();
        assert_eq!(content.rendered, r##"<p>How to be a great software engineer?</p>
<p>Someone asked me this question today and I didn’t have an answer. After thinking for a while, I came up with a list of what I try to do myself.</p>
<p>Disclaimer: I don't think I am a great engineer, but I would love to have listened to that myself when I started my career, over 20 years ago.</p>
<p>I will divide this in parts, non-technical and technical</p>
<h2>Non technical</h2>
<h3>Have a honest image of yourself</h3>
<p>You finished university and learned a lot. You solved many hard problems.
It's common to think you are awesome and the smartest person in the planet.
Some day in your life, you will find that you are not and that there are many developers much better than you. Not in capacity, but in wisdom and knowledge. <strong>The earlier you find that, the better.</strong> This will drive you to improve yourself as you now recognize better your weakest points.</p>
"##)
    }
}
