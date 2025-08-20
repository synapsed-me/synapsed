//! LRU cache implementation module

// Re-export lru crate types
pub use lru::{LruCache as BaseLruCache};

use std::sync::Arc;
use parking_lot::RwLock;
use crate::error::Result;
use bytes::Bytes;
use async_trait::async_trait;

/// Thread-safe LRU cache wrapper
pub struct LruCache<K, V> {
    inner: Arc<RwLock<BaseLruCache<K, V>>>,
}

impl<K: std::hash::Hash + Eq, V> LruCache<K, V> {
    /// Create a new LRU cache with the specified capacity
    pub fn new(capacity: std::num::NonZeroUsize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(BaseLruCache::new(capacity))),
        }
    }

    /// Insert a key-value pair
    pub fn put(&self, key: K, value: V) -> Option<V> {
        self.inner.write().put(key, value)
    }

    /// Get a value by key
    pub fn get(&self, key: &K) -> Option<V> 
    where
        V: Clone,
    {
        self.inner.write().get(key).cloned()
    }

    /// Remove a value by key
    pub fn remove(&self, key: &K) -> Option<V> {
        self.inner.write().pop(key)
    }

    /// Get the current size
    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.inner.write().clear()
    }
}

/// LRU cache backend implementation for the cache layer
pub struct LruCacheBackend {
    cache: LruCache<Vec<u8>, Bytes>,
}

impl LruCacheBackend {
    /// Create a new LRU cache backend
    pub fn new(capacity: std::num::NonZeroUsize) -> Self {
        Self {
            cache: LruCache::new(capacity),
        }
    }
}

#[async_trait]
impl super::CacheBackend for LruCacheBackend {
    async fn get(&self, key: &[u8]) -> Result<Option<Bytes>> {
        Ok(self.cache.get(&key.to_vec()))
    }

    async fn put(&self, key: &[u8], value: Bytes) -> Result<()> {
        self.cache.put(key.to_vec(), value);
        Ok(())
    }

    async fn remove(&self, key: &[u8]) -> Result<()> {
        let _ = self.cache.remove(&key.to_vec());
        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        self.cache.clear();
        Ok(())
    }
}