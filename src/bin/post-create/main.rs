/*
[ID]: # (dbe35a35-7e40-480f-9e7b-409e8d6d77c7)
[DATE]: # (2016-06-25 00:25:23.342)
[AUTHOR]: # (thiago)
 */

use std::fmt::Write;
use chrono::Utc;
use clap::{arg, Parser};
use uuid::Uuid;

mod test_data;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the author. If empty, OS user real name is being used
    #[arg(short, long)]
    name: Option<String>,

    #[arg(short, long)]
    title: Option<String>,
}

fn get_name(args: &Args) -> String {
    if let Some(ref name) = args.name {
        return name.clone();
    }

    let name = whoami::realname();
    if name.is_empty() {
        return whoami::username();
    }
    return name;
}

fn render_header(id: &str, name: &str, date: &str, title: Option<&str>) -> String {
    let mut buf = String::new();

    let _ = writeln!(&mut buf, "[ID]: # ({})", id);
    let _ = writeln!(&mut buf, "[DATE]: # ({})", date);
    let _ = writeln!(&mut buf, "[AUTHOR]: # ({})", name);
    let _ = writeln!(&mut buf, "");
    if let Some(title) = title {
        let _ = writeln!(&mut buf, "# {}", title);
    } else {
        let _ = writeln!(&mut buf, "# Replace with title");
    }
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

fn main() {
    let args = Args::parse();

    let id = Uuid::new_v4().to_string();
    let name = get_name(&args);
    let date = Utc::now().format("%Y-%m-%d %H:%M:%S.000");

    let header = render_header(&id, &name, &date.to_string(), args.title.as_deref());
    println!("{}", header);
    println!("{}", render_body());
}

#[cfg(test)]
mod tests {
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
}
