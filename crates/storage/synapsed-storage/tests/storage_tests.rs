//! Core storage functionality tests

mod common;

use synapsed_storage::Storage;
use common::*;

/// Test basic get/put/delete operations
async fn test_basic_operations(fixture: StorageTestFixture) {
    let storage = &fixture.storage;
    let key = b"test-key";
    let value = b"test-value";
    
    // Test 1: Get non-existent key returns None
    let result = storage.get(key).await.expect("Get should succeed");
    assert_eq!(result, None, "Non-existent key should return None");
    
    // Test 2: Put and get value
    storage.put(key, value).await.expect("Put should succeed");
    let result = storage.get(key).await.expect("Get should succeed");
    assert_eq!(result.as_ref().map(|b| b.as_ref()), Some(&value[..]), "Retrieved value should match");
    
    // Test 3: Update existing value
    let new_value = b"updated-value";
    storage.put(key, new_value).await.expect("Update should succeed");
    let result = storage.get(key).await.expect("Get should succeed");
    assert_eq!(result.as_ref().map(|b| b.as_ref()), Some(&new_value[..]), "Updated value should match");
    
    // Test 4: Delete value
    storage.delete(key).await.expect("Delete should succeed");
    let result = storage.get(key).await.expect("Get should succeed");
    assert_eq!(result, None, "Deleted key should return None");
}

test_all_backends!(basic_operations, test_basic_operations);

/// Test batch operations
async fn test_batch_operations(fixture: StorageTestFixture) {
    let storage = &fixture.storage;
    
    // Prepare test data
    let keys: Vec<String> = (0..10).map(|i| format!("key-{}", i)).collect();
    let values: Vec<Vec<u8>> = (0..10).map(|i| format!("value-{}", i).into_bytes()).collect();
    
    // Test 1: Batch put
    for (key, value) in keys.iter().zip(values.iter()) {
        storage.put(key.as_bytes(), value).await.expect("Put should succeed");
    }
    
    // Test 2: Verify all values are stored
    for (key, value) in keys.iter().zip(values.iter()) {
        let result = storage.get(key.as_bytes()).await.expect("Get should succeed");
        assert_eq!(result.as_ref().map(|b| b.as_ref()), Some(&value[..]), "Value should match");
    }
    
    // Test 3: Delete all values
    for key in keys.iter() {
        storage.delete(key.as_bytes()).await.expect("Delete should succeed");
    }
}

test_all_backends!(batch_operations, test_batch_operations);

/// Test concurrent access
async fn test_concurrent_access(fixture: StorageTestFixture) {
    use futures::future::join_all;
    
    let storage = fixture.storage;
    let num_tasks = 100;
    let num_ops_per_task = 10;
    
    // Spawn concurrent tasks
    let tasks: Vec<_> = (0..num_tasks)
        .map(|task_id| {
            let storage = storage.clone();
            tokio::spawn(async move {
                for op_id in 0..num_ops_per_task {
                    let key = format!("task-{}-op-{}", task_id, op_id);
                    let value = format!("value-{}-{}", task_id, op_id).into_bytes();
                    
                    // Write
                    storage.put(key.as_bytes(), &value).await.expect("Put should succeed");
                    
                    // Read back
                    let result = storage.get(key.as_bytes()).await.expect("Get should succeed");
                    assert_eq!(result.as_ref().map(|b| b.as_ref()), Some(&value[..]), "Value should match");
                    
                    // Delete
                    storage.delete(key.as_bytes()).await.expect("Delete should succeed");
                }
            })
        })
        .collect();
    
    // Wait for all tasks to complete
    let results = join_all(tasks).await;
    for result in results {
        result.expect("Task should complete successfully");
    }
    
    // No need to verify empty storage without list_keys method
}

test_all_backends!(concurrent_access, test_concurrent_access);

/// Test large value handling
async fn test_large_values(fixture: StorageTestFixture) {
    let storage = &fixture.storage;
    
    // Test various sizes
    let sizes = vec![
        1024,           // 1 KB
        1024 * 1024,    // 1 MB
        10 * 1024 * 1024, // 10 MB
    ];
    
    for size in sizes {
        let key = format!("large-value-{}", size);
        let value = generate_test_data(size);
        
        // Store large value
        storage.put(key.as_bytes(), &value).await
            .expect(&format!("Should store {} byte value", size));
        
        // Retrieve and verify
        let result = storage.get(key.as_bytes()).await
            .expect(&format!("Should retrieve {} byte value", size));
        assert_eq!(result.as_ref().map(|b| b.as_ref()), Some(&value[..]), "Large value should match");
        
        // Clean up
        storage.delete(key.as_bytes()).await.expect("Delete should succeed");
    }
}

test_all_backends!(large_values, test_large_values);

/// Test error conditions
async fn test_error_conditions(fixture: StorageTestFixture) {
    let storage = &fixture.storage;
    
    // Test 1: Empty key (if not allowed by implementation)
    let result = storage.put(b"", b"value").await;
    // Some backends might allow empty keys, so we just check it doesn't panic
    let _ = result;
    
    // Test 2: Very long key
    let long_key = "x".repeat(10000);
    let result = storage.put(long_key.as_bytes(), b"value").await;
    // Check if it succeeds or fails gracefully
    match result {
        Ok(_) => {
            // If it succeeds, verify we can retrieve it
            let get_result = storage.get(long_key.as_bytes()).await.expect("Get should work");
            assert_eq!(get_result.as_ref().map(|b| b.as_ref()), Some(&b"value"[..]));
            storage.delete(long_key.as_bytes()).await.expect("Delete should work");
        }
        Err(_) => {
            // If it fails, that's also acceptable - just ensure no panic
        }
    }
}

test_all_backends!(error_conditions, test_error_conditions);

/// Test key patterns and special characters
async fn test_key_patterns(fixture: StorageTestFixture) {
    let storage = &fixture.storage;
    
    // Test various key patterns
    let test_keys = vec![
        "simple-key",
        "key/with/slashes",
        "key.with.dots",
        "key-with-unicode-🦀",
        "key with spaces",
        "key\nwith\nnewlines",
        "key\twith\ttabs",
    ];
    
    for key in test_keys {
        let value = format!("value-for-{}", key).into_bytes();
        
        // Try to store with special key
        match storage.put(key.as_bytes(), &value).await {
            Ok(_) => {
                // If storage accepts the key, verify retrieval
                let result = storage.get(key.as_bytes()).await.expect("Get should succeed");
                assert_eq!(result.as_ref().map(|b| b.as_ref()), Some(&value[..]), "Value should match for key: {}", key);
                storage.delete(key.as_bytes()).await.expect("Delete should succeed");
            }
            Err(_) => {
                // Some backends might reject certain characters - that's OK
            }
        }
    }
}