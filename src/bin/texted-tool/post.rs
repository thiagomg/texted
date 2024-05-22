use std::fmt::Write;
use std::fs::{create_dir, File};
use std::path::PathBuf;

use chrono::{NaiveDate, Utc};
use uuid::Uuid;

use texted::util::os_helper::get_name;

use crate::{PostArgs, PostOutput};

fn get_author(args: &PostArgs) -> String {
    if let Some(ref name) = args.name {
        return name.clone();
    }

    get_name()
}

fn render_header(id: &str, name: &str, date: &str, title: Option<&str>) -> String {
    let mut buf = String::new();

    let _ = writeln!(&mut buf, "<!--");
    let _ = writeln!(&mut buf, "[ID]: # ({})", id);
    let _ = writeln!(&mut buf, "[DATE]: # ({})", date);
    let _ = writeln!(&mut buf, "[AUTHOR]: # ({})", name);
    let _ = writeln!(&mut buf, "[TAGS]: # ()");
    let _ = writeln!(&mut buf, "-->");
    let _ = writeln!(&mut buf, "");
    if let Some(title) = title {
        let _ = writeln!(&mut buf, "# {}", title);
    } else {
        let _ = writeln!(&mut buf, "# Replace with title");
    }
    let _ = writeln!(&mut buf, "");
    buf
}

fn render_body() -> String {
    let mut buf = String::new();

    let _ = writeln!(&mut buf, "This is a body example");
    let _ = writeln!(&mut buf, "Please remove it and replace with your content");
    let _ = writeln!(&mut buf, "");
    let _ = writeln!(&mut buf, "<!-- more -->");
    let _ = writeln!(&mut buf, "");
    let _ = writeln!(&mut buf, "And this is the rest of your post");

    buf
}

fn post_url_from_title(title: &str, date: &NaiveDate) -> String {
    let alpha_chars: String = title.chars()
        .filter(|&c| c.is_alphanumeric() || c == ' ')
        .map(|c| if c == ' ' { '_' } else { c })
        .map(|c| c.to_ascii_lowercase())
        .collect();

    let mut url = String::new();
    let mut prev_char = None;

    for c in alpha_chars.chars() {
        if c != '_' || prev_char != Some('_') {
            url.push(c);
        }
        prev_char = Some(c);
    }

    let url = unidecode::unidecode(&url);
    let date = date.format("%Y%m%d");

    format!("{}_{}", date, url)
}

pub fn post_cmd(args: PostArgs) {
    let id = Uuid::new_v4().to_string();
    let name = get_author(&args);
    let date = Utc::now();
    let date_str = date.format("%Y-%m-%d %H:%M:%S.000");

    let req_title = match args.output {
        PostOutput::Stdout => false,
        _ => true,
    };

    if req_title && args.title.is_none() {
        eprintln!("For file and dir outputs, title is required");
        return;
    }

    let header = render_header(&id, &name, &date_str.to_string(), args.title.as_deref());
    let body = render_body();

    match args.output {
        PostOutput::Stdout => {
            println!("{}", header);
            println!("{}", body);
        }
        PostOutput::File => {
            use std::io::Write;
            let file_name = post_url_from_title(args.title.unwrap().as_str(), &date.date_naive());
            let file_name = format!("{}.md", file_name);
            println!("Creating file {}", file_name);
            let mut file = File::create(&file_name).unwrap();
            file.write_all(header.as_bytes()).unwrap();
            file.write_all(body.as_bytes()).unwrap();
        }
        PostOutput::Dir => {
            use std::io::Write;
            let dir_name = post_url_from_title(args.title.unwrap().as_str(), &date.date_naive());
            let file_name = "index.md";
            let full_path: PathBuf = PathBuf::from(&dir_name).join(file_name);
            println!("Creating dir post {}", full_path.to_str().unwrap());
            create_dir(dir_name).expect("Error create directory");
            let mut file = File::create(&full_path).unwrap();
            file.write_all(header.as_bytes()).unwrap();
            file.write_all(body.as_bytes()).unwrap();
        }
    };
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use crate::test_data::POST_DATA;

    use super::*;

    #[test]
    fn test_happy_case() {
        let id = "bcfc427f-f9f3-4442-bfc2-deca95db96d5";
        let name = "Thiago";
        let date = "2024-02-27 06:20:53.000";
        let title = "This is a title";
        let header = render_header(&id, &name, &date, Some(title));

        assert_eq!(header, POST_DATA);
    }

    #[test]
    fn test_url_from_title() {
        //let date = Utc::now();
        let date = NaiveDate::from_ymd_opt(2024, 02, 29).unwrap();
        let title = "Post title of mine ábaco - dir2";
        let url = post_url_from_title(title, &date);
        assert_eq!(url, "20240229_post_title_of_mine_abaco_dir2");
    }
}