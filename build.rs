use std::fs::File;
use std::path::{Path, PathBuf};
use std::{env, fs, io};

use flate2::write::GzEncoder;
use flate2::Compression;

fn get_file_name(path: &Path) -> PathBuf {
    let last = path.file_name().unwrap().to_str().unwrap();
    let file_name = format!("{}.tar.gz", last);
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = PathBuf::from(out_dir);
    out_dir.join(file_name)
}

fn delete_old_archive(path: &Path) {
    let archive_path = get_file_name(path);
    let _ = fs::remove_file(archive_path);
}

fn compress_dir(path: &Path) -> io::Result<()> {
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
