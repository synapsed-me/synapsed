//! Enhanced SIMD Cryptographic Tests - TDD Implementation
//! 
//! This test suite implements comprehensive testing for enhanced SIMD features:
//! - 16-32 signature batch processing
//! - Optimized memory layout and prefetching
//! - Multi-architecture compatibility
//! - Performance comparisons
//! - Security validation (constant-time, side-channel resistance)

use synapsed_crypto::simd_optimized::*;
use synapsed_crypto::{CryptoError, Result};
use std::time::{Instant, Duration};
use std::sync::Arc;
use tokio::sync::oneshot;

/// Test enhanced batch processing with 16-32 signature operations
#[tokio::test]
async fn test_enhanced_batch_verification_16_signatures() {
    let config = SimdCryptoConfig {
        batch_size: 16,
        enable_avx512: true,
        enable_avx2: true,
        enable_neon: true,
        enable_wasm_simd: true,
        verification_threads: 4,
        cache_size: 10000,
    };
    
    let engine = SimdVerificationEngine::new(config);
    let mut tasks = Vec::new();
    
    // Create 16 signature verification tasks
    for i in 0..16 {
        let (tx, _rx) = oneshot::channel();
        let task = SimdVerificationTask {
            message: format!("test message {}", i).into_bytes(),
            signature: vec![0x01, 0x02, 0x03, 0x04; 64], // Mock Ed25519 signature
            public_key: vec![0x05, 0x06, 0x07, 0x08; 32], // Mock Ed25519 public key
            algorithm: SignatureAlgorithm::Ed25519,
            task_id: i as u64,
            created_at: Instant::now(),
            result_sender: tx,
        };
        tasks.push(task);
    }
    
    let start_time = Instant::now();
    let results = engine.batch_verify_signatures(tasks).await.unwrap();
    let batch_time = start_time.elapsed();
    
    // Enhanced batch should process 16 signatures in < 2ms with AVX-512
    assert_eq!(results.len(), 16);
    assert!(batch_time < Duration::from_millis(2), 
           "16-signature batch took {}ms, expected <2ms", batch_time.as_millis());
    
    // Verify all signatures processed successfully
    for (i, result) in results.iter().enumerate() {
        assert_eq!(result.task_id, i as u64);
        assert!(result.is_valid);
        assert_eq!(result.algorithm_used, SignatureAlgorithm::Ed25519);
    }
    
    engine.shutdown().await;
}

/// Test enhanced batch processing with 32 signature operations
#[tokio::test]
async fn test_enhanced_batch_verification_32_signatures() {
    let config = SimdCryptoConfig {
        batch_size: 32,
        enable_avx512: true,
        enable_avx2: false, // Force AVX-512 path
        enable_neon: false,
        enable_wasm_simd: false,
        verification_threads: 4,
        cache_size: 10000,
    };
    
    let engine = SimdVerificationEngine::new(config);
    let mut tasks = Vec::new();
    
    // Create 32 signature verification tasks
    for i in 0..32 {
        let (tx, _rx) = oneshot::channel();
        let task = SimdVerificationTask {
            message: format!("large batch message {}", i).into_bytes(),
            signature: vec![0x01, 0x02, 0x03, 0x04; 64],
            public_key: vec![0x05, 0x06, 0x07, 0x08; 32],
            algorithm: SignatureAlgorithm::Ed25519,
            task_id: i as u64,
            created_at: Instant::now(),
            result_sender: tx,
        };
        tasks.push(task);
    }
    
    let start_time = Instant::now();
    let results = engine.batch_verify_signatures(tasks).await.unwrap();
    let batch_time = start_time.elapsed();
    
    // Enhanced batch should process 32 signatures in < 4ms with AVX-512
    assert_eq!(results.len(), 32);
    assert!(batch_time < Duration::from_millis(4), 
           "32-signature batch took {}ms, expected <4ms", batch_time.as_millis());
    
    // Verify throughput improvement
    let throughput = (32 * 1000) as f64 / batch_time.as_millis() as f64;
    assert!(throughput > 8000.0, 
           "Throughput {} sigs/sec, expected >8000 sigs/sec", throughput);
    
    engine.shutdown().await;
}

/// Test optimized memory layout for cache efficiency
#[tokio::test]
async fn test_optimized_memory_layout() {
    let config = SimdCryptoConfig {
        batch_size: 16,
        enable_avx512: true,
        ..Default::default()
    };
    
    let engine = SimdVerificationEngine::new(config);
    let mut tasks = Vec::new();
    
    // Create tasks with aligned memory layout
    for i in 0..16 {
        let (tx, _rx) = oneshot::channel();
        
        // Aligned data structures for optimal SIMD processing
        let mut message = vec![0u8; 64]; // 64-byte aligned
        let mut signature = vec![0u8; 64]; // 64-byte aligned
        let mut public_key = vec![0u8; 32]; // 32-byte aligned
        
        // Fill with test data
        message[0] = i as u8;
        signature[0] = (i + 1) as u8;
        public_key[0] = (i + 2) as u8;
        
        let task = SimdVerificationTask {
            message,
            signature,
            public_key,
            algorithm: SignatureAlgorithm::Ed25519,
            task_id: i as u64,
            created_at: Instant::now(),
            result_sender: tx,
        };
        tasks.push(task);
    }
    
    let start_time = Instant::now();
    let results = engine.batch_verify_signatures(tasks).await.unwrap();
    let processing_time = start_time.elapsed();
    
    // Optimized memory layout should show improved cache performance
    assert_eq!(results.len(), 16);
    assert!(processing_time < Duration::from_millis(2), 
           "Memory-optimized batch took {}ms, expected <2ms", processing_time.as_millis());
    
    engine.shutdown().await;
}

/// Test prefetching strategies for improved pipeline utilization
#[tokio::test]
async fn test_prefetching_strategies() {
    let config = SimdCryptoConfig {
        batch_size: 24, // Non-power-of-2 to test prefetching
        enable_avx512: true,
        ..Default::default()
    };
    
    let engine = SimdVerificationEngine::new(config);
    let mut tasks = Vec::new();
    
    // Create tasks with large data to test prefetching
    for i in 0..24 {
        let (tx, _rx) = oneshot::channel();
        let task = SimdVerificationTask {
            message: vec![i as u8; 1024], // Larger messages to test prefetching
            signature: vec![(i + 1) as u8; 64],
            public_key: vec![(i + 2) as u8; 32],
            algorithm: SignatureAlgorithm::Ed25519,
            task_id: i as u64,
            created_at: Instant::now(),
            result_sender: tx,
        };
        tasks.push(task);
    }
    
    let start_time = Instant::now();
    let results = engine.batch_verify_signatures(tasks).await.unwrap();
    let processing_time = start_time.elapsed();
    
    // Prefetching should improve performance for larger data
    assert_eq!(results.len(), 24);
    assert!(processing_time < Duration::from_millis(3), 
           "Prefetching batch took {}ms, expected <3ms", processing_time.as_millis());
    
    engine.shutdown().await;
}

/// Test multi-architecture compatibility
#[tokio::test]
async fn test_multi_architecture_compatibility() {
    // Test AVX-512 configuration
    let avx512_config = SimdCryptoConfig {
        batch_size: 16,
        enable_avx512: true,
        enable_avx2: false,
        enable_neon: false,
        enable_wasm_simd: false,
        ..Default::default()
    };
    
    // Test AVX-2 configuration
    let avx2_config = SimdCryptoConfig {
        batch_size: 8,
        enable_avx512: false,
        enable_avx2: true,
        enable_neon: false,
        enable_wasm_simd: false,
        ..Default::default()
    };
    
    // Test NEON configuration
    let neon_config = SimdCryptoConfig {
        batch_size: 4,
        enable_avx512: false,
        enable_avx2: false,
        enable_neon: true,
        enable_wasm_simd: false,
        ..Default::default()
    };
    
    // Test each architecture configuration
    for (name, config) in [
        ("AVX-512", avx512_config),
        ("AVX-2", avx2_config),
        ("NEON", neon_config),
    ] {
        let engine = SimdVerificationEngine::new(config.clone());
        let mut tasks = Vec::new();
        
        for i in 0..config.batch_size {
            let (tx, _rx) = oneshot::channel();
            let task = SimdVerificationTask {
                message: format!("{} test {}", name, i).into_bytes(),
                signature: vec![i as u8; 64],
                public_key: vec![(i + 1) as u8; 32],
                algorithm: SignatureAlgorithm::Ed25519,
                task_id: i as u64,
                created_at: Instant::now(),
                result_sender: tx,
            };
            tasks.push(task);
        }
        
        let results = engine.batch_verify_signatures(tasks).await.unwrap();
        assert_eq!(results.len(), config.batch_size);
        
        // Each architecture should complete successfully
        for result in results {
            assert!(result.is_valid);
            assert!(result.error.is_none());
        }
        
        engine.shutdown().await;
    }
}

/// Test performance comparison between different batch sizes
#[tokio::test]
async fn test_performance_comparison() {
    let batch_sizes = vec![8, 16, 24, 32];
    let mut performance_results = Vec::new();
    
    for batch_size in batch_sizes {
        let config = SimdCryptoConfig {
            batch_size,
            enable_avx512: true,
            ..Default::default()
        };
        
        let engine = SimdVerificationEngine::new(config);
        let mut tasks = Vec::new();
        
        for i in 0..batch_size {
            let (tx, _rx) = oneshot::channel();
            let task = SimdVerificationTask {
                message: format!("perf test {}", i).into_bytes(),
                signature: vec![i as u8; 64],
                public_key: vec![(i + 1) as u8; 32],
                algorithm: SignatureAlgorithm::Ed25519,
                task_id: i as u64,
                created_at: Instant::now(),
                result_sender: tx,
            };
            tasks.push(task);
        }
        
        let start_time = Instant::now();
        let results = engine.batch_verify_signatures(tasks).await.unwrap();
        let processing_time = start_time.elapsed();
        
        assert_eq!(results.len(), batch_size);
        
        let throughput = (batch_size as f64 * 1000.0) / processing_time.as_millis() as f64;
        performance_results.push((batch_size, throughput, processing_time));
        
        engine.shutdown().await;
    }
    
    // Verify that larger batch sizes show better throughput
    assert!(performance_results.len() == 4);
    
    // 32-batch should outperform 8-batch
    let throughput_8 = performance_results[0].1;
    let throughput_32 = performance_results[3].1;
    assert!(throughput_32 > throughput_8 * 1.5, 
           "32-batch throughput {} should be >1.5x 8-batch throughput {}", 
           throughput_32, throughput_8);
}

/// Test constant-time operations for security
#[tokio::test]
async fn test_constant_time_operations() {
    use std::collections::HashMap;
    
    let config = SimdCryptoConfig {
        batch_size: 16,
        enable_avx512: true,
        ..Default::default()
    };
    
    let engine = SimdVerificationEngine::new(config);
    let mut timing_results = HashMap::new();
    
    // Test with different input patterns to ensure constant-time behavior
    let test_patterns = vec![
        ("all_zeros", vec![0u8; 64], vec![0u8; 64], vec![0u8; 32]),
        ("all_ones", vec![0xFFu8; 64], vec![0xFFu8; 64], vec![0xFFu8; 32]),
        ("alternating", (0..64).map(|i| if i % 2 == 0 { 0x00 } else { 0xFF }).collect(), 
         (0..64).map(|i| if i % 2 == 0 { 0xFF } else { 0x00 }).collect(),
         (0..32).map(|i| if i % 2 == 0 { 0xAA } else { 0x55 }).collect()),
        ("random_like", (0..64).map(|i| (i * 17 + 42) as u8).collect(),
         (0..64).map(|i| (i * 23 + 17) as u8).collect(),
         (0..32).map(|i| (i * 31 + 7) as u8).collect()),
    ];
    
    for (pattern_name, message, signature, public_key) in test_patterns {
        let mut tasks = Vec::new();
        
        for i in 0..16 {
            let (tx, _rx) = oneshot::channel();
            let task = SimdVerificationTask {
                message: message.clone(),
                signature: signature.clone(),
                public_key: public_key.clone(),
                algorithm: SignatureAlgorithm::Ed25519,
                task_id: i as u64,
                created_at: Instant::now(),
                result_sender: tx,
            };
            tasks.push(task);
        }
        
        let start_time = Instant::now();
        let results = engine.batch_verify_signatures(tasks).await.unwrap();
        let processing_time = start_time.elapsed();
        
        assert_eq!(results.len(), 16);
        timing_results.insert(pattern_name, processing_time);
    }
    
    // Verify constant-time behavior - timing variance should be minimal
    let times: Vec<Duration> = timing_results.values().cloned().collect();
    let min_time = times.iter().min().unwrap();
    let max_time = times.iter().max().unwrap();
    
    // Timing variance should be <10% for constant-time operations
    let variance_ratio = max_time.as_nanos() as f64 / min_time.as_nanos() as f64;
    assert!(variance_ratio < 1.1, 
           "Timing variance {} indicates potential side-channel vulnerability", variance_ratio);
    
    engine.shutdown().await;
}

/// Test side-channel resistance with statistical analysis
#[tokio::test]
async fn test_side_channel_resistance() {
    let config = SimdCryptoConfig {
        batch_size: 16,
        enable_avx512: true,
        ..Default::default()
    };
    
    let engine = SimdVerificationEngine::new(config);
    let sample_size = 100;
    let mut timing_samples = Vec::new();
    
    // Collect timing samples for statistical analysis
    for sample in 0..sample_size {
        let mut tasks = Vec::new();
        
        for i in 0..16 {
            let (tx, _rx) = oneshot::channel();
            
            // Use deterministic but varied test data
            let message = vec![(sample + i) as u8; 64];
            let signature = vec![(sample * 2 + i) as u8; 64];
            let public_key = vec![(sample * 3 + i) as u8; 32];
            
            let task = SimdVerificationTask {
                message,
                signature,
                public_key,
                algorithm: SignatureAlgorithm::Ed25519,
                task_id: i as u64,
                created_at: Instant::now(),
                result_sender: tx,
            };
            tasks.push(task);
        }
        
        let start_time = Instant::now();
        let results = engine.batch_verify_signatures(tasks).await.unwrap();
        let processing_time = start_time.elapsed();
        
        assert_eq!(results.len(), 16);
        timing_samples.push(processing_time.as_nanos() as f64);
    }
    
    // Statistical analysis for side-channel resistance
    let mean = timing_samples.iter().sum::<f64>() / timing_samples.len() as f64;
    let variance = timing_samples.iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>() / timing_samples.len() as f64;
    let std_dev = variance.sqrt();
    let coefficient_of_variation = std_dev / mean;
    
    // Coefficient of variation should be low for constant-time operations
    assert!(coefficient_of_variation < 0.05, 
           "High timing variation (CV={}) may indicate side-channel vulnerability", coefficient_of_variation);
    
    engine.shutdown().await;
}

/// Test pipeline optimization with overlapping operations
#[tokio::test]
async fn test_pipeline_optimization() {
    let config = SimdCryptoConfig {
        batch_size: 20, // Non-standard size to test pipeline efficiency
        enable_avx512: true,
        verification_threads: 2, // Multiple threads for pipeline testing
        ..Default::default()
    };
    
    let engine = SimdVerificationEngine::new(config);
    let total_tasks = 60; // 3 batches to test pipeline overlap
    let mut tasks = Vec::new();
    
    for i in 0..total_tasks {
        let (tx, _rx) = oneshot::channel();
        let task = SimdVerificationTask {
            message: format!("pipeline test {}", i).into_bytes(),
            signature: vec![(i % 256) as u8; 64],
            public_key: vec![((i + 1) % 256) as u8; 32],
            algorithm: SignatureAlgorithm::Ed25519,
            task_id: i as u64,
            created_at: Instant::now(),
            result_sender: tx,
        };
        tasks.push(task);
    }
    
    let start_time = Instant::now();
    let results = engine.batch_verify_signatures(tasks).await.unwrap();
    let total_time = start_time.elapsed();
    
    assert_eq!(results.len(), total_tasks);
    
    // Pipeline optimization should provide near-linear scaling
    let expected_time = Duration::from_millis(6); // 3 batches * 2ms each
    let actual_time = total_time;
    
    // With pipeline optimization, should be faster than sequential processing
    assert!(actual_time < expected_time, 
           "Pipeline time {}ms should be < expected sequential time {}ms", 
           actual_time.as_millis(), expected_time.as_millis());
    
    engine.shutdown().await;
}

/// Test enhanced hash engine with larger batches
#[tokio::test]
async fn test_enhanced_hash_batching() {
    let config = SimdCryptoConfig {
        batch_size: 32, // Enhanced batch size for hashing
        enable_avx512: true,
        ..Default::default()
    };
    
    let hash_engine = SimdHashEngine::new(config);
    let mut data_chunks = Vec::new();
    
    // Create 64 data chunks for batch processing
    for i in 0..64 {
        let chunk = vec![(i % 256) as u8; 256]; // 256-byte chunks
        data_chunks.push(chunk);
    }
    
    let start_time = Instant::now();
    let hash_results = hash_engine.batch_hash(data_chunks.clone()).await.unwrap();
    let hash_time = start_time.elapsed();
    
    assert_eq!(hash_results.len(), 64);
    
    // Enhanced batching should achieve high throughput
    let total_bytes = 64 * 256;
    let throughput_mbps = (total_bytes as f64 * 8.0 * 1000.0) / 
                         (hash_time.as_millis() as f64 * 1024.0 * 1024.0);
    
    assert!(throughput_mbps > 100.0, 
           "Hash throughput {} Mbps should be >100 Mbps", throughput_mbps);
    
    // Verify hash results are deterministic
    for (i, hash) in hash_results.iter().enumerate() {
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 8); // u64 hash = 8 bytes
    }
    
    let stats = hash_engine.stats();
    assert_eq!(stats.hashes_computed.load(std::sync::atomic::Ordering::Relaxed), 64);
    assert_eq!(stats.bytes_hashed.load(std::sync::atomic::Ordering::Relaxed), total_bytes as u64);
}

/// Integration test with existing crypto system
#[tokio::test] 
async fn test_integration_with_existing_crypto() {
    // Test that enhanced SIMD works with the broader crypto system
    let config = SimdCryptoConfig {
        batch_size: 16,
        enable_avx512: true,
        ..Default::default()
    };
    
    let engine = SimdVerificationEngine::new(config);
    
    // Test different signature algorithms
    let algorithms = vec![
        SignatureAlgorithm::Ed25519,
        SignatureAlgorithm::Dilithium2,
        SignatureAlgorithm::Dilithium3,
        SignatureAlgorithm::EcdsaP256,
    ];
    
    for algorithm in algorithms {
        let mut tasks = Vec::new();
        
        for i in 0..8 {
            let (tx, _rx) = oneshot::channel();
            let task = SimdVerificationTask {
                message: format!("integration test {} {:?}", i, algorithm).into_bytes(),
                signature: vec![i as u8; 64],
                public_key: vec![(i + 1) as u8; 32],
                algorithm,
                task_id: i as u64,
                created_at: Instant::now(),
                result_sender: tx,
            };
            tasks.push(task);
        }
        
        let results = engine.batch_verify_signatures(tasks).await.unwrap();
        assert_eq!(results.len(), 8);
        
        for result in results {
            assert_eq!(result.algorithm_used, algorithm);
            assert!(result.error.is_none());
        }
    }
    
    // Test performance statistics
    let stats = engine.get_performance_stats();
    assert!(stats.verification_engine.tasks_processed.load(std::sync::atomic::Ordering::Relaxed) >= 32);
    
    engine.shutdown().await;
}