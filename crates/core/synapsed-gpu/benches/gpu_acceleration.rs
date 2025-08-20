//! GPU acceleration performance benchmarks.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;

// Import GPU acceleration components
// Note: In practice, these would import from synapsed-gpu
// For compilation, we'll use mock implementations

fn benchmark_kyber768_keygen(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("kyber768_keygen");
    
    for batch_size in [1, 16, 64, 256, 1024].iter() {
        group.bench_with_input(
            BenchmarkId::new("gpu", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    // Mock GPU Kyber768 key generation
                    let seeds = vec![0u8; batch_size * 32];
                    black_box(mock_gpu_kyber768_keygen(&seeds, batch_size).await)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("cpu", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    // Mock CPU Kyber768 key generation
                    let seeds = vec![0u8; batch_size * 32];
                    black_box(mock_cpu_kyber768_keygen(&seeds, batch_size).await)
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_sha256_batch(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("sha256_batch");
    
    for batch_size in [1, 16, 64, 256, 1024].iter() {
        let data_size = 1024; // 1KB per hash
        
        group.bench_with_input(
            BenchmarkId::new("gpu", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    let data = vec![0u8; batch_size * data_size];
                    black_box(mock_gpu_sha256_batch(&data, batch_size).await)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("cpu", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    let data = vec![0u8; batch_size * data_size];
                    black_box(mock_cpu_sha256_batch(&data, batch_size).await)
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_aes_encryption(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("aes_encryption");
    
    for batch_size in [1, 16, 64, 256, 1024].iter() {
        let data_size = 4096; // 4KB per encryption
        
        group.bench_with_input(
            BenchmarkId::new("gpu", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    let data = vec![0u8; batch_size * data_size];
                    let keys = vec![1u8; batch_size * 32];
                    black_box(mock_gpu_aes_encrypt(&data, &keys, batch_size).await)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("cpu", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    let data = vec![0u8; batch_size * data_size];
                    let keys = vec![1u8; batch_size * 32];
                    black_box(mock_cpu_aes_encrypt(&data, &keys, batch_size).await)
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_memory_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("memory_operations");
    
    for size_mb in [1, 4, 16, 64, 256].iter() {
        let size_bytes = size_mb * 1024 * 1024;
        
        group.bench_with_input(
            BenchmarkId::new("gpu_memcpy", size_mb),
            &size_bytes,
            |b, &size_bytes| {
                b.to_async(&rt).iter(|| async {
                    black_box(mock_gpu_memcpy(size_bytes).await)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("cpu_memcpy", size_mb),
            &size_bytes,
            |b, &size_bytes| {
                b.to_async(&rt).iter(|| async {
                    black_box(mock_cpu_memcpy(size_bytes).await)
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_matrix_multiplication(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("matrix_multiplication");
    
    for size in [64, 128, 256, 512, 1024].iter() {
        group.bench_with_input(
            BenchmarkId::new("gpu", size),
            size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let a = vec![1.0f32; size * size];
                    let b = vec![2.0f32; size * size];
                    black_box(mock_gpu_matrix_mul(&a, &b, size, size, size).await)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("cpu", size),
            size,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let a = vec![1.0f32; size * size];
                    let b = vec![2.0f32; size * size];
                    black_box(mock_cpu_matrix_mul(&a, &b, size, size, size).await)
                });
            },
        );
    }
    
    group.finish();
}

// Mock implementations for benchmarking

async fn mock_gpu_kyber768_keygen(seeds: &[u8], batch_size: usize) -> (Vec<u8>, Vec<u8>) {
    // Simulate GPU processing time
    tokio::time::sleep(tokio::time::Duration::from_micros(
        (batch_size as u64 * 10).min(1000)
    )).await;
    
    let pk_size = batch_size * 1184;
    let sk_size = batch_size * 2400;
    (vec![0u8; pk_size], vec![0u8; sk_size])
}

async fn mock_cpu_kyber768_keygen(seeds: &[u8], batch_size: usize) -> (Vec<u8>, Vec<u8>) {
    // Simulate CPU processing time (slower for small batches, scales linearly)
    tokio::time::sleep(tokio::time::Duration::from_micros(
        batch_size as u64 * 100
    )).await;
    
    let pk_size = batch_size * 1184;
    let sk_size = batch_size * 2400;
    (vec![0u8; pk_size], vec![0u8; sk_size])
}

async fn mock_gpu_sha256_batch(data: &[u8], batch_size: usize) -> Vec<u8> {
    // GPU is efficient for large batches
    tokio::time::sleep(tokio::time::Duration::from_micros(
        (batch_size as u64 * 2).min(500)
    )).await;
    
    vec![0u8; batch_size * 32]
}

async fn mock_cpu_sha256_batch(data: &[u8], batch_size: usize) -> Vec<u8> {
    // CPU scales linearly
    tokio::time::sleep(tokio::time::Duration::from_micros(
        batch_size as u64 * 50
    )).await;
    
    vec![0u8; batch_size * 32]
}

async fn mock_gpu_aes_encrypt(data: &[u8], keys: &[u8], batch_size: usize) -> Vec<u8> {
    // GPU has setup overhead but scales well
    tokio::time::sleep(tokio::time::Duration::from_micros(
        100 + (batch_size as u64 * 5)
    )).await;
    
    vec![0u8; data.len() + batch_size * 16] // +16 for auth tags
}

async fn mock_cpu_aes_encrypt(data: &[u8], keys: &[u8], batch_size: usize) -> Vec<u8> {
    // CPU encryption scales linearly
    tokio::time::sleep(tokio::time::Duration::from_micros(
        batch_size as u64 * 80
    )).await;
    
    vec![0u8; data.len() + batch_size * 16]
}

async fn mock_gpu_memcpy(size_bytes: usize) -> Vec<u8> {
    // GPU memory copy has high bandwidth but setup cost
    let time_us = 50 + (size_bytes as u64 / 10000); // ~10 GB/s effective
    tokio::time::sleep(tokio::time::Duration::from_micros(time_us)).await;
    
    vec![0u8; size_bytes]
}

async fn mock_cpu_memcpy(size_bytes: usize) -> Vec<u8> {
    // CPU memory copy
    let time_us = size_bytes as u64 / 5000; // ~5 GB/s
    tokio::time::sleep(tokio::time::Duration::from_micros(time_us)).await;
    
    vec![0u8; size_bytes]
}

async fn mock_gpu_matrix_mul(a: &[f32], b: &[f32], m: usize, n: usize, k: usize) -> Vec<f32> {
    // GPU matrix multiplication - very efficient for large matrices
    let ops = m * n * k;
    let time_us = 10 + (ops as u64 / 1000000); // ~1 GFLOPS base + overhead
    tokio::time::sleep(tokio::time::Duration::from_micros(time_us)).await;
    
    vec![1.0f32; m * k]
}

async fn mock_cpu_matrix_mul(a: &[f32], b: &[f32], m: usize, n: usize, k: usize) -> Vec<f32> {
    // CPU matrix multiplication - scales with operations
    let ops = m * n * k;
    let time_us = ops as u64 / 100000; // ~100 MFLOPS
    tokio::time::sleep(tokio::time::Duration::from_micros(time_us)).await;
    
    vec![1.0f32; m * k]
}

criterion_group!(
    benches,
    benchmark_kyber768_keygen,
    benchmark_sha256_batch,
    benchmark_aes_encryption,
    benchmark_memory_operations,
    benchmark_matrix_multiplication
);

criterion_main!(benches);