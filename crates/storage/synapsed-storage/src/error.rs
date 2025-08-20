//! Error types for the storage module

use thiserror::Error;

/// Type alias for Results using StorageError
pub type Result<T> = std::result::Result<T, StorageError>;

/// Main error type for storage operations
#[derive(Error, Debug)]
pub enum StorageError {
    /// Backend-specific error
    #[error("Backend error: {0}")]
    Backend(#[from] BackendError),

    /// Compression-related error
    #[error("Compression error: {0}")]
    Compression(#[from] CompressionError),

    /// Cache-related error
    #[error("Cache error: {0}")]
    Cache(#[from] CacheError),

    /// Network-related error (for distributed storage)
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Operation timeout
    #[error("Operation timeout")]
    Timeout,

    /// Storage is full
    #[error("Storage is full")]
    StorageFull,

    /// Key not found
    #[error("Key not found")]
    NotFound,

    /// Invalid key format
    #[error("Invalid key: {0}")]
    InvalidKey(String),

    /// Invalid value format
    #[error("Invalid value: {0}")]
    InvalidValue(String),

    /// Transaction conflict
    #[error("Transaction conflict")]
    TransactionConflict,

    /// Storage is read-only
    #[error("Storage is read-only")]
    ReadOnly,

    /// Generic I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Unsupported operation
    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    /// Other error with custom message
    #[error("{0}")]
    Other(String),
}

/// Backend-specific errors
#[derive(Error, Debug)]
pub enum BackendError {
    /// RocksDB error
    #[cfg(feature = "rocksdb")]
    #[error("RocksDB error: {0}")]
    RocksDb(#[from] rocksdb::Error),

    /// Sled error
    #[cfg(feature = "sled")]
    #[error("Sled error: {0}")]
    Sled(#[from] sled::Error),

    /// SQLite error
    #[cfg(feature = "sqlite")]
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// Redis error
    #[cfg(feature = "redis")]
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// Memory backend error
    #[error("Memory backend error: {0}")]
    Memory(String),

    /// Generic backend error
    #[error("Backend error: {0}")]
    Other(String),
}

/// Compression-related errors
#[derive(Error, Debug)]
pub enum CompressionError {
    /// LZ4 compression error
    #[cfg(feature = "lz4")]
    #[error("LZ4 error: {0}")]
    Lz4(String),

    /// Zstandard compression error
    #[cfg(feature = "zstd")]
    #[error("Zstd error: {0}")]
    Zstd(#[from] std::io::Error),

    /// Snappy compression error
    #[cfg(feature = "snap")]
    #[error("Snappy error: {0}")]
    Snappy(String),

    /// Compression ratio too low
    #[error("Compression ratio too low: {0}")]
    LowRatio(f64),

    /// Decompression size mismatch
    #[error("Decompression size mismatch: expected {expected}, got {actual}")]
    SizeMismatch { 
        /// Expected size in bytes
        expected: usize, 
        /// Actual size in bytes
        actual: usize 
    },

    /// Unknown compression algorithm
    #[error("Unknown compression algorithm: {0}")]
    UnknownAlgorithm(String),

    /// Generic compression error
    #[error("Compression error: {0}")]
    Other(String),
}

/// Cache-related errors
#[derive(Error, Debug)]
pub enum CacheError {
    /// Cache capacity exceeded
    #[error("Cache capacity exceeded: {0}")]
    CapacityExceeded(usize),

    /// Cache miss (for required items)
    #[error("Cache miss for required key")]
    CacheMiss,

    /// Cache corruption
    #[error("Cache corruption detected")]
    Corruption,

    /// Invalid TTL value
    #[error("Invalid TTL: {0}")]
    InvalidTtl(String),

    /// Generic cache error
    #[error("Cache error: {0}")]
    Other(String),
}

/// Network-related errors for distributed storage
#[derive(Error, Debug)]
pub enum NetworkError {
    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Timeout error
    #[error("Network timeout")]
    Timeout,

    /// Protocol error
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Node not found
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    /// Consensus error
    #[error("Consensus error: {0}")]
    Consensus(String),

    /// Replication error
    #[error("Replication error: {0}")]
    Replication(String),

    /// Generic network error
    #[error("Network error: {0}")]
    Other(String),
}

impl StorageError {
    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            StorageError::Timeout
                | StorageError::Network(_)
                | StorageError::TransactionConflict
        )
    }

    /// Check if the error indicates data not found
    pub fn is_not_found(&self) -> bool {
        matches!(self, StorageError::NotFound)
    }

    /// Check if the error is due to storage being full
    pub fn is_storage_full(&self) -> bool {
        matches!(self, StorageError::StorageFull)
    }
}

// Implement From for common error types
impl From<String> for StorageError {
    fn from(s: String) -> Self {
        StorageError::Other(s)
    }
}

impl From<&str> for StorageError {
    fn from(s: &str) -> Self {
        StorageError::Other(s.to_string())
    }
}