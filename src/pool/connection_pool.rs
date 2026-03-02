use crate::device::atx_client::AtxClient;
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

/// LRU connection pool for AtxClient instances.
/// Replaces Python `SmartConnectionPool` in aio_pool.py.
#[derive(Clone)]
pub struct ConnectionPool {
    cache: Cache<String, Arc<AtxClient>>,
}

#[allow(dead_code)]
impl ConnectionPool {
    pub fn new(max_size: u64, idle_timeout: Duration) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_size)
            .time_to_idle(idle_timeout)
            .build();

        Self { cache }
    }

    /// Get or create an AtxClient connection for the given device.
    pub async fn get_or_create(
        &self,
        udid: &str,
        ip: &str,
        port: i64,
    ) -> Arc<AtxClient> {
        let key = udid.to_string();

        if let Some(client) = self.cache.get(&key).await {
            return client;
        }

        let client = Arc::new(AtxClient::new(ip, port, udid));
        self.cache.insert(key, client.clone()).await;
        client
    }

    /// Remove a connection from the pool.
    pub async fn remove(&self, udid: &str) {
        self.cache.invalidate(udid).await;
    }

    /// Get pool statistics.
    pub fn stats(&self) -> serde_json::Value {
        serde_json::json!({
            "total": self.cache.entry_count(),
            "max_size": self.cache.policy().max_capacity().unwrap_or(0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_or_create_new() {
        let pool = ConnectionPool::new(10, Duration::from_secs(60));
        let client = pool.get_or_create("device1", "192.168.1.1", 7912).await;
        assert_eq!(client.udid, "device1");
    }

    #[tokio::test]
    async fn test_get_or_create_cached() {
        let pool = ConnectionPool::new(10, Duration::from_secs(60));
        let c1 = pool.get_or_create("device1", "192.168.1.1", 7912).await;
        let c2 = pool.get_or_create("device1", "192.168.1.1", 7912).await;
        // Same Arc should be returned
        assert!(Arc::ptr_eq(&c1, &c2));
    }

    #[tokio::test]
    async fn test_remove() {
        let pool = ConnectionPool::new(10, Duration::from_secs(60));
        let c1 = pool.get_or_create("device1", "192.168.1.1", 7912).await;
        pool.remove("device1").await;
        let c2 = pool.get_or_create("device1", "192.168.1.1", 7912).await;
        // After remove, new Arc should be created
        assert!(!Arc::ptr_eq(&c1, &c2));
    }

    #[tokio::test]
    async fn test_stats() {
        let pool = ConnectionPool::new(100, Duration::from_secs(60));
        pool.get_or_create("d1", "10.0.0.1", 7912).await;
        // Allow moka to process
        tokio::time::sleep(Duration::from_millis(50)).await;
        let stats = pool.stats();
        assert!(stats.get("total").is_some());
        assert_eq!(stats["max_size"], 100);
    }
}
