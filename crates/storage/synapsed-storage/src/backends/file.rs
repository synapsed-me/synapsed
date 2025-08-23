//! Simple file-based storage backend

use crate::{error::Result, traits::Storage, StorageError};
use async_trait::async_trait;
use bytes::Bytes;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

/// Simple file-based storage using a single JSON file
#[derive(Clone)]
pub struct FileStorage {
    path: PathBuf,
    cache: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
}

impl FileStorage {
    /// Create new file storage
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| StorageError::Io(e))?;
        }
        
        let storage = Self {
            path,
            cache: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Load existing data if file exists
        if storage.path.exists() {
            let data = std::fs::read_to_string(&storage.path)
                .map_err(|e| StorageError::Io(e))?;
                
            if !data.is_empty() {
                let map: HashMap<String, String> = serde_json::from_str(&data)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                    
                let mut cache = storage.cache.blocking_write();
                for (k, v) in map {
                    cache.insert(
                        hex::decode(&k).unwrap_or_else(|_| k.into_bytes()),
                        hex::decode(&v).unwrap_or_else(|_| v.into_bytes()),
                    );
                }
            }
        }
        
        Ok(storage)
    }
    
    /// Save cache to file
    async fn save(&self) -> Result<()> {
        let cache = self.cache.read().await;
        
        // Convert to hex strings for JSON serialization
        let map: HashMap<String, String> = cache
            .iter()
            .map(|(k, v)| (hex::encode(k), hex::encode(v)))
            .collect();
            
        let json = serde_json::to_string_pretty(&map)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
            
        fs::write(&self.path, json).await
            .map_err(|e| StorageError::Io(e))?;
            
        Ok(())
    }
}

#[async_trait]
impl Storage for FileStorage {
    type Error = StorageError;
    
    async fn get(&self, key: &[u8]) -> Result<Option<Bytes>> {
        let cache = self.cache.read().await;
        Ok(cache.get(key).map(|v| Bytes::copy_from_slice(v)))
    }
    
    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        {
            let mut cache = self.cache.write().await;
            cache.insert(key.to_vec(), value.to_vec());
        }
        self.save().await
    }
    
    async fn delete(&self, key: &[u8]) -> Result<()> {
        {
            let mut cache = self.cache.write().await;
            cache.remove(key);
        }
        self.save().await
    }
    
    async fn exists(&self, key: &[u8]) -> Result<bool> {
        let cache = self.cache.read().await;
        Ok(cache.contains_key(key))
    }
    
    async fn flush(&self) -> Result<()> {
        self.save().await
    }
    
    async fn list(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>> {
        let cache = self.cache.read().await;
        let keys: Vec<Vec<u8>> = cache
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        Ok(keys)
    }
}