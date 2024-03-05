use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

const CONFIG_SAMPLE: &str = r#"[personal]
activity_start_year = 2000

# For the file locations, If you want it to be relative to the executable directory
# use ${exe_dir}/location
[paths]
template_dir = "res/template"
public_dir = "res/public"
posts_dir = "posts"
pages_dir = "pages"

# Default file name if using directory instead of files
[defaults]
index_file_name = "index.md"

[server]
address = "0.0.0.0"
port = 8001
"#;

pub(crate) fn write_sample_cfg(file_path: &PathBuf) {
    let mut file = File::create(&file_path).unwrap();
    file.write_all(CONFIG_SAMPLE.as_bytes()).unwrap();
}
