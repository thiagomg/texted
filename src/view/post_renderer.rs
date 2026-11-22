use std::io;
use std::io::ErrorKind;

use ramhorns::Template;

use crate::comments::{CommentView, CommentsContext};
use crate::content::Content;
use crate::text_utils::format_date_time;

#[derive(ramhorns::Content)]
struct ViewTag<'a> {
    tag: &'a str,
    url: String,
}

#[derive(ramhorns::Content)]
struct ViewComment<'a> {
    author: &'a str,
    date: &'a str,
    time: &'a str,
    body_html: &'a str,
}

#[derive(ramhorns::Content)]
struct ViewItem<'a> {
    #[ramhorns(flatten)]
    route: &'a crate::routes::RouteContext,
    errors: Vec<String>,
    id: &'a str,
    link: &'a str,
    author: &'a str,
    tags: &'a Vec<ViewTag<'a>>,
    date: &'a str,
    time: &'a str,
    post_title: &'a str,
    post_content: &'a str,
    comments_enabled: bool,
    comments: Vec<ViewComment<'a>>,
    comment_count: i64,
    captcha_id: &'a str,
    captcha_question: &'a str,
    captcha_svg: &'a str,
    comment_error: &'a str,
    comment_posted: bool,
}

pub struct PostRenderer<'a> {
    pub template: Template<'a>,
}

impl PostRenderer<'_> {
    pub fn new(view_tpl_src: &str) -> io::Result<PostRenderer<'_>> {
        let template = match Template::new(view_tpl_src) {
            Ok(x) => x,
            Err(e) => {
                return Err(io::Error::new(
                    ErrorKind::InvalidInput,
                    format!("Error parsing post view template: {}", e),
                ));
            }
        };

        Ok(PostRenderer { template })
    }

    #[cfg(test)]
    pub fn render(&self, content: &Content, comments: &CommentsContext) -> String {
        self.render_route(content, comments, &crate::routes::RouteContext::default())
    }

    pub fn render_route(
        &self,
        content: &Content,
        comments: &CommentsContext,
        route: &crate::routes::RouteContext,
    ) -> String {
        let tags: &Vec<ViewTag> = &content
            .header
            .tags
            .iter()
            .map(|t| ViewTag {
                tag: t.as_str(),
                url: if route.listing_url.is_empty() {
                    String::new()
                } else {
                    crate::routes::tag_url(&route.listing_url, t)
                },
            })
            .collect();
        let (date, time) = format_date_time(&content.header.date);
        let comment_views: Vec<ViewComment> =
            comments.comments.iter().map(to_view_comment).collect();
        let rendered_page = self.template.render(&ViewItem {
            route,
            errors: vec![],
            id: content.header.id.0.as_str(),
            link: content.link.as_str(),
            author: content.header.author.as_str(),
            tags,
            date: date.as_str(),
            time: time.as_str(),
            post_title: content.title.as_str(),
            post_content: content.rendered.as_str(),
            comments_enabled: comments.enabled,
            comments: comment_views,
            comment_count: comments.comment_count,
            captcha_id: comments.captcha_id.as_str(),
            captcha_question: comments.captcha_question.as_str(),
            captcha_svg: comments.captcha_svg.as_str(),
            comment_error: comments.error.as_str(),
            comment_posted: comments.posted,
        });

        rendered_page
    }
}

fn to_view_comment(comment: &CommentView) -> ViewComment<'_> {
    ViewComment {
        author: comment.author.as_str(),
        date: comment.date.as_str(),
        time: comment.time.as_str(),
        body_html: comment.body_html.as_str(),
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
            link: "my-post".to_string(),
            title: "<post-title>".to_string(),
            rendered: "<post-content>".to_string(),
        };
        let res = post_renderer.render(&content, &crate::comments::CommentsContext::disabled());
        assert_eq!(
            res,
            r##"
TITLE=[<post-title>]
AUTHOR=[&lt;Thiago&gt;]
DATE=[2024-01-02]
TIME=[03:04:05]
TAGS=[(&lt;rust&gt;)(programming)]
POST_CONTENT=[<post-content>]"##
        );
    }

    #[test]
    fn render_view_with_comments() {
        let template_src = r##"
LINK=[{{link}}]
COMMENTS_ENABLED=[{{comments_enabled}}]
COUNT=[{{comment_count}}]
{{#comments}}COMMENT={{author}}/{{date}}/{{{body_html}}}
{{/comments}}
CAPTCHA=[{{captcha_question}}]
ERROR=[{{#comment_error}}{{comment_error}}{{/comment_error}}]
POSTED=[{{comment_posted}}]
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
                author: "Thiago".to_string(),
                tags: vec![],
            },
            link: "my-post".to_string(),
            title: "title".to_string(),
            rendered: "body".to_string(),
        };
        let comments = crate::comments::CommentsContext {
            enabled: true,
            comments: vec![crate::comments::CommentView {
                author: "<Ada>".to_string(),
                date: "2026-04-09".to_string(),
                time: "12:00:00".to_string(),
                body_html: "Nice<br>\npost".to_string(),
            }],
            comment_count: 1,
            captcha_id: "cap-id".to_string(),
            captcha_question: "What is 1 + 2?".to_string(),
            captcha_svg: "<svg></svg>".to_string(),
            error: "The captcha answer is incorrect. Please try again.".to_string(),
            posted: false,
        };
        let res = post_renderer.render(&content, &comments);
        assert_eq!(
            res,
            r##"
LINK=[my-post]
COMMENTS_ENABLED=[true]
COUNT=[1]
COMMENT=&lt;Ada&gt;/2026-04-09/Nice<br>
post

CAPTCHA=[What is 1 + 2?]
ERROR=[The captcha answer is incorrect. Please try again.]
POSTED=[false]"##
        );
    }
}
