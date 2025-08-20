//! Basic integration test to ensure everything compiles and works

use synapsed_storage::{
    backends::memory::MemoryStorage,
    config::MemoryConfig,
    Storage,
    ObservableStorageBuilder,
    factory::{StorageFactory, StorageBackend},
};
use std::sync::Arc;

#[tokio::test]
async fn test_basic_memory_storage() {
    let storage = MemoryStorage::new(MemoryConfig::default());
    
    // Test basic operations
    storage.put(b"test_key", b"test_value").await.unwrap();
    let value = storage.get(b"test_key").await.unwrap();
    assert_eq!(value.unwrap().as_ref(), b"test_value");
    
    storage.delete(b"test_key").await.unwrap();
    let value = storage.get(b"test_key").await.unwrap();
    assert!(value.is_none());
}

#[tokio::test]
async fn test_observable_storage() {
    let base_storage = Arc::new(MemoryStorage::new(MemoryConfig::default()));
    let observable_storage = ObservableStorageBuilder::new()
        .build(base_storage);
    
    // Subscribe to events
    let mut event_receiver = observable_storage.subscribe();
    
    // Test basic operations
    observable_storage.put(b"test_key", b"test_value").await.unwrap();
    
    // Should receive an event
    let event = event_receiver.try_recv();
    assert!(event.is_ok());
    
    let value = observable_storage.get(b"test_key").await.unwrap();
    assert_eq!(value.unwrap().as_ref(), b"test_value");
}

#[tokio::test]
async fn test_storage_factory() {
    // Create a basic memory storage through factory
    let storage = StorageFactory::create(
        StorageBackend::Memory { capacity: Some(100) },
        false,
    )
    .await
    .unwrap();
    
    // Test operations
    storage.put(b"factory_key", b"factory_value").await.unwrap();
    let value = storage.get(b"factory_key").await.unwrap();
    assert_eq!(value.unwrap().as_ref(), b"factory_value");
}

#[tokio::test]
async fn test_observable_factory() {
    // Create an observable memory storage through factory
    let storage = StorageFactory::create(
        StorageBackend::Memory { capacity: None },
        true,
    )
    .await
    .unwrap();
    
    // Test operations
    storage.put(b"obs_key", b"obs_value").await.unwrap();
    assert!(storage.exists(b"obs_key").await.unwrap());
    
    storage.flush().await.unwrap();
}