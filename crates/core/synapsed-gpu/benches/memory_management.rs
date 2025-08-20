//! GPU memory management performance benchmarks.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;

fn benchmark_memory_allocation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("memory_allocation");
    group.sample_size(50);
    
    for size_mb in [1, 4, 16, 64, 256].iter() {
        let size_bytes = size_mb * 1024 * 1024;
        
        group.bench_with_input(
            BenchmarkId::new("gpu_malloc", size_mb),
            &size_bytes,
            |b, &size_bytes| {
                b.to_async(&rt).iter(|| async {
                    black_box(mock_gpu_malloc(size_bytes).await)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("pooled_alloc", size_mb),
            &size_bytes,
            |b, &size_bytes| {
                b.to_async(&rt).iter(|| async {
                    black_box(mock_pooled_allocation(size_bytes).await)
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_memory_transfer(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("memory_transfer");
    group.sample_size(30);
    
    for size_mb in [1, 4, 16, 64, 256].iter() {
        let size_bytes = size_mb * 1024 * 1024;
        
        group.bench_with_input(
            BenchmarkId::new("host_to_device", size_mb),
            &size_bytes,
            |b, &size_bytes| {
                b.to_async(&rt).iter(|| async {
                    let data = vec![1u8; size_bytes];
                    black_box(mock_host_to_device_transfer(&data).await)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("device_to_host", size_mb),
            &size_bytes,
            |b, &size_bytes| {
                b.to_async(&rt).iter(|| async {
                    black_box(mock_device_to_host_transfer(size_bytes).await)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("device_to_device", size_mb),
            &size_bytes,
            |b, &size_bytes| {
                b.to_async(&rt).iter(|| async {
                    black_box(mock_device_to_device_copy(size_bytes).await)
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_memory_patterns(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("memory_access_patterns");
    group.sample_size(40);
    
    let size_bytes = 16 * 1024 * 1024; // 16MB test size
    
    for pattern in ["sequential", "strided_2", "strided_4", "strided_8", "random"].iter() {
        group.bench_with_input(
            BenchmarkId::new("read_pattern", pattern),
            pattern,
            |b, &pattern| {
                b.to_async(&rt).iter(|| async {
                    black_box(mock_memory_access_pattern(pattern, size_bytes, "read").await)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("write_pattern", pattern),
            pattern,
            |b, &pattern| {
                b.to_async(&rt).iter(|| async {
                    black_box(mock_memory_access_pattern(pattern, size_bytes, "write").await)
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_memory_coalescing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("memory_coalescing");
    group.sample_size(30);
    
    for thread_count in [32, 64, 128, 256, 512].iter() {
        for access_size in [4, 8, 16, 32].iter() {
            group.bench_with_input(
                BenchmarkId::new("coalesced", format!("{}t_{}b", thread_count, access_size)),
                &(thread_count, access_size),
                |b, &(thread_count, access_size)| {
                    b.to_async(&rt).iter(|| async {
                        black_box(mock_coalesced_access(*thread_count, *access_size, true).await)
                    });
                },
            );
            
            group.bench_with_input(
                BenchmarkId::new("uncoalesced", format!("{}t_{}b", thread_count, access_size)),
                &(thread_count, access_size),
                |b, &(thread_count, access_size)| {
                    b.to_async(&rt).iter(|| async {
                        black_box(mock_coalesced_access(*thread_count, *access_size, false).await)
                    });
                },
            );
        }
    }
    
    group.finish();
}

fn benchmark_memory_pooling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("memory_pooling");
    group.sample_size(100);
    
    // Test different allocation sizes and frequencies
    for (alloc_size, alloc_count) in [(1024, 100), (64*1024, 50), (1024*1024, 10)].iter() {
        group.bench_with_input(
            BenchmarkId::new("pool_enabled", format!("{}b_{}x", alloc_size, alloc_count)),
            &(alloc_size, alloc_count),
            |b, &(alloc_size, alloc_count)| {
                b.to_async(&rt).iter(|| async {
                    black_box(mock_pooled_allocations(*alloc_size, *alloc_count, true).await)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("pool_disabled", format!("{}b_{}x", alloc_size, alloc_count)),
            &(alloc_size, alloc_count),
            |b, &(alloc_size, alloc_count)| {
                b.to_async(&rt).iter(|| async {
                    black_box(mock_pooled_allocations(*alloc_size, *alloc_count, false).await)
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_memory_alignment(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("memory_alignment");
    group.sample_size(50);
    
    let size_bytes = 4 * 1024 * 1024; // 4MB
    
    for alignment in [1, 4, 16, 32, 64, 128, 256, 512].iter() {
        group.bench_with_input(
            BenchmarkId::new("aligned_alloc", alignment),
            alignment,
            |b, &alignment| {
                b.to_async(&rt).iter(|| async {
                    black_box(mock_aligned_allocation(size_bytes, alignment).await)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("aligned_access", alignment),
            alignment,
            |b, &alignment| {
                b.to_async(&rt).iter(|| async {
                    black_box(mock_aligned_access(size_bytes, alignment).await)
                });
            },
        );
    }
    
    group.finish();
}

// Mock GPU memory operations

async fn mock_gpu_malloc(size_bytes: usize) -> Vec<u8> {
    // Simulate GPU allocation overhead
    let base_overhead = tokio::time::Duration::from_micros(10);
    let size_factor = tokio::time::Duration::from_nanos(size_bytes as u64 / 1000);
    
    tokio::time::sleep(base_overhead + size_factor).await;
    vec![0u8; size_bytes]
}

async fn mock_pooled_allocation(size_bytes: usize) -> Vec<u8> {
    // Pooled allocation should be faster for repeated allocations
    let base_overhead = tokio::time::Duration::from_micros(2);
    let size_factor = tokio::time::Duration::from_nanos(size_bytes as u64 / 5000);
    
    tokio::time::sleep(base_overhead + size_factor).await;
    vec![0u8; size_bytes]
}

async fn mock_host_to_device_transfer(data: &[u8]) -> Vec<u8> {
    // Simulate PCIe transfer bandwidth (~12 GB/s for PCIe 3.0 x16)
    let transfer_time = tokio::time::Duration::from_nanos(data.len() as u64 * 83); // ~12 GB/s
    let overhead = tokio::time::Duration::from_micros(5);
    
    tokio::time::sleep(transfer_time + overhead).await;
    data.to_vec()
}

async fn mock_device_to_host_transfer(size_bytes: usize) -> Vec<u8> {
    // Similar to host-to-device but slightly different overhead
    let transfer_time = tokio::time::Duration::from_nanos(size_bytes as u64 * 90); // ~11 GB/s
    let overhead = tokio::time::Duration::from_micros(3);
    
    tokio::time::sleep(transfer_time + overhead).await;
    vec![0u8; size_bytes]
}

async fn mock_device_to_device_copy(size_bytes: usize) -> Vec<u8> {
    // Device-to-device should be faster (GPU memory bandwidth ~500 GB/s)
    let transfer_time = tokio::time::Duration::from_nanos(size_bytes as u64 * 2);
    let overhead = tokio::time::Duration::from_micros(1);
    
    tokio::time::sleep(transfer_time + overhead).await;
    vec![0u8; size_bytes]
}

async fn mock_memory_access_pattern(pattern: &str, size_bytes: usize, operation: &str) -> Vec<u8> {
    // Different access patterns have different performance characteristics
    let base_time = match (pattern, operation) {
        ("sequential", "read") => size_bytes as u64 * 1,   // Best case
        ("sequential", "write") => size_bytes as u64 * 2,
        ("strided_2", "read") => size_bytes as u64 * 2,    // 2x stride
        ("strided_2", "write") => size_bytes as u64 * 3,
        ("strided_4", "read") => size_bytes as u64 * 3,    // 4x stride
        ("strided_4", "write") => size_bytes as u64 * 4,
        ("strided_8", "read") => size_bytes as u64 * 4,    // 8x stride
        ("strided_8", "write") => size_bytes as u64 * 6,
        ("random", "read") => size_bytes as u64 * 8,       // Worst case
        ("random", "write") => size_bytes as u64 * 10,
        _ => size_bytes as u64 * 4,
    };
    
    tokio::time::sleep(tokio::time::Duration::from_nanos(base_time)).await;
    vec![0u8; size_bytes]
}

async fn mock_coalesced_access(thread_count: usize, access_size: usize, coalesced: bool) -> Vec<u8> {
    let total_bytes = thread_count * access_size;
    
    // Coalesced accesses are much more efficient
    let efficiency_factor = if coalesced { 1 } else { 4 }; // 4x penalty for uncoalesced
    let access_time = tokio::time::Duration::from_nanos(total_bytes as u64 * efficiency_factor);
    
    tokio::time::sleep(access_time).await;
    vec![0u8; total_bytes]
}

async fn mock_pooled_allocations(alloc_size: usize, alloc_count: usize, pool_enabled: bool) -> Vec<Vec<u8>> {
    let mut allocations = Vec::with_capacity(alloc_count);
    
    for _ in 0..alloc_count {
        if pool_enabled {
            // Pool allocation: fast after first few allocations
            let alloc_time = tokio::time::Duration::from_nanos(100);
            tokio::time::sleep(alloc_time).await;
        } else {
            // Direct allocation: consistent overhead each time
            let alloc_time = tokio::time::Duration::from_micros(5);
            tokio::time::sleep(alloc_time).await;
        }
        
        allocations.push(vec![0u8; alloc_size]);
    }
    
    allocations
}

async fn mock_aligned_allocation(size_bytes: usize, alignment: usize) -> Vec<u8> {
    // Higher alignment may require more work to find suitable memory
    let alignment_overhead = match alignment {
        1 => tokio::time::Duration::from_nanos(10),
        4..=16 => tokio::time::Duration::from_nanos(50),
        32..=64 => tokio::time::Duration::from_nanos(100),
        128..=256 => tokio::time::Duration::from_nanos(200),
        _ => tokio::time::Duration::from_nanos(500),
    };
    
    let base_time = tokio::time::Duration::from_nanos(size_bytes as u64 / 1000);
    tokio::time::sleep(base_time + alignment_overhead).await;
    
    vec![0u8; size_bytes]
}

async fn mock_aligned_access(size_bytes: usize, alignment: usize) -> Vec<u8> {
    // Well-aligned accesses are more efficient
    let efficiency_factor = match alignment {
        1..=3 => 4,    // Unaligned: significant penalty
        4..=15 => 2,   // Partially aligned: some penalty
        16..=31 => 1,  // Well aligned: good performance
        _ => 1,        // Highly aligned: best performance
    };
    
    let access_time = tokio::time::Duration::from_nanos(size_bytes as u64 * efficiency_factor);
    tokio::time::sleep(access_time).await;
    
    vec![0u8; size_bytes]
}

criterion_group!(
    memory_benchmarks,
    benchmark_memory_allocation,
    benchmark_memory_transfer,
    benchmark_memory_patterns,
    benchmark_memory_coalescing,
    benchmark_memory_pooling,
    benchmark_memory_alignment
);

criterion_main!(memory_benchmarks);