//! Enhanced SIMD Cryptographic Benchmarks
//! 
//! Comprehensive benchmarks for enhanced SIMD features:
//! - 16-32 signature batch processing performance
//! - Memory optimization validation
//! - Multi-architecture comparison
//! - Side-channel resistance measurement

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use synapsed_crypto::simd_optimized::*;
use tokio::sync::oneshot;
use std::time::Instant;

/// Benchmark enhanced batch verification with different sizes
fn bench_enhanced_batch_verification(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("enhanced_batch_verification");
    
    // Test different batch sizes: 8, 16, 24, 32
    for batch_size in [8, 16, 24, 32].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("ed25519_avx512", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    let config = SimdCryptoConfig {
                        batch_size: batch_size as usize,
                        enable_avx512: true,
                        enable_avx2: false,
                        enable_neon: false,
                        enable_wasm_simd: false,
                        verification_threads: 1,
                        cache_size: 1000,
                    };
                    
                    let engine = SimdVerificationEngine::new(config);
                    let mut tasks = Vec::new();
                    
                    for i in 0..batch_size {
                        let (tx, _rx) = oneshot::channel();
                        let task = SimdVerificationTask {
                            message: format!("benchmark message {}", i).into_bytes(),
                            signature: vec![i as u8; 64],
                            public_key: vec![(i + 1) as u8; 32],
                            algorithm: SignatureAlgorithm::Ed25519,
                            task_id: i as u64,
                            created_at: Instant::now(),
                            result_sender: tx,
                        };
                        tasks.push(task);
                    }
                    
                    let _results = engine.batch_verify_signatures(tasks).await.unwrap();
                    engine.shutdown().await;
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("dilithium3_avx512", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    let config = SimdCryptoConfig {
                        batch_size: batch_size as usize,
                        enable_avx512: true,
                        enable_avx2: false,
                        enable_neon: false,
                        enable_wasm_simd: false,
                        verification_threads: 1,
                        cache_size: 1000,
                    };
                    
                    let engine = SimdVerificationEngine::new(config);
                    let mut tasks = Vec::new();
                    
                    for i in 0..batch_size {
                        let (tx, _rx) = oneshot::channel();
                        let task = SimdVerificationTask {
                            message: format!("dilithium benchmark {}", i).into_bytes(),
                            signature: vec![i as u8; 3309], // Dilithium3 signature size
                            public_key: vec![(i + 1) as u8; 1952], // Dilithium3 public key size
                            algorithm: SignatureAlgorithm::Dilithium3,
                            task_id: i as u64,
                            created_at: Instant::now(),
                            result_sender: tx,
                        };
                        tasks.push(task);
                    }
                    
                    let _results = engine.batch_verify_signatures(tasks).await.unwrap();
                    engine.shutdown().await;
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark hash engine enhancements
fn bench_enhanced_hash_engine(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("enhanced_hash_engine");
    
    // Test different chunk counts: 16, 32, 64, 128
    for chunk_count in [16, 32, 64, 128].iter() {
        group.throughput(Throughput::Elements(*chunk_count as u64));
        
        group.bench_with_input(
            BenchmarkId::new("batch_hash_avx512", chunk_count),
            chunk_count,
            |b, &chunk_count| {
                b.to_async(&rt).iter(|| async {
                    let config = SimdCryptoConfig {
                        batch_size: 32,
                        enable_avx512: true,
                        enable_avx2: false,
                        enable_neon: false,
                        enable_wasm_simd: false,
                        verification_threads: 1,
                        cache_size: 1000,
                    };
                    
                    let hash_engine = SimdHashEngine::new(config);
                    let mut data_chunks = Vec::new();
                    
                    for i in 0..chunk_count {
                        let chunk = vec![(i % 256) as u8; 256]; // 256-byte chunks
                        data_chunks.push(chunk);
                    }
                    
                    let _results = hash_engine.batch_hash(data_chunks).await.unwrap();
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark architecture-specific optimizations
fn bench_architecture_comparison(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("architecture_comparison");
    group.throughput(Throughput::Elements(16));
    
    // AVX-512 configuration
    group.bench_function("avx512_16_sigs", |b| {
        b.to_async(&rt).iter(|| async {
            let config = SimdCryptoConfig {
                batch_size: 16,
                enable_avx512: true,
                enable_avx2: false,
                enable_neon: false,
                enable_wasm_simd: false,
                verification_threads: 1,
                cache_size: 1000,
            };
            
            let engine = SimdVerificationEngine::new(config);
            let mut tasks = Vec::new();
            
            for i in 0..16 {
                let (tx, _rx) = oneshot::channel();
                let task = SimdVerificationTask {
                    message: format!("avx512 test {}", i).into_bytes(),
                    signature: vec![i as u8; 64],
                    public_key: vec![(i + 1) as u8; 32],
                    algorithm: SignatureAlgorithm::Ed25519,
                    task_id: i as u64,
                    created_at: Instant::now(),
                    result_sender: tx,
                };
                tasks.push(task);
            }
            
            let _results = engine.batch_verify_signatures(tasks).await.unwrap();
            engine.shutdown().await;
        });
    });
    
    // AVX-2 configuration
    group.bench_function("avx2_16_sigs", |b| {
        b.to_async(&rt).iter(|| async {
            let config = SimdCryptoConfig {
                batch_size: 16,
                enable_avx512: false,
                enable_avx2: true,
                enable_neon: false,
                enable_wasm_simd: false,
                verification_threads: 1,
                cache_size: 1000,
            };
            
            let engine = SimdVerificationEngine::new(config);
            let mut tasks = Vec::new();
            
            for i in 0..16 {
                let (tx, _rx) = oneshot::channel();
                let task = SimdVerificationTask {
                    message: format!("avx2 test {}", i).into_bytes(),
                    signature: vec![i as u8; 64],
                    public_key: vec![(i + 1) as u8; 32],
                    algorithm: SignatureAlgorithm::Ed25519,
                    task_id: i as u64,
                    created_at: Instant::now(),
                    result_sender: tx,
                };
                tasks.push(task);
            }
            
            let _results = engine.batch_verify_signatures(tasks).await.unwrap();
            engine.shutdown().await;
        });
    });
    
    // NEON configuration
    group.bench_function("neon_8_sigs", |b| {
        b.to_async(&rt).iter(|| async {
            let config = SimdCryptoConfig {
                batch_size: 8,
                enable_avx512: false,
                enable_avx2: false,
                enable_neon: true,
                enable_wasm_simd: false,
                verification_threads: 1,
                cache_size: 1000,
            };
            
            let engine = SimdVerificationEngine::new(config);
            let mut tasks = Vec::new();
            
            for i in 0..8 {
                let (tx, _rx) = oneshot::channel();
                let task = SimdVerificationTask {
                    message: format!("neon test {}", i).into_bytes(),
                    signature: vec![i as u8; 64],
                    public_key: vec![(i + 1) as u8; 32],
                    algorithm: SignatureAlgorithm::Ed25519,
                    task_id: i as u64,
                    created_at: Instant::now(),
                    result_sender: tx,
                };
                tasks.push(task);
            }
            
            let _results = engine.batch_verify_signatures(tasks).await.unwrap();
            engine.shutdown().await;
        });
    });
    
    group.finish();
}

/// Benchmark constant-time validation and side-channel resistance
fn bench_security_features(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("security_features");
    group.throughput(Throughput::Elements(100));
    
    group.bench_function("constant_time_validation", |b| {
        b.to_async(&rt).iter(|| async {
            let config = SimdCryptoConfig {
                batch_size: 16,
                enable_avx512: true,
                enable_avx2: false,
                enable_neon: false,
                enable_wasm_simd: false,
                verification_threads: 1,
                cache_size: 1000,
            };
            
            let engine = SimdVerificationEngine::new(config);
            
            // Test with 100 iterations for statistical analysis
            for _iteration in 0..100 {
                let mut tasks = Vec::new();
                
                for i in 0..16 {
                    let (tx, _rx) = oneshot::channel();
                    let task = SimdVerificationTask {
                        message: vec![(i % 256) as u8; 64], // Consistent size
                        signature: vec![(i + 1) as u8; 64], // Consistent size
                        public_key: vec![(i + 2) as u8; 32], // Consistent size
                        algorithm: SignatureAlgorithm::Ed25519,
                        task_id: i as u64,
                        created_at: Instant::now(),
                        result_sender: tx,
                    };
                    tasks.push(task);
                }
                
                let _results = engine.batch_verify_signatures(tasks).await.unwrap();
            }
            
            engine.shutdown().await;
        });
    });
    
    group.finish();
}

/// Benchmark memory layout optimizations
fn bench_memory_optimizations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("memory_optimizations");
    group.throughput(Throughput::Elements(32));
    
    group.bench_function("cache_optimized_layout", |b| {
        b.to_async(&rt).iter(|| async {
            let config = SimdCryptoConfig {
                batch_size: 32,
                enable_avx512: true,
                enable_avx2: false,
                enable_neon: false,
                enable_wasm_simd: false,
                verification_threads: 1,
                cache_size: 1000,
            };
            
            let engine = SimdVerificationEngine::new(config);
            let mut tasks = Vec::new();
            
            // Create tasks with mixed algorithms to test sorting optimization
            for i in 0..32 {
                let (tx, _rx) = oneshot::channel();
                let algorithm = match i % 3 {
                    0 => SignatureAlgorithm::Ed25519,
                    1 => SignatureAlgorithm::Dilithium3,
                    _ => SignatureAlgorithm::EcdsaP256,
                };
                
                let (sig_len, key_len) = match algorithm {
                    SignatureAlgorithm::Ed25519 => (64, 32),
                    SignatureAlgorithm::Dilithium3 => (3309, 1952),
                    SignatureAlgorithm::EcdsaP256 => (64, 33),
                    _ => (64, 32),
                };
                
                let task = SimdVerificationTask {
                    message: format!("cache opt test {}", i).into_bytes(),
                    signature: vec![i as u8; sig_len],
                    public_key: vec![(i + 1) as u8; key_len],
                    algorithm,
                    task_id: i as u64,
                    created_at: Instant::now(),
                    result_sender: tx,
                };
                tasks.push(task);
            }
            
            let _results = engine.batch_verify_signatures(tasks).await.unwrap();
            engine.shutdown().await;
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_enhanced_batch_verification,
    bench_enhanced_hash_engine,
    bench_architecture_comparison,
    bench_security_features,
    bench_memory_optimizations
);

criterion_main!(benches);