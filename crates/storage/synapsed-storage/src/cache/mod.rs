//! Cache layer implementations

use crate::{error::Result, traits::Storage, CacheConfig, StorageError};
use async_trait::async_trait;
use bytes::Bytes;
use std::sync::Arc;

pub mod lru;
#[cfg(feature = "advanced-cache")]
pub mod distributed;

/// Cache layer that wraps a storage backend
pub struct CacheLayer<S: Storage + ?Sized> {
    inner: Arc<S>,
    cache: Arc<dyn CacheBackend>,
}

impl<S: Storage + ?Sized> CacheLayer<S> {
    /// Create a new cache layer
    pub fn new(inner: Arc<S>, config: CacheConfig) -> Result<Self> {
        let cache: Arc<dyn CacheBackend> = match config.cache_type {
            crate::config::CacheType::Lru => {
                let capacity = std::num::NonZeroUsize::new(config.max_entries)
                    .ok_or_else(|| StorageError::Config("Cache size must be greater than zero".to_string()))?;
                Arc::new(lru::LruCacheBackend::new(capacity))
            },
            _ => return Err(StorageError::Config("Unsupported cache type".to_string())),
        };

        Ok(Self { inner, cache })
    }
    
    /// Clear all entries from the cache
    pub async fn clear_cache(&self) -> Result<()> {
        self.cache.clear().await
    }
}

#[async_trait]
impl<S: Storage + ?Sized> Storage for CacheLayer<S> {
    type Error = StorageError;

    async fn get(&self, key: &[u8]) -> Result<Option<Bytes>> {
        // Check cache first
        if let Some(value) = self.cache.get(key).await? {
            return Ok(Some(value));
        }

        // Cache miss - fetch from storage
        let value = self.inner.get(key).await.map_err(|_| StorageError::Backend(
            crate::error::BackendError::Other("Backend get failed".to_string())
        ))?;

        // Update cache if value exists
        if let Some(ref val) = value {
            self.cache.put(key, val.clone()).await?;
        }

        Ok(value)
    }

    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        // Write through to storage
        self.inner.put(key, value).await.map_err(|_| StorageError::Backend(
            crate::error::BackendError::Other("Backend put failed".to_string())
        ))?;

        // Update cache
        self.cache.put(key, Bytes::copy_from_slice(value)).await?;

        Ok(())
    }

    async fn delete(&self, key: &[u8]) -> Result<()> {
        // Delete from storage
        self.inner.delete(key).await.map_err(|_| StorageError::Backend(
            crate::error::BackendError::Other("Backend delete failed".to_string())
        ))?;

        // Remove from cache
        self.cache.remove(key).await?;

        Ok(())
    }
}

/// Trait for cache backend implementations
#[async_trait]
pub(crate) trait CacheBackend: Send + Sync {
    async fn get(&self, key: &[u8]) -> Result<Option<Bytes>>;
    async fn put(&self, key: &[u8], value: Bytes) -> Result<()>;
    async fn remove(&self, key: &[u8]) -> Result<()>;
    async fn clear(&self) -> Result<()>;
}