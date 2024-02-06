use std::collections::HashMap;
use std::io;
use std::io::ErrorKind;
use std::path::PathBuf;
use crate::post::Post;

pub struct PostCache {

    // UUID, post
    pub posts: HashMap<String, Post>,
    pub link_to_uuid: HashMap<String, String>,
    // TODO: add sorted list per date

    // TODO: Add cache for rendered post

}

impl PostCache {
    
    pub fn new() -> PostCache {
        PostCache { posts: Default::default(), link_to_uuid: Default::default() }
    }

    fn get_link_from_path(path: &PathBuf) -> io::Result<&str> {
        if let Some(file_name) = path.file_name() {
            if file_name != "index.md" {
                return Err(io::Error::new(ErrorKind::InvalidInput, "Invalid post file"));
            }
        }

        let p = path.parent().ok_or(io::Error::new(ErrorKind::InvalidInput, "Could not find post link"))?;
        match p.file_name() {
            Some(last_dir) => Ok(last_dir.to_str().unwrap()),
            None => Err(io::Error::new(ErrorKind::InvalidInput, "Invalid post link"))
        }
    }

    pub fn add(&mut self, post: Post) -> io::Result<()> {
        let link = Self::get_link_from_path(&post.header.file_name)?;
        self.link_to_uuid.insert(link.to_string(), post.header.id.clone());
        self.posts.insert(post.header.id.clone(), post);

        Ok(())
    }

    pub fn from_uuid(&self, uuid: &str) -> Option<&Post> {
        self.posts.get(uuid)
    }

    pub fn from_link(&self, link: &str) -> Option<&Post> {
        match self.link_to_uuid.get(link) {
            Some(uuid) => self.from_uuid(uuid),
            None => None,
        }
    }

}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::PathBuf;
    use crate::post::Header;
    use super::*;

    #[test]
    fn test_extract_link() {
        let file_name = PathBuf::from("/home/thiago/src/texted2/posts/20200522_how_to_write_a_code_review/index.md");
        let link = PostCache::get_link_from_path(&file_name).unwrap();
        assert_eq!(link, "20200522_how_to_write_a_code_review");
    }

    #[test]
    fn test_extract_link_error() {
        let file_name = PathBuf::from("/home/thiago/src/texted2/posts/20200522_how_to_write_a_code_review/inddex.md");
        let link = PostCache::get_link_from_path(&file_name);
        assert!(link.is_err());
    }

    #[test]
    fn test_happy_case() -> io::Result<()> {
        let mut cache = PostCache::new();
        cache.add(Post {
            header: Header {
                file_name: PathBuf::from("/home/thiago/src/texted2/posts/20200522_how_to_write_a_code_review/index.md"),
                id: "cbca23f4-9cb9-11ea-a1df-83d8f0a5e3cb".to_string(),
                date: "2020-05-22 10:54:25.000".to_string(),
                author: "thiago".to_string(),
            },
            title: "How to write a Code Review".to_string(),
            content: "There is always those quite obvious things such as don't be a jerk. Those are not the ones I will be talking now.".to_string(),
        })?;
        cache.add(Post {
            header: Header {
                file_name: PathBuf::from("/home/thiago/src/texted2/posts/20220402_what_i_learned/index.md"),
                id: "a63bd715-a3fe-4788-b0e1-2a3153778544".to_string(),
                date: "2022-04-02 12:05:00.000".to_string(),
                author: "thiago".to_string(),
            },
            title: "What I learned after 20+ years of software development".to_string(),
            content: "How to be a great software engineer?\n\nSomeone asked me this question today and I didn’t have an answer".to_string(),
        })?;

        assert!(cache.posts.contains_key("a63bd715-a3fe-4788-b0e1-2a3153778544"));
        assert!(cache.posts.contains_key("cbca23f4-9cb9-11ea-a1df-83d8f0a5e3cb"));

        assert_eq!(cache.link_to_uuid.get("20200522_how_to_write_a_code_review").unwrap(), "cbca23f4-9cb9-11ea-a1df-83d8f0a5e3cb");
        assert_eq!(cache.link_to_uuid.get("20220402_what_i_learned").unwrap(), "a63bd715-a3fe-4788-b0e1-2a3153778544");

        Ok(())
    }

}