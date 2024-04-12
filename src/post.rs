use fmt::Display;
use std::fmt::Formatter;
use std::{fmt, fs, io};
use std::io::ErrorKind;
use std::path::PathBuf;
use std::str::Lines;
use chrono::NaiveDateTime;
use lazy_static::lazy_static;
use regex::Regex;
use crate::text_utils::parse_date_time;

#[derive(Debug)]
pub struct Header {
    pub file_name: PathBuf,
    pub id: String,
    pub date: NaiveDateTime,
    pub author: String,
    pub tags: Vec<String>,
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

        Self::from_string(file_name, &lines, header_only)
    }

    pub fn from_string(file_name: &PathBuf, content: &String, header_only: bool) -> io::Result<Post> {
        let (header, lines, maybe_line) = Self::parse_texted_header(file_name, content.lines())?;
        let (title, lines, _title_line) = Self::parse_title(lines, maybe_line);
        let content = Self::parse_content(header_only, lines);

        Ok(Post {
            header,
            title,
            content,
        })
    }

    fn parse_content(header_only: bool, mut lines: Lines) -> String {
        let content = if header_only {
            let mut content = String::new();
            while let Some(line) = lines.next() {
                if line.contains("<!-- more -->") {
                    break;
                }
                content.push_str(line);
                content.push('\n');
            }
            content
        } else {
            let mut content = String::new();
            while let Some(line) = lines.next() {
                content.push_str(line);
                content.push('\n');
            }
            content
        };
        content
    }

    fn parse_texted_header<'a>(file_name: &PathBuf, lines: Lines<'a>) -> io::Result<(Header, Lines<'a>, Option<&'a str>)> {
        let mut id: String = "".to_string();
        let mut date: String = "".to_string();
        let mut author: String = "".to_string();
        let mut tags: String = "".to_string();

        let mut lines = lines.clone();
        let mut maybe_line = lines.next();

        // Skip optional HTML comment in the beginning
        let mut start_with_comment = false;

        loop {
            if let Some(line) = maybe_line {
                let line = line.trim();

                // Empty lines are ok
                if line.is_empty() {
                    maybe_line = lines.next();
                    continue;
                }

                if line == "<!--" {
                    maybe_line = lines.next();
                    start_with_comment = true;
                }
                break;
            } else {
                break;
            }
        }

        loop {
            if let Some(line) = maybe_line {
                if line.is_empty() {
                    maybe_line = lines.next();
                    continue;
                }

                let (key, val) = match Self::extract_texted_header(line) {
                    None => break,
                    Some((k, v)) => (k, v),
                };

                match key {
                    "ID" => id = val.to_string(),
                    "DATE" => date = val.to_string(),
                    "AUTHOR" => author = val.to_string(),
                    "TAGS" => tags = val.to_string(),
                    _ => {}
                }
            } else {
                break;
            }
            maybe_line = lines.next();
        }

        if start_with_comment {
            // Let's find the end of the comment
            loop {
                if let Some(line) = maybe_line {
                    let line = line.trim();

                    // Empty lines are ok.
                    if line.is_empty() {
                        maybe_line = lines.next();
                        continue;
                    }

                    if line == "-->" {
                        break;
                    }
                } else {
                    panic!("End of comment in the header is missing - file={}", file_name.to_str().unwrap());
                }

                maybe_line = lines.next();
            }
        }


        if id.is_empty() && date.is_empty() && author.is_empty() && tags.is_empty() {
            return Err(io::Error::new(ErrorKind::InvalidData, format!("Invalid texted header")));
        }

        let tags = Self::extract_tags(&tags);
        let date = match parse_date_time(&date) {
            Ok(d) => Ok(d),
            Err(e) => {
                Err(io::Error::new(ErrorKind::InvalidData, format!("{} - file={}", e, file_name.to_str().unwrap())))
            }
        }?;

        let header = Header {
            file_name: file_name.clone(),
            id,
            date,
            author,
            tags,
        };

        Ok((header, lines, maybe_line))
    }

    fn parse_title<'a>(lines: Lines<'a>, mut maybe_line: Option<&'a str>) -> (String, Lines<'a>, Option<&'a str>) {
        let mut lines = lines;
        let title = loop {
            if let Some(line) = maybe_line {
                if line.starts_with("# ") {
                    let title = line[2..line.len()].to_string();
                    break title;
                }
            } else {
                let title = "".to_string();
                break title;
            }
            maybe_line = lines.next();
        };
        return (title, lines, maybe_line);
    }

    fn extract_texted_header(line: &str) -> Option<(&str, &str)> {
        lazy_static! {
            static ref HEADER_REGEX : Regex = Regex::new(
                r"\[(?P<key>\w+)\]: # \((?P<value>.+)\)"
            ).unwrap();
            //                 r"(?P<key>\w+): (?P<value>.+)"
        }
        Self::extract_header(line, &HEADER_REGEX)
    }

    fn extract_jekyll_header(line: &str) -> Option<(&str, &str)> {
        lazy_static! {
            static ref HEADER_REGEX : Regex = Regex::new(
                r"(?P<key>\w+): (?P<value>.+)"
            ).unwrap();
        }
        Self::extract_header(line, &HEADER_REGEX)
    }

    fn extract_header<'a>(line: &'a str, header_regex: &Regex) -> Option<(&'a str, &'a str)> {
        let res = header_regex.captures(line).and_then(|cap| {
            let key = cap.name("key").map(|key| key.as_str());
            let val = cap.name("value").map(|key| key.as_str());
            match (key, val) {
                (Some(key), Some(val)) => Some((key, val)),
                _ => None
            }
        });

        res
    }

    fn extract_tags(tags_str: &str) -> Vec<String> {
        let x = tags_str.split(' ')
            .filter(|x| !x.is_empty())
            .map(|s| s.to_string())
            .collect();
        x
    }
}


#[cfg(test)]
mod tests {
    use crate::test_data::POST_DATA;
    use super::*;

    #[test]
    fn test_extract_texted_header() {
        let res = Post::extract_texted_header("[ID]: # (a63bd715-a3fe-4788-b0e1-2a3153778544)");
        assert_eq!(res, Some(("ID", "a63bd715-a3fe-4788-b0e1-2a3153778544")));
        let res = Post::extract_texted_header("[DATE]: # (2022-04-02 12:05:00.000)");
        assert_eq!(res, Some(("DATE", "2022-04-02 12:05:00.000")));
        let res = Post::extract_texted_header("[AUTHOR]: # (thiago)");
        assert_eq!(res, Some(("AUTHOR", "thiago")));
        let res = Post::extract_texted_header("[TAGS]: # (rust something-else)");
        assert_eq!(res, Some(("TAGS", "rust something-else")));

        let res = Post::extract_texted_header("[AUTHOR]: (thiago)");
        assert!(res.is_none());
    }

    #[test]
    fn test_extract_jekyll_header() {
        let res = Post::extract_jekyll_header("layout: default");
        assert_eq!(res, Some(("layout", "default")));
        let res = Post::extract_jekyll_header("title: Hitbox");
        assert_eq!(res, Some(("title", "Hitbox")));
        let res = Post::extract_jekyll_header("parent: API Documentation");
        assert_eq!(res, Some(("parent", "API Documentation")));
    }

    #[test]
    fn test_from_string() {
        let file_name = PathBuf::from("posts/20200522_how_to_write_a_code_review/index.md");
        let post = Post::from_string(&file_name, &POST_DATA.to_string(), true).unwrap();
        println!("{}", post);
        assert_eq!(post.content, r##"How to be a great software engineer?

Someone asked me this question today and I didn’t have an answer. After thinking for a while, I came up with a list of what I try to do myself.

Disclaimer: I don't think I am a great engineer, but I would love to have listened to that myself when I started my career, over 20 years ago.

I will divide this in parts, non-technical and technical

"##);
    }

    #[test]
    fn test_extract_tags() {
        let tags_str = "one two three   four";
        let tags = Post::extract_tags(tags_str);
        assert_eq!(tags, ["one", "two", "three", "four"]);
    }

    #[test]
    fn test_lines() {
        let file_name = PathBuf::from("posts/20200522_how_to_write_a_code_review/index.md");
        let content = r##"

<!--

[ID]: # (21c1e9ad-4ebb-4168-a543-fbf77cc35a85)

[DATE]: # (2024-02-12 22:54:00.000)

[AUTHOR]: # (thiago)

-->        "##;

        let (header, lines, next_line) = Post::parse_texted_header(&file_name, content.lines()).unwrap();
        println!("{:?}", &header);
    }

    #[test]
    fn test_lines_2() {
        let file_name = PathBuf::from("posts/20200522_how_to_write_a_code_review/index.md");
        let content = r##"
---
layout: default
title: Hitbox
parent: API Documentation
---

## Hitbox
"##;

        let result = Post::parse_texted_header(&file_name, content.lines());
        println!("{:?}", &result);
    }
}
