//! Factory for creating storage backends
//!
//! This module provides a simple factory for creating storage backends with
//! optional observability features.

use crate::{
    backends::memory::MemoryStorage,
    config::{MemoryConfig, StorageConfig},
    error::{Result, StorageError},
    observable::{ObservableStorageBuilder, MonitoringConfig},
    traits::Storage,
    StorageBuilder,
};
use std::sync::Arc;

/// Storage backend type selector
#[derive(Debug, Clone)]
pub enum StorageBackend {
    /// In-memory storage (for testing and development)
    Memory {
        /// Initial capacity for the memory storage
        capacity: Option<usize>,
    },
    #[cfg(feature = "rocksdb")]
    /// RocksDB persistent storage
    RocksDb {
        /// Database path
        path: String,
    },
    #[cfg(feature = "sled")]
    /// Sled embedded database
    Sled {
        /// Database path
        path: String,
    },
    #[cfg(feature = "sqlite")]
    /// SQLite database
    Sqlite {
        /// Database path (or ":memory:" for in-memory)
        path: String,
        /// Connection pool size
        pool_size: u32,
    },
    #[cfg(feature = "redis")]
    /// Redis distributed storage
    Redis {
        /// Redis connection URL
        url: String,
        /// Key prefix
        prefix: Option<String>,
    },
}

/// Factory for creating storage backends
pub struct StorageFactory;

impl StorageFactory {
    /// Create a storage backend with optional observability
    pub async fn create(
        backend: StorageBackend,
        observable: bool,
    ) -> Result<Arc<dyn Storage<Error = StorageError>>> {
        let storage = Self::create_base(backend).await?;
        
        if observable {
            let observable_storage = ObservableStorageBuilder::new()
                .build(storage.clone());
            Ok(Arc::new(observable_storage) as Arc<dyn Storage<Error = StorageError>>)
        } else {
            Ok(storage)
        }
    }

    /// Create a base storage backend without observability
    async fn create_base(
        backend: StorageBackend,
    ) -> Result<Arc<dyn Storage<Error = StorageError>>> {
        match backend {
            StorageBackend::Memory { capacity } => {
                let storage = match capacity {
                    Some(cap) => MemoryStorage::with_capacity(cap),
                    None => MemoryStorage::new(MemoryConfig::default()),
                };
                Ok(Arc::new(storage))
            }
            #[cfg(feature = "rocksdb")]
            StorageBackend::RocksDb { path } => {
                use crate::backends::rocksdb::{RocksDbStorage, RocksDbConfig};
                let config = RocksDbConfig { path };
                let storage = RocksDbStorage::new(config)?;
                Ok(Arc::new(storage))
            }
            #[cfg(feature = "sled")]
            StorageBackend::Sled { path } => {
                use crate::backends::sled::{SledStorage, SledConfig};
                let config = SledConfig { path };
                let storage = SledStorage::new(config)?;
                Ok(Arc::new(storage))
            }
            #[cfg(feature = "sqlite")]
            StorageBackend::Sqlite { path, pool_size } => {
                use crate::backends::sqlite::{SqliteStorage, SqliteConfig};
                let config = SqliteConfig { 
                    path: path.clone(),
                    pool_size,
                };
                let storage = SqliteStorage::new(config).await?;
                Ok(Arc::new(storage))
            }
            #[cfg(feature = "redis")]
            StorageBackend::Redis { url, prefix } => {
                use crate::backends::redis::{RedisStorage, RedisConfig};
                let config = RedisConfig { url, prefix };
                let storage = RedisStorage::new(config).await?;
                Ok(Arc::new(storage))
            }
            #[allow(unreachable_patterns)]
            _ => {
                Err(StorageError::Config(
                    "Storage backend not enabled in features".to_string(),
                ))
            }
        }
    }

    /// Create storage from a configuration enum
    pub async fn from_config(
        config: StorageConfig,
    ) -> Result<Arc<dyn Storage<Error = StorageError>>> {
        let builder = StorageBuilder::new(config);
        builder.build().await
    }
}

/// Builder pattern for creating storage with advanced options
pub struct AdvancedStorageBuilder {
    backend: StorageBackend,
    observable: bool,
    monitoring_config: Option<MonitoringConfig>,
}

impl AdvancedStorageBuilder {
    /// Create a new builder with the specified backend
    pub fn new(backend: StorageBackend) -> Self {
        Self {
            backend,
            observable: false,
            monitoring_config: None,
        }
    }

    /// Enable observability
    pub fn with_observability(mut self) -> Self {
        self.observable = true;
        self
    }

    /// Set custom monitoring configuration
    pub fn with_monitoring_config(mut self, config: MonitoringConfig) -> Self {
        self.observable = true;
        self.monitoring_config = Some(config);
        self
    }

    /// Build the storage instance
    pub async fn build(self) -> Result<Arc<dyn Storage<Error = StorageError>>> {
        let storage = StorageFactory::create_base(self.backend).await?;
        
        if self.observable {
            let mut builder = ObservableStorageBuilder::new();
            if let Some(config) = self.monitoring_config {
                builder = builder.with_monitoring_config(config);
            }
            Ok(Arc::new(builder.build(storage)) as Arc<dyn Storage<Error = StorageError>>)
        } else {
            Ok(storage)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_factory_create_memory() {
        let storage = StorageFactory::create(
            StorageBackend::Memory { capacity: Some(100) },
            false,
        )
        .await
        .unwrap();

        // Test basic operations
        storage.put(b"key", b"value").await.unwrap();
        let value = storage.get(b"key").await.unwrap();
        assert_eq!(value, Some(bytes::Bytes::from("value")));
    }

    #[tokio::test]
    async fn test_factory_create_observable() {
        let storage = StorageFactory::create(
            StorageBackend::Memory { capacity: None },
            true,
        )
        .await
        .unwrap();

        // Test operations work with observable wrapper
        storage.put(b"test", b"data").await.unwrap();
        assert!(storage.exists(b"test").await.unwrap());
    }

    #[tokio::test]
    async fn test_advanced_builder() {
        let storage = AdvancedStorageBuilder::new(StorageBackend::Memory { capacity: Some(50) })
            .with_observability()
            .build()
            .await
            .unwrap();

        // Test operations
        storage.put(b"test", b"data").await.unwrap();
        assert!(storage.exists(b"test").await.unwrap());
    }
}