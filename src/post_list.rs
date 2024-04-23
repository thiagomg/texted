use std::{fs, io};
use std::path::{Path, PathBuf};

use anyhow::Context;
use anyhow::Result;

pub struct PostList {
    pub root_dir: PathBuf,
    pub post_file: String,
}

impl PostList {
    pub fn retrieve_files(&self) -> io::Result<Vec<PathBuf>> {
        let mut posts = vec![];
        let entries = fs::read_dir(self.root_dir.as_path())?;
        for entry in entries {
            if let Ok(entry) = entry {
                if let Ok(file_type) = entry.file_type() {
                    if !file_type.is_file() {
                        continue;
                    }
                    let file_name = entry.file_name();
                    if let Some(file_name) = file_name.to_str() {
                        // Check if the file has a .md extension
                        if file_name.ends_with(".md") || file_name.ends_with(".html") || file_name.ends_with(".htm") {
                            posts.push(entry.path());
                        }
                    }
                }
            }
        }
        Ok(posts)
    }

    pub fn retrieve_dirs(&self) -> Result<Vec<(PathBuf, String)>> {
        // Per directory, we should have a file called post.md
        let dirs = Self::list_dirs(self.root_dir.as_path())?;
        // Filtering only the dirs with a post inside
        let post_dirs = Self::filter_dirs(&self.post_file, dirs);
        Ok(post_dirs)
    }

    fn list_dirs(posts_dir: &Path) -> Result<Vec<PathBuf>> {
        let mut dirs: Vec<PathBuf> = vec![];
        let entries = fs::read_dir(posts_dir)
            .context(format!("Could not list dirs from [{}]", posts_dir.to_str().unwrap()))?;

        for entry in entries {
            if let Ok(path) = entry {
                if let Ok(file_type) = path.file_type() {
                    if file_type.is_dir() {
                        dirs.push(path.path());
                    }
                }
            }
        }
        Ok(dirs)
    }

    fn filter_dirs(post_file: &str, dirs: Vec<PathBuf>) -> Vec<(PathBuf, String)> {
        let mut post_dirs = vec![];
        for dir in dirs {
            if let Some(file_name) = Self::contains_file(&dir, post_file).unwrap() {
                post_dirs.push((dir, file_name));
            }
        }
        post_dirs
    }

    fn contains_file(dir: &PathBuf, base_name: &str) -> Result<Option<String>> {
        let entries = fs::read_dir(dir)
            .context(format!("Could not read directory {}", dir.to_str().unwrap()))?;
        for entry in entries {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let file_name = entry.file_name().to_str().unwrap().to_string();
                if file_name.contains(base_name) {
                    return Ok(Some(file_name.to_string()));
                }
            }
        }

        Ok(None)
    }
}

