use std::{fs, io};
use std::path::{Path, PathBuf};

pub struct PostList {
    pub root_dir: PathBuf,
    pub post_file: String,
}

impl PostList {
    pub fn retrieve_dirs(&self) -> io::Result<Vec<PathBuf>> {
        // Per directory, we should have a file called post.md
        let dirs = Self::list_dirs(self.root_dir.as_path())?;
        // Filtering only the dirs with a post inside
        let post_dirs = Self::filter_dirs(&self.post_file, dirs);
        Ok(post_dirs)
    }

    fn list_dirs(posts_dir: &Path) -> io::Result<Vec<PathBuf>> {
        let mut dirs: Vec<PathBuf> = vec![];
        let entries = fs::read_dir(posts_dir)?;
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

    fn filter_dirs(post_file: &str, dirs: Vec<PathBuf>) -> Vec<PathBuf> {
        let mut post_dirs = vec![];
        for dir in dirs {
            let post_path = dir.as_path().join(post_file);
            if post_path.exists() {
                // TODO: Validate the content of the post.md file is valid
                post_dirs.push(dir);
            }
        }
        post_dirs
    }

}

#[cfg(test)]
mod tests {
    use crate::post::Post;
    use super::*;

    #[test]
    fn test_happy_case() -> io::Result<()> {
        let root_dir = PathBuf::from("/home/thiago/src/texted2/posts");
        let post_file = "index.md".to_string();
        let post_list = PostList { root_dir, post_file };

        let dirs = post_list.retrieve_dirs()?;
        for dir in dirs.as_slice() {
            let p = dir.join("index.md");
            let post = Post::from(&p, true)?;
            println!("{}\n{}\n=-=-=-=-=-=-=-=-=-=-", p.to_str().unwrap(), post);
        }
        Ok(())
    }
}
