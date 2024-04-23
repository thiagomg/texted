use std::collections::HashMap;
use std::io;
use std::sync::{Arc, RwLock};

pub struct ContentCache<T> {
    cache: Option<HashMap<String, Arc<T>>>,
    lock: RwLock<i32>,
}

impl<T> ContentCache<T> {
    pub fn new() -> Self {
        let cache = Some(HashMap::new());
        let lock = RwLock::new(0);
        ContentCache {
            cache,
            lock,
        }
    }

    pub fn non_caching() -> Self {
        let cache = None;
        let lock = RwLock::new(0);
        ContentCache {
            cache,
            lock,
        }
    }

    fn add(&mut self, key: String, content: T) -> Arc<T> {
        if let Some(ref mut cache) = self.cache {
            let _lock = self.lock.write().unwrap();
            cache.insert(key.clone(), Arc::new(content));
            cache.get(&key).unwrap().clone()
        } else {
            Arc::new(content)
        }
    }

    pub fn get_post_or<F>(&mut self, link: &str, generator_fn: F) -> io::Result<Arc<T>>
        where
            F: FnOnce() -> io::Result<T>,
    {
        self.get_or(format!("post-{}", link), generator_fn)
    }

    pub fn get_page_or<F>(&mut self, link: &str, generator_fn: F) -> io::Result<Arc<T>>
        where
            F: FnOnce() -> io::Result<T>,
    {
        self.get_or(format!("page-{}", link), generator_fn)
    }

    fn get_or<F>(&mut self, key: String, generator_fn: F) -> io::Result<Arc<T>>
        where
            F: FnOnce() -> io::Result<T>,
    {
        let res = self.get(&key);
        if res.is_none() {
            let content = generator_fn()?;
            Ok(self.add(key.clone(), content))
        } else {
            Ok(res.clone().unwrap())
        }
    }

    fn get(&mut self, key: &str) -> Option<Arc<T>> {
        if let Some(ref cache) = self.cache {
            let _reader = self.lock.read().unwrap();
            if let Some(content) = cache.get(key) {
                return Some(content.clone());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_no_cache() {
        let mut cache = ContentCache::non_caching();
        let content = cache.get_post_or("some_link", || {
            Ok("post-1-content".to_string())
        });
        assert_eq!(content.unwrap().as_str(), "post-1-content");
        assert_eq!(cache.get("some_link"), None);
    }
}
