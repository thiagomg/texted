use std::io;
use std::io::ErrorKind;

use ramhorns::Template;

use crate::content::Content;
use crate::text_utils::format_date_time;

#[derive(ramhorns::Content)]
struct ViewTag<'a> {
    tag: &'a str,
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

pub struct PostRenderer<'a> {
    pub template: Template<'a>,
}

impl PostRenderer<'_> {
    pub fn new(view_tpl_src: &str) -> io::Result<PostRenderer> {
        let template = match Template::new(view_tpl_src) {
            Ok(x) => x,
            Err(e) => {
                return Err(io::Error::new(ErrorKind::InvalidInput, format!("Error parsing post view template: {}", e)));
            }
        };

        Ok(PostRenderer {
            template,
        })
    }

    pub fn render(&self, content: &Content) -> String {
        let ref tags: Vec<ViewTag> = content.header.tags.iter().map(|t| ViewTag { tag: t.as_str() }).collect();
        let (date, time) = format_date_time(&content.header.date);
        let rendered_page = self.template.render(&ViewItem {
            errors: vec![],
            id: content.header.id.0.as_str(),
            author: content.header.author.as_str(),
            tags,
            date: date.as_str(),
            time: time.as_str(),
            post_title: content.title.as_str(),
            post_content: content.rendered.as_str(),
        });

        rendered_page
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    use crate::content::{Content, ContentHeader, PostId};
    use crate::view::post_renderer::PostRenderer;

    #[test]
    fn render_view() {
        let template_src = r##"
TITLE=[{{{post_title}}}]
AUTHOR=[{{author}}]
DATE=[{{date}}]
TIME=[{{time}}]
TAGS=[{{#tags}}({{tag}}){{/tags}}]
POST_CONTENT=[{{{post_content}}}]
"##;
        let post_renderer = PostRenderer::new(template_src).unwrap();
        let content = Content {
            header: ContentHeader {
                file_name: PathBuf::from("file_name.md"),
                id: PostId("post-id".to_string()),
                date: NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(2024, 01, 02).unwrap(),
                    NaiveTime::from_hms_opt(3, 4, 5).unwrap(),
                ),
                author: "<Thiago>".to_string(),
                tags: vec!["<rust>".to_string(), "programming".to_string()],
            },
            link: "".to_string(),
            title: "<post-title>".to_string(),
            rendered: "<post-content>".to_string(),
        };
        let res = post_renderer.render(&content);
        assert_eq!(res, r##"
TITLE=[<post-title>]
AUTHOR=[&lt;Thiago&gt;]
DATE=[2024-01-02]
TIME=[03:04:05]
TAGS=[(&lt;rust&gt;)(programming)]
POST_CONTENT=[<post-content>]"##);
    }
}