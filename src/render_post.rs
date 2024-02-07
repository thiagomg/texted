use std::io;

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
                        },
                        None => return Err(io::Error::new(io::ErrorKind::InvalidData, "Error finding end of comment")),
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

pub fn render_post(md_text: &str) -> io::Result<String> {
    let buf = remove_comments(md_text)?;
    Ok(markdown::to_html(buf.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

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