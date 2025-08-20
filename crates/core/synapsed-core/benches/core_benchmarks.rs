//! Benchmark tests for synapsed-core performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::HashMap;
use std::time::Duration;
use synapsed_core::{
    config::{ConfigValue, ConfigManager, FileConfigSource, ConfigFormat, EnvConfigSource},
    error::SynapsedError,
    network::{NetworkAddress, NetworkMessage, NetworkStats},
    traits::{Observable, HealthStatus, HealthLevel},
};

// Helper function to create test data
fn create_test_config_value(size: usize) -> ConfigValue {
    let mut obj = HashMap::new();
    for i in 0..size {
        obj.insert(format!("key_{}", i), ConfigValue::String(format!("value_{}", i)));
    }
    ConfigValue::Object(obj)
}

fn create_test_network_message(payload_size: usize) -> NetworkMessage {
    let payload = (0..payload_size).map(|i| (i % 256) as u8).collect();
    NetworkMessage::new("benchmark.message", payload)
        .with_header("content-type", "application/octet-stream")
        .with_header("benchmark", "true")
}

fn create_large_config_structure() -> ConfigValue {
    let mut root = HashMap::new();
    
    // Create nested database config
    let mut database = HashMap::new();
    database.insert("host".to_string(), ConfigValue::String("localhost".to_string()));
    database.insert("port".to_string(), ConfigValue::Integer(5432));
    database.insert("username".to_string(), ConfigValue::String("user".to_string()));
    database.insert("password".to_string(), ConfigValue::String("password".to_string()));
    database.insert("ssl".to_string(), ConfigValue::Boolean(true));
    database.insert("pool_size".to_string(), ConfigValue::Integer(20));
    root.insert("database".to_string(), ConfigValue::Object(database));
    
    // Create network config with arrays
    let mut network = HashMap::new();
    network.insert("listen_address".to_string(), ConfigValue::String("0.0.0.0:8080".to_string()));
    let servers = vec![
        ConfigValue::String("server1.example.com".to_string()),
        ConfigValue::String("server2.example.com".to_string()),
        ConfigValue::String("server3.example.com".to_string()),
    ];
    network.insert("servers".to_string(), ConfigValue::Array(servers));
    root.insert("network".to_string(), ConfigValue::Object(network));
    
    // Create large array of worker configs
    let mut workers = Vec::new();
    for i in 0..100 {
        let mut worker = HashMap::new();
        worker.insert("id".to_string(), ConfigValue::Integer(i));
        worker.insert("name".to_string(), ConfigValue::String(format!("worker-{}", i)));
        worker.insert("threads".to_string(), ConfigValue::Integer(4));
        worker.insert("enabled".to_string(), ConfigValue::Boolean(i % 2 == 0));
        workers.push(ConfigValue::Object(worker));
    }
    root.insert("workers".to_string(), ConfigValue::Array(workers));
    
    ConfigValue::Object(root)
}

// Error handling benchmarks
fn bench_error_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_creation");
    
    group.bench_function("config_error", |b| {
        b.iter(|| {
            black_box(SynapsedError::config("Configuration error occurred"))
        })
    });
    
    group.bench_function("network_error", |b| {
        b.iter(|| {
            black_box(SynapsedError::network("Network connection failed"))
        })
    });
    
    group.bench_function("application_error", |b| {
        b.iter(|| {
            black_box(SynapsedError::application(
                "Application failed to process request",
                "user_id=12345, operation=create_user"
            ))
        })
    });
    
    group.finish();
}

fn bench_error_classification(c: &mut Criterion) {
    let errors = vec![
        SynapsedError::config("config error"),
        SynapsedError::network("network error"),
        SynapsedError::invalid_input("validation error"),
        SynapsedError::internal("internal error"),
        SynapsedError::timeout("timeout error"),
    ];
    
    let mut group = c.benchmark_group("error_classification");
    
    group.bench_function("is_retryable", |b| {
        b.iter(|| {
            for error in &errors {
                black_box(error.is_retryable());
            }
        })
    });
    
    group.bench_function("is_client_error", |b| {
        b.iter(|| {
            for error in &errors {
                black_box(error.is_client_error());
            }
        })
    });
    
    group.bench_function("is_server_error", |b| {
        b.iter(|| {
            for error in &errors {
                black_box(error.is_server_error());
            }
        })
    });
    
    group.finish();
}

// Configuration benchmarks
fn bench_config_value_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_value_access");
    
    let small_config = create_test_config_value(10);
    let medium_config = create_test_config_value(100);
    let large_config = create_test_config_value(1000);
    
    group.bench_function("small_object_access", |b| {
        b.iter(|| {
            if let ConfigValue::Object(obj) = &small_config {
                for i in 0..10 {
                    black_box(obj.get(&format!("key_{}", i)));
                }
            }
        })
    });
    
    group.bench_function("medium_object_access", |b| {
        b.iter(|| {
            if let ConfigValue::Object(obj) = &medium_config {
                for i in 0..100 {
                    black_box(obj.get(&format!("key_{}", i)));
                }
            }
        })
    });
    
    group.bench_function("large_object_access", |b| {
        b.iter(|| {
            if let ConfigValue::Object(obj) = &large_config {
                for i in 0..100 { // Sample only first 100 to keep benchmark reasonable
                    black_box(obj.get(&format!("key_{}", i)));
                }
            }
        })
    });
    
    group.finish();
}

fn bench_config_value_conversion(c: &mut Criterion) {
    let string_val = ConfigValue::String("test_string".to_string());
    let int_val = ConfigValue::Integer(42);
    let bool_val = ConfigValue::Boolean(true);
    let float_val = ConfigValue::Float(3.14159);
    
    let mut group = c.benchmark_group("config_value_conversion");
    
    group.bench_function("string_conversion", |b| {
        b.iter(|| {
            black_box(string_val.as_string())
        })
    });
    
    group.bench_function("integer_conversion", |b| {
        b.iter(|| {
            black_box(int_val.as_integer())
        })
    });
    
    group.bench_function("boolean_conversion", |b| {
        b.iter(|| {
            black_box(bool_val.as_boolean())
        })
    });
    
    group.bench_function("float_conversion", |b| {
        b.iter(|| {
            black_box(float_val.as_float())
        })
    });
    
    group.bench_function("integer_to_float_conversion", |b| {
        b.iter(|| {
            black_box(int_val.as_float())
        })
    });
    
    group.finish();
}

fn bench_config_nested_access(c: &mut Criterion) {
    let complex_config = create_large_config_structure();
    
    let mut group = c.benchmark_group("config_nested_access");
    
    group.bench_function("shallow_access", |b| {
        b.iter(|| {
            if let ConfigValue::Object(obj) = &complex_config {
                black_box(obj.get("database"));
                black_box(obj.get("network"));
                black_box(obj.get("workers"));
            }
        })
    });
    
    group.bench_function("deep_access", |b| {
        b.iter(|| {
            if let ConfigValue::Object(root) = &complex_config {
                if let Some(ConfigValue::Object(database)) = root.get("database") {
                    black_box(database.get("host"));
                    black_box(database.get("port"));
                    black_box(database.get("ssl"));
                }
            }
        })
    });
    
    group.bench_function("array_iteration", |b| {
        b.iter(|| {
            if let ConfigValue::Object(root) = &complex_config {
                if let Some(ConfigValue::Array(workers)) = root.get("workers") {
                    for worker in workers.iter().take(10) { // Sample first 10
                        black_box(worker);
                    }
                }
            }
        })
    });
    
    group.finish();
}

// Network benchmarks
fn bench_network_address_parsing(c: &mut Criterion) {
    let addresses = vec![
        "127.0.0.1:8080",
        "tcp://192.168.1.1:9090",
        "p2p://12D3KooWBhV9dP6FDvFzRK8f2nbDXDDZ7NZ8xJ3Q4k5v7W8X9Y2A",
        "did://did:example:alice",
        "multiaddr:///ip4/127.0.0.1/tcp/8080",
        "webrtc://stun:stun.example.com:3478",
        "custom://some-custom-address",
    ];
    
    let mut group = c.benchmark_group("network_address_parsing");
    
    for addr_str in &addresses {
        group.bench_with_input(
            BenchmarkId::new("parse", addr_str),
            addr_str,
            |b, addr_str| {
                b.iter(|| {
                    black_box(addr_str.parse::<NetworkAddress>())
                })
            },
        );
    }
    
    group.finish();
}

fn bench_network_message_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("network_message_creation");
    group.throughput(Throughput::Bytes(1024));
    
    let payload_sizes = vec![64, 256, 1024, 4096, 16384];
    
    for size in payload_sizes {
        group.bench_with_input(
            BenchmarkId::new("create_message", size),
            &size,
            |b, &size| {
                let payload = (0..size).map(|i| (i % 256) as u8).collect::<Vec<_>>();
                b.iter(|| {
                    black_box(NetworkMessage::new("test.message", payload.clone()))
                })
            },
        );
    }
    
    group.finish();
}

fn bench_network_message_with_metadata(c: &mut Criterion) {
    let mut group = c.benchmark_group("network_message_metadata");
    
    group.bench_function("with_headers", |b| {
        b.iter(|| {
            let mut msg = NetworkMessage::new("test.message", vec![1, 2, 3, 4]);
            msg = msg.with_header("header1", "value1");
            msg = msg.with_header("header2", "value2");
            msg = msg.with_header("header3", "value3");
            msg = msg.with_header("header4", "value4");
            msg = msg.with_header("header5", "value5");
            black_box(msg);
        })
    });
    
    group.bench_function("with_addresses", |b| {
        b.iter(|| {
            let sender = NetworkAddress::PeerId("sender123".to_string());
            let recipient = NetworkAddress::PeerId("recipient456".to_string());
            let msg = NetworkMessage::new("test.message", vec![1, 2, 3, 4])
                .with_sender(sender)
                .with_recipient(recipient);
            black_box(msg);
        })
    });
    
    group.finish();
}

fn bench_network_stats_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("network_stats_updates");
    
    group.bench_function("record_operations", |b| {
        let mut stats = NetworkStats::new();
        b.iter(|| {
            stats.record_bytes_sent(1024);
            stats.record_bytes_received(2048);
            stats.record_message_sent();
            stats.record_message_received();
            stats.record_connection();
            stats.update_uptime(black_box(60));
        })
    });
    
    group.bench_function("calculate_rates", |b| {
        let mut stats = NetworkStats::new();
        stats.record_bytes_sent(1000000);
        stats.record_bytes_received(2000000);
        stats.record_message_sent();
        stats.record_message_received();
        stats.update_uptime(3600);
        
        b.iter(|| {
            black_box(stats.throughput());
            black_box(stats.message_rate());
            black_box(stats.error_rate());
        })
    });
    
    group.finish();
}

// Serialization benchmarks
fn bench_serialization(c: &mut Criterion) {
    let config = create_large_config_structure();
    let message = create_test_network_message(1024);
    let error = SynapsedError::application("Benchmark error", "context=performance_test");
    
    let mut group = c.benchmark_group("serialization");
    
    group.bench_function("config_json_serialize", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(&config).unwrap())
        })
    });
    
    group.bench_function("config_json_deserialize", |b| {
        let json = serde_json::to_string(&config).unwrap();
        b.iter(|| {
            black_box(serde_json::from_str::<ConfigValue>(&json).unwrap())
        })
    });
    
    group.bench_function("message_json_serialize", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(&message).unwrap())
        })
    });
    
    group.bench_function("message_json_deserialize", |b| {
        let json = serde_json::to_string(&message).unwrap();
        b.iter(|| {
            black_box(serde_json::from_str::<NetworkMessage>(&json).unwrap())
        })
    });
    
    group.bench_function("error_json_serialize", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(&error).unwrap())
        })
    });
    
    group.bench_function("bincode_config_serialize", |b| {
        b.iter(|| {
            black_box(bincode::serialize(&config).unwrap())
        })
    });
    
    group.bench_function("bincode_config_deserialize", |b| {
        let binary = bincode::serialize(&config).unwrap();
        b.iter(|| {
            black_box(bincode::deserialize::<ConfigValue>(&binary).unwrap())
        })
    });
    
    group.finish();
}

// Memory allocation benchmarks
fn bench_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");
    
    group.bench_function("config_value_creation", |b| {
        b.iter(|| {
            black_box(create_test_config_value(100))
        })
    });
    
    group.bench_function("network_message_creation", |b| {
        b.iter(|| {
            black_box(create_test_network_message(1024))
        })
    });
    
    group.bench_function("error_creation_chain", |b| {
        b.iter(|| {
            let inner = SynapsedError::storage("Database connection failed");
            let middle = SynapsedError::internal(&format!("Service error: {}", inner));
            let outer = SynapsedError::application("Request failed", &middle.to_string());
            black_box(outer);
        })
    });
    
    group.finish();
}

// Concurrent access benchmarks
fn bench_concurrent_access(c: &mut Criterion) {
    let config = std::sync::Arc::new(create_large_config_structure());
    let message = std::sync::Arc::new(create_test_network_message(1024));
    
    let mut group = c.benchmark_group("concurrent_access");
    
    group.bench_function("config_concurrent_read", |b| {
        b.iter(|| {
            let config_ref = config.clone();
            std::thread::spawn(move || {
                if let ConfigValue::Object(obj) = &*config_ref {
                    black_box(obj.get("database"));
                }
            }).join().unwrap();
        })
    });
    
    group.bench_function("message_concurrent_read", |b| {
        b.iter(|| {
            let message_ref = message.clone();
            std::thread::spawn(move || {
                black_box(message_ref.payload_size());
                black_box(&message_ref.headers);
            }).join().unwrap();
        })
    });
    
    group.finish();
}

// Performance regression tests
fn bench_performance_regression(c: &mut Criterion) {
    let mut group = c.benchmark_group("performance_regression");
    
    // These benchmarks establish baselines for performance regression detection
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(1000);
    
    group.bench_function("baseline_error_operations", |b| {
        b.iter(|| {
            let error = SynapsedError::config("test error");
            black_box(error.is_retryable());
            black_box(error.is_client_error());
            black_box(error.to_string());
            black_box(format!("{:?}", error));
        })
    });
    
    group.bench_function("baseline_config_operations", |b| {
        let config = create_test_config_value(50);
        b.iter(|| {
            if let ConfigValue::Object(obj) = &config {
                for i in 0..10 {
                    black_box(obj.get(&format!("key_{}", i)));
                }
            }
        })
    });
    
    group.bench_function("baseline_network_operations", |b| {
        b.iter(|| {
            let addr = "127.0.0.1:8080".parse::<NetworkAddress>().unwrap();
            let message = NetworkMessage::new("test", vec![1, 2, 3, 4])
                .with_header("test", "value");
            black_box(addr.protocol());
            black_box(message.payload_size());
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_error_creation,
    bench_error_classification,
    bench_config_value_access,
    bench_config_value_conversion,
    bench_config_nested_access,
    bench_network_address_parsing,
    bench_network_message_creation,
    bench_network_message_with_metadata,
    bench_network_stats_updates,
    bench_serialization,
    bench_memory_allocation,
    bench_concurrent_access,
    bench_performance_regression
);

criterion_main!(benches);