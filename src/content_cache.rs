use std::collections::HashMap;
use std::io;
use std::sync::{Arc, RwLock};

pub struct ContentCache {
    cache: HashMap<String, Arc<String>>,
    lock: RwLock<i32>,
}

impl ContentCache {
    pub fn new() -> Self {
        let cache = HashMap::new();
        let lock = RwLock::new(0);
        ContentCache {
            cache,
            lock,
        }
    }

    fn add(&mut self, key: String, content: String) {
        let _lock = self.lock.write().unwrap();
        self.cache.insert(key, Arc::new(content));
    }
    
    pub fn get_post_or<F>(&mut self, link: &str, generator_fn: F) -> io::Result<Arc<String>>
        where
            F: FnOnce() -> io::Result<String>,
    {
        self.get_or(format!("post-{}", link), generator_fn)
    }

    pub fn get_page_or<F>(&mut self, link: &str, generator_fn: F) -> io::Result<Arc<String>>
        where
            F: FnOnce() -> io::Result<String>,
    {
        self.get_or(format!("page-{}", link), generator_fn)
    }

    fn get_or<F>(&mut self, key: String, generator_fn: F) -> io::Result<Arc<String>>
        where
            F: FnOnce() -> io::Result<String>,
    {
        let res = self.get(&key);
        if res.is_none() {
            let content = generator_fn()?;
            self.add(key.clone(), content);
            Ok(self.get(&key).unwrap())
        } else {
            Ok(res.clone().unwrap())
        }
    }

    fn get(&mut self, key: &str) -> Option<Arc<String>> {
        let _reader = self.lock.read().unwrap();
        if let Some(content) = self.cache.get(key) {
            Some(content.clone())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_post() {
        let mut cache = ContentCache::new();
        let content = cache.get_post("some_link");
        assert_eq!(content, None);
        cache.add_post("some_link", "post-1-content".to_string());
        let content = cache.get_post("some_link").unwrap();
        assert_eq!(content.as_str(), "post-1-content");
    }

    #[test]
    fn test_get_or() {
        let mut cache = ContentCache::new();
        let content = cache.get_or("post-1".to_string(), || {
            Ok("post-1-content".to_string())
        });
        assert_eq!(content.unwrap().as_str(), "post-1-content");
    }

    #[test]
    fn test_get_post_or() {
        let mut cache = ContentCache::new();
        let content = cache.get_post_or("some_link", || {
            Ok("post-1-content".to_string())
        });
        assert_eq!(content.unwrap().as_str(), "post-1-content");
    }

    #[test]
    fn test_get_page_or() {
        let mut cache = ContentCache::new();
        let content = cache.get_post_or("some_link", || {
            Ok("post-1-content".to_string())
        });
        assert_eq!(content.unwrap().as_str(), "post-1-content");
    }
}
