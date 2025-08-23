//! Storage backend implementations

#[cfg(feature = "memory")]
pub mod memory;

#[cfg(feature = "memory")]
pub mod observable_memory;

// File and SQLite backends (always available for MCP)
pub mod file;
pub mod sqlite;

#[cfg(feature = "rocksdb")]
pub mod rocksdb;

#[cfg(feature = "sled")]
pub mod sled;

#[cfg(feature = "redis")]
pub mod redis;

// Re-export backend implementations
#[cfg(feature = "memory")]
pub use memory::MemoryStorage;

#[cfg(feature = "memory")]
pub use observable_memory::ObservableMemoryStorage;

// Always available
pub use file::FileStorage;
pub use sqlite::SqliteStorage;

#[cfg(feature = "rocksdb")]
pub use rocksdb::RocksDbStorage;

#[cfg(feature = "sled")]
pub use self::sled::SledStorage;

#[cfg(feature = "redis")]
pub use redis::RedisStorage;