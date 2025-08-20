//! Integration tests for memory backend

use synapsed_storage::backends::memory::MemoryStorage;
use synapsed_storage::traits::Storage;
use synapsed_storage::config::MemoryConfig;
use bytes::Bytes;

#[tokio::test]
async fn test_memory_backend_basic_operations() {
    let storage = MemoryStorage::new(MemoryConfig {
        initial_capacity: 1024,
        max_memory_bytes: 0,
    });
    
    // Test empty storage
    let result = storage.get(b"non_existent").await.unwrap();
    assert!(result.is_none());
    
    // Test put and get
    storage.put(b"key1", b"value1").await.unwrap();
    let value = storage.get(b"key1").await.unwrap();
    assert_eq!(value, Some(Bytes::from("value1")));
    
    // Test exists
    assert!(storage.exists(b"key1").await.unwrap());
    assert!(!storage.exists(b"key2").await.unwrap());
    
    // Test delete
    storage.delete(b"key1").await.unwrap();
    assert!(!storage.exists(b"key1").await.unwrap());
}

#[tokio::test]
async fn test_memory_backend_overwrite() {
    let storage = MemoryStorage::new(MemoryConfig {
        initial_capacity: 1024,
        max_memory_bytes: 0,
    });
    
    // Put initial value
    storage.put(b"key", b"value1").await.unwrap();
    assert_eq!(storage.get(b"key").await.unwrap(), Some(Bytes::from("value1")));
    
    // Overwrite with new value
    storage.put(b"key", b"value2").await.unwrap();
    assert_eq!(storage.get(b"key").await.unwrap(), Some(Bytes::from("value2")));
}

#[tokio::test]
async fn test_memory_backend_binary_data() {
    let storage = MemoryStorage::new(MemoryConfig {
        initial_capacity: 1024,
        max_memory_bytes: 0,
    });
    
    // Test with binary data
    let binary_data = vec![0u8, 1, 2, 3, 255, 254, 253];
    storage.put(b"binary_key", &binary_data).await.unwrap();
    
    let retrieved = storage.get(b"binary_key").await.unwrap().unwrap();
    assert_eq!(retrieved.as_ref(), &binary_data);
}

#[tokio::test]
async fn test_memory_backend_empty_values() {
    let storage = MemoryStorage::new(MemoryConfig {
        initial_capacity: 1024,
        max_memory_bytes: 0,
    });
    
    // Test empty value
    storage.put(b"empty", b"").await.unwrap();
    let value = storage.get(b"empty").await.unwrap();
    assert_eq!(value, Some(Bytes::from("")));
    
    // Empty value should still make key exist
    assert!(storage.exists(b"empty").await.unwrap());
}

#[tokio::test]
async fn test_memory_backend_concurrent_access() {
    use std::sync::Arc;
    use tokio::task;
    
    let storage = Arc::new(MemoryStorage::new(MemoryConfig {
        initial_capacity: 1024,
        max_memory_bytes: 0,
    }));
    let mut handles = vec![];
    
    // Spawn multiple tasks writing to different keys
    for i in 0..10 {
        let storage_clone = Arc::clone(&storage);
        let handle = task::spawn(async move {
            let key = format!("key{}", i);
            let value = format!("value{}", i);
            storage_clone.put(key.as_bytes(), value.as_bytes()).await.unwrap();
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Verify all values were written
    for i in 0..10 {
        let key = format!("key{}", i);
        let expected_value = format!("value{}", i);
        let value = storage.get(key.as_bytes()).await.unwrap();
        assert_eq!(value, Some(Bytes::from(expected_value)));
    }
}

#[tokio::test]
async fn test_memory_backend_large_values() {
    let storage = MemoryStorage::new(MemoryConfig {
        initial_capacity: 1024,
        max_memory_bytes: 0,
    });
    
    // Create a large value (1MB)
    let large_value = vec![42u8; 1024 * 1024];
    storage.put(b"large_key", &large_value).await.unwrap();
    
    let retrieved = storage.get(b"large_key").await.unwrap().unwrap();
    assert_eq!(retrieved.len(), large_value.len());
    assert_eq!(retrieved.as_ref(), &large_value);
}

#[tokio::test]
async fn test_memory_backend_special_characters_in_keys() {
    let storage = MemoryStorage::new(MemoryConfig {
        initial_capacity: 1024,
        max_memory_bytes: 0,
    });
    
    // Test with various special characters in keys
    let test_cases = vec![
        (b"key with spaces".as_ref(), b"value1".as_ref()),
        (b"key/with/slashes".as_ref(), b"value2".as_ref()),
        (b"key:with:colons".as_ref(), b"value3".as_ref()),
        (b"key.with.dots".as_ref(), b"value4".as_ref()),
        (b"\x00\x01\x02".as_ref(), b"value5".as_ref()), // binary keys
    ];
    
    for (key, value) in test_cases {
        storage.put(key, value).await.unwrap();
        let retrieved = storage.get(key).await.unwrap();
        assert_eq!(retrieved, Some(Bytes::copy_from_slice(value)));
    }
}