use fmt::Display;
use std::fmt::Formatter;
use std::{fmt, fs, io};
use std::path::PathBuf;
use lazy_static::lazy_static;
use regex::Regex;

pub struct Header {
    pub file_name: PathBuf,
    pub id: String,
    pub date: String, // TODO: convert to date time
    pub author: String,
}

pub struct Post {
    pub header: Header,
    pub title: String,
    pub content: String,
}

impl Display for Post {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "id={}, date={}, author={}\ntitle={}\ncontent:\n{}",
            self.header.id,
            self.header.date,
            self.header.author,
            self.title,
            self.content
        )
    }
}

/// Example of post
/// [ID]: # (a63bd715-a3fe-4788-b0e1-2a3153778544)
/// [DATE]: # (2022-04-02 12:05:00.000)
/// [AUTHOR]: # (thiago)
///
/// # What I learned after 20+ years of software development
impl Post {
    pub fn from(file_name: &PathBuf, header_only: bool) -> io::Result<Post> {
        let lines = fs::read_to_string(file_name)?;

        Ok(Self::from_string(file_name, &lines, header_only))
    }

    pub fn from_string(file_name: &PathBuf, content: &String, header_only: bool) -> Post {
        let mut lines = content.lines();

        let mut id: String = "".to_string();
        let mut date: String = "".to_string();
        let mut author: String = "".to_string();
        let mut title: String = "".to_string();
        let mut content: String;

        let mut maybe_line = lines.next();
        loop {
            if let Some(line) = maybe_line {
                let (key, val) = match Self::extract_header(line) {
                    None => break,
                    Some((k, v)) => (k, v),
                };

                match key {
                    "ID" => id = val.to_string(),
                    "DATE" => date = val.to_string(),
                    "AUTHOR" => author = val.to_string(),
                    _ => {}
                }
            }
            maybe_line = lines.next();
        }

        // After the header, comes the title
        loop {
            if let Some(line) = maybe_line {
                if line.starts_with("# ") {
                    title = line[2..line.len()].to_string();
                    break;
                }
            }
            maybe_line = lines.next();
        }

        if header_only {
            content = String::new();
            while let Some(line) = lines.next() {
                if line.contains("<!-- more -->") {
                    break;
                }
                content.push_str(line);
                content.push('\n');
            }
        } else {
            content = String::new();
            while let Some(line) = lines.next() {
                content.push_str(line);
                content.push('\n');
            }
        }

        Post {
            header: Header {
                file_name: file_name.clone(),
                id,
                date,
                author,
            },
            title,
            content,
        }
    }

    fn extract_header(line: &str) -> Option<(&str,&str)> {
        lazy_static! {
            static ref HEADER_REGEX : Regex = Regex::new(
                r"\[(?P<key>\w+)\]: # \((?P<value>.+)\)"
            ).unwrap();
        }

        let res = HEADER_REGEX.captures(line).and_then(|cap| {
            let key = cap.name("key").map(|key| key.as_str());
            let val = cap.name("value").map(|key| key.as_str());
            match (key, val) {
                (Some(key), Some(val)) => Some((key, val)),
                _ => None
            }
        });

        res
    }
}


#[cfg(test)]
mod tests {
    use crate::test_data::POST_DATA;
    use super::*;

    #[test]
    fn test_extract_header() {
        let res = Post::extract_header("[ID]: # (a63bd715-a3fe-4788-b0e1-2a3153778544)");
        assert_eq!(res, Some(("ID", "a63bd715-a3fe-4788-b0e1-2a3153778544")));
        let res = Post::extract_header("[DATE]: # (2022-04-02 12:05:00.000)");
        assert_eq!(res, Some(("DATE", "2022-04-02 12:05:00.000")));
        let res = Post::extract_header("[AUTHOR]: # (thiago)");
        assert_eq!(res, Some(("AUTHOR", "thiago")));

        let res = Post::extract_header("[AUTHOR]: (thiago)");
        assert!(res.is_none());
    }

    #[test]
    fn test_from_string() {
        let file_name = PathBuf::from("/home/thiago/src/texted2/posts/20200522_how_to_write_a_code_review/index.md");
        let post = Post::from_string(&file_name, &POST_DATA.to_string(), true);
        println!("{}", post);
    }
}
