use std::collections::HashMap;
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

    pub fn get_post(&self, link: &str) -> Option<Arc<T>>
    {
        let key = format!("post-{}", link);
        self.get(key.as_str())
    }

    pub fn get_page(&self, link: &str) -> Option<Arc<T>>
    {
        let key = format!("page-{}", link);
        self.get(key.as_str())
    }

    pub fn add_post(&mut self, link: &str, content: T, expire_after: Expire) -> Arc<T>
    {
        let key = format!("post-{}", link);
        self.add(key, content, expire_after)
    }

    pub fn add_page(&mut self, link: &str, content: T, expire_after: Expire) -> Arc<T>
    {
        let key = format!("page-{}", link);
        self.add(key, content, expire_after)
    }

    pub fn get(&self, key: &str) -> Option<Arc<T>> {
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
    use chrono::Duration;
    use std::sync::Arc;

    #[test]
    fn test_content_cache_new() {
        let cache: ContentCache<String> = ContentCache::new();
        assert!(cache.cache.is_some());
        assert_eq!(*cache.lock.read().unwrap(), 0);
    }

    #[test]
    fn test_content_cache_non_caching() {
        let cache: ContentCache<String> = ContentCache::non_caching();
        assert!(cache.cache.is_none());
        assert_eq!(*cache.lock.read().unwrap(), 0);
    }

    #[test]
    fn test_add_and_get_post_never_expires() {
        let mut cache = ContentCache::new();
        let content = "Hello, world!".to_string();
        let link = "test-post";

        let cached_content = cache.add_post(link, content.clone(), Expire::Never);
        assert_eq!(Arc::strong_count(&cached_content), 2); // Check if Arc count increased

        let retrieved_content = cache.get_post(link).unwrap();
        assert_eq!(retrieved_content.as_ref(), &content);
    }

    #[test]
    fn test_add_and_get_post_expires_after() {
        let mut cache = ContentCache::new();
        let content = "Hello, world!".to_string();
        let link = "test-post-expiring";

        let expire_after = Expire::After(Duration::milliseconds(100));
        let cached_content = cache.add_post(link, content.clone(), expire_after);

        // Retrieve immediately, should not expire yet
        let retrieved_content = cache.get_post(link).unwrap();
        assert_eq!(cached_content.as_ref(), &content);
        assert_eq!(retrieved_content.as_ref(), &content);

        // Simulate passage of time and expire the cache
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert!(cache.get_post(link).is_none());
    }

    #[test]
    fn test_add_and_get_page() {
        let mut cache = ContentCache::new();
        let content = "Page content".to_string();
        let link = "test-page";

        let cached_content = cache.add_page(link, content.clone(), Expire::Never);
        assert_eq!(Arc::strong_count(&cached_content), 2);

        let retrieved_content = cache.get_page(link).unwrap();
        assert_eq!(retrieved_content.as_ref(), &content);
    }

    #[test]
    fn test_get_nonexistent_key() {
        let cache: ContentCache<String> = ContentCache::new();
        assert!(cache.get("nonexistent-key").is_none());
    }

    #[test]
    fn test_non_caching_behavior() {
        let mut cache: ContentCache<String> = ContentCache::non_caching();
        let content = "Non-cached content".to_string();
        let link = "non-cached-post";

        let cached_content = cache.add_post(link, content.clone(), Expire::Never);
        assert_eq!(Arc::strong_count(&cached_content), 1); // No caching, so only one Arc count

        assert!(cache.get_post(link).is_none());
    }
}
