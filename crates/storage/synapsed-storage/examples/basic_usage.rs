//! Basic usage examples for synapsed-storage

use synapsed_storage::{
    Storage, StorageBuilder, StorageConfig, StorageError,
    config::MemoryConfig,
};
use std::error::Error;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    // Example 1: Simple in-memory storage
    simple_storage_example().await?;
    
    // Example 2: Storage with caching
    cached_storage_example().await?;
    
    // Example 3: Storage with compression
    compressed_storage_example().await?;
    
    // Example 4: Batch operations
    batch_operations_example().await?;
    
    // Example 5: Error handling
    error_handling_example().await?;
    
    Ok(())
}

/// Example 1: Simple in-memory storage
async fn simple_storage_example() -> Result<()> {
    println!("=== Simple Storage Example ===");
    
    // Create a basic in-memory storage
    let storage = StorageBuilder::new(StorageConfig::Memory(MemoryConfig {
        initial_capacity: 1024 * 1024, // 1MB
        max_memory_bytes: 0, // unlimited
    }))
    .build()
    .await?;
    
    // Store a value
    let key = "user:123";
    let value = serde_json::json!({
        "name": "Alice",
        "email": "alice@example.com",
        "age": 30
    });
    let value_bytes = serde_json::to_vec(&value)?;
    
    storage.put(key.as_bytes(), &value_bytes).await?;
    println!("Stored user data for key: {}", key);
    
    // Retrieve the value
    if let Some(retrieved) = storage.get(key.as_bytes()).await? {
        let user: serde_json::Value = serde_json::from_slice(&retrieved)?;
        println!("Retrieved user: {}", serde_json::to_string_pretty(&user)?);
    }
    
    // Update the value
    let updated_value = serde_json::json!({
        "name": "Alice",
        "email": "alice@example.com",
        "age": 31,
        "updated_at": "2024-01-01T00:00:00Z"
    });
    let updated_bytes = serde_json::to_vec(&updated_value)?;
    storage.put(key.as_bytes(), &updated_bytes).await?;
    println!("Updated user data");
    
    // Delete the value
    storage.delete(key.as_bytes()).await?;
    println!("Deleted user data");
    
    // Verify deletion
    let result = storage.get(key.as_bytes()).await?;
    assert_eq!(result, None);
    println!("Verified deletion: key not found\n");
    
    Ok(())
}

/// Example 2: Storage with caching
async fn cached_storage_example() -> Result<()> {
    println!("=== Cached Storage Example ===");
    
    use synapsed_storage::{CacheConfig, config::CacheType};
    
    // Create storage with LRU cache
    let storage = StorageBuilder::new(StorageConfig::Memory(MemoryConfig {
        initial_capacity: 10 * 1024 * 1024, // 10MB
        max_memory_bytes: 0, // unlimited
    }))
    .with_cache(CacheConfig {
        cache_type: CacheType::Lru,
        max_entries: 1000,
        max_memory_bytes: 0, // unlimited
        ttl_seconds: Some(300), // 5 minutes TTL
        collect_stats: false,
    })
    .build()
    .await?;
    
    // Demonstrate cache benefits with repeated access
    let key = "frequently-accessed-data";
    let large_value = vec![0u8; 100_000]; // 100KB of data
    
    // First access - goes to backend
    let start = std::time::Instant::now();
    storage.put(key.as_bytes(), &large_value).await?;
    let put_duration = start.elapsed();
    println!("Initial put took: {:?}", put_duration);
    
    // Multiple reads - should hit cache
    for i in 1..=5 {
        let start = std::time::Instant::now();
        let _ = storage.get(key.as_bytes()).await?;
        let get_duration = start.elapsed();
        println!("Get #{} took: {:?} (cached)", i, get_duration);
    }
    
    println!();
    Ok(())
}

/// Example 3: Storage with compression
async fn compressed_storage_example() -> Result<()> {
    println!("=== Compressed Storage Example ===");
    
    use synapsed_storage::{CompressionConfig, config::CompressionAlgorithm};
    
    // Create storage with compression
    let storage = StorageBuilder::new(StorageConfig::Memory(MemoryConfig {
        initial_capacity: 10 * 1024 * 1024,
        max_memory_bytes: 0, // unlimited
    }))
    .with_compression(CompressionConfig {
        enabled: true,
        algorithm: CompressionAlgorithm::Zstd,
        min_size: 1000, // Only compress values > 1KB
        level: 3,
    })
    .build()
    .await?;
    
    // Store compressible data (repetitive pattern)
    let key = "log-data";
    let log_entry = "2024-01-01 00:00:00 INFO Application started successfully\n";
    let large_log = log_entry.repeat(1000); // ~60KB of repetitive data
    let log_bytes = large_log.as_bytes().to_vec();
    
    println!("Original size: {} bytes", log_bytes.len());
    storage.put(key.as_bytes(), &log_bytes).await?;
    
    // Retrieve and verify
    let retrieved = storage.get(key.as_bytes()).await?.expect("Value should exist");
    assert_eq!(retrieved, log_bytes);
    println!("Retrieved size: {} bytes (decompressed)", retrieved.len());
    println!("Compression transparent to the user\n");
    
    Ok(())
}

/// Example 4: Batch operations
async fn batch_operations_example() -> Result<()> {
    println!("=== Batch Operations Example ===");
    
    let storage = StorageBuilder::new(StorageConfig::Memory(MemoryConfig {
        initial_capacity: 10 * 1024 * 1024,
        max_memory_bytes: 0, // unlimited
    }))
    .build()
    .await?;
    
    // Store multiple related items
    let user_prefix = "user:";
    let users = vec![
        ("user:1001", r#"{"name": "Alice", "role": "admin"}"#),
        ("user:1002", r#"{"name": "Bob", "role": "user"}"#),
        ("user:1003", r#"{"name": "Charlie", "role": "user"}"#),
        ("user:1004", r#"{"name": "Diana", "role": "moderator"}"#),
        ("user:1005", r#"{"name": "Eve", "role": "user"}"#),
    ];
    
    // Batch insert
    println!("Inserting {} users...", users.len());
    for (key, value) in &users {
        storage.put(key.as_bytes(), value.as_bytes()).await?;
    }
    
    // Storage trait doesn't have list_keys method
    // For this example, we'll get specific keys we know exist
    let user_keys: Vec<String> = users.iter().map(|(k, _)| k.to_string()).collect();
    println!("Processing {} users with prefix '{}'", user_keys.len(), user_prefix);
    
    // Retrieve and display all users
    for key in &user_keys {
        if let Some(data) = storage.get(key.as_bytes()).await? {
            let user_str = String::from_utf8_lossy(&data);
            println!("  {}: {}", key, user_str);
        }
    }
    
    // Storage trait doesn't have clear method
    // For this example, we'll delete known keys
    for (key, _) in &users {
        storage.delete(key.as_bytes()).await?;
    }
    println!("\nDeleted all user data");
    
    // Verify deletion by trying to get a known key
    let result = storage.get("user:1001".as_bytes()).await?;
    assert_eq!(result, None);
    println!("Verified user data is deleted\n");
    
    Ok(())
}

/// Example 5: Error handling
async fn error_handling_example() -> Result<()> {
    println!("=== Error Handling Example ===");
    
    let storage = StorageBuilder::new(StorageConfig::Memory(MemoryConfig {
        initial_capacity: 1024 * 1024,
        max_memory_bytes: 0, // unlimited
    }))
    .build()
    .await?;
    
    // Handle non-existent key
    match storage.get(b"non-existent-key").await {
        Ok(None) => println!("Key not found (expected)"),
        Ok(Some(_)) => println!("Unexpected: found non-existent key"),
        Err(e) => println!("Error retrieving key: {}", e),
    }
    
    // Handle large key (some backends might have limits)
    let large_key = "x".repeat(10_000);
    match storage.put(large_key.as_bytes(), &[1, 2, 3]).await {
        Ok(()) => {
            println!("Large key accepted");
            storage.delete(large_key.as_bytes()).await?;
        }
        Err(e) => println!("Large key rejected: {}", e),
    }
    
    // Demonstrate proper error propagation
    fn process_user_data(data: Option<Vec<u8>>) -> Result<String> {
        match data {
            Some(bytes) => {
                let user: serde_json::Value = serde_json::from_slice(&bytes)
                    .map_err(|e| Box::new(synapsed_storage::StorageError::Serialization(e.to_string())) as Box<dyn Error>)?;
                Ok(user.to_string())
            }
            None => Err(Box::new(synapsed_storage::StorageError::NotFound)),
        }
    }
    
    // Test error handling
    let result = storage.get(b"user:missing").await?;
    match process_user_data(result.map(|b| b.to_vec())) {
        Ok(user) => println!("Found user: {}", user),
        Err(e) => println!("Expected error: {}", e),
    }
    
    println!("\nError handling complete");
    
    Ok(())
}