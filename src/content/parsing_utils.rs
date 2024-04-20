use std::io;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::str::Lines;

use lazy_static::lazy_static;
use regex::Regex;

use crate::content::{ContentHeader, PostId};
use crate::content::content_renderer::RenderOptions;
use crate::text_utils::parse_date_time;

pub fn parse_texted_header<'a>(file_name: &PathBuf, lines: Lines<'a>) -> io::Result<(ContentHeader, Lines<'a>, Option<&'a str>)> {
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

            let (key, val) = match extract_texted_header(line) {
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
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("End of comment in the header is missing - file={}", file_name.to_str().unwrap()))
                );
            }

            maybe_line = lines.next();
        }
    }

    if id.is_empty() && date.is_empty() && author.is_empty() && tags.is_empty() {
        return Err(io::Error::new(ErrorKind::InvalidData, "Invalid texted header".to_string()));
    }

    let tags = extract_tags(&tags);
    let date = match parse_date_time(&date) {
        Ok(d) => Ok(d),
        Err(e) => {
            Err(io::Error::new(ErrorKind::InvalidData, format!("{} - file={}", e, file_name.to_str().unwrap())))
        }
    }?;

    let header = ContentHeader {
        file_name: file_name.clone(),
        id: PostId(id),
        date,
        author,
        tags,
    };

    Ok((header, lines, maybe_line))
}

pub fn parse_title_markdown<'a>(lines: Lines<'a>, mut maybe_line: Option<&'a str>) -> (String, Lines<'a>, Option<&'a str>) {
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

pub fn parse_title_html<'a>(lines: Lines<'a>, mut maybe_line: Option<&'a str>) -> (String, Lines<'a>, Option<&'a str>) {
    lazy_static! {
            static ref TITLE_REGEX : Regex = Regex::new(
                r"<h[12]>(?P<title>.+)</h[12]>"
            ).unwrap();
        }

    let mut lines = lines;
    let title = loop {
        if let Some(line) = maybe_line {
            if let Some(title) = TITLE_REGEX.captures(line).and_then(|cap| {
                cap.name("title").map(|v| v.as_str())
            }) {
                break title.to_string();
            }
        } else {
            let title = "".to_string();
            break title;
        }
        maybe_line = lines.next();
    };
    return (title, lines, maybe_line);
}

pub fn extract_content(mut lines: Lines, render_options: &RenderOptions) -> String {
    match render_options {
        RenderOptions::PreviewOnly(_img_prefix) => {
            let mut content = String::new();
            while let Some(line) = lines.next() {
                if line.contains("<!-- more -->") {
                    break;
                }
                content.push_str(line);
                content.push('\n');
            }
            content
        }
        RenderOptions::FullContent => {
            let mut content = String::new();
            while let Some(line) = lines.next() {
                content.push_str(line);
                content.push('\n');
            }
            content
        }
    }
}

fn extract_tags(tags_str: &str) -> Vec<String> {
    let x = tags_str.split(' ')
        .filter(|x| !x.is_empty())
        .map(|s| s.to_string())
        .collect();
    x
}

fn extract_texted_header(line: &str) -> Option<(&str, &str)> {
    lazy_static! {
            static ref HEADER_REGEX : Regex = Regex::new(r"\[(?P<key>\w+)\]: # \((?P<value>.+)\)").unwrap();
        }
    extract_header_key_val(line, &HEADER_REGEX)
}

fn extract_header_key_val<'a>(line: &'a str, header_regex: &Regex) -> Option<(&'a str, &'a str)> {
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

pub fn remove_comments(md_post: &str) -> io::Result<String> {
    let mut res: String = String::new();
    let mut slice = Some(md_post);

    let start_comment = "<!--";
    let end_comment = "-->";

    loop {
        if let Some(block) = slice {
            let maybe_start = block.find(start_comment);
            let md_buf: &str = match maybe_start {
                Some(start) => {
                    let to_render: &str = &block[0..start];

                    let next: &str = &block[(start + start_comment.len())..];
                    match next.find(end_comment) {
                        Some(end) => {
                            slice = Some(&next[(end + end_comment.len())..]);
                        }
                        None => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "Error finding end of comment",
                            ));
                        }
                    };

                    to_render
                }
                None => {
                    slice = None;
                    block
                }
            };
            res.push_str(md_buf);
        } else {
            break;
        }
    }

    Ok(res)
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    use super::*;

    #[test]
    fn test_extract_texted_header() {
        let res = extract_texted_header("[ID]: # (a63bd715-a3fe-4788-b0e1-2a3153778544)");
        assert_eq!(res, Some(("ID", "a63bd715-a3fe-4788-b0e1-2a3153778544")));
        let res = extract_texted_header("[DATE]: # (2022-04-02 12:05:00.000)");
        assert_eq!(res, Some(("DATE", "2022-04-02 12:05:00.000")));
        let res = extract_texted_header("[AUTHOR]: # (thiago)");
        assert_eq!(res, Some(("AUTHOR", "thiago")));
        let res = extract_texted_header("[TAGS]: # (rust something-else)");
        assert_eq!(res, Some(("TAGS", "rust something-else")));

        let res = extract_texted_header("[AUTHOR]: (thiago)");
        assert!(res.is_none());
    }

    #[test]
    fn test_extract_tags() {
        let tags_str = "one two three   four";
        let tags = extract_tags(tags_str);
        assert_eq!(tags, ["one", "two", "three", "four"]);
    }

    #[test]
    fn test_lines_texted() {
        let file_name = PathBuf::from("posts/20200522_how_to_write_a_code_review/index.md");
        let content = r##"

<!--

[ID]: # (21c1e9ad-4ebb-4168-a543-fbf77cc35a85)

[DATE]: # (2024-02-12 22:54:00.000)

[AUTHOR]: # (thiago)

-->        "##;

        let (header, _lines, _next_line) = parse_texted_header(&file_name, content.lines()).unwrap();
        let date = NaiveDate::from_ymd_opt(2024, 02, 12).unwrap();
        let time = NaiveTime::from_hms_opt(22, 54, 00).unwrap();
        let expected = ContentHeader {
            file_name: PathBuf::from("posts/20200522_how_to_write_a_code_review/index.md"),
            id: PostId("21c1e9ad-4ebb-4168-a543-fbf77cc35a85".to_string()),
            date: NaiveDateTime::new(date, time),
            author: "thiago".to_string(),
            tags: vec![],
        };
        assert_eq!(header, expected);
    }

    #[test]
    fn test_parse_removes_comment() {
        let content = r#"Some text.<!-- more -->Wo<!-- xyz -->rd"#;
        let res = remove_comments(content).unwrap();
        println!("[{}]", res);
        println!("-------------------");

        let content = r#"Some text.Word"#;
        let res = remove_comments(content).unwrap();
        println!("[{}]", res);
        println!("-------------------");

        let content = r#""#;
        let res = remove_comments(content).unwrap();
        println!("[{}]", res);
        println!("-------------------");

        let content = r#"<!-- more --><!-- xyz -->"#;
        let res = remove_comments(content).unwrap();
        println!("[{}]", res);
        println!("-------------------");

        let content = r#"<!-- more -->"#;
        let res = remove_comments(content).unwrap();
        println!("[{}]", res);
        println!("-------------------");
    }
}