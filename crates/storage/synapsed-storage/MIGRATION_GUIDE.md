# Migration Guide: Upgrading to Substrate & Serventis Integration

This guide helps existing synapsed-storage users migrate to the new version with substrate and serventis integration.

## Overview

The new version of synapsed-storage introduces:
- **Substrate Integration**: Event-driven storage with real-time event streams
- **Serventis Monitoring**: Comprehensive observability and monitoring capabilities
- **Observable Storage**: Reactive storage with health monitoring and metrics export

## Migration Strategies

### 1. Gradual Migration (Recommended)

This approach allows you to gradually adopt new features while maintaining existing functionality.

#### Step 1: Update Dependencies

```toml
[dependencies]
synapsed-storage = { version = "0.2.0", features = ["memory", "substrates", "serventis"] }
```

#### Step 2: Basic Migration

```rust
// Before: Basic storage
use synapsed_storage::{StorageBuilder, StorageConfig, MemoryConfig};

let storage = StorageBuilder::new(StorageConfig::Memory(MemoryConfig::default()))
    .build()
    .await?;

// After: Same functionality with observability
use synapsed_storage::{
    StorageBuilder, StorageConfig, MemoryConfig,
    ObservableStorageBuilder, MonitoringConfig,
};

// Option 1: Keep existing code (no changes needed)
let storage = StorageBuilder::new(StorageConfig::Memory(MemoryConfig::default()))
    .build()
    .await?;

// Option 2: Add observability gradually
let base_storage = StorageBuilder::new(StorageConfig::Memory(MemoryConfig::default()))
    .build()
    .await?;

let observable_storage = ObservableStorageBuilder::new(
    base_storage,
    "my_service".to_string(),
    "my_storage".to_string(),
)
.build();
```

#### Step 3: Add Event Monitoring (Optional)

```rust
use synapsed_storage::{
    EventDrivenStorageBuilder, StorageEvent,
};

// Wrap existing storage with event monitoring
let event_storage = EventDrivenStorageBuilder::new(
    existing_storage,
    "storage_events".to_string(),
)
.with_cache_monitoring()
.build();

// Subscribe to events
let mut event_receiver = event_storage.subscribe();
tokio::spawn(async move {
    while let Ok(event) = event_receiver.recv().await {
        match event {
            StorageEvent::KeyCreated { key, .. } => {
                println!("New key created: {:?}", key);
            }
            StorageEvent::KeyDeleted { key, .. } => {
                println!("Key deleted: {:?}", key);
            }
            _ => {}
        }
    }
});
```

#### Step 4: Add Service Monitoring (Optional)

```rust
use synapsed_storage::{
    ServentisStorageBuilder, StorageServiceSignal, MonitorCondition,
};

// Add service monitoring
let monitored_storage = ServentisStorageBuilder::new(
    existing_storage,
    "my_storage_service".to_string(),
)
.build();

// Add monitoring handlers
monitored_storage.add_signal_handler(|signal| {
    match signal {
        StorageServiceSignal::Failed { operation, error, .. } => {
            eprintln!("Storage operation failed: {} - {}", operation, error);
        }
        _ => {}
    }
}).await;

// Get monitoring reports
let monitor = monitored_storage.get_monitor().await;
if matches!(monitor.condition, MonitorCondition::Critical) {
    eprintln!("Storage in critical condition: {}", monitor.assessment);
}
```

### 2. Full Migration to Observable Storage

For new applications or when you want full observability features:

```rust
use synapsed_storage::{
    backends::memory::MemoryStorage,
    config::MemoryConfig,
    observable::{
        ObservableStorageBuilder, MonitoringConfig, MetricsFormat,
        ObservableStorage, ReactiveStorage, MonitoredStorage,
    },
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create base storage
    let base_storage = Arc::new(MemoryStorage::new(MemoryConfig::default()));
    
    // Create comprehensive observable storage
    let mut storage = ObservableStorageBuilder::new(
        base_storage,
        "my_application_storage".to_string(),
        "storage_events".to_string(),
    )
    .with_monitoring_config(MonitoringConfig {
        enable_events: true,
        enable_performance: true,
        enable_health_checks: true,
        interval_ms: 1000,
        max_buffer_size: 10000,
    })
    .build();

    // Enable full monitoring
    storage.enable_full_monitoring().await?;

    // Set up reactive event handling
    storage.on_event(|event| {
        // Handle important events
        if let Some(key) = event.key() {
            let key_str = String::from_utf8_lossy(key);
            if key_str.starts_with("critical_") {
                println!("Critical data modified: {}", key_str);
            }
        }
    }).await?;

    // Use storage normally
    storage.put(b"user:123", b"user_data").await?;
    let value = storage.get(b"user:123").await?;

    // Get comprehensive health information
    let health = storage.health_check().await?;
    println!("Storage health: {:?}", health.overall_status);

    // Export metrics for external monitoring
    let prometheus_metrics = storage.export_metrics(MetricsFormat::Prometheus).await?;
    println!("Metrics: {}", String::from_utf8_lossy(&prometheus_metrics));

    Ok(())
}
```

## API Changes and Compatibility

### Backward Compatibility

The core `Storage` trait remains unchanged:

```rust
#[async_trait]
pub trait Storage: Send + Sync {
    type Error: Error + Send + Sync + 'static;
    
    async fn get(&self, key: &[u8]) -> Result<Option<Bytes>, Self::Error>;
    async fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Self::Error>;
    async fn delete(&self, key: &[u8]) -> Result<(), Self::Error>;
    async fn exists(&self, key: &[u8]) -> Result<bool, Self::Error>;
    async fn flush(&self) -> Result<(), Self::Error>;
}
```

All existing storage backends continue to work without changes.

### New Traits

Three new trait hierarchies are introduced:

1. **ObservableStorage**: Basic observability with monitoring and reporting
2. **ReactiveStorage**: Real-time event handling and key monitoring
3. **MonitoredStorage**: Health checks and metrics export

These traits extend the base `Storage` trait, so existing code continues to work.

### New Event Types

Substrate integration introduces event types:

```rust
pub enum StorageEvent {
    KeyCreated { key: Vec<u8>, value_size: usize, timestamp: u64, transaction_id: Option<Uuid> },
    KeyUpdated { key: Vec<u8>, old_value_size: usize, new_value_size: usize, timestamp: u64, transaction_id: Option<Uuid> },
    KeyDeleted { key: Vec<u8>, timestamp: u64, transaction_id: Option<Uuid> },
    KeyRead { key: Vec<u8>, value_size: Option<usize>, cache_hit: bool, timestamp: u64 },
    // ... other event types
}
```

### New Monitoring Types

Serventis integration introduces monitoring types:

```rust
pub struct StorageMonitor {
    pub monitor_id: String,
    pub service_id: String,
    pub condition: MonitorCondition,
    pub confidence: f64,
    pub assessment: String,
    pub timestamp: u64,
    pub metrics: MonitorMetrics,
}

pub enum MonitorCondition {
    Optimal,
    Degraded,  
    Critical,
    Failed,
    Unknown,
}
```

## Performance Considerations

### Event Overhead

Event-driven storage has minimal overhead:
- ~1-5% latency increase
- ~10-20MB additional memory for event buffers
- No overhead when events are not subscribed to

### Monitoring Overhead

Service monitoring has configurable overhead:
- ~2-8% latency increase depending on monitoring features
- ~5-15MB additional memory for metrics collection
- Can be tuned with `MonitoringConfig`

### Optimization Tips

1. **Tune Buffer Sizes**: Adjust `max_buffer_size` in `MonitoringConfig`
2. **Selective Features**: Only enable monitoring features you need
3. **Batch Operations**: Use batched operations for high-throughput scenarios
4. **Event Filtering**: Use circuit filters to process only relevant events

## Feature Flags

Control integration features with Cargo features:

```toml
[dependencies.synapsed-storage]
version = "0.2.0"
default-features = false
features = [
    "memory",           # Memory backend
    "substrates",       # Event-driven storage
    "serventis",        # Service monitoring  
    "observable",       # Observable storage traits
    "metrics",          # Metrics collection
]
```

## Troubleshooting

### Common Issues

#### 1. High Memory Usage

**Symptoms**: Increasing memory usage over time
**Cause**: Large event buffers or unbounded event subscriptions
**Solution**: 
```rust
let config = MonitoringConfig {
    max_buffer_size: 1000, // Reduce buffer size
    interval_ms: 5000,     // Reduce monitoring frequency
    ..Default::default()
};
```

#### 2. Performance Degradation

**Symptoms**: Slower storage operations
**Cause**: Too many event handlers or monitoring overhead
**Solution**:
```rust
// Disable unnecessary features
let config = MonitoringConfig {
    enable_events: false,        // Disable events if not needed
    enable_performance: true,    // Keep essential monitoring
    enable_health_checks: false, // Disable if not needed
    ..Default::default()
};
```

#### 3. Event Loss

**Symptoms**: Missing events in event handlers
**Cause**: Event buffer overflow
**Solution**:
```rust
let config = MonitoringConfig {
    max_buffer_size: 50000, // Increase buffer size
    ..Default::default()
};

// Or process events faster
storage.on_event(|event| {
    // Process quickly and return
    tokio::spawn(async move {
        // Do heavy processing in separate task
        process_event_async(event).await;
    });
}).await?;
```

### Debugging

Enable debug logging:

```rust
use tracing_subscriber;

// Initialize tracing
tracing_subscriber::init();

// Storage operations will now log debug information
```

Monitor performance:

```rust
let monitor = storage.get_monitor().await;
println!("Avg latency: {:.2}ms", monitor.metrics.avg_latency_ms);
println!("Throughput: {:.2} ops/sec", monitor.metrics.throughput_ops_per_sec);
println!("Error rate: {:.2}%", monitor.metrics.error_rate * 100.0);
```

## Testing

### Unit Tests

Test new functionality:

```rust
#[tokio::test]
async fn test_event_monitoring() {
    let storage = create_observable_storage().await;
    let mut events = storage.subscribe_events();
    
    storage.put(b"test", b"value").await.unwrap();
    
    let event = events.recv().await.unwrap();
    assert!(matches!(event, StorageEvent::KeyCreated { .. }));
}
```

### Integration Tests

Test with real monitoring:

```rust
#[tokio::test]
async fn test_health_monitoring() {
    let storage = create_monitored_storage().await;
    
    // Perform operations
    for i in 0..100 {
        storage.put(format!("key_{}", i).as_bytes(), b"value").await.unwrap();
    }
    
    let health = storage.health_check().await.unwrap();
    assert!(matches!(health.overall_status, HealthStatus::Healthy));
}
```

## Getting Help

- **Documentation**: Check the updated API documentation
- **Examples**: See `examples/substrate_serventis_example.rs`
- **Issues**: Report migration issues on GitHub
- **Performance**: Use built-in metrics export for performance analysis

## Summary

The new substrate and serventis integration provides powerful observability features while maintaining full backward compatibility. Migration can be done gradually, and existing code continues to work without changes.

Key benefits:
- ✅ **Backward Compatible**: Existing code works unchanged
- ✅ **Gradual Migration**: Add features incrementally  
- ✅ **Real-time Events**: Subscribe to storage events
- ✅ **Service Monitoring**: Comprehensive health and performance monitoring
- ✅ **Metrics Export**: Integration with monitoring systems
- ✅ **Reactive Programming**: Event-driven storage operations

Choose the migration strategy that best fits your application's needs!