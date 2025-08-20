//! Performance benchmarks for synapsed-storage

use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use synapsed_storage::{Storage, StorageBuilder, StorageConfig};
use synapsed_storage::config::MemoryConfig;
use tokio::runtime::Runtime;

/// Benchmark configuration
const SMALL_VALUE_SIZE: usize = 100;
const MEDIUM_VALUE_SIZE: usize = 10_000;
const LARGE_VALUE_SIZE: usize = 1_000_000;
const NUM_KEYS: usize = 1000;

/// Create a runtime for async benchmarks
fn create_runtime() -> Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create runtime")
}

/// Generate test data
fn generate_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
}

/// Benchmark individual put operations
fn bench_put_operations(c: &mut Criterion) {
    let runtime = create_runtime();
    
    let mut group = c.benchmark_group("put_operations");
    
    for size in &[SMALL_VALUE_SIZE, MEDIUM_VALUE_SIZE, LARGE_VALUE_SIZE] {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("memory", size),
            size,
            |b, &size| {
                b.to_async(&runtime).iter_batched(
                    || {
                        // Setup: create storage and test data
                        let storage = runtime.block_on(async {
                            StorageBuilder::new(StorageConfig::Memory(MemoryConfig {
                                initial_capacity: 100 * 1024 * 1024, // 100MB
                                max_memory_bytes: 0, // unlimited
                            }))
                            .build()
                            .await
                            .expect("Failed to create storage")
                        });
                        let value = generate_data(size);
                        let key = uuid::Uuid::new_v4().to_string();
                        (storage, key, value)
                    },
                    |(storage, key, value)| async move {
                        storage.put(key.as_bytes(), &value).await.expect("Put failed");
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    
    group.finish();
}

/// Benchmark get operations
fn bench_get_operations(c: &mut Criterion) {
    let runtime = create_runtime();
    
    let mut group = c.benchmark_group("get_operations");
    
    for size in &[SMALL_VALUE_SIZE, MEDIUM_VALUE_SIZE, LARGE_VALUE_SIZE] {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("memory", size),
            size,
            |b, &size| {
                b.to_async(&runtime).iter_batched(
                    || {
                        // Setup: create storage with pre-populated data
                        let storage = runtime.block_on(async {
                            let storage = StorageBuilder::new(StorageConfig::Memory(MemoryConfig {
                                initial_capacity: 100 * 1024 * 1024,
                                max_memory_bytes: 0, // unlimited
                            }))
                            .build()
                            .await
                            .expect("Failed to create storage");
                            
                            // Pre-populate with test data
                            let key = "bench-key";
                            let value = generate_data(size);
                            storage.put(key.as_bytes(), &value).await.expect("Put failed");
                            
                            storage
                        });
                        storage
                    },
                    |storage| async move {
                        let _ = storage.get(b"bench-key").await.expect("Get failed");
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    
    group.finish();
}

/// Benchmark batch operations
fn bench_batch_operations(c: &mut Criterion) {
    let runtime = create_runtime();
    
    let mut group = c.benchmark_group("batch_operations");
    group.throughput(Throughput::Elements(NUM_KEYS as u64));
    
    group.bench_function("batch_put_1000_keys", |b| {
        b.to_async(&runtime).iter_batched(
            || {
                // Setup: create storage and test data
                let storage = runtime.block_on(async {
                    StorageBuilder::new(StorageConfig::Memory(MemoryConfig {
                        initial_capacity: 100 * 1024 * 1024,
                        max_memory_bytes: 0, // unlimited
                    }))
                    .build()
                    .await
                    .expect("Failed to create storage")
                });
                
                let data: Vec<(String, Vec<u8>)> = (0..NUM_KEYS)
                    .map(|i| {
                        let key = format!("key-{:06}", i);
                        let value = generate_data(SMALL_VALUE_SIZE);
                        (key, value)
                    })
                    .collect();
                    
                (storage, data)
            },
            |(storage, data)| async move {
                for (key, value) in data {
                    storage.put(key.as_bytes(), &value).await.expect("Put failed");
                }
            },
            BatchSize::SmallInput,
        );
    });
    
    group.finish();
}

/// Benchmark concurrent operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let runtime = create_runtime();
    
    let mut group = c.benchmark_group("concurrent_operations");
    
    for num_tasks in &[10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("concurrent_puts", num_tasks),
            num_tasks,
            |b, &num_tasks| {
                b.to_async(&runtime).iter_batched(
                    || {
                        // Setup: create storage
                        runtime.block_on(async {
                            StorageBuilder::new(StorageConfig::Memory(MemoryConfig {
                                initial_capacity: 100 * 1024 * 1024,
                                max_memory_bytes: 0, // unlimited
                            }))
                            .build()
                            .await
                            .expect("Failed to create storage")
                        })
                    },
                    |storage| async move {
                        let tasks: Vec<_> = (0..num_tasks)
                            .map(|i| {
                                let storage = storage.clone();
                                let key = format!("concurrent-key-{}", i);
                                let value = generate_data(SMALL_VALUE_SIZE);
                                
                                tokio::spawn(async move {
                                    storage.put(key.as_bytes(), &value).await.expect("Put failed");
                                })
                            })
                            .collect();
                            
                        futures::future::join_all(tasks).await;
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    
    group.finish();
}

/// Benchmark list operations
fn bench_list_operations(c: &mut Criterion) {
    let runtime = create_runtime();
    
    let mut group = c.benchmark_group("list_operations");
    
    for num_keys in &[100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("list_keys", num_keys),
            num_keys,
            |b, &num_keys| {
                b.to_async(&runtime).iter_batched(
                    || {
                        // Setup: create storage with pre-populated data
                        runtime.block_on(async {
                            let storage = StorageBuilder::new(StorageConfig::Memory(MemoryConfig {
                                initial_capacity: 100 * 1024 * 1024,
                                max_memory_bytes: 0, // unlimited
                            }))
                            .build()
                            .await
                            .expect("Failed to create storage");
                            
                            // Pre-populate
                            for i in 0..num_keys {
                                let key = format!("list-key-{:06}", i);
                                let value = vec![i as u8; 10];
                                storage.put(key.as_bytes(), &value).await.expect("Put failed");
                            }
                            
                            storage
                        })
                    },
                    |storage| async move {
                        // list_keys method not available in Storage trait
                        // For benchmark purposes, simulate by getting known keys
                        for i in 0..100 {
                            let key = format!("list-key-{:06}", i);
                            let _ = storage.get(key.as_bytes()).await;
                        }
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_put_operations,
    bench_get_operations,
    bench_batch_operations,
    bench_concurrent_operations,
    bench_list_operations
);

criterion_main!(benches);