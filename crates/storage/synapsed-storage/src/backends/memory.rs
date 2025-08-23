//! In-memory storage backend for testing and development

use crate::error::{Result, StorageError};
use crate::traits::{Storage, StorageStats};
use crate::config::MemoryConfig;
use async_trait::async_trait;
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// In-memory storage implementation
#[derive(Clone, Debug)]
pub struct MemoryStorage {
    data: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
    stats: Arc<RwLock<StorageStats>>,
    config: MemoryConfig,
}

impl MemoryStorage {
    /// Create a new memory storage instance with config
    pub fn new(config: MemoryConfig) -> Self {
        let capacity = if config.initial_capacity > 0 {
            config.initial_capacity
        } else {
            1024
        };
        
        Self {
            data: Arc::new(RwLock::new(HashMap::with_capacity(capacity))),
            stats: Arc::new(RwLock::new(StorageStats::default())),
            config,
        }
    }

    /// Create with initial capacity (legacy method)
    pub fn with_capacity(capacity: usize) -> Self {
        let config = MemoryConfig {
            initial_capacity: capacity,
            max_memory_bytes: 0,
        };
        Self::new(config)
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new(MemoryConfig::default())
    }
}

#[async_trait]
impl Storage for MemoryStorage {
    type Error = StorageError;

    async fn get(&self, key: &[u8]) -> Result<Option<Bytes>> {
        let data = self.data.read().unwrap();
        let mut stats = self.stats.write().unwrap();
        stats.get_count += 1;
        
        Ok(data.get(key).map(|v| Bytes::copy_from_slice(v)))
    }

    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let mut data = self.data.write().unwrap();
        let mut stats = self.stats.write().unwrap();
        
        // Check memory limits if configured
        if self.config.max_memory_bytes > 0 {
            let current_size = stats.size_bytes as usize;
            let new_entry_size = key.len() + value.len();
            let is_update = data.contains_key(key);
            let old_entry_size = if is_update {
                data.get(key).map(|v| key.len() + v.len()).unwrap_or(0)
            } else {
                0
            };
            
            let projected_size = current_size - old_entry_size + new_entry_size;
            if projected_size > self.config.max_memory_bytes {
                return Err(StorageError::StorageFull);
            }
        }
        
        let is_new = !data.contains_key(key);
        data.insert(key.to_vec(), value.to_vec());
        
        if is_new {
            stats.key_count += 1;
        }
        stats.put_count += 1;
        stats.size_bytes = data.values().map(|v| v.len() as u64).sum();
        
        Ok(())
    }

    async fn delete(&self, key: &[u8]) -> Result<()> {
        let mut data = self.data.write().unwrap();
        let mut stats = self.stats.write().unwrap();
        
        if data.remove(key).is_some() {
            stats.key_count = stats.key_count.saturating_sub(1);
            stats.delete_count += 1;
            stats.size_bytes = data.values().map(|v| v.len() as u64).sum();
        }
        
        Ok(())
    }

    async fn exists(&self, key: &[u8]) -> Result<bool> {
        let data = self.data.read().unwrap();
        Ok(data.contains_key(key))
    }

    async fn flush(&self) -> Result<()> {
        // No-op for memory storage
        Ok(())
    }
    
    async fn list(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>> {
        let data = self.data.read().unwrap();
        let keys: Vec<Vec<u8>> = data
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_storage_is_empty() {
        let storage = MemoryStorage::default();
        let result = storage.get(b"key").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_put_and_get() {
        let storage = MemoryStorage::default();
        
        // Put a value
        storage.put(b"key", b"value").await.unwrap();
        
        // Get the value back
        let result = storage.get(b"key").await.unwrap();
        assert_eq!(result, Some(Bytes::from("value")));
    }

    #[tokio::test]
    async fn test_delete() {
        let storage = MemoryStorage::default();
        
        // Put a value
        storage.put(b"key", b"value").await.unwrap();
        
        // Verify it exists
        assert!(storage.exists(b"key").await.unwrap());
        
        // Delete it
        storage.delete(b"key").await.unwrap();
        
        // Verify it's gone
        assert!(!storage.exists(b"key").await.unwrap());
        assert!(storage.get(b"key").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_update_existing_key() {
        let storage = MemoryStorage::default();
        
        // Put initial value
        storage.put(b"key", b"value1").await.unwrap();
        
        // Update with new value
        storage.put(b"key", b"value2").await.unwrap();
        
        // Get the updated value
        let result = storage.get(b"key").await.unwrap();
        assert_eq!(result, Some(Bytes::from("value2")));
    }

    #[tokio::test]
    async fn test_multiple_keys() {
        let storage = MemoryStorage::default();
        
        // Put multiple values
        storage.put(b"key1", b"value1").await.unwrap();
        storage.put(b"key2", b"value2").await.unwrap();
        storage.put(b"key3", b"value3").await.unwrap();
        
        // Get them back
        assert_eq!(storage.get(b"key1").await.unwrap(), Some(Bytes::from("value1")));
        assert_eq!(storage.get(b"key2").await.unwrap(), Some(Bytes::from("value2")));
        assert_eq!(storage.get(b"key3").await.unwrap(), Some(Bytes::from("value3")));
    }

    #[tokio::test]
    async fn test_delete_non_existent_key() {
        let storage = MemoryStorage::default();
        
        // Delete non-existent key should not panic
        storage.delete(b"non-existent").await.unwrap();
    }

    #[tokio::test]
    async fn test_exists() {
        let storage = MemoryStorage::default();
        
        // Check non-existent key
        assert!(!storage.exists(b"key").await.unwrap());
        
        // Put a value
        storage.put(b"key", b"value").await.unwrap();
        
        // Check it exists
        assert!(storage.exists(b"key").await.unwrap());
    }

    #[tokio::test]
    async fn test_flush() {
        let storage = MemoryStorage::default();
        
        // Flush should succeed (no-op for memory storage)
        storage.flush().await.unwrap();
    }

    #[tokio::test]
    async fn test_with_capacity() {
        let storage = MemoryStorage::with_capacity(100);
        
        // Should work the same as regular new
        storage.put(b"key", b"value").await.unwrap();
        assert_eq!(storage.get(b"key").await.unwrap(), Some(Bytes::from("value")));
    }

    #[tokio::test]
    async fn test_clone() {
        let storage1 = MemoryStorage::default();
        storage1.put(b"key", b"value").await.unwrap();
        
        // Clone should share the same data
        let storage2 = storage1.clone();
        assert_eq!(storage2.get(b"key").await.unwrap(), Some(Bytes::from("value")));
        
        // Changes in one should reflect in the other
        storage2.put(b"key2", b"value2").await.unwrap();
        assert_eq!(storage1.get(b"key2").await.unwrap(), Some(Bytes::from("value2")));
    }
}