//! Property-based tests for synapsed-storage

mod common;

use proptest::prelude::*;
use synapsed_storage::{StorageBuilder, StorageConfig};
use synapsed_storage::config::MemoryConfig;
use tokio::runtime::Runtime;

/// Maximum key and value sizes for property tests
const MAX_KEY_SIZE: usize = 1000;
const MAX_VALUE_SIZE: usize = 10000;
const MAX_OPERATIONS: usize = 100;

/// Create a runtime for tests
fn create_runtime() -> Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create runtime")
}

/// Strategy for generating valid storage keys
fn key_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9_/-]{1,100}")
        .expect("Invalid regex")
}

/// Strategy for generating storage values
fn value_strategy() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..MAX_VALUE_SIZE)
}

/// Operations we can perform on storage
#[derive(Debug, Clone)]
enum StorageOp {
    Put(String, Vec<u8>),
    Get(String),
    Delete(String),
}

// Implement Arbitrary trait for property testing
impl proptest::arbitrary::Arbitrary for StorageOp {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    
    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        operation_strategy().boxed()
    }
}

/// Strategy for generating storage operations
fn operation_strategy() -> impl Strategy<Value = StorageOp> {
    prop_oneof![
        (key_strategy(), value_strategy()).prop_map(|(k, v)| StorageOp::Put(k, v)),
        key_strategy().prop_map(StorageOp::Get),
        key_strategy().prop_map(StorageOp::Delete),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    // Property: Put then Get returns the same value
    #[test]
    fn test_put_then_get_returns_same_value(
        key: String,
        value: Vec<u8>,
    ) {
        let runtime = create_runtime();
        runtime.block_on(async {
            let storage = StorageBuilder::new(StorageConfig::Memory(MemoryConfig {
                initial_capacity: 10 * 1024 * 1024,
                max_memory_bytes: 0,
            }))
            .build()
            .await
            .expect("Failed to create storage");
            
            // Put value
            storage.put(key.as_bytes(), &value).await
                .expect("Put should succeed");
            
            // Get value
            let result = storage.get(key.as_bytes()).await
                .expect("Get should succeed");
            
            prop_assert_eq!(result.map(|b| b.to_vec()), Some(value));
            Ok(())
        }).expect("Test failed");
    }
    
    // Property: Delete removes the value
    #[test]
    fn test_delete_removes_value(
        key: String,
        value: Vec<u8>,
    ) {
        let runtime = create_runtime();
        runtime.block_on(async {
            let storage = StorageBuilder::new(StorageConfig::Memory(MemoryConfig {
                initial_capacity: 10 * 1024 * 1024,
                max_memory_bytes: 0,
            }))
            .build()
            .await
            .expect("Failed to create storage");
            
            // Put value
            storage.put(key.as_bytes(), &value).await
                .expect("Put should succeed");
            
            // Delete value
            storage.delete(key.as_bytes()).await
                .expect("Delete should succeed");
            
            // Get should return None
            let result = storage.get(key.as_bytes()).await
                .expect("Get should succeed");
            
            prop_assert_eq!(result, None);
            Ok(())
        }).expect("Test failed");
    }
    
    // Property: Multiple puts with same key updates value
    #[test]
    fn test_multiple_puts_updates_value(
        key: String,
        values: Vec<Vec<u8>>,
    ) {
        if values.is_empty() {
            return Ok(());
        }
        
        let runtime = create_runtime();
        runtime.block_on(async {
            let storage = StorageBuilder::new(StorageConfig::Memory(MemoryConfig {
                initial_capacity: 10 * 1024 * 1024,
                max_memory_bytes: 0,
            }))
            .build()
            .await
            .expect("Failed to create storage");
            
            // Put all values
            for value in &values {
                storage.put(key.as_bytes(), value).await
                    .expect("Put should succeed");
            }
            
            // Get should return the last value
            let result = storage.get(key.as_bytes()).await
                .expect("Get should succeed");
            
            let last_value = values.last().unwrap();
            prop_assert_eq!(result.map(|b| b.to_vec()), Some(last_value.clone()));
            Ok(())
        }).expect("Test failed");
    }
    
    // Property: Operations maintain consistency
    #[test]
    fn test_operations_maintain_consistency(
        operations: Vec<StorageOp>,
    ) {
        let runtime = create_runtime();
        runtime.block_on(async {
            let storage = StorageBuilder::new(StorageConfig::Memory(MemoryConfig {
                initial_capacity: 10 * 1024 * 1024,
                max_memory_bytes: 0,
            }))
            .build()
            .await
            .expect("Failed to create storage");
            
            let mut expected: std::collections::HashMap<String, Vec<u8>> = 
                std::collections::HashMap::new();
            
            // Apply operations
            for op in operations {
                match op {
                    StorageOp::Put(key, value) => {
                        storage.put(key.as_bytes(), &value).await
                            .expect("Put should succeed");
                        expected.insert(key, value);
                    }
                    StorageOp::Get(key) => {
                        let result = storage.get(key.as_bytes()).await
                            .expect("Get should succeed");
                        let expected_value = expected.get(&key).cloned();
                        prop_assert_eq!(result.map(|b| b.to_vec()), expected_value);
                    }
                    StorageOp::Delete(key) => {
                        storage.delete(key.as_bytes()).await
                            .expect("Delete should succeed");
                        expected.remove(&key);
                    }
                }
            }
            
            // Verify all expected values
            for (key, expected_value) in expected {
                let result = storage.get(key.as_bytes()).await
                    .expect("Get should succeed");
                prop_assert_eq!(result.map(|b| b.to_vec()), Some(expected_value));
            }
            
            Ok(())
        }).expect("Test failed");
    }
    
    // Property: Concurrent operations don't corrupt data
    #[test]
    fn test_concurrent_operations_dont_corrupt_data(
        keys: Vec<(String, Vec<u8>)>,
    ) {
        if keys.is_empty() {
            return Ok(());
        }
        
        let runtime = create_runtime();
        runtime.block_on(async {
            let storage = StorageBuilder::new(StorageConfig::Memory(MemoryConfig {
                initial_capacity: 10 * 1024 * 1024,
                max_memory_bytes: 0,
            }))
            .build()
            .await
            .expect("Failed to create storage");
            
            // Launch concurrent operations
            let storage = std::sync::Arc::new(storage);
            let mut handles = vec![];
            
            for (key, value) in keys.clone() {
                let storage_clone = storage.clone();
                let handle = tokio::spawn(async move {
                    // Put
                    storage_clone.put(key.as_bytes(), &value).await.expect("Put 1");
                    let r1 = storage_clone.get(key.as_bytes()).await.expect("Get 1");
                    assert_eq!(r1.map(|b| b.to_vec()), Some(value.clone()));
                    
                    storage_clone.put(key.as_bytes(), &value).await.expect("Put 2");
                    let r2 = storage_clone.get(key.as_bytes()).await.expect("Get 2");
                    assert_eq!(r2.map(|b| b.to_vec()), Some(value));
                    
                    storage_clone.delete(key.as_bytes()).await.expect("Delete");
                });
                handles.push(handle);
            }
            
            // Wait for all operations to complete
            for handle in handles {
                handle.await.expect("Task panicked");
            }
            
            // Verify all keys are deleted
            for (key, _) in keys {
                let result = storage.get(key.as_bytes()).await
                    .expect("Get should succeed");
                prop_assert_eq!(result, None);
            }
            
            Ok(())
        }).expect("Test failed");
    }
}

#[cfg(test)]
mod test_specific_cases {
    use super::*;
    
    #[tokio::test]
    async fn test_empty_key() {
        let storage = StorageBuilder::new(StorageConfig::Memory(MemoryConfig {
            initial_capacity: 1024,
            max_memory_bytes: 0,
        }))
        .build()
        .await
        .expect("Failed to create storage");
        
        // Empty key should work
        storage.put(b"", b"value").await.expect("Put empty key");
        let result = storage.get(b"").await.expect("Get empty key");
        assert_eq!(result.map(|b| b.to_vec()), Some(b"value".to_vec()));
    }
    
    #[tokio::test]
    async fn test_empty_value() {
        let storage = StorageBuilder::new(StorageConfig::Memory(MemoryConfig {
            initial_capacity: 1024,
            max_memory_bytes: 0,
        }))
        .build()
        .await
        .expect("Failed to create storage");
        
        // Empty value should work
        storage.put(b"key", b"").await.expect("Put empty value");
        let result = storage.get(b"key").await.expect("Get empty value");
        assert_eq!(result.map(|b| b.to_vec()), Some(vec![]));
    }
    
    #[tokio::test]
    async fn test_large_values() {
        let storage = StorageBuilder::new(StorageConfig::Memory(MemoryConfig {
            initial_capacity: 10 * 1024 * 1024,
            max_memory_bytes: 0,
        }))
        .build()
        .await
        .expect("Failed to create storage");
        
        // Test with 1MB value
        let large_value = vec![42u8; 1024 * 1024];
        storage.put(b"large", &large_value).await.expect("Put large value");
        let result = storage.get(b"large").await.expect("Get large value");
        assert_eq!(result.map(|b| b.to_vec()), Some(large_value));
    }
}