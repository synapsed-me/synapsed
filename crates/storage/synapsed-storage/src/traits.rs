//! Core storage traits that define the storage abstraction layer

use async_trait::async_trait;
use bytes::Bytes;
use std::error::Error;

/// Core storage trait that all backends must implement
#[async_trait]
pub trait Storage: Send + Sync {
    /// Error type for storage operations
    type Error: Error + Send + Sync + 'static;

    /// Get a value by key
    async fn get(&self, key: &[u8]) -> Result<Option<Bytes>, Self::Error>;

    /// Store a key-value pair
    async fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Self::Error>;

    /// Delete a key
    async fn delete(&self, key: &[u8]) -> Result<(), Self::Error>;

    /// Check if a key exists
    async fn exists(&self, key: &[u8]) -> Result<bool, Self::Error> {
        Ok(self.get(key).await?.is_some())
    }

    /// Flush any pending writes
    async fn flush(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Batched operations for improved throughput
#[async_trait]
pub trait BatchedStorage: Storage {
    /// Get multiple values
    async fn batch_get(&self, keys: &[&[u8]]) -> Result<Vec<Option<Bytes>>, Self::Error>;

    /// Store multiple key-value pairs
    async fn batch_put(&self, items: &[(&[u8], &[u8])]) -> Result<(), Self::Error>;

    /// Delete multiple keys
    async fn batch_delete(&self, keys: &[&[u8]]) -> Result<(), Self::Error>;
}

/// Iteration support for range queries
#[async_trait]
pub trait IterableStorage: Storage {
    /// Iterator type for this storage
    type Iterator: StorageIterator<Error = Self::Error>;

    /// Iterate over a key range
    async fn iter(
        &self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
    ) -> Result<Self::Iterator, Self::Error>;

    /// Iterate with a key prefix
    async fn prefix_iter(&self, prefix: &[u8]) -> Result<Self::Iterator, Self::Error>;
}

/// Iterator trait for storage traversal
#[async_trait]
pub trait StorageIterator: Send {
    /// Error type for iterator operations
    type Error: Error + Send + Sync + 'static;

    /// Get next key-value pair
    async fn next(&mut self) -> Result<Option<(Bytes, Bytes)>, Self::Error>;

    /// Seek to a specific key
    async fn seek(&mut self, key: &[u8]) -> Result<(), Self::Error>;
}

/// Transaction support for atomic operations
#[async_trait]
pub trait TransactionalStorage: Storage {
    /// Transaction type for this storage
    type Transaction: StorageTransaction<Error = Self::Error>;

    /// Begin a new transaction
    async fn begin_transaction(&self) -> Result<Self::Transaction, Self::Error>;
}

/// Transaction operations
#[async_trait]
pub trait StorageTransaction: Send {
    /// Error type for transaction operations
    type Error: Error + Send + Sync + 'static;

    /// Get a value within the transaction
    async fn get(&self, key: &[u8]) -> Result<Option<Bytes>, Self::Error>;

    /// Store a key-value pair within the transaction
    async fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), Self::Error>;

    /// Delete a key within the transaction
    async fn delete(&mut self, key: &[u8]) -> Result<(), Self::Error>;

    /// Commit the transaction
    async fn commit(self) -> Result<(), Self::Error>;

    /// Rollback the transaction
    async fn rollback(self) -> Result<(), Self::Error>;
}

/// Snapshot support for consistent backups
#[async_trait]
pub trait SnapshotStorage: Storage {
    /// Snapshot type for this storage
    type Snapshot: StorageSnapshot<Error = Self::Error>;

    /// Create a snapshot of the current state
    async fn create_snapshot(&self) -> Result<Self::Snapshot, Self::Error>;

    /// Restore from a snapshot
    async fn restore_snapshot(&self, snapshot: Self::Snapshot) -> Result<(), Self::Error>;
}

/// Snapshot operations
#[async_trait]
pub trait StorageSnapshot: Send {
    /// Error type for snapshot operations
    type Error: Error + Send + Sync + 'static;

    /// Export snapshot to bytes
    async fn export(&self) -> Result<Bytes, Self::Error>;

    /// Get snapshot metadata
    async fn metadata(&self) -> Result<SnapshotMetadata, Self::Error>;
}

/// Metadata for storage snapshots
#[derive(Debug, Clone)]
pub struct SnapshotMetadata {
    /// Timestamp when the snapshot was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Size of the snapshot in bytes
    pub size_bytes: u64,
    /// Number of keys in the snapshot
    pub key_count: u64,
    /// Optional description
    pub description: Option<String>,
}

/// Watch/Subscribe support for change notifications
#[async_trait]
pub trait WatchableStorage: Storage {
    /// Watcher type for this storage
    type Watcher: StorageWatcher<Error = Self::Error>;

    /// Watch for changes to a specific key
    async fn watch(&self, key: &[u8]) -> Result<Self::Watcher, Self::Error>;

    /// Watch for changes to keys with a specific prefix
    async fn watch_prefix(&self, prefix: &[u8]) -> Result<Self::Watcher, Self::Error>;
}

/// Watcher for storage changes
#[async_trait]
pub trait StorageWatcher: Send {
    /// Error type for watcher operations
    type Error: Error + Send + Sync + 'static;

    /// Wait for the next change event
    async fn next_event(&mut self) -> Result<Option<WatchEvent>, Self::Error>;

    /// Stop watching
    async fn cancel(self) -> Result<(), Self::Error>;
}

/// Event types for storage watchers
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// A key was created or updated
    Put { 
        /// The key that was created or updated
        key: Bytes, 
        /// The new value stored at the key
        value: Bytes 
    },
    /// A key was deleted
    Delete { 
        /// The key that was deleted
        key: Bytes 
    },
}

/// Storage statistics and metrics
#[async_trait]
pub trait StorageMetrics: Storage {
    /// Get current storage statistics
    async fn stats(&self) -> Result<StorageStats, Self::Error>;
}

/// Storage statistics
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    /// Total number of keys
    pub key_count: u64,
    /// Total size in bytes
    pub size_bytes: u64,
    /// Number of get operations
    pub get_count: u64,
    /// Number of put operations
    pub put_count: u64,
    /// Number of delete operations
    pub delete_count: u64,
    /// Cache hit rate (if applicable)
    pub cache_hit_rate: Option<f64>,
    /// Compression ratio (if applicable)
    pub compression_ratio: Option<f64>,
}