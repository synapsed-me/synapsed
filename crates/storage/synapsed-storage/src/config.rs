//! Configuration structures for storage backends and layers

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main storage configuration enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StorageConfig {
    /// In-memory storage configuration
    #[cfg(feature = "memory")]
    Memory(MemoryConfig),
    
    /// RocksDB storage configuration
    #[cfg(feature = "rocksdb")]
    RocksDb(RocksDbConfig),
    
    /// Sled storage configuration
    #[cfg(feature = "sled")]
    Sled(SledConfig),
    
    /// SQLite storage configuration
    #[cfg(feature = "sqlite")]
    Sqlite(SqliteConfig),
    
    /// Redis storage configuration
    #[cfg(feature = "redis")]
    Redis(RedisConfig),
    
    /// Distributed storage configuration
    #[cfg(feature = "distributed")]
    Distributed(DistributedConfig),
}

/// Configuration for in-memory storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Initial capacity for the memory map
    #[serde(default = "default_memory_capacity")]
    pub initial_capacity: usize,
    
    /// Maximum memory usage in bytes (0 = unlimited)
    #[serde(default)]
    pub max_memory_bytes: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            initial_capacity: default_memory_capacity(),
            max_memory_bytes: 0,
        }
    }
}

/// Configuration for RocksDB storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocksDbConfig {
    /// Path to the RocksDB directory
    pub path: PathBuf,
    
    /// RocksDB-specific options
    #[serde(default)]
    pub options: RocksDbOptions,
}

/// RocksDB-specific options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocksDbOptions {
    /// Block cache size in bytes
    #[serde(default = "default_block_cache_size")]
    pub block_cache_size: usize,
    
    /// Write buffer size in bytes
    #[serde(default = "default_write_buffer_size")]
    pub write_buffer_size: usize,
    
    /// Maximum number of open files
    #[serde(default = "default_max_open_files")]
    pub max_open_files: i32,
    
    /// Number of background threads
    #[serde(default = "default_num_threads")]
    pub num_threads: u32,
    
    /// Enable compression
    #[serde(default = "default_true")]
    pub compression: bool,
    
    /// Enable bloom filters
    #[serde(default = "default_true")]
    pub bloom_filter: bool,
}

/// Configuration for Sled storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SledConfig {
    /// Path to the Sled directory
    pub path: PathBuf,
    
    /// Cache capacity in bytes
    #[serde(default = "default_sled_cache_capacity")]
    pub cache_capacity: u64,
    
    /// Flush every N milliseconds
    #[serde(default = "default_flush_every_ms")]
    pub flush_every_ms: Option<u64>,
    
    /// Use compression
    #[serde(default)]
    pub use_compression: bool,
}

/// Configuration for SQLite storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteConfig {
    /// Path to the SQLite database file
    pub path: PathBuf,
    
    /// Connection pool size
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
    
    /// Enable WAL mode
    #[serde(default = "default_true")]
    pub wal_mode: bool,
    
    /// Synchronous mode
    #[serde(default = "default_synchronous")]
    pub synchronous: SqliteSynchronous,
}

/// SQLite synchronous modes
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SqliteSynchronous {
    /// Minimum sync (fastest, least safe)
    Off,
    /// Sync at critical moments
    Normal,
    /// Full sync (slowest, most safe)
    Full,
}

/// Configuration for Redis storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL
    pub url: String,
    
    /// Connection pool size
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
    
    /// Key prefix for namespacing
    #[serde(default)]
    pub key_prefix: String,
    
    /// Default TTL in seconds (0 = no expiry)
    #[serde(default)]
    pub default_ttl_secs: u64,
}

/// Configuration for distributed storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedConfig {
    /// List of node addresses
    pub nodes: Vec<String>,
    
    /// Replication factor
    #[serde(default = "default_replication_factor")]
    pub replication_factor: u32,
    
    /// Consistency level
    #[serde(default)]
    pub consistency_level: ConsistencyLevel,
    
    /// Timeout for operations
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    
    /// Enable Raft consensus
    #[serde(default)]
    pub use_raft: bool,
}

/// Consistency levels for distributed storage
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ConsistencyLevel {
    /// Read/write from any node
    One,
    /// Read/write from majority of nodes
    #[default]
    Quorum,
    /// Read/write from all nodes
    All,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Type of cache to use
    #[serde(default = "default_cache_type")]
    pub cache_type: CacheType,
    
    /// Maximum number of entries
    #[serde(default = "default_max_entries")]
    pub max_entries: usize,
    
    /// Maximum memory usage in bytes
    #[serde(default = "default_max_memory")]
    pub max_memory_bytes: usize,
    
    /// Time-to-live in seconds (None = no expiry)
    #[serde(default)]
    pub ttl_seconds: Option<u64>,
    
    /// Cache statistics collection
    #[serde(default = "default_true")]
    pub collect_stats: bool,
}

/// Cache types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheType {
    /// Least Recently Used
    Lru,
    /// Least Frequently Used
    Lfu,
    /// Adaptive Replacement Cache
    Arc,
    /// Two-Queue cache
    TwoQueue,
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Enable compression
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Compression algorithm to use
    #[serde(default = "default_compression_algorithm")]
    pub algorithm: CompressionAlgorithm,
    
    /// Minimum size in bytes to compress
    #[serde(default = "default_min_compression_size")]
    pub min_size: usize,
    
    /// Compression level (algorithm-specific)
    #[serde(default = "default_compression_level")]
    pub level: u32,
}

/// Compression algorithms
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionAlgorithm {
    /// LZ4 - fast compression
    Lz4,
    /// Zstandard - balanced compression
    Zstd,
    /// Snappy - Google's compression
    Snappy,
    /// No compression
    None,
}

// Default value functions
fn default_memory_capacity() -> usize {
    1024 * 1024 // 1MB
}

fn default_block_cache_size() -> usize {
    64 * 1024 * 1024 // 64MB
}

fn default_write_buffer_size() -> usize {
    16 * 1024 * 1024 // 16MB
}

fn default_max_open_files() -> i32 {
    1000
}

fn default_num_threads() -> u32 {
    num_cpus::get() as u32
}

fn default_sled_cache_capacity() -> u64 {
    64 * 1024 * 1024 // 64MB
}

fn default_flush_every_ms() -> Option<u64> {
    Some(1000) // 1 second
}

fn default_pool_size() -> u32 {
    10
}

fn default_synchronous() -> SqliteSynchronous {
    SqliteSynchronous::Normal
}

fn default_replication_factor() -> u32 {
    3
}

fn default_timeout_secs() -> u64 {
    30
}

fn default_cache_type() -> CacheType {
    CacheType::Lru
}

fn default_max_entries() -> usize {
    10_000
}

fn default_max_memory() -> usize {
    64 * 1024 * 1024 // 64MB
}

fn default_compression_algorithm() -> CompressionAlgorithm {
    CompressionAlgorithm::Lz4
}

fn default_min_compression_size() -> usize {
    1024 // 1KB
}

fn default_compression_level() -> u32 {
    3 // Medium compression
}

fn default_true() -> bool {
    true
}

impl Default for RocksDbOptions {
    fn default() -> Self {
        Self {
            block_cache_size: default_block_cache_size(),
            write_buffer_size: default_write_buffer_size(),
            max_open_files: default_max_open_files(),
            num_threads: default_num_threads(),
            compression: true,
            bloom_filter: true,
        }
    }
}