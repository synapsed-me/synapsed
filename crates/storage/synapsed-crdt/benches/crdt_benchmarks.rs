//! Benchmarks for CRDT implementations

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use synapsed_crdt::*;
use tokio::runtime::Runtime;

// Helper to create runtime for async benchmarks
fn create_runtime() -> Runtime {
    Runtime::new().unwrap()
}

// RGA Benchmarks
fn bench_rga_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("rga_insert");
    
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("sequential", size), size, |b, &size| {
            let rt = create_runtime();
            b.iter(|| {
                rt.block_on(async {
                    let actor_id = ActorId::new();
                    let mut rga = Rga::new(actor_id);
                    
                    for i in 0..size {
                        rga.insert_at_offset(i, 'a').await.unwrap();
                    }
                    
                    black_box(rga.len())
                })
            });
        });
    }
    
    group.finish();
}

fn bench_rga_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("rga_delete");
    
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("sequential", size), size, |b, &size| {
            let rt = create_runtime();
            b.iter(|| {
                rt.block_on(async {
                    let actor_id = ActorId::new();
                    let mut rga = Rga::new(actor_id);
                    
                    // First insert
                    for i in 0..size {
                        rga.insert_at_offset(i, 'a').await.unwrap();
                    }
                    
                    // Then delete
                    for _ in 0..size/2 {
                        rga.delete_at_offset(0).await.unwrap();
                    }
                    
                    black_box(rga.len())
                })
            });
        });
    }
    
    group.finish();
}

fn bench_rga_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("rga_merge");
    
    for size in [100, 1000, 5000].iter() {
        group.bench_with_input(BenchmarkId::new("two_documents", size), size, |b, &size| {
            let rt = create_runtime();
            b.iter(|| {
                rt.block_on(async {
                    let actor1 = ActorId::new();
                    let actor2 = ActorId::new();
                    let mut rga1 = Rga::new(actor1);
                    let mut rga2 = Rga::new(actor2);
                    
                    // Each RGA gets half the insertions
                    for i in 0..size/2 {
                        rga1.insert_at_offset(i, 'a').await.unwrap();
                        rga2.insert_at_offset(i, 'b').await.unwrap();
                    }
                    
                    // Merge
                    rga1.merge(&rga2).await.unwrap();
                    
                    black_box(rga1.len())
                })
            });
        });
    }
    
    group.finish();
}

// OR-Set Benchmarks
fn bench_or_set_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("or_set");
    
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("add", size), size, |b, &size| {
            let rt = create_runtime();
            b.iter(|| {
                rt.block_on(async {
                    let actor_id = ActorId::new();
                    let mut set = OrSet::new(actor_id);
                    
                    for i in 0..size {
                        set.add(format!("item{}", i)).await.unwrap();
                    }
                    
                    black_box(set.len())
                })
            });
        });
        
        group.bench_with_input(BenchmarkId::new("add_remove", size), size, |b, &size| {
            let rt = create_runtime();
            b.iter(|| {
                rt.block_on(async {
                    let actor_id = ActorId::new();
                    let mut set = OrSet::new(actor_id);
                    
                    // Add elements
                    for i in 0..size {
                        set.add(format!("item{}", i)).await.unwrap();
                    }
                    
                    // Remove half
                    for i in 0..size/2 {
                        set.remove(&format!("item{}", i)).await.unwrap();
                    }
                    
                    black_box(set.len())
                })
            });
        });
    }
    
    group.finish();
}

// PN-Counter Benchmarks
fn bench_pn_counter_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("pn_counter");
    
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("increment", size), size, |b, &size| {
            let rt = create_runtime();
            b.iter(|| {
                rt.block_on(async {
                    let actor_id = ActorId::new();
                    let mut counter = PnCounter::new(actor_id);
                    
                    for _ in 0..size {
                        counter.increment(1).await.unwrap();
                    }
                    
                    black_box(counter.value())
                })
            });
        });
        
        group.bench_with_input(BenchmarkId::new("mixed_ops", size), size, |b, &size| {
            let rt = create_runtime();
            b.iter(|| {
                rt.block_on(async {
                    let actor_id = ActorId::new();
                    let mut counter = PnCounter::new(actor_id);
                    
                    for i in 0..size {
                        if i % 2 == 0 {
                            counter.increment(1).await.unwrap();
                        } else {
                            counter.decrement(1).await.unwrap();
                        }
                    }
                    
                    black_box(counter.value())
                })
            });
        });
    }
    
    group.finish();
}

// LWW Register Benchmarks
fn bench_lww_register(c: &mut Criterion) {
    let mut group = c.benchmark_group("lww_register");
    
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("set", size), size, |b, &size| {
            let rt = create_runtime();
            b.iter(|| {
                rt.block_on(async {
                    let actor_id = ActorId::new();
                    let mut lww = LwwRegister::new(actor_id);
                    
                    for i in 0..size {
                        lww.set(format!("value{}", i)).await.unwrap();
                    }
                    
                    black_box(lww.get())
                })
            });
        });
    }
    
    group.finish();
}

// Memory usage benchmarks
fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    
    group.bench_function("rga_size_10k", |b| {
        let rt = create_runtime();
        b.iter(|| {
            rt.block_on(async {
                let actor_id = ActorId::new();
                let mut rga = Rga::new(actor_id);
                
                for i in 0..10000 {
                    rga.insert_at_offset(i, 'a').await.unwrap();
                }
                
                black_box(rga.size_bytes())
            })
        });
    });
    
    group.bench_function("or_set_size_10k", |b| {
        let rt = create_runtime();
        b.iter(|| {
            rt.block_on(async {
                let actor_id = ActorId::new();
                let mut set = OrSet::new(actor_id);
                
                for i in 0..10000 {
                    set.add(format!("item{}", i)).await.unwrap();
                }
                
                black_box(set.size_bytes())
            })
        });
    });
    
    group.finish();
}

// Concurrent operations benchmark
fn bench_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_ops");
    
    group.bench_function("rga_concurrent_inserts", |b| {
        let rt = create_runtime();
        b.iter(|| {
            rt.block_on(async {
                let actor1 = ActorId::new();
                let actor2 = ActorId::new();
                let mut rga1 = Rga::new(actor1);
                let mut rga2 = Rga::new(actor2);
                
                // Simulate concurrent operations
                let ops1 = async {
                    for i in 0..500 {
                        rga1.insert_at_offset(i, 'a').await.unwrap();
                    }
                };
                
                let ops2 = async {
                    for i in 0..500 {
                        rga2.insert_at_offset(i, 'b').await.unwrap();
                    }
                };
                
                // Run concurrently
                tokio::join!(ops1, ops2);
                
                // Merge results
                rga1.merge(&rga2).await.unwrap();
                
                black_box(rga1.len())
            })
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_rga_insert,
    bench_rga_delete,
    bench_rga_merge,
    bench_or_set_operations,
    bench_pn_counter_operations,
    bench_lww_register,
    bench_memory_usage,
    bench_concurrent_operations
);

criterion_main!(benches);