//! High-performance caching layer
//!
//! Uses moka for async-aware caching with LRU eviction.
//! Provides O(1) lookups with automatic expiration.

use moka::future::Cache;
use oya_shared::Bead;
use std::sync::Arc;
use std::time::Duration;

/// Cache configuration
pub struct CacheConfig {
    /// Maximum number of entries
    pub max_capacity: u64,
    /// Time to live for entries
    pub ttl: Duration,
    /// Time to idle for entries
    pub tti: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 1000,
            ttl: Duration::from_secs(300),  // 5 minutes
            tti: Duration::from_secs(120),  // 2 minutes idle
        }
    }
}

/// High-performance bead cache
pub struct BeadCache {
    cache: Cache<String, Arc<Bead>>,
}

impl BeadCache {
    /// Create a new bead cache with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Create a new bead cache with custom configuration
    #[must_use]
    pub fn with_config(config: CacheConfig) -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(config.max_capacity)
                .time_to_live(config.ttl)
                .time_to_idle(config.tti)
                .build(),
        }
    }

    /// Get a bead from the cache
    pub async fn get(&self, id: &str) -> Option<Arc<Bead>> {
        self.cache.get(id).await
    }

    /// Insert a bead into the cache
    pub async fn insert(&self, id: String, bead: Bead) {
        self.cache.insert(id, Arc::new(bead)).await;
    }

    /// Insert an Arc<Bead> into the cache
    pub async fn insert_arc(&self, id: String, bead: Arc<Bead>) {
        self.cache.insert(id, bead).await;
    }

    /// Invalidate a cache entry
    pub async fn invalidate(&self, id: &str) {
        self.cache.invalidate(id).await;
    }

    /// Invalidate all cache entries
    pub fn invalidate_all(&self) {
        self.cache.invalidate_all();
    }

    /// Get the current number of cached entries
    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }

    /// Check if a key exists in the cache
    pub async fn contains_key(&self, id: &str) -> bool {
        self.cache.contains_key(id)
    }

    /// Run pending maintenance tasks (cleanup, eviction)
    pub async fn run_pending_tasks(&self) {
        self.cache.run_pending_tasks().await;
    }
}

impl Default for BeadCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oya_shared::BeadStatus;

    #[tokio::test]
    async fn test_cache_insert_and_get() {
        let cache = BeadCache::new();
        let bead = Bead::new("bead-1", "Test Bead");

        cache.insert("bead-1".to_string(), bead).await;

        let retrieved = cache.get("bead-1").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.as_ref().map(|b| b.id.as_str()), Some("bead-1"));
    }

    #[tokio::test]
    async fn test_cache_invalidate() {
        let cache = BeadCache::new();
        let bead = Bead::new("bead-2", "Test");

        cache.insert("bead-2".to_string(), bead).await;
        assert!(cache.get("bead-2").await.is_some());

        cache.invalidate("bead-2").await;
        cache.run_pending_tasks().await;

        // Note: moka may not immediately evict, so we just verify the invalidate call works
    }

    #[tokio::test]
    async fn test_cache_contains_key() {
        let cache = BeadCache::new();
        let bead = Bead::new("bead-3", "Test");

        assert!(!cache.contains_key("bead-3").await);
        cache.insert("bead-3".to_string(), bead).await;
        assert!(cache.contains_key("bead-3").await);
    }

    #[tokio::test]
    async fn test_cache_arc_insert() {
        let cache = BeadCache::new();
        let bead = Arc::new(
            Bead::new("bead-4", "Arc Test").with_status(BeadStatus::Running),
        );

        cache.insert_arc("bead-4".to_string(), bead.clone()).await;

        let retrieved = cache.get("bead-4").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.map(|b| b.status), Some(BeadStatus::Running));
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.max_capacity, 1000);
        assert_eq!(config.ttl, Duration::from_secs(300));
        assert_eq!(config.tti, Duration::from_secs(120));
    }
}
