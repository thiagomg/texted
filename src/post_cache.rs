use std::cmp::Ordering;
use std::collections::HashMap;
use std::io;
use std::io::ErrorKind;
use std::path::PathBuf;

use chrono::NaiveDateTime;

use crate::post::{Post, PostId};

pub struct PostItem {
    pub post: Post,
    pub link: String,
}

pub struct PostCache {
    posts: HashMap<PostId, PostItem>,
    posts_summary: HashMap<PostId, PostItem>,

    // As we change those two inside, we need to make sure they are not being changed from the outside
    // To prevent mismatches
    link_to_uuid: HashMap<String, PostId>,
    post_list: Vec<(NaiveDateTime, PostId)>,

    post_file_name: String,

    tag_map: HashMap<String, u32>,
    tags: Vec<String>,
}

impl PostCache {
    pub fn new(post_file_name: &str) -> PostCache {
        PostCache {
            posts: Default::default(),
            posts_summary: Default::default(),
            link_to_uuid: Default::default(),
            post_list: Default::default(),
            post_file_name: post_file_name.to_string(),
            tag_map: Default::default(),
            tags: Default::default(),
        }
    }

    // TODO: Remove this function
    fn get_link_from_path(&self, path: &PathBuf) -> io::Result<String> {
        let post_type = if let Some(file_name) = path.file_name() {
            match file_name.to_str().unwrap() {
                x if x == self.post_file_name => 'D',
                x if x.ends_with(".md") => 'F',
                x if x.ends_with(".html") || x.ends_with(".htm") => 'F',
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

    // TODO: Move this function to Post
    pub fn get_link_from_path_2(post: &Post) -> io::Result<String> {
        let path: &PathBuf = &post.header.file_name;
        let post_file_name = path.to_str().unwrap();
        let post_type = if let Some(file_name) = path.file_name() {
            match file_name.to_str().unwrap() {
                x if x == post_file_name => 'D',
                x if x.ends_with(".md") => 'F',
                x if x.ends_with(".html") || x.ends_with(".htm") => 'F',
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

    pub fn add(&mut self, post: Post, summary: Option<Post>) -> io::Result<()> {
        let link = self.get_link_from_path(&post.header.file_name)?.to_string();

        // Used for lookups from links
        self.link_to_uuid.insert(link.clone(), post.header.id.clone());
        self.post_list.push((post.header.date.clone(), post.header.id.clone()));

        let post_item = PostItem {
            post,
            link: link.clone(),
        };

        // Update tags map
        for tag in post_item.post.header.tags.iter() {
            *self.tag_map.entry(tag.clone()).or_insert(0) += 1;
        }

        self.posts.insert(post_item.post.header.id.clone(), post_item);

        if let Some(summary) = summary {
            let summary_item = PostItem {
                post: summary,
                link,
            };
            self.posts_summary.insert(summary_item.post.header.id.clone(), summary_item);
        }

        Ok(())
    }

    pub fn sort(&mut self) {
        self.post_list.sort_by(|a, b| {
            let (da, _) = a;
            let (db, _) = b;
            db.cmp(da)
        });

        let tag_list = Self::sort_by_frequency(&self.tag_map);
        self.tags = tag_list.iter().map(|tp| tp.0.to_string()).collect();
    }

    fn sort_by_frequency(h: &HashMap::<String, u32>) -> Vec<(&str, u32)> {
        let mut freq_list: Vec<(&str, u32)> = vec![];
        for (k, v) in h.iter() {
            freq_list.push((k.as_str(), *v));
        }

        freq_list.sort_by(|a, b| {
            let (_, count_a) = a;
            let (_, count_b) = b;
            match count_a.cmp(count_b) {
                Ordering::Less => Ordering::Greater,
                Ordering::Equal => Ordering::Equal,
                Ordering::Greater => Ordering::Less,
            }
        });

        freq_list
    }

    // pub fn with_uuid(&self, uuid: &PostId) -> Option<&Post> {
    //     match self.posts.get(uuid) {
    //         None => None,
    //         Some(post_item) => Some(&post_item.post)
    //     }
    // }
    //
    // pub fn with_link(&self, link: &str) -> Option<&Post> {
    //     match self.link_to_uuid.get(link) {
    //         Some(uuid) => self.with_uuid(uuid),
    //         None => None,
    //     }
    // }

    pub fn post_list(&self) -> &Vec<(NaiveDateTime, PostId)> {
        &self.post_list
    }

    pub fn posts(&self) -> &HashMap<PostId, PostItem> {
        &self.posts
    }

    pub fn summaries(&self) -> &HashMap<PostId, PostItem> {
        &self.posts_summary
    }

    pub fn tags(&self) -> &Vec<String> {
        &self.tags
    }
}

#[cfg(test)]
mod tests {
    // #[test]
    // fn test_map() {
    //     let mut cache = HashMap::<String, u32>::new();
    //
    //     for c in "abcdefghijklkmno".chars() {
    //         *cache.entry(format!("{}", c)).or_insert(0) += 1;
    //     }
    //     for c in "fghijk".chars() {
    //         *cache.entry(format!("{}", c)).or_insert(0) += 1;
    //     }
    //     for c in "hihi".chars() {
    //         *cache.entry(format!("{}", c)).or_insert(0) += 1;
    //     }
    //
    //     let res = PostCache::sort_by_frequency(&cache);
    //     assert_eq!(res, vec![
    //         ("h", 4), ("i", 4), ("k", 3), ("f", 2), ("g", 2), ("j", 2),
    //         ("a", 1), ("l", 1), ("o", 1), ("b", 1), ("m", 1), ("d", 1),
    //         ("e", 1), ("n", 1), ("c", 1)]
    //     );
    // }
    //
    // #[test]
    // fn test_extract_link() {
    //     let cache = PostCache::new("index.md");
    //     let file_name = PathBuf::from("posts/20200522_how_to_write_a_code_review/index.md");
    //     let link = cache.get_link_from_path(&file_name).unwrap();
    //     assert_eq!(link, "20200522_how_to_write_a_code_review");
    // }
    //
    // #[test]
    // fn test_happy_case() -> io::Result<()> {
    //     let mut cache = PostCache::new("index.md");
    //     cache.add(Post {
    //         header: Header {
    //             file_name: PathBuf::from("posts/20200522_how_to_write_a_code_review/index.md"),
    //             id: PostId("cbca23f4-9cb9-11ea-a1df-83d8f0a5e3cb".to_string()),
    //             date: parse_date_time("2020-05-22 10:54:25.000").unwrap(),
    //             author: "thiago".to_string(),
    //             format: ContentFormat::Texted,
    //             tags: vec!["codereview".to_string()],
    //         },
    //         title: "How to write a Code Review".to_string(),
    //         content: "There is always those quite obvious things such as don't be a jerk. Those are not the ones I will be talking now.".to_string(),
    //     })?;
    //     cache.add(Post {
    //         header: Header {
    //             file_name: PathBuf::from("posts/20220402_what_i_learned/index.md"),
    //             id: PostId("a63bd715-a3fe-4788-b0e1-2a3153778544".to_string()),
    //             date: parse_date_time("2022-04-02 12:05:00.000").unwrap(),
    //             author: "thiago".to_string(),
    //             format: ContentFormat::Texted,
    //             tags: vec![],
    //         },
    //         title: "What I learned after 20+ years of software development".to_string(),
    //         content: "How to be a great software engineer?\n\nSomeone asked me this question today and I didn’t have an answer".to_string(),
    //     })?;
    //
    //     assert!(cache.posts.contains_key(PostId("a63bd715-a3fe-4788-b0e1-2a3153778544".to_string())));
    //     assert!(cache.posts.contains_key(PostId("cbca23f4-9cb9-11ea-a1df-83d8f0a5e3cb".to_string())));
    //
    //     assert_eq!(cache.link_to_uuid.get("20200522_how_to_write_a_code_review").unwrap(), "cbca23f4-9cb9-11ea-a1df-83d8f0a5e3cb");
    //     assert_eq!(cache.link_to_uuid.get("20220402_what_i_learned").unwrap(), "a63bd715-a3fe-4788-b0e1-2a3153778544");
    //
    //     Ok(())
    // }
}