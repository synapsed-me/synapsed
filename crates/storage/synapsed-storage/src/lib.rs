//! Synapsed Storage - Flexible storage abstraction for the Synapsed ecosystem
//!
//! This crate provides a unified interface for various storage backends with
//! support for caching, compression, and distributed operations.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod backends;
pub mod cache;
pub mod compression;
pub mod config;
pub mod error;
pub mod traits;

#[cfg(feature = "distributed")]
pub mod distributed;

#[cfg(feature = "metrics")]
pub mod metrics;

pub mod utils;

pub mod observable;
pub mod factory;

// Re-export commonly used types
pub use config::{CacheConfig, CompressionConfig, StorageConfig};
pub use error::{Result, StorageError};
pub use traits::{
    BatchedStorage, IterableStorage, Storage, StorageIterator, StorageTransaction,
    TransactionalStorage,
};

// Re-export core types for better integration
pub use synapsed_core::{SynapsedError, SynapsedResult};
pub use synapsed_core::traits::{Observable, Configurable, Identifiable, Validatable};

// Map StorageError to SynapsedError
impl From<StorageError> for SynapsedError {
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::NotFound => SynapsedError::NotFound("Storage key not found".to_string()),
            StorageError::InvalidKey(key) => SynapsedError::InvalidInput(format!("Invalid key: {}", key)),
            StorageError::InvalidValue(value) => SynapsedError::InvalidInput(format!("Invalid value: {}", value)),
            StorageError::TransactionConflict => SynapsedError::Internal("Transaction conflict".to_string()),
            StorageError::ReadOnly => SynapsedError::PermissionDenied("Storage is read-only".to_string()),
            StorageError::Serialization(msg) => SynapsedError::Serialization(msg),
            StorageError::Deserialization(msg) => SynapsedError::Serialization(msg),
            StorageError::Unsupported(msg) => SynapsedError::InvalidInput(format!("Unsupported operation: {}", msg)),
            StorageError::Timeout => SynapsedError::Timeout("Storage operation timeout".to_string()),
            StorageError::StorageFull => SynapsedError::Storage("Storage is full".to_string()),
            StorageError::Config(msg) => SynapsedError::Configuration(msg),
            StorageError::Io(e) => SynapsedError::Internal(e.to_string()),
            StorageError::Other(msg) => SynapsedError::Internal(msg),
            // Handle nested errors
            StorageError::Backend(e) => SynapsedError::Storage(e.to_string()),
            StorageError::Compression(e) => SynapsedError::Internal(e.to_string()),
            StorageError::Cache(e) => SynapsedError::Internal(e.to_string()),
            StorageError::Network(e) => SynapsedError::Network(e.to_string()),
        }
    }
}

// Re-export observable types
pub use observable::{
    ObservableStorage,
    ObservableStorageBuilder, HealthStatus, MonitoringConfig, MetricsFormat,
};

use std::sync::Arc;

/// Main storage builder for constructing storage instances with layers
pub struct StorageBuilder {
    config: StorageConfig,
    cache_config: Option<CacheConfig>,
    compression_config: Option<CompressionConfig>,
    #[cfg(feature = "metrics")]
    metrics_config: Option<metrics::MetricsConfig>,
}

impl StorageBuilder {
    /// Create a new storage builder with the given configuration
    pub fn new(config: StorageConfig) -> Self {
        Self {
            config,
            cache_config: None,
            compression_config: None,
            #[cfg(feature = "metrics")]
            metrics_config: None,
        }
    }

    /// Add a cache layer with the specified configuration
    pub fn with_cache(mut self, config: CacheConfig) -> Self {
        self.cache_config = Some(config);
        self
    }

    /// Add a compression layer with the specified configuration
    pub fn with_compression(mut self, config: CompressionConfig) -> Self {
        self.compression_config = Some(config);
        self
    }

    /// Add metrics collection with the specified configuration
    #[cfg(feature = "metrics")]
    pub fn with_metrics(mut self, config: metrics::MetricsConfig) -> Self {
        self.metrics_config = Some(config);
        self
    }

    /// Build the storage instance with all configured layers
    pub async fn build(self) -> Result<Arc<dyn Storage<Error = StorageError>>> {
        // Build base storage backend
        let mut storage: Arc<dyn Storage<Error = StorageError>> = match self.config {
            #[cfg(feature = "memory")]
            StorageConfig::Memory(cfg) => Arc::new(backends::memory::MemoryStorage::new(cfg)),
            
            #[cfg(feature = "rocksdb")]
            StorageConfig::RocksDb(cfg) => Arc::new(backends::rocksdb::RocksDbStorage::new(cfg)?),
            
            #[cfg(feature = "sled")]
            StorageConfig::Sled(cfg) => Arc::new(backends::sled::SledStorage::new(cfg)?),
            
            #[cfg(feature = "sqlite")]
            StorageConfig::Sqlite(cfg) => Arc::new(backends::sqlite::SqliteStorage::new(cfg).await?),
            
            #[cfg(feature = "redis")]
            StorageConfig::Redis(cfg) => Arc::new(backends::redis::RedisStorage::new(cfg).await?),
            
            #[cfg(feature = "distributed")]
            StorageConfig::Distributed(cfg) => {
                Arc::new(distributed::DistributedStorage::new(cfg).await?)
            }
            
            #[allow(unreachable_patterns)]
            _ => {
                return Err(StorageError::Config(
                    "Storage backend not enabled in features".to_string(),
                ))
            }
        };

        // Apply metrics layer if configured
        #[cfg(feature = "metrics")]
        if let Some(metrics_cfg) = self.metrics_config {
            storage = Arc::new(metrics::MetricsLayer::new(storage, metrics_cfg));
        }

        // Apply compression layer if configured
        if let Some(compression_cfg) = self.compression_config {
            storage = Arc::new(compression::CompressionLayer::new(storage, compression_cfg)?);
        }

        // Apply cache layer if configured
        if let Some(cache_cfg) = self.cache_config {
            storage = Arc::new(cache::CacheLayer::new(storage, cache_cfg)?);
        }

        Ok(storage)
    }
}

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        config::{CacheConfig, CompressionConfig, StorageConfig},
        error::{Result, StorageError},
        traits::{BatchedStorage, IterableStorage, Storage},
        StorageBuilder,
    };
}