//! SQLite storage backend

use crate::{error::Result, traits::Storage, StorageError};
use async_trait::async_trait;
use bytes::Bytes;
use std::path::Path;

/// SQLite storage (simplified for now, would use rusqlite in production)
/// For now, this delegates to FileStorage with .db extension
pub struct SqliteStorage {
    inner: super::file::FileStorage,
}

impl SqliteStorage {
    /// Create new SQLite storage
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            inner: super::file::FileStorage::new(path)?,
        })
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    type Error = StorageError;
    
    async fn get(&self, key: &[u8]) -> Result<Option<Bytes>> {
        self.inner.get(key).await
    }
    
    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.inner.put(key, value).await
    }
    
    async fn delete(&self, key: &[u8]) -> Result<()> {
        self.inner.delete(key).await
    }
    
    async fn exists(&self, key: &[u8]) -> Result<bool> {
        self.inner.exists(key).await
    }
    
    async fn flush(&self) -> Result<()> {
        self.inner.flush().await
    }
    
    async fn list(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>> {
        self.inner.list(prefix).await
    }
}