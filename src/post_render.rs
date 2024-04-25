use std::io;
use std::io::ErrorKind;

use markdown::Options;

use crate::content::parsing_utils::remove_comments;

pub fn change_images(post_name: &str, md_post: &str) -> String {
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
                parsed_string.push_str(")");

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

pub fn render_post(md_text: &str, img_prefix: Option<&str>) -> io::Result<String> {
    let buf = remove_comments(md_text)?;
    let buf = if let Some(img_prefix) = img_prefix {
        change_images(img_prefix, buf.as_str())
    } else {
        buf
    };
    match markdown::to_html_with_options(buf.as_str(), &Options::gfm()) {
        Ok(x) => Ok(x),
        Err(e) => Err(io::Error::new(ErrorKind::InvalidInput, e.reason.as_str())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_prefix_empty_label() {
        let content = "something![](url.png)osadiosa";
        let parsed = change_images("post_name/", content);
        assert_eq!(parsed, "something![](post_name/url.png)osadiosa");
        let parsed = change_images("post_name", content);
        assert_eq!(parsed, "something![](post_name/url.png)osadiosa");
    }

    #[test]
    fn test_add_prefix() {
        let content = "something![imagelabel](url.png)osadiosa";
        let parsed = change_images("post_name/", content);
        assert_eq!(parsed, "something![imagelabel](post_name/url.png)osadiosa");
        let parsed = change_images("post_name", content);
        assert_eq!(parsed, "something![imagelabel](post_name/url.png)osadiosa");
    }
}
