use std::{env, fs, io};
use std::fs::File;
use std::path::PathBuf;

use flate2::Compression;
use flate2::write::GzEncoder;

fn get_file_name(path: &PathBuf) -> PathBuf {
    let last = path.file_name().unwrap().to_str().unwrap();
    let parent: PathBuf = path.parent().unwrap().into();
    let file_name = format!("{}.tar.gz", last);
    parent.join(file_name)
}

fn delete_old_archive(path: &PathBuf) {
    let archive_path = get_file_name(path);
    let _ = fs::remove_file(archive_path);
}

fn compress_dir(path: &PathBuf) -> io::Result<()> {
    let archive_path = get_file_name(path);
    let tar_gz = File::create(archive_path)?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = tar::Builder::new(enc);
    tar.append_dir_all(".", path)?;
    Ok(())
}

fn compress_resources() {
    let current_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let res_dir = PathBuf::from(&current_dir).join("res");
    delete_old_archive(&res_dir);
    compress_dir(&res_dir).unwrap()
}

fn main() {
    compress_resources();
}
