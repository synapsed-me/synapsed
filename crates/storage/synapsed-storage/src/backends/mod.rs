//! Storage backend implementations

#[cfg(feature = "memory")]
pub mod memory;

#[cfg(feature = "memory")]
pub mod observable_memory;

#[cfg(feature = "rocksdb")]
pub mod rocksdb;

#[cfg(feature = "sled")]
pub mod sled;

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "redis")]
pub mod redis;

// Re-export backend implementations
#[cfg(feature = "memory")]
pub use memory::MemoryStorage;

#[cfg(feature = "memory")]
pub use observable_memory::ObservableMemoryStorage;

#[cfg(feature = "rocksdb")]
pub use rocksdb::RocksDbStorage;

#[cfg(feature = "sled")]
pub use self::sled::SledStorage;

#[cfg(feature = "sqlite")]
pub use sqlite::SqliteStorage;

#[cfg(feature = "redis")]
pub use redis::RedisStorage;