//! Kyber768 GPU-specific performance benchmarks.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;

fn benchmark_kyber768_gpu_keygen(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("kyber768_gpu_keygen");
    group.sample_size(20); // Reduce sample size for GPU benchmarks
    
    for batch_size in [1, 8, 32, 128, 512].iter() {
        group.bench_with_input(
            BenchmarkId::new("keygen", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    let seeds = generate_seeds(batch_size * 32);
                    black_box(mock_gpu_kyber768_keygen(&seeds, batch_size).await)
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_kyber768_gpu_encaps(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("kyber768_gpu_encaps");
    group.sample_size(20);
    
    for batch_size in [1, 8, 32, 128, 512].iter() {
        group.bench_with_input(
            BenchmarkId::new("encaps", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    let public_keys = generate_public_keys(batch_size);
                    let messages = generate_seeds(batch_size * 32);
                    black_box(mock_gpu_kyber768_encaps(&public_keys, &messages, batch_size).await)
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_kyber768_gpu_decaps(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("kyber768_gpu_decaps");
    group.sample_size(20);
    
    for batch_size in [1, 8, 32, 128, 512].iter() {
        group.bench_with_input(
            BenchmarkId::new("decaps", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    let secret_keys = generate_secret_keys(batch_size);
                    let ciphertexts = generate_ciphertexts(batch_size);
                    black_box(mock_gpu_kyber768_decaps(&secret_keys, &ciphertexts, batch_size).await)
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_kyber768_full_workflow(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("kyber768_full_workflow");
    group.sample_size(10); // Reduce for full workflow
    
    for batch_size in [1, 8, 32, 128].iter() {
        group.bench_with_input(
            BenchmarkId::new("full_kem", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    black_box(mock_gpu_kyber768_full_workflow(batch_size).await)
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_kyber768_ntt_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("kyber768_ntt");
    group.sample_size(50);
    
    for batch_size in [1, 16, 64, 256].iter() {
        group.bench_with_input(
            BenchmarkId::new("ntt_forward", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    let polynomials = generate_polynomials(batch_size);
                    black_box(mock_gpu_ntt_forward(&polynomials, batch_size).await)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("ntt_inverse", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    let polynomials = generate_polynomials(batch_size);
                    black_box(mock_gpu_ntt_inverse(&polynomials, batch_size).await)
                });
            },
        );
    }
    
    group.finish();
}

fn benchmark_kyber768_memory_patterns(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("kyber768_memory");
    group.sample_size(30);
    
    // Test different memory access patterns
    for pattern in ["sequential", "strided", "random"].iter() {
        for batch_size in [32, 128, 512].iter() {
            group.bench_with_input(
                BenchmarkId::new(*pattern, batch_size),
                &(pattern, batch_size),
                |b, &(pattern, batch_size)| {
                    b.to_async(&rt).iter(|| async {
                        black_box(mock_gpu_memory_pattern(pattern, *batch_size).await)
                    });
                },
            );
        }
    }
    
    group.finish();
}

// Mock GPU implementations for benchmarking

async fn mock_gpu_kyber768_keygen(seeds: &[u8], batch_size: usize) -> (Vec<u8>, Vec<u8>) {
    // Simulate GPU kernel launch overhead and computation
    let base_overhead = tokio::time::Duration::from_micros(100);
    let compute_time = tokio::time::Duration::from_micros(batch_size as u64 * 20);
    
    tokio::time::sleep(base_overhead + compute_time).await;
    
    // Kyber768 key sizes
    let pk_size = batch_size * 1184;
    let sk_size = batch_size * 2400;
    
    (vec![0u8; pk_size], vec![0u8; sk_size])
}

async fn mock_gpu_kyber768_encaps(public_keys: &[u8], messages: &[u8], batch_size: usize) -> (Vec<u8>, Vec<u8>) {
    let base_overhead = tokio::time::Duration::from_micros(80);
    let compute_time = tokio::time::Duration::from_micros(batch_size as u64 * 15);
    
    tokio::time::sleep(base_overhead + compute_time).await;
    
    let ct_size = batch_size * 1088; // Ciphertext size
    let ss_size = batch_size * 32;   // Shared secret size
    
    (vec![0u8; ct_size], vec![0u8; ss_size])
}

async fn mock_gpu_kyber768_decaps(secret_keys: &[u8], ciphertexts: &[u8], batch_size: usize) -> Vec<u8> {
    let base_overhead = tokio::time::Duration::from_micros(70);
    let compute_time = tokio::time::Duration::from_micros(batch_size as u64 * 18);
    
    tokio::time::sleep(base_overhead + compute_time).await;
    
    let ss_size = batch_size * 32; // Shared secret size
    vec![0u8; ss_size]
}

async fn mock_gpu_kyber768_full_workflow(batch_size: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    // Simulate complete KEM workflow
    let seeds = generate_seeds(batch_size * 32);
    let (pk, sk) = mock_gpu_kyber768_keygen(&seeds, batch_size).await;
    let messages = generate_seeds(batch_size * 32);
    let (ct, ss1) = mock_gpu_kyber768_encaps(&pk, &messages, batch_size).await;
    let ss2 = mock_gpu_kyber768_decaps(&sk, &ct, batch_size).await;
    
    // Verify shared secrets match (in real implementation)
    assert_eq!(ss1, ss2);
    
    (pk, ct, ss1)
}

async fn mock_gpu_ntt_forward(polynomials: &[i16], batch_size: usize) -> Vec<i16> {
    // NTT is highly parallel and benefits greatly from GPU
    let base_overhead = tokio::time::Duration::from_micros(50);
    let compute_time = tokio::time::Duration::from_micros(batch_size as u64 * 5);
    
    tokio::time::sleep(base_overhead + compute_time).await;
    
    polynomials.to_vec()
}

async fn mock_gpu_ntt_inverse(polynomials: &[i16], batch_size: usize) -> Vec<i16> {
    let base_overhead = tokio::time::Duration::from_micros(50);
    let compute_time = tokio::time::Duration::from_micros(batch_size as u64 * 5);
    
    tokio::time::sleep(base_overhead + compute_time).await;
    
    polynomials.to_vec()
}

async fn mock_gpu_memory_pattern(pattern: &str, batch_size: usize) -> Vec<u8> {
    // Different memory access patterns have different GPU performance characteristics
    let compute_time = match pattern {
        "sequential" => tokio::time::Duration::from_micros(batch_size as u64 * 2), // Best case
        "strided" => tokio::time::Duration::from_micros(batch_size as u64 * 4),    // Moderate
        "random" => tokio::time::Duration::from_micros(batch_size as u64 * 8),     // Worst case
        _ => tokio::time::Duration::from_micros(batch_size as u64 * 4),
    };
    
    tokio::time::sleep(compute_time).await;
    
    vec![0u8; batch_size * 1184] // Public key size
}

// Helper functions for generating test data

fn generate_seeds(count: usize) -> Vec<u8> {
    let mut seeds = Vec::with_capacity(count);
    let mut state = 0x12345678u32;
    
    for _ in 0..count {
        state = state.wrapping_mul(1103515245).wrapping_add(12345);
        seeds.push((state >> 24) as u8);
    }
    
    seeds
}

fn generate_public_keys(batch_size: usize) -> Vec<u8> {
    vec![1u8; batch_size * 1184] // Kyber768 public key size
}

fn generate_secret_keys(batch_size: usize) -> Vec<u8> {
    vec![2u8; batch_size * 2400] // Kyber768 secret key size
}

fn generate_ciphertexts(batch_size: usize) -> Vec<u8> {
    vec![3u8; batch_size * 1088] // Kyber768 ciphertext size
}

fn generate_polynomials(batch_size: usize) -> Vec<i16> {
    // Kyber768 uses polynomials of degree 256
    let mut poly = Vec::with_capacity(batch_size * 256);
    let mut state = 0x9abcdef0u32;
    
    for _ in 0..(batch_size * 256) {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        poly.push((state % 3329) as i16); // Kyber modulus
    }
    
    poly
}

criterion_group!(
    kyber768_benchmarks,
    benchmark_kyber768_gpu_keygen,
    benchmark_kyber768_gpu_encaps,
    benchmark_kyber768_gpu_decaps,
    benchmark_kyber768_full_workflow,
    benchmark_kyber768_ntt_operations,
    benchmark_kyber768_memory_patterns
);

criterion_main!(kyber768_benchmarks);