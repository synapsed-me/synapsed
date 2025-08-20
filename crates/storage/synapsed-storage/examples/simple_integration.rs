//! Simple integration example showing basic functionality

use synapsed_storage::{
    backends::memory::MemoryStorage,
    config::MemoryConfig,
    Storage,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Synapsed Storage Integration Example ===\n");
    
    // Create a basic memory storage
    println!("1. Creating memory storage...");
    let storage = MemoryStorage::new(MemoryConfig {
        initial_capacity: 1024,
        max_memory_bytes: 0, // unlimited
    });
    
    // Basic key-value operations
    println!("2. Testing basic operations...");
    
    // Put
    let key = b"test_key";
    let value = b"Hello, Synapsed!";
    storage.put(key, value).await?;
    println!("   ✓ Put operation successful");
    
    // Get
    let retrieved = storage.get(key).await?;
    match retrieved {
        Some(data) => {
            println!("   ✓ Get operation successful: {:?}", 
                std::str::from_utf8(&data).unwrap_or("<binary>"));
        }
        None => println!("   ✗ Key not found"),
    }
    
    // Exists
    let exists = storage.exists(key).await?;
    println!("   ✓ Exists check: {}", exists);
    
    // Delete
    storage.delete(key).await?;
    println!("   ✓ Delete operation successful");
    
    // Verify deletion
    let after_delete = storage.get(key).await?;
    match after_delete {
        Some(_) => println!("   ✗ Key still exists after delete"),
        None => println!("   ✓ Key successfully deleted"),
    }
    
    println!("\n3. Testing Observable storage wrapper...");
    
    // Use observable storage for monitoring
    use synapsed_storage::ObservableStorageBuilder;
    let observable = ObservableStorageBuilder::new()
        .build(Arc::new(storage));
    
    // Subscribe to events
    let mut event_rx = observable.subscribe();
    
    // Perform operations with monitoring
    for i in 0..5 {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        observable.put(key.as_bytes(), value.as_bytes()).await?;
    }
    println!("   ✓ Stored 5 key-value pairs with event monitoring");
    
    // Check events
    let mut event_count = 0;
    while let Ok(event) = event_rx.try_recv() {
        event_count += 1;
        println!("   ✓ Received event: {:?}", event.event_type);
    }
    println!("   ✓ Total events received: {}", event_count);
    
    
    
    println!("\n✅ All basic functionality working correctly!");
    
    Ok(())
}