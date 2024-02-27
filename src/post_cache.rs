use std::collections::HashMap;
use std::io;
use std::io::ErrorKind;
use std::path::PathBuf;
use chrono::NaiveDateTime;
use crate::post::Post;

pub struct PostItem {
    pub post: Post,
    pub link: String,
}

pub struct PostCache {
    // UUID, post
    pub posts: HashMap<String, PostItem>,
    pub link_to_uuid: HashMap<String, String>,
    pub post_list: Vec<(NaiveDateTime, String)>,

    pub post_file_name: String,
    // TODO: Add cache for rendered post
}

impl PostCache {
    pub fn new(post_file_name: &str) -> PostCache {
        PostCache {
            posts: Default::default(),
            link_to_uuid: Default::default(),
            post_list: Default::default(),
            post_file_name: post_file_name.to_string(),
        }
    }

    fn get_link_from_path(&self, path: &PathBuf) -> io::Result<String> {
        let post_type = if let Some(file_name) = path.file_name() {
            match file_name.to_str().unwrap() {
                x if x == self.post_file_name => 'D',
                x if x.ends_with(".md") => 'F',
                _ => return Err(io::Error::new(ErrorKind::InvalidInput, "Invalid post file")),
            }
        } else {
            return Err(io::Error::new(ErrorKind::InvalidInput, "Invalid post path"));
        };

        if post_type == 'D' {
            // post_type = D means it's a directory with files inside
            let p = path.parent().ok_or(io::Error::new(ErrorKind::InvalidInput, "Could not find post link"))?;
            match p.file_name() {
                Some(last_dir) => Ok(last_dir.to_str().unwrap().to_string()),
                None => Err(io::Error::new(ErrorKind::InvalidInput, "Invalid post link"))
            }
        } else {
            // Post type = F means it's a file in the posts directory
            let file_without_ext = path.file_stem().unwrap();
            Ok(file_without_ext.to_str().unwrap().to_string())
        }
    }

    pub fn add(&mut self, post: Post) -> io::Result<()> {
        let link = self.get_link_from_path(&post.header.file_name)?.to_string();

        // Used for lookups from links
        self.link_to_uuid.insert(link.clone(), post.header.id.clone());
        self.post_list.push((post.header.date.clone(), post.header.id.clone()));

        let post_item = PostItem {
            post,
            link,
        };
        self.posts.insert(post_item.post.header.id.clone(), post_item);

        Ok(())
    }

    pub fn sort(&mut self) {
        self.post_list.sort_by(|a, b| {
            let (da, _) = a;
            let (db, _) = b;
            db.cmp(da)
        });
    }

    pub fn from_uuid(&self, uuid: &str) -> Option<&Post> {
        match self.posts.get(uuid) {
            None => None,
            Some(post_item) => Some(&post_item.post)
        }
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
    use crate::text_utils::parse_date_time;
    use super::*;

    #[test]
    fn test_extract_link() {
        let cache = PostCache::new("index.md");
        let file_name = PathBuf::from("posts/20200522_how_to_write_a_code_review/index.md");
        let link = cache.get_link_from_path(&file_name).unwrap();
        assert_eq!(link, "20200522_how_to_write_a_code_review");
    }

    #[test]
    fn test_happy_case() -> io::Result<()> {
        let mut cache = PostCache::new("index.md");
        cache.add(Post {
            header: Header {
                file_name: PathBuf::from("posts/20200522_how_to_write_a_code_review/index.md"),
                id: "cbca23f4-9cb9-11ea-a1df-83d8f0a5e3cb".to_string(),
                date: parse_date_time("2020-05-22 10:54:25.000").unwrap(),
                author: "thiago".to_string(),
            },
            title: "How to write a Code Review".to_string(),
            content: "There is always those quite obvious things such as don't be a jerk. Those are not the ones I will be talking now.".to_string(),
        })?;
        cache.add(Post {
            header: Header {
                file_name: PathBuf::from("posts/20220402_what_i_learned/index.md"),
                id: "a63bd715-a3fe-4788-b0e1-2a3153778544".to_string(),
                date: parse_date_time("2022-04-02 12:05:00.000").unwrap(),
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