pub mod captcha;
pub mod store;

use crate::text_utils::format_date_time;

pub use captcha::{CaptchaChallenge, CaptchaStore};
pub use store::{Comment, CommentStore};

pub const MAX_AUTHOR_LEN: usize = 80;
pub const MAX_BODY_LEN: usize = 4000;

pub struct CommentView {
    pub author: String,
    pub date: String,
    pub time: String,
    pub body_html: String,
}

pub struct CommentsContext {
    pub enabled: bool,
    pub comments: Vec<CommentView>,
    pub comment_count: i64,
    pub captcha_id: String,
    pub captcha_question: String,
    pub captcha_svg: String,
    pub error: String,
    pub posted: bool,
}

impl CommentsContext {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            comments: vec![],
            comment_count: 0,
            captcha_id: String::new(),
            captcha_question: String::new(),
            captcha_svg: String::new(),
            error: String::new(),
            posted: false,
        }
    }

    pub fn from_comments(
        comments: Vec<Comment>,
        captcha: CaptchaChallenge,
        error: String,
        posted: bool,
    ) -> Self {
        let views: Vec<CommentView> = comments.iter().map(Comment::to_view).collect();
        let comment_count = views.len() as i64;
        Self {
            enabled: true,
            comments: views,
            comment_count,
            captcha_id: captcha.id,
            captcha_question: captcha.question,
            captcha_svg: captcha.svg,
            error,
            posted,
        }
    }
}

impl Comment {
    pub fn to_view(&self) -> CommentView {
        let (date, time) = format_date_time(&self.date);
        CommentView {
            author: self.author.clone(),
            date,
            time,
            body_html: format_comment_body(&self.body),
        }
    }
}

pub fn format_comment_body(body: &str) -> String {
    escape_html(body)
        .replace("\r\n", "<br>\n")
        .replace('\n', "<br>\n")
        .replace('\r', "<br>\n")
}

pub fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

pub fn comment_error_message(code: &str) -> String {
    match code {
        "captcha" => "The captcha answer is incorrect. Please try again.".to_string(),
        "empty" => "Name and comment are required.".to_string(),
        "too_long" => "The comment or name is too long.".to_string(),
        "invalid" => "The comment could not be saved.".to_string(),
        _ => "Could not publish the comment.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_html() {
        assert_eq!(
            escape_html("<script>alert(\"x\")</script>"),
            "&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;"
        );
    }

    #[test]
    fn test_format_comment_body_newlines() {
        let html = format_comment_body("hello\nworld");
        assert_eq!(html, "hello<br>\nworld");
    }

    #[test]
    fn test_format_comment_body_escapes_then_breaks() {
        let html = format_comment_body("<b>hi</b>\nthere");
        assert_eq!(html, "&lt;b&gt;hi&lt;/b&gt;<br>\nthere");
    }
}
