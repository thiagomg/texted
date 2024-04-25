use std::collections::HashMap;
use std::io;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Duration, Utc};

pub struct ContentCache<T> {
    cache: Option<CacheMap<T>>,
    lock: RwLock<i32>,
}

type CacheMap<T> = HashMap<String, CacheValue<T>>;

pub enum Expire {
    Never,
    After(Duration),
}

struct CacheValue<T> {
    expire_date: DateTime<Utc>,
    value: Arc<T>,
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

    fn add(&mut self, key: String, content: T, expire_after: Expire) -> Arc<T> {
        if let Some(ref mut cache) = self.cache {
            let expire_after = match expire_after {
                Expire::Never => DateTime::<Utc>::MAX_UTC,
                Expire::After(duration) => Utc::now() + duration,
            };

            let _lock = self.lock.write().unwrap();
            cache.insert(key.clone(), CacheValue {
                expire_date: expire_after,
                value: Arc::new(content),
            });
            let item = cache.get(&key).unwrap();
            item.value.clone()
        } else {
            Arc::new(content)
        }
    }

    pub fn get_post_or<F>(&mut self, link: &str, expire_after: Expire, generator_fn: F) -> io::Result<Arc<T>>
        where
            F: FnOnce() -> io::Result<T>,
    {
        self.get_or(format!("post-{}", link), expire_after, generator_fn)
    }

    pub fn get_page_or<F>(&mut self, link: &str, expire_after: Expire, generator_fn: F) -> io::Result<Arc<T>>
        where
            F: FnOnce() -> io::Result<T>,
    {
        self.get_or(format!("page-{}", link), expire_after, generator_fn)
    }

    fn get_or<F>(&mut self, key: String, expire_after: Expire, generator_fn: F) -> io::Result<Arc<T>>
        where
            F: FnOnce() -> io::Result<T>,
    {
        let res = self.get(&key);
        if res.is_none() {
            let content = generator_fn()?;
            Ok(self.add(key.clone(), content, expire_after))
        } else {
            Ok(res.unwrap().clone())
        }
    }

    fn get(&mut self, key: &str) -> Option<Arc<T>> {
        if let Some(ref cache) = self.cache {
            let _reader = self.lock.read().unwrap();
            if let Some(cache_value) = cache.get(key) {
                let now = Utc::now();
                if now > cache_value.expire_date {
                    return None;
                }
                // if cache_value.expire_date
                return Some(cache_value.value.clone());
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
        let content = cache.get_or("post-1".to_string(), Expire::Never, || {
            Ok("post-1-content".to_string())
        });
        assert_eq!(content.unwrap().as_str(), "post-1-content");
    }

    #[test]
    fn test_get_post_or() {
        let mut cache = ContentCache::new();
        let content = cache.get_post_or("some_link", Expire::Never, || {
            Ok("post-1-content".to_string())
        });
        assert_eq!(content.unwrap().as_str(), "post-1-content");
    }

    #[test]
    fn test_get_page_or() {
        let mut cache = ContentCache::new();
        let content = cache.get_post_or("some_link", Expire::Never, || {
            Ok("post-1-content".to_string())
        });
        assert_eq!(content.unwrap().as_str(), "post-1-content");
    }

    #[test]
    fn test_get_page_or_expire() {
        let mut cache = ContentCache::new();
        let mut value = 0;
        let content = cache.get_post_or("some_link", Expire::After(Duration::milliseconds(100)), || {
            value += 1;
            Ok("post-1-content".to_string())
        });
        assert_eq!(content.unwrap().as_str(), "post-1-content");
        let content = cache.get_post_or("some_link", Expire::After(Duration::milliseconds(100)), || {
            value += 10;
            Ok("post-1-content".to_string())
        });
        assert_eq!(content.unwrap().as_str(), "post-1-content");

        std::thread::sleep(std::time::Duration::from_millis(200));
        let content = cache.get_post_or("some_link", Expire::After(Duration::milliseconds(100)), || {
            value += 100;
            Ok("post-1-content".to_string())
        });
        assert_eq!(content.unwrap().as_str(), "post-1-content");
        assert_eq!(value, 101);
    }

    #[test]
    fn test_no_cache() {
        let mut cache = ContentCache::non_caching();
        let content = cache.get_post_or("some_link", Expire::Never, || {
            Ok("post-1-content".to_string())
        });
        assert_eq!(content.unwrap().as_str(), "post-1-content");
        assert_eq!(cache.get("some_link"), None);
    }
}
