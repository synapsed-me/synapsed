//! Example of creating a custom storage backend

use synapsed_storage::{Storage, StorageError, Result};
use async_trait::async_trait;
use bytes::Bytes;

/// Example custom backend that logs all operations
struct LoggingStorage {
    inner: synapsed_storage::backends::memory::MemoryStorage,
}

impl LoggingStorage {
    fn new() -> Self {
        use synapsed_storage::config::MemoryConfig;
        Self {
            inner: synapsed_storage::backends::memory::MemoryStorage::new(MemoryConfig {
                initial_capacity: 10 * 1024 * 1024, // 10MB
                max_memory_bytes: 0, // unlimited
            }),
        }
    }
}

#[async_trait]
impl Storage for LoggingStorage {
    type Error = StorageError;
    
    async fn get(&self, key: &[u8]) -> Result<Option<Bytes>> {
        println!("GET: {:?}", String::from_utf8_lossy(key));
        self.inner.get(key).await
    }
    
    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        println!("PUT: {:?} = {:?}", 
            String::from_utf8_lossy(key),
            String::from_utf8_lossy(value)
        );
        self.inner.put(key, value).await
    }
    
    async fn delete(&self, key: &[u8]) -> Result<()> {
        println!("DELETE: {:?}", String::from_utf8_lossy(key));
        self.inner.delete(key).await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let storage = LoggingStorage::new();
    
    // All operations will be logged
    storage.put(b"key1", b"value1").await?;
    storage.get(b"key1").await?;
    storage.delete(b"key1").await?;
    
    Ok(())
}