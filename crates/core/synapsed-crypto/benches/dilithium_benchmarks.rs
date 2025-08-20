//! Benchmarks for Dilithium digital signature algorithm
//!
//! This benchmark suite measures the performance of:
//! - Key generation
//! - Signing
//! - Verification
//! - Serialization/deserialization
//!
//! For all three parameter sets (Dilithium2, Dilithium3, Dilithium5)

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use synapsed_crypto::prelude::*;
use synapsed_crypto::dilithium::{DilithiumPublicKey, DilithiumSecretKey, DilithiumSignature};
use synapsed_crypto::params::dilithium::{dilithium3::K as DILITHIUM3_K};
use synapsed_crypto::random::DefaultRng;
use synapsed_crypto::traits::Serializable;

fn bench_dilithium_keygen(c: &mut Criterion) {
    let mut group = c.benchmark_group("dilithium_keygen");
    
    group.bench_function("dilithium2", |b| {
        let mut rng = DefaultRng::default();
        b.iter(|| {
            let (pk, sk) = Dilithium2::generate_keypair(&mut rng).unwrap();
            black_box((pk, sk))
        });
    });
    
    group.bench_function("dilithium3", |b| {
        let mut rng = DefaultRng::default();
        b.iter(|| {
            let (pk, sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
            black_box((pk, sk))
        });
    });
    
    group.bench_function("dilithium5", |b| {
        let mut rng = DefaultRng::default();
        b.iter(|| {
            let (pk, sk) = Dilithium5::generate_keypair(&mut rng).unwrap();
            black_box((pk, sk))
        });
    });
    
    group.finish();
}

fn bench_dilithium_sign(c: &mut Criterion) {
    let mut group = c.benchmark_group("dilithium_sign");
    
    // Pre-generate keys for signing benchmarks
    let mut rng = DefaultRng::default();
    let (_, sk2) = Dilithium2::generate_keypair(&mut rng).unwrap();
    let (_, sk3) = Dilithium3::generate_keypair(&mut rng).unwrap();
    let (_, sk5) = Dilithium5::generate_keypair(&mut rng).unwrap();
    
    // Test with different message sizes
    let message_sizes = vec![32, 64, 256, 1024, 4096];
    
    for size in message_sizes {
        let message = vec![0x42u8; size];
        
        group.bench_with_input(
            BenchmarkId::new("dilithium2", size),
            &message,
            |b, msg| {
                let mut rng = DefaultRng::default();
                b.iter(|| {
                    let sig = Dilithium2::sign(&sk2, msg, &mut rng).unwrap();
                    black_box(sig)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("dilithium3", size),
            &message,
            |b, msg| {
                let mut rng = DefaultRng::default();
                b.iter(|| {
                    let sig = Dilithium3::sign(&sk3, msg, &mut rng).unwrap();
                    black_box(sig)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("dilithium5", size),
            &message,
            |b, msg| {
                let mut rng = DefaultRng::default();
                b.iter(|| {
                    let sig = Dilithium5::sign(&sk5, msg, &mut rng).unwrap();
                    black_box(sig)
                });
            },
        );
    }
    
    group.finish();
}

fn bench_dilithium_verify(c: &mut Criterion) {
    let mut group = c.benchmark_group("dilithium_verify");
    
    // Pre-generate keys and signatures for verification benchmarks
    let mut rng = DefaultRng::default();
    let (pk2, sk2) = Dilithium2::generate_keypair(&mut rng).unwrap();
    let (pk3, sk3) = Dilithium3::generate_keypair(&mut rng).unwrap();
    let (pk5, sk5) = Dilithium5::generate_keypair(&mut rng).unwrap();
    
    let message_sizes = vec![32, 64, 256, 1024, 4096];
    
    for size in message_sizes {
        let message = vec![0x42u8; size];
        
        let mut rng = DefaultRng::default();
        let sig2 = Dilithium2::sign(&sk2, &message, &mut rng).unwrap();
        let sig3 = Dilithium3::sign(&sk3, &message, &mut rng).unwrap();
        let sig5 = Dilithium5::sign(&sk5, &message, &mut rng).unwrap();
        
        group.bench_with_input(
            BenchmarkId::new("dilithium2", size),
            &message,
            |b, msg| {
                b.iter(|| {
                    let result = Dilithium2::verify(&pk2, msg, &sig2).unwrap();
                    black_box(result)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("dilithium3", size),
            &message,
            |b, msg| {
                b.iter(|| {
                    let result = Dilithium3::verify(&pk3, msg, &sig3).unwrap();
                    black_box(result)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("dilithium5", size),
            &message,
            |b, msg| {
                b.iter(|| {
                    let result = Dilithium5::verify(&pk5, msg, &sig5).unwrap();
                    black_box(result)
                });
            },
        );
    }
    
    group.finish();
}

fn bench_dilithium_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("dilithium_serialization");
    
    // Pre-generate keys for serialization benchmarks
    let mut rng = DefaultRng::default();
    let (pk3, sk3) = Dilithium3::generate_keypair(&mut rng).unwrap();
    
    group.bench_function("public_key_to_bytes", |b| {
        b.iter(|| {
            let bytes = pk3.to_bytes();
            black_box(bytes)
        });
    });
    
    group.bench_function("public_key_from_bytes", |b| {
        let bytes = pk3.to_bytes();
        b.iter(|| {
            let pk = DilithiumPublicKey::<DILITHIUM3_K>::from_bytes(&bytes).unwrap();
            black_box(pk)
        });
    });
    
    group.bench_function("secret_key_to_bytes", |b| {
        b.iter(|| {
            let bytes = sk3.to_bytes();
            black_box(bytes)
        });
    });
    
    group.bench_function("secret_key_from_bytes", |b| {
        let bytes = sk3.to_bytes();
        b.iter(|| {
            let sk = DilithiumSecretKey::<DILITHIUM3_K>::from_bytes(&bytes).unwrap();
            black_box(sk)
        });
    });
    
    group.finish();
}

fn bench_dilithium_full_cycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("dilithium_full_cycle");
    
    let message = b"Benchmark message for full signature cycle";
    
    group.bench_function("dilithium2", |b| {
        let mut rng = DefaultRng::default();
        b.iter(|| {
            // Full signature cycle
            let (pk, sk) = Dilithium2::generate_keypair(&mut rng).unwrap();
            let sig = Dilithium2::sign(&sk, message, &mut rng).unwrap();
            let result = Dilithium2::verify(&pk, message, &sig).unwrap();
            black_box(result)
        });
    });
    
    group.bench_function("dilithium3", |b| {
        let mut rng = DefaultRng::default();
        b.iter(|| {
            // Full signature cycle
            let (pk, sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
            let sig = Dilithium3::sign(&sk, message, &mut rng).unwrap();
            let result = Dilithium3::verify(&pk, message, &sig).unwrap();
            black_box(result)
        });
    });
    
    group.bench_function("dilithium5", |b| {
        let mut rng = DefaultRng::default();
        b.iter(|| {
            // Full signature cycle
            let (pk, sk) = Dilithium5::generate_keypair(&mut rng).unwrap();
            let sig = Dilithium5::sign(&sk, message, &mut rng).unwrap();
            let result = Dilithium5::verify(&pk, message, &sig).unwrap();
            black_box(result)
        });
    });
    
    group.finish();
}

fn bench_dilithium_batch_verify(c: &mut Criterion) {
    let mut group = c.benchmark_group("dilithium_batch_verify");
    
    // Pre-generate multiple signatures for batch verification
    let mut rng = DefaultRng::default();
    let (pk, sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
    let batch_sizes = vec![10, 50, 100];
    
    for batch_size in batch_sizes {
        let messages: Vec<Vec<u8>> = (0..batch_size)
            .map(|i| format!("Message {i}").into_bytes())
            .collect();
        
        let mut rng = DefaultRng::default();
        let signatures: Vec<DilithiumSignature> = messages
            .iter()
            .map(|msg| Dilithium3::sign(&sk, msg, &mut rng).unwrap())
            .collect();
        
        group.bench_with_input(
            BenchmarkId::new("batch_size", batch_size),
            &(messages, signatures),
            |b, (msgs, sigs)| {
                b.iter(|| {
                    for (msg, sig) in msgs.iter().zip(sigs.iter()) {
                        let result = Dilithium3::verify(&pk, msg, sig).unwrap();
                        black_box(result);
                    }
                });
            },
        );
    }
    
    group.finish();
}

fn bench_dilithium_parallel(c: &mut Criterion) {
    use std::thread;
    use std::sync::Arc;
    
    let mut group = c.benchmark_group("dilithium_parallel");
    
    let message = b"Parallel benchmark message";
    
    for thread_count in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("dilithium3_sign", thread_count),
            thread_count,
            |b, &thread_count| {
                let mut rng = DefaultRng::default();
                let (_, sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
                let sk = Arc::new(sk);
                
                b.iter(|| {
                    let handles: Vec<_> = (0..thread_count)
                        .map(|_| {
                            let sk = sk.clone();
                            thread::spawn(move || {
                                let mut rng = DefaultRng::default();
                                let sig = Dilithium3::sign(&sk, message, &mut rng).unwrap();
                                black_box(sig)
                            })
                        })
                        .collect();
                    
                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_dilithium_keygen,
    bench_dilithium_sign,
    bench_dilithium_verify,
    bench_dilithium_serialization,
    bench_dilithium_full_cycle,
    bench_dilithium_batch_verify,
    bench_dilithium_parallel
);

criterion_main!(benches);