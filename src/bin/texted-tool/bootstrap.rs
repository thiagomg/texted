use std::{fs, io};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use chrono::{DateTime, Local};
use regex::Regex;

use crate::BootstrapArgs;
use crate::decompress::decompress_files;

fn get_sample_cfg() -> &'static str {
    let sample_cfg = include_str!("../../../texted.toml");
    sample_cfg
}

fn write_texted_cfg(out_dir: &PathBuf) -> io::Result<()> {
    let file = File::create(out_dir.join("texted.toml"))?;
    let mut writer = BufWriter::new(file);

    let sample_cfg = get_sample_cfg();
    let sample_cfg = replace_paths(&out_dir, sample_cfg);
    let sample_cfg = replace_date(&sample_cfg);

    writer.write_all(sample_cfg.as_bytes())?;

    writer.flush()
}

fn replace_paths(prefix: &PathBuf, config_data: &str) -> String {
    let prefix = prefix.to_str().unwrap();
    let prefix = if prefix.ends_with("/") {
        prefix[0..prefix.len() - 1].to_string()
    } else {
        prefix.to_string()
    };

    // Regex pattern to match img tags
    let res_regex = Regex::new(r#"(res)/\w+"#).unwrap();

    // Replace img tags with prefixed src attribute
    let result = res_regex.replace_all(config_data, |captures: &regex::Captures| {
        let src = captures.get(1).unwrap().as_str();
        let prefixed_src = prefix.to_string();
        captures.get(0).unwrap().as_str().replace(src, &prefixed_src)
    });

    result.to_string()
}

fn get_current_date() -> String {
    let current_local: DateTime<Local> = Local::now();
    let today = current_local.format("%Y-%m-%d").to_string();
    today
}

fn replace_date(config_data: &str) -> String {
    let today = get_current_date();
    config_data.replace("2024-04-22", &today)
}

pub fn bootstrap_cmd(args: BootstrapArgs) {
    let out_path = PathBuf::from(&args.out_dir);
    let out_path = match fs::canonicalize(out_path) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error converting path to absolute: {} - {}", &args.out_dir, e);
            return;
        }
    };

    if !fs::metadata(&out_path).unwrap().is_dir() {
        eprintln!("Output path must be a directory: {}", out_path.to_str().unwrap());
        return;
    }

    if let Err(e) = decompress_files(&out_path) {
        eprintln!("Error bootstrapping: {}", e);
        return;
    };

    if let Err(e) = write_texted_cfg(&out_path) {
        eprintln!("Error writing Texted configuration: {}", e);
        return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_cfg() {
        let cfg = get_sample_cfg();
        let res = replace_paths(&PathBuf::from("/abs/path/"), cfg);
        assert!(res.contains(r##"template_dir = "/abs/path/template""##));
        assert!(res.contains(r##"public_dir = "/abs/path/public""##));
        assert!(res.contains(r##"posts_dir = "/abs/path/posts""##));
        assert!(res.contains(r##"pages_dir = "/abs/path/pages""##));
        let res = replace_date(&res);
        let blog_start_date = format!("blog_start_date = {}", get_current_date());
        assert!(res.contains(&blog_start_date));
    }
}