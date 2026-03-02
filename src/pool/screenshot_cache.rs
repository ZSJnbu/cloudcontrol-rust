use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::watch;

/// Entry in the screenshot cache.
struct CacheEntry {
    data: Vec<u8>,
    created_at: Instant,
}

/// Screenshot cache with short TTL and request deduplication.
/// Replaces Python `ScreenshotCache` + `_screenshot_pending` dedup.
pub struct ScreenshotCache {
    cache: DashMap<String, CacheEntry>,
    /// Pending request deduplication: key → watch sender
    pending: DashMap<String, Arc<watch::Sender<Option<Vec<u8>>>>>,
    max_size: usize,
    ttl: Duration,
}

impl ScreenshotCache {
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            cache: DashMap::new(),
            pending: DashMap::new(),
            max_size,
            ttl,
        }
    }

    /// Get cached screenshot data if still valid.
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        if let Some(entry) = self.cache.get(key) {
            if entry.created_at.elapsed() < self.ttl {
                return Some(entry.data.clone());
            }
            drop(entry);
            self.cache.remove(key);
        }
        None
    }

    /// Store screenshot data in cache.
    pub fn set(&self, key: &str, data: Vec<u8>) {
        // Evict oldest if at capacity
        if self.cache.len() >= self.max_size {
            // Extract key first to drop the iterator and its read locks
            // before calling remove() which needs a write lock.
            let first_key = self.cache.iter().next().map(|e| e.key().clone());
            if let Some(k) = first_key {
                self.cache.remove(&k);
            }
        }

        self.cache.insert(
            key.to_string(),
            CacheEntry {
                data,
                created_at: Instant::now(),
            },
        );
    }

    /// Try to subscribe to a pending request for deduplication.
    /// Returns Some(receiver) if another request is already in-flight.
    pub fn try_subscribe(&self, key: &str) -> Option<watch::Receiver<Option<Vec<u8>>>> {
        self.pending.get(key).map(|sender| sender.subscribe())
    }

    /// Register a pending request. Returns the sender to publish the result.
    pub fn register_pending(&self, key: &str) -> Arc<watch::Sender<Option<Vec<u8>>>> {
        let (tx, _) = watch::channel(None);
        let tx = Arc::new(tx);
        self.pending.insert(key.to_string(), tx.clone());
        tx
    }

    /// Clear a pending request.
    pub fn clear_pending(&self, key: &str) {
        self.pending.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_set_and_get() {
        let cache = ScreenshotCache::new(10, Duration::from_secs(10));
        cache.set("key1", vec![1, 2, 3]);
        let result = cache.get("key1");
        assert_eq!(result, Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_cache_get_nonexistent() {
        let cache = ScreenshotCache::new(10, Duration::from_secs(10));
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_cache_ttl_expiry() {
        let cache = ScreenshotCache::new(10, Duration::from_millis(1));
        cache.set("key1", vec![1, 2, 3]);
        // Wait for TTL to expire
        std::thread::sleep(Duration::from_millis(10));
        assert!(cache.get("key1").is_none());
    }

    #[test]
    fn test_cache_max_size_eviction() {
        let cache = ScreenshotCache::new(2, Duration::from_secs(10));
        cache.set("key1", vec![1]);
        cache.set("key2", vec![2]);
        // Third insert should evict one entry
        cache.set("key3", vec![3]);
        // Total entries should not exceed max_size
        let total = cache.cache.len();
        assert!(total <= 2, "Cache should evict when full, got {} entries", total);
    }

    #[test]
    fn test_pending_register_and_subscribe() {
        let cache = ScreenshotCache::new(10, Duration::from_secs(10));
        let _sender = cache.register_pending("key1");
        // Another caller should be able to subscribe
        let rx = cache.try_subscribe("key1");
        assert!(rx.is_some(), "Should be able to subscribe to pending request");
    }

    #[test]
    fn test_pending_clear() {
        let cache = ScreenshotCache::new(10, Duration::from_secs(10));
        let _sender = cache.register_pending("key1");
        cache.clear_pending("key1");
        assert!(cache.try_subscribe("key1").is_none());
    }

    #[test]
    fn test_pending_no_existing() {
        let cache = ScreenshotCache::new(10, Duration::from_secs(10));
        assert!(cache.try_subscribe("nonexistent").is_none());
    }
}
