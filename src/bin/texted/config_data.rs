use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use chrono::{DateTime, Local};

const CONFIG_SAMPLE: &str = r#"[personal]
activity_start_year = 2000
blog_start_date = {{TODAY}}

# For the file locations, If you want it to be relative to the executable directory
# use ${exe_dir}/location
[paths]
template_dir = "template"
public_dir = "public"
posts_dir = "posts"
pages_dir = "pages"

# Default file name if using directory instead of files
[defaults]
index_base_name = "index"
page_size = 10
cache_enabled = true

[server]
address = "0.0.0.0"
port = 8001
"#;

pub(crate) fn write_sample_cfg(file_path: &PathBuf) {
    let mut file = File::create(&file_path).unwrap();
    file.write_all(get_sample_cfg().as_bytes()).unwrap();
}

fn get_sample_cfg() -> String {
    let current_local: DateTime<Local> = Local::now();
    let today = current_local.format("%Y-%m-%d").to_string();
    CONFIG_SAMPLE.replace("{{TODAY}}", &today)
}
