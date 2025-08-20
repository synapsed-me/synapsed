//! # Synapsed Storage
//! 
//! A simple, test-driven storage implementation following SPARC methodology

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod backends;
pub mod error;
pub mod traits;
pub mod types;

// Re-export commonly used types
pub use error::{Result, StorageError};
pub use traits::{
    Storage, BatchedStorage, IterableStorage, StorageIterator,
    TransactionalStorage, StorageTransaction, 
    DocumentStore, BlobStore, SyncableStorage, ValueStore
};
pub use types::*;
pub use backends::memory::MemoryStorage;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::Storage;

    #[tokio::test]
    async fn test_memory_storage_integration() {
        let storage = MemoryStorage::new();
        
        // Test basic operations
        storage.put(b"test_key", b"test_value").await.unwrap();
        let value = storage.get(b"test_key").await.unwrap();
        assert_eq!(value.unwrap().as_ref(), b"test_value");
    }
}