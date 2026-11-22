use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use spdlog::warn;
use uuid::Uuid;

use super::{MAX_AUTHOR_LEN, MAX_BODY_LEN};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub author: String,
    #[serde(with = "comment_date_format")]
    pub date: NaiveDateTime,
    pub body: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CommentError {
    Empty,
    TooLong,
    InvalidPost,
    Io(String),
}

impl CommentError {
    pub fn code(&self) -> &'static str {
        match self {
            CommentError::Empty => "empty",
            CommentError::TooLong => "too_long",
            CommentError::InvalidPost => "invalid",
            CommentError::Io(_) => "invalid",
        }
    }
}

impl From<io::Error> for CommentError {
    fn from(value: io::Error) -> Self {
        CommentError::Io(value.to_string())
    }
}

pub struct CommentStore {
    dir: PathBuf,
    lock: Mutex<()>,
}

impl CommentStore {
    pub fn new(dir: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&dir)?;
        Ok(Self {
            dir,
            lock: Mutex::new(()),
        })
    }

    pub fn load(&self, post_name: &str) -> io::Result<Vec<Comment>> {
        if !is_safe_post_name(post_name) {
            return Err(io::Error::new(ErrorKind::InvalidInput, "Invalid post name"));
        }

        let _guard = self.lock.lock().unwrap();
        let path = self.file_path(post_name);
        if !path.exists() {
            return Ok(vec![]);
        }

        let data = fs::read_to_string(&path)?;
        Ok(parse_jsonl(&path, &data))
    }

    pub fn append(&self, post_name: &str, comment: &Comment) -> Result<(), CommentError> {
        if !is_safe_post_name(post_name) {
            return Err(CommentError::InvalidPost);
        }

        let line = serde_json::to_string(comment)
            .map_err(|e| CommentError::Io(e.to_string()))?;
        if line.contains('\n') {
            return Err(CommentError::Io(
                "Serialized comment contained a newline".to_string(),
            ));
        }

        let _guard = self.lock.lock().unwrap();
        let path = self.file_path(post_name);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }

    fn file_path(&self, post_name: &str) -> PathBuf {
        self.dir.join(format!("{}.jsonl", post_name))
    }
}

impl Comment {
    pub fn new(author: String, body: String) -> Result<Self, CommentError> {
        let author = author.trim().to_string();
        let body = body.trim().to_string();

        if author.is_empty() || body.is_empty() {
            return Err(CommentError::Empty);
        }
        if author.len() > MAX_AUTHOR_LEN || body.len() > MAX_BODY_LEN {
            return Err(CommentError::TooLong);
        }
        if author.chars().any(|c| c.is_control())
            || body.chars().any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
        {
            return Err(CommentError::Empty);
        }

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            author,
            date: Local::now().naive_local(),
            body,
        })
    }
}

pub fn is_safe_post_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 200 {
        return false;
    }
    if name.contains("..") || name.contains('/') || name.contains('\\') || name.contains('\0') {
        return false;
    }
    Path::new(name).file_name().and_then(|s| s.to_str()) == Some(name)
}

fn parse_jsonl(path: &Path, data: &str) -> Vec<Comment> {
    let mut comments = vec![];
    for (i, line) in data.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<Comment>(line) {
            Ok(comment) => comments.push(comment),
            Err(e) => {
                warn!(
                    "Skipping malformed comment in {} line {}: {}",
                    path.display(),
                    i + 1,
                    e
                );
            }
        }
    }
    comments
}

mod comment_date_format {
    use chrono::NaiveDateTime;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    pub fn serialize<S>(date: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&date.format(FORMAT).to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn temp_store() -> (CommentStore, PathBuf) {
        let dir = std::env::temp_dir().join(format!("texted-comments-{}", Uuid::new_v4()));
        let store = CommentStore::new(dir.clone()).unwrap();
        (store, dir)
    }

    fn sample_comment(author: &str, body: &str) -> Comment {
        Comment {
            id: "11111111-1111-1111-1111-111111111111".to_string(),
            author: author.to_string(),
            date: NaiveDate::from_ymd_opt(2026, 4, 9)
                .unwrap()
                .and_hms_opt(12, 30, 0)
                .unwrap(),
            body: body.to_string(),
        }
    }

    #[test]
    fn test_safe_post_name() {
        assert!(is_safe_post_name("post_without_images"));
        assert!(is_safe_post_name("html-post"));
        assert!(!is_safe_post_name("../secret"));
        assert!(!is_safe_post_name("a/b"));
        assert!(!is_safe_post_name("a\\b"));
        assert!(!is_safe_post_name(""));
        assert!(!is_safe_post_name("has\0null"));
    }

    #[test]
    fn test_comment_new_validates() {
        assert!(Comment::new("Ada".to_string(), "Hello".to_string()).is_ok());
        assert_eq!(
            Comment::new("  ".to_string(), "Hello".to_string()).unwrap_err(),
            CommentError::Empty
        );
        assert_eq!(
            Comment::new("Ada".to_string(), "  ".to_string()).unwrap_err(),
            CommentError::Empty
        );
        assert_eq!(
            Comment::new("A".repeat(MAX_AUTHOR_LEN + 1), "Hello".to_string()).unwrap_err(),
            CommentError::TooLong
        );
        assert_eq!(
            Comment::new("Ada".to_string(), "B".repeat(MAX_BODY_LEN + 1)).unwrap_err(),
            CommentError::TooLong
        );
    }

    #[test]
    fn test_jsonl_roundtrip() {
        let (store, dir) = temp_store();
        let comment = sample_comment("Ada", "Nice post");
        store.append("my_post", &comment).unwrap();

        let loaded = store.load("my_post").unwrap();
        assert_eq!(loaded, vec![comment.clone()]);

        let file = fs::read_to_string(dir.join("my_post.jsonl")).unwrap();
        assert_eq!(file.lines().count(), 1);
        assert!(file.ends_with('\n'));
        let parsed: serde_json::Value = serde_json::from_str(file.trim()).unwrap();
        assert_eq!(parsed["author"], "Ada");
        assert_eq!(parsed["body"], "Nice post");
        assert_eq!(parsed["date"], "2026-04-09 12:30:00");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_jsonl_appends_one_object_per_line() {
        let (store, dir) = temp_store();
        store
            .append("post", &sample_comment("Ada", "First"))
            .unwrap();
        store
            .append(
                "post",
                &Comment {
                    id: "22222222-2222-2222-2222-222222222222".to_string(),
                    author: "Bob".to_string(),
                    date: NaiveDate::from_ymd_opt(2026, 4, 9)
                        .unwrap()
                        .and_hms_opt(12, 31, 0)
                        .unwrap(),
                    body: "Second\nwith newline".to_string(),
                },
            )
            .unwrap();

        let file = fs::read_to_string(dir.join("post.jsonl")).unwrap();
        let lines: Vec<&str> = file.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 2);
        for line in &lines {
            serde_json::from_str::<Comment>(line).unwrap();
        }

        let loaded = store.load("post").unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[1].body, "Second\nwith newline");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_load_skips_malformed_lines() {
        let (store, dir) = temp_store();
        let path = dir.join("post.jsonl");
        fs::write(
            &path,
            r#"{"id":"1","author":"Ada","date":"2026-04-09 12:30:00","body":"ok"}
not json
{"id":"2","author":"Bob","date":"2026-04-09 12:31:00","body":"also ok"}
"#,
        )
        .unwrap();

        let loaded = store.load("post").unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].author, "Ada");
        assert_eq!(loaded[1].author, "Bob");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_load_missing_file_is_empty() {
        let (store, dir) = temp_store();
        let loaded = store.load("no_such_post").unwrap();
        assert!(loaded.is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_rejects_unsafe_post_name_on_write() {
        let (store, dir) = temp_store();
        let comment = sample_comment("Ada", "Hi");
        assert_eq!(
            store.append("../escape", &comment).unwrap_err(),
            CommentError::InvalidPost
        );
        let _ = fs::remove_dir_all(dir);
    }
}
