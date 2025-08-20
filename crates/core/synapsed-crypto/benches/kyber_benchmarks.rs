//! Benchmarks for Kyber key encapsulation mechanism
//!
//! This benchmark suite measures the performance of:
//! - Key generation
//! - Encapsulation
//! - Decapsulation
//! - Serialization/deserialization
//!
//! For all three parameter sets (Kyber512, Kyber768, Kyber1024)

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use synapsed_crypto::prelude::*;
use synapsed_crypto::kyber::{PublicKey as KyberPublicKey, SecretKey as KyberSecretKey};
use synapsed_crypto::params::kyber::{kyber768::K as KYBER768_K};
use synapsed_crypto::random::DefaultRng;
use synapsed_crypto::traits::Serializable;

fn bench_kyber_keygen(c: &mut Criterion) {
    let mut group = c.benchmark_group("kyber_keygen");
    
    group.bench_function("kyber512", |b| {
        let mut rng = DefaultRng::default();
        b.iter(|| {
            let (pk, sk) = Kyber512::generate_keypair(&mut rng).unwrap();
            black_box((pk, sk))
        });
    });
    
    group.bench_function("kyber768", |b| {
        let mut rng = DefaultRng::default();
        b.iter(|| {
            let (pk, sk) = Kyber768::generate_keypair(&mut rng).unwrap();
            black_box((pk, sk))
        });
    });
    
    group.bench_function("kyber1024", |b| {
        let mut rng = DefaultRng::default();
        b.iter(|| {
            let (pk, sk) = Kyber1024::generate_keypair(&mut rng).unwrap();
            black_box((pk, sk))
        });
    });
    
    group.finish();
}

fn bench_kyber_encapsulate(c: &mut Criterion) {
    let mut group = c.benchmark_group("kyber_encapsulate");
    
    // Pre-generate keys for encapsulation benchmarks
    let mut rng = DefaultRng::default();
    let (pk512, _) = Kyber512::generate_keypair(&mut rng).unwrap();
    let (pk768, _) = Kyber768::generate_keypair(&mut rng).unwrap();
    let (pk1024, _) = Kyber1024::generate_keypair(&mut rng).unwrap();
    
    group.bench_function("kyber512", |b| {
        let mut rng = DefaultRng::default();
        b.iter(|| {
            let (ct, ss) = Kyber512::encapsulate(&pk512, &mut rng).unwrap();
            black_box((ct, ss))
        });
    });
    
    group.bench_function("kyber768", |b| {
        let mut rng = DefaultRng::default();
        b.iter(|| {
            let (ct, ss) = Kyber768::encapsulate(&pk768, &mut rng).unwrap();
            black_box((ct, ss))
        });
    });
    
    group.bench_function("kyber1024", |b| {
        let mut rng = DefaultRng::default();
        b.iter(|| {
            let (ct, ss) = Kyber1024::encapsulate(&pk1024, &mut rng).unwrap();
            black_box((ct, ss))
        });
    });
    
    group.finish();
}

fn bench_kyber_decapsulate(c: &mut Criterion) {
    let mut group = c.benchmark_group("kyber_decapsulate");
    
    // Pre-generate keys and ciphertexts for decapsulation benchmarks
    let mut rng = DefaultRng::default();
    let (pk512, sk512) = Kyber512::generate_keypair(&mut rng).unwrap();
    let (ct512, _) = Kyber512::encapsulate(&pk512, &mut rng).unwrap();
    
    let (pk768, sk768) = Kyber768::generate_keypair(&mut rng).unwrap();
    let (ct768, _) = Kyber768::encapsulate(&pk768, &mut rng).unwrap();
    
    let (pk1024, sk1024) = Kyber1024::generate_keypair(&mut rng).unwrap();
    let (ct1024, _) = Kyber1024::encapsulate(&pk1024, &mut rng).unwrap();
    
    group.bench_function("kyber512", |b| {
        b.iter(|| {
            let ss = Kyber512::decapsulate(&sk512, &ct512).unwrap();
            black_box(ss)
        });
    });
    
    group.bench_function("kyber768", |b| {
        b.iter(|| {
            let ss = Kyber768::decapsulate(&sk768, &ct768).unwrap();
            black_box(ss)
        });
    });
    
    group.bench_function("kyber1024", |b| {
        b.iter(|| {
            let ss = Kyber1024::decapsulate(&sk1024, &ct1024).unwrap();
            black_box(ss)
        });
    });
    
    group.finish();
}

fn bench_kyber_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("kyber_serialization");
    
    // Pre-generate keys for serialization benchmarks
    let mut rng = DefaultRng::default();
    let (pk768, sk768) = Kyber768::generate_keypair(&mut rng).unwrap();
    
    group.bench_function("public_key_to_bytes", |b| {
        b.iter(|| {
            let bytes = pk768.to_bytes();
            black_box(bytes)
        });
    });
    
    group.bench_function("public_key_from_bytes", |b| {
        let bytes = pk768.to_bytes();
        b.iter(|| {
            let pk = KyberPublicKey::<KYBER768_K>::from_bytes(&bytes).unwrap();
            black_box(pk)
        });
    });
    
    group.bench_function("secret_key_to_bytes", |b| {
        b.iter(|| {
            let bytes = sk768.to_bytes();
            black_box(bytes)
        });
    });
    
    group.bench_function("secret_key_from_bytes", |b| {
        let bytes = sk768.to_bytes();
        b.iter(|| {
            let sk = KyberSecretKey::<KYBER768_K>::from_bytes(&bytes).unwrap();
            black_box(sk)
        });
    });
    
    group.finish();
}

fn bench_kyber_full_cycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("kyber_full_cycle");
    
    group.bench_function("kyber512", |b| {
        let mut rng = DefaultRng::default();
        b.iter(|| {
            // Full KEM cycle
            let (pk, sk) = Kyber512::generate_keypair(&mut rng).unwrap();
            let (ct, ss1) = Kyber512::encapsulate(&pk, &mut rng).unwrap();
            let ss2 = Kyber512::decapsulate(&sk, &ct).unwrap();
            assert_eq!(ss1.as_ref(), ss2.as_ref());
            black_box((ss1, ss2))
        });
    });
    
    group.bench_function("kyber768", |b| {
        let mut rng = DefaultRng::default();
        b.iter(|| {
            // Full KEM cycle
            let (pk, sk) = Kyber768::generate_keypair(&mut rng).unwrap();
            let (ct, ss1) = Kyber768::encapsulate(&pk, &mut rng).unwrap();
            let ss2 = Kyber768::decapsulate(&sk, &ct).unwrap();
            assert_eq!(ss1.as_ref(), ss2.as_ref());
            black_box((ss1, ss2))
        });
    });
    
    group.bench_function("kyber1024", |b| {
        let mut rng = DefaultRng::default();
        b.iter(|| {
            // Full KEM cycle
            let (pk, sk) = Kyber1024::generate_keypair(&mut rng).unwrap();
            let (ct, ss1) = Kyber1024::encapsulate(&pk, &mut rng).unwrap();
            let ss2 = Kyber1024::decapsulate(&sk, &ct).unwrap();
            assert_eq!(ss1.as_ref(), ss2.as_ref());
            black_box((ss1, ss2))
        });
    });
    
    group.finish();
}

fn bench_kyber_parallel(c: &mut Criterion) {
    use std::thread;
    
    let mut group = c.benchmark_group("kyber_parallel");
    
    for thread_count in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("kyber768_keygen", thread_count),
            thread_count,
            |b, &thread_count| {
                b.iter(|| {
                    let handles: Vec<_> = (0..thread_count)
                        .map(|_| {
                            thread::spawn(|| {
                                let mut rng = DefaultRng::default();
                                let (pk, sk) = Kyber768::generate_keypair(&mut rng).unwrap();
                                black_box((pk, sk))
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
    bench_kyber_keygen,
    bench_kyber_encapsulate,
    bench_kyber_decapsulate,
    bench_kyber_serialization,
    bench_kyber_full_cycle,
    bench_kyber_parallel
);

criterion_main!(benches);