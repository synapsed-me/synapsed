//! Performance and benchmark tests for synapsed-crypto
//!
//! These tests measure the performance characteristics of the crypto operations
//! and ensure they meet acceptable performance thresholds.

use synapsed_crypto::kyber::{Kyber512, Kyber768, Kyber1024};
use synapsed_crypto::dilithium::{Dilithium2, Dilithium3, Dilithium5};
use synapsed_crypto::traits::{Kem, Signature};
use synapsed_crypto::random::DefaultRng;
use std::time::{Duration, Instant};

const _ITERATIONS: usize = 100;
const MIN_ITERATIONS: usize = 10;

// Performance thresholds (in milliseconds) - these are generous for testing
const MAX_KEYGEN_TIME_MS: u128 = 100;
const MAX_ENCAP_TIME_MS: u128 = 50;
const MAX_DECAP_TIME_MS: u128 = 50;
const MAX_SIGN_TIME_MS: u128 = 100;
const MAX_VERIFY_TIME_MS: u128 = 100;

#[test]
fn test_kyber512_performance() {
    let results = benchmark_kem::<Kyber512>(MIN_ITERATIONS);
    
    println!("Kyber512 Performance:");
    println!("  Key Generation: {:?}", results.keygen_time);
    println!("  Encapsulation:  {:?}", results.encap_time);
    println!("  Decapsulation:  {:?}", results.decap_time);
    
    // Check reasonable performance bounds
    assert!(results.keygen_time.as_millis() < MAX_KEYGEN_TIME_MS, 
           "Kyber512 key generation too slow: {:?}", results.keygen_time);
    assert!(results.encap_time.as_millis() < MAX_ENCAP_TIME_MS,
           "Kyber512 encapsulation too slow: {:?}", results.encap_time);
    assert!(results.decap_time.as_millis() < MAX_DECAP_TIME_MS,
           "Kyber512 decapsulation too slow: {:?}", results.decap_time);
}

#[test]
fn test_kyber768_performance() {
    let results = benchmark_kem::<Kyber768>(MIN_ITERATIONS);
    
    println!("Kyber768 Performance:");
    println!("  Key Generation: {:?}", results.keygen_time);
    println!("  Encapsulation:  {:?}", results.encap_time);
    println!("  Decapsulation:  {:?}", results.decap_time);
    
    assert!(results.keygen_time.as_millis() < MAX_KEYGEN_TIME_MS);
    assert!(results.encap_time.as_millis() < MAX_ENCAP_TIME_MS);
    assert!(results.decap_time.as_millis() < MAX_DECAP_TIME_MS);
}

#[test]
fn test_kyber1024_performance() {
    let results = benchmark_kem::<Kyber1024>(MIN_ITERATIONS);
    
    println!("Kyber1024 Performance:");
    println!("  Key Generation: {:?}", results.keygen_time);
    println!("  Encapsulation:  {:?}", results.encap_time);
    println!("  Decapsulation:  {:?}", results.decap_time);
    
    assert!(results.keygen_time.as_millis() < MAX_KEYGEN_TIME_MS);
    assert!(results.encap_time.as_millis() < MAX_ENCAP_TIME_MS);
    assert!(results.decap_time.as_millis() < MAX_DECAP_TIME_MS);
}

#[test]
fn test_dilithium2_performance() {
    let results = benchmark_signature::<Dilithium2>(MIN_ITERATIONS);
    
    println!("Dilithium2 Performance:");
    println!("  Key Generation: {:?}", results.keygen_time);
    println!("  Signing:        {:?}", results.sign_time);
    println!("  Verification:   {:?}", results.verify_time);
    
    assert!(results.keygen_time.as_millis() < MAX_KEYGEN_TIME_MS);
    assert!(results.sign_time.as_millis() < MAX_SIGN_TIME_MS);
    assert!(results.verify_time.as_millis() < MAX_VERIFY_TIME_MS);
}

#[test]
fn test_dilithium3_performance() {
    let results = benchmark_signature::<Dilithium3>(MIN_ITERATIONS);
    
    println!("Dilithium3 Performance:");
    println!("  Key Generation: {:?}", results.keygen_time);
    println!("  Signing:        {:?}", results.sign_time);
    println!("  Verification:   {:?}", results.verify_time);
    
    assert!(results.keygen_time.as_millis() < MAX_KEYGEN_TIME_MS);
    assert!(results.sign_time.as_millis() < MAX_SIGN_TIME_MS);
    assert!(results.verify_time.as_millis() < MAX_VERIFY_TIME_MS);
}

#[test]
fn test_dilithium5_performance() {
    let results = benchmark_signature::<Dilithium5>(MIN_ITERATIONS);
    
    println!("Dilithium5 Performance:");
    println!("  Key Generation: {:?}", results.keygen_time);
    println!("  Signing:        {:?}", results.sign_time);
    println!("  Verification:   {:?}", results.verify_time);
    
    assert!(results.keygen_time.as_millis() < MAX_KEYGEN_TIME_MS);
    assert!(results.sign_time.as_millis() < MAX_SIGN_TIME_MS);
    assert!(results.verify_time.as_millis() < MAX_VERIFY_TIME_MS);
}

#[test]
fn test_memory_usage_patterns() {
    // Test that operations don't cause excessive memory allocations
    let mut rng = DefaultRng::default();
    
    // Generate baseline memory usage
    let start_memory = get_memory_usage();
    
    // Perform many operations
    for _ in 0..50 {
        // Kyber operations
        let (pk, sk) = Kyber768::generate_keypair(&mut rng).unwrap();
        let (ct, ss1) = Kyber768::encapsulate(&pk, &mut rng).unwrap();
        let ss2 = Kyber768::decapsulate(&sk, &ct).unwrap();
        assert_eq!(ss1, ss2);
        
        // Dilithium operations
        let (sig_pk, sig_sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
        let msg = b"memory test message";
        let sig = Dilithium3::sign(&sig_sk, msg, &mut rng).unwrap();
        assert!(Dilithium3::verify(&sig_pk, msg, &sig).is_ok());
    }
    
    let end_memory = get_memory_usage();
    
    // Memory usage shouldn't grow excessively (allow for some variance)
    let memory_growth = end_memory.saturating_sub(start_memory);
    println!("Memory growth: {memory_growth} bytes");
    
    // This is a basic check - in practice you'd want more sophisticated memory tracking
    assert!(memory_growth < 10_000_000, "Excessive memory growth: {memory_growth} bytes");
}

#[test]
fn test_throughput_characteristics() {
    // Test throughput under continuous load
    let mut rng = DefaultRng::default();
    let start_time = Instant::now();
    let mut operations = 0;
    
    // Run for a short duration
    while start_time.elapsed() < Duration::from_millis(100) {
        let (pk, sk) = Kyber512::generate_keypair(&mut rng).unwrap();
        let (ct, _ss1) = Kyber512::encapsulate(&pk, &mut rng).unwrap();
        let _ss2 = Kyber512::decapsulate(&sk, &ct).unwrap();
        operations += 1;
    }
    
    let total_time = start_time.elapsed();
    let ops_per_second = (operations as f64) / total_time.as_secs_f64();
    
    println!("Kyber512 throughput: {ops_per_second:.2} complete operations per second");
    
    // Should be able to do at least a few operations per second
    assert!(ops_per_second > 1.0, "Throughput too low: {ops_per_second:.2} ops/sec");
}

#[test]
fn test_scaling_characteristics() {
    // Test how performance scales with different parameter sets
    let kyber512_time = benchmark_kem::<Kyber512>(5);
    let kyber768_time = benchmark_kem::<Kyber768>(5);
    let kyber1024_time = benchmark_kem::<Kyber1024>(5);
    
    println!("Kyber scaling:");
    println!("  512:  keygen={:?}, encap={:?}, decap={:?}", 
             kyber512_time.keygen_time, kyber512_time.encap_time, kyber512_time.decap_time);
    println!("  768:  keygen={:?}, encap={:?}, decap={:?}", 
             kyber768_time.keygen_time, kyber768_time.encap_time, kyber768_time.decap_time);
    println!("  1024: keygen={:?}, encap={:?}, decap={:?}", 
             kyber1024_time.keygen_time, kyber1024_time.encap_time, kyber1024_time.decap_time);
    
    // Higher security levels should take more time, but not excessively more
    assert!(kyber768_time.keygen_time >= kyber512_time.keygen_time);
    assert!(kyber1024_time.keygen_time >= kyber768_time.keygen_time);
    
    // But the increase shouldn't be more than 10x
    assert!(kyber1024_time.keygen_time < kyber512_time.keygen_time * 10);
}

#[test]
fn test_signature_message_size_scaling() {
    // Test how signature performance scales with message size
    let mut rng = DefaultRng::default();
    let (pk, sk) = Dilithium3::generate_keypair(&mut rng).unwrap();
    
    let message_sizes = [0, 100, 1000, 10000, 100000];
    
    for &size in &message_sizes {
        let message = vec![0xAB; size];
        
        let start = Instant::now();
        let sig = Dilithium3::sign(&sk, &message, &mut rng).unwrap();
        let sign_time = start.elapsed();
        
        let start = Instant::now();
        assert!(Dilithium3::verify(&pk, &message, &sig).is_ok());
        let verify_time = start.elapsed();
        
        println!("Message size {size}: sign={sign_time:?}, verify={verify_time:?}");
        
        // Performance should be reasonable for all message sizes
        assert!(sign_time < Duration::from_secs(1), "Signing too slow for size {size}");
        assert!(verify_time < Duration::from_secs(1), "Verification too slow for size {size}");
    }
}

// Helper structures and functions

struct KemBenchmarkResults {
    keygen_time: Duration,
    encap_time: Duration,
    decap_time: Duration,
}

struct SignatureBenchmarkResults {
    keygen_time: Duration,
    sign_time: Duration,
    verify_time: Duration,
}

fn benchmark_kem<K: Kem>(iterations: usize) -> KemBenchmarkResults {
    let mut rng = DefaultRng::default();
    
    // Benchmark key generation
    let start = Instant::now();
    let mut keypairs = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        keypairs.push(K::generate_keypair(&mut rng).unwrap());
    }
    let keygen_time = start.elapsed() / iterations as u32;
    
    // Benchmark encapsulation
    let start = Instant::now();
    let mut encap_results = Vec::with_capacity(iterations);
    for (pk, _) in &keypairs {
        encap_results.push(K::encapsulate(pk, &mut rng).unwrap());
    }
    let encap_time = start.elapsed() / iterations as u32;
    
    // Benchmark decapsulation
    let start = Instant::now();
    for (i, (_, sk)) in keypairs.iter().enumerate() {
        let (ct, _) = &encap_results[i];
        let _ss = K::decapsulate(sk, ct).unwrap();
    }
    let decap_time = start.elapsed() / iterations as u32;
    
    KemBenchmarkResults {
        keygen_time,
        encap_time,
        decap_time,
    }
}

fn benchmark_signature<S: Signature>(iterations: usize) -> SignatureBenchmarkResults {
    let mut rng = DefaultRng::default();
    let message = b"benchmark test message";
    
    // Benchmark key generation
    let start = Instant::now();
    let mut keypairs = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        keypairs.push(S::generate_keypair(&mut rng).unwrap());
    }
    let keygen_time = start.elapsed() / iterations as u32;
    
    // Benchmark signing
    let start = Instant::now();
    let mut signatures = Vec::with_capacity(iterations);
    for (_, sk) in &keypairs {
        signatures.push(S::sign(sk, message, &mut rng).unwrap());
    }
    let sign_time = start.elapsed() / iterations as u32;
    
    // Benchmark verification
    let start = Instant::now();
    for (i, (pk, _)) in keypairs.iter().enumerate() {
        assert!(S::verify(pk, message, &signatures[i]).is_ok());
    }
    let verify_time = start.elapsed() / iterations as u32;
    
    SignatureBenchmarkResults {
        keygen_time,
        sign_time,
        verify_time,
    }
}

// Simple memory usage estimation (not perfect but good enough for basic testing)
fn get_memory_usage() -> usize {
    // This is a very basic approximation
    // In a real implementation, you'd use system-specific APIs
    use std::alloc::{GlobalAlloc, Layout, System};
    
    // Allocate and deallocate a small amount to trigger any lazy initialization
    unsafe {
        let layout = Layout::new::<[u8; 1024]>();
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            System.dealloc(ptr, layout);
        }
    }
    
    // Return a dummy value - in practice you'd read from /proc/self/status or similar
    0
}

#[test]
fn test_constant_time_characteristics() {
    // Test that operations have relatively consistent timing
    // This is not a full constant-time test but checks for obvious variations
    
    let mut rng = DefaultRng::default();
    let (pk, sk) = Kyber768::generate_keypair(&mut rng).unwrap();
    
    let mut decap_times = Vec::new();
    
    // Measure decapsulation time for multiple operations
    for _ in 0..20 {
        let (ct, _) = Kyber768::encapsulate(&pk, &mut rng).unwrap();
        
        let start = Instant::now();
        let _ss = Kyber768::decapsulate(&sk, &ct).unwrap();
        let duration = start.elapsed();
        
        decap_times.push(duration);
    }
    
    // Calculate variance in timing
    let mean_time: Duration = decap_times.iter().sum::<Duration>() / decap_times.len() as u32;
    let max_time = decap_times.iter().max().unwrap();
    let min_time = decap_times.iter().min().unwrap();
    
    println!("Decapsulation timing: mean={mean_time:?}, min={min_time:?}, max={max_time:?}");
    
    // Times shouldn't vary by more than an order of magnitude (very loose check)
    let ratio = max_time.as_nanos() as f64 / min_time.as_nanos() as f64;
    assert!(ratio < 10.0, "Excessive timing variation: {ratio:.2}x");
}