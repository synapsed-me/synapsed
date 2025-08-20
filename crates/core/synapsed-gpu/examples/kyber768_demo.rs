//! Kyber768 post-quantum cryptography demonstration.

use synapsed_gpu::{
    FallbackProcessor, FallbackConfig, Kyber768FallbackParams, FallbackReason,
    Result,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::init();
    
    println!("🔐 Synapsed Kyber768 Post-Quantum Cryptography Demo");
    println!("==================================================");
    
    println!("\nKyber768 is a post-quantum key encapsulation mechanism (KEM) that provides");
    println!("security against both classical and quantum computer attacks.");
    println!("This demo shows batch key generation, encapsulation, and decapsulation.");
    
    // Initialize fallback processor (works on both GPU and CPU)
    let config = FallbackConfig::default();
    let processor = FallbackProcessor::new(config);
    
    // Demo parameters
    let batch_sizes = vec![1, 4, 16, 64];
    
    println!("\n" + "=".repeat(60).as_str());
    println!("Kyber768 Performance Comparison");
    println!("=".repeat(60));
    println!("{:<12} {:<15} {:<15} {:<15} {:<15}", "Batch Size", "KeyGen (ms)", "Encaps (ms)", "Decaps (ms)", "Total (ms)");
    println!("-".repeat(60));
    
    for &batch_size in &batch_sizes {
        let performance = run_kyber768_benchmark(&processor, batch_size).await?;
        
        println!("{:<12} {:<15.2} {:<15.2} {:<15.2} {:<15.2}", 
            batch_size,
            performance.keygen_time.as_millis(),
            performance.encaps_time.as_millis(), 
            performance.decaps_time.as_millis(),
            performance.total_time.as_millis()
        );
    }
    
    println!("-".repeat(60));
    
    // Detailed walkthrough with batch size 4
    println!("\n" + "=".repeat(60).as_str());
    println!("Detailed Kyber768 Walkthrough (Batch Size: 4)");
    println!("=".repeat(60));
    
    demonstrate_kyber768_workflow(&processor, 4).await?;
    
    // Security information
    println!("\n" + "=".repeat(60).as_str());
    println!("Kyber768 Security Properties");
    println!("=".repeat(60));
    print_security_information();
    
    // Performance characteristics
    println!("\n" + "=".repeat(60).as_str());
    println!("Performance Characteristics");
    println!("=".repeat(60));
    print_performance_characteristics(&processor).await;
    
    println!("\n🎉 Kyber768 demonstration completed!");
    println!("💡 This implementation provides post-quantum security for your applications.");
    
    Ok(())
}

struct PerformanceMetrics {
    keygen_time: std::time::Duration,
    encaps_time: std::time::Duration,
    decaps_time: std::time::Duration,
    total_time: std::time::Duration,
}

async fn run_kyber768_benchmark(processor: &FallbackProcessor, batch_size: u32) -> Result<PerformanceMetrics> {
    // Generate random seeds
    let seeds = generate_random_seeds(batch_size);
    let mut params = Kyber768FallbackParams::default();
    params.batch_size = batch_size;
    params.use_parallel = true;
    
    // Key Generation
    let keygen_start = std::time::Instant::now();
    let keygen_result = processor.kyber768_keygen_fallback(
        &seeds,
        &params,
        FallbackReason::Testing,
    ).await?;
    let keygen_time = keygen_start.elapsed();
    
    let (public_keys, secret_keys) = keygen_result.data;
    
    // Encapsulation
    let messages = generate_random_seeds(batch_size);
    let encaps_start = std::time::Instant::now();
    let encaps_result = processor.kyber768_encaps_fallback(
        &public_keys,
        &messages,
        &params,
        FallbackReason::Testing,
    ).await?;
    let encaps_time = encaps_start.elapsed();
    
    let (ciphertexts, shared_secrets_alice) = encaps_result.data;
    
    // Decapsulation
    let decaps_start = std::time::Instant::now();
    let decaps_result = processor.kyber768_decaps_fallback(
        &secret_keys,
        &ciphertexts,
        &params,
        FallbackReason::Testing,
    ).await?;
    let decaps_time = decaps_start.elapsed();
    
    let shared_secrets_bob = decaps_result.data;
    
    // Verify correctness
    assert_eq!(shared_secrets_alice, shared_secrets_bob, "Shared secrets must match!");
    
    Ok(PerformanceMetrics {
        keygen_time,
        encaps_time,
        decaps_time,
        total_time: keygen_time + encaps_time + decaps_time,
    })
}

async fn demonstrate_kyber768_workflow(processor: &FallbackProcessor, batch_size: u32) -> Result<()> {
    println!("\n📋 Step-by-Step Kyber768 Workflow:");
    
    // Step 1: Key Generation
    println!("\n1. 🔑 Key Generation Phase");
    println!("   Generating {} Kyber768 key pairs...", batch_size);
    
    let seeds = generate_random_seeds(batch_size);
    let mut params = Kyber768FallbackParams::default();
    params.batch_size = batch_size;
    params.use_parallel = true;
    
    let keygen_start = std::time::Instant::now();
    let keygen_result = processor.kyber768_keygen_fallback(
        &seeds,
        &params,
        FallbackReason::Testing,
    ).await?;
    let keygen_time = keygen_start.elapsed();
    
    let (public_keys, secret_keys) = keygen_result.data;
    
    println!("   ✅ Generated {} key pairs in {:?}", batch_size, keygen_time);
    println!("   📊 Public key size: {} bytes each", 1184);
    println!("   📊 Secret key size: {} bytes each", 2400);
    println!("   📊 Total public keys: {} bytes", public_keys.len());
    println!("   📊 Total secret keys: {} bytes", secret_keys.len());
    
    // Step 2: Encapsulation
    println!("\n2. 📦 Encapsulation Phase");
    println!("   Alice encapsulates shared secrets using Bob's public keys...");
    
    let messages = generate_random_seeds(batch_size);
    let encaps_start = std::time::Instant::now();
    let encaps_result = processor.kyber768_encaps_fallback(
        &public_keys,
        &messages,
        &params,
        FallbackReason::Testing,
    ).await?;
    let encaps_time = encaps_start.elapsed();
    
    let (ciphertexts, shared_secrets_alice) = encaps_result.data;
    
    println!("   ✅ Encapsulated {} shared secrets in {:?}", batch_size, encaps_time);
    println!("   📊 Ciphertext size: {} bytes each", 1088);
    println!("   📊 Shared secret size: {} bytes each", 32);
    println!("   📊 Total ciphertexts: {} bytes", ciphertexts.len());
    println!("   📊 Alice's shared secrets: {} bytes", shared_secrets_alice.len());
    
    // Step 3: Decapsulation
    println!("\n3. 🔓 Decapsulation Phase");
    println!("   Bob decapsulates shared secrets using his secret keys...");
    
    let decaps_start = std::time::Instant::now();
    let decaps_result = processor.kyber768_decaps_fallback(
        &secret_keys,
        &ciphertexts,
        &params,
        FallbackReason::Testing,
    ).await?;
    let decaps_time = decaps_start.elapsed();
    
    let shared_secrets_bob = decaps_result.data;
    
    println!("   ✅ Decapsulated {} shared secrets in {:?}", batch_size, decaps_time);
    println!("   📊 Bob's shared secrets: {} bytes", shared_secrets_bob.len());
    
    // Step 4: Verification
    println!("\n4. ✅ Verification Phase");
    if shared_secrets_alice == shared_secrets_bob {
        println!("   🎉 SUCCESS: All shared secrets match!");
        println!("   🔐 Secure communication channels established.");
        println!("   🛡️  Post-quantum security achieved.");
        
        // Show first few bytes of shared secrets for verification
        for i in 0..std::cmp::min(batch_size as usize, 2) {
            let start = i * 32;
            let end = start + 8; // Show first 8 bytes
            if end <= shared_secrets_alice.len() {
                println!("   📋 Shared secret #{}: {:02x?}...", i + 1, &shared_secrets_alice[start..end]);
            }
        }
    } else {
        println!("   ❌ ERROR: Shared secrets do not match!");
        return Err(synapsed_gpu::GpuError::FallbackError {
            message: "Kyber768 verification failed".to_string(),
        });
    }
    
    // Performance Summary
    let total_time = keygen_time + encaps_time + decaps_time;
    let ops_per_sec = (batch_size * 3) as f64 / total_time.as_secs_f64();
    
    println!("\n📊 Performance Summary:");
    println!("   ⏱️  Key generation: {:?} ({:.1} keys/sec)", keygen_time, batch_size as f64 / keygen_time.as_secs_f64());
    println!("   ⏱️  Encapsulation: {:?} ({:.1} ops/sec)", encaps_time, batch_size as f64 / encaps_time.as_secs_f64());
    println!("   ⏱️  Decapsulation: {:?} ({:.1} ops/sec)", decaps_time, batch_size as f64 / decaps_time.as_secs_f64());
    println!("   ⏱️  Total time: {:?}", total_time);
    println!("   🚀 Overall throughput: {:.1} operations/second", ops_per_sec);
    
    Ok(())
}

fn print_security_information() {
    println!("\n🛡️ Security Level: NIST Level 3 (equivalent to AES-192)");
    println!("🔒 Quantum Security: Secure against Shor's algorithm");
    println!("📏 Key Sizes:");
    println!("   • Public Key: 1,184 bytes");
    println!("   • Secret Key: 2,400 bytes");
    println!("   • Ciphertext: 1,088 bytes");
    println!("   • Shared Secret: 32 bytes");
    println!("\n🏗️ Algorithmic Foundation:");
    println!("   • Based on Module Learning with Errors (M-LWE)");
    println!("   • Lattice-based cryptography");
    println!("   • Resistant to quantum attacks via Grover's algorithm");
    println!("\n🎯 Use Cases:");
    println!("   • TLS/SSL key exchange replacement");
    println!("   • Secure messaging protocols");
    println!("   • VPN and secure tunnel establishment");
    println!("   • IoT device secure pairing");
    println!("   • Blockchain and cryptocurrency applications");
}

async fn print_performance_characteristics(processor: &FallbackProcessor) {
    println!("\n⚡ GPU vs CPU Performance:");
    
    // Small batch characteristics
    let should_fallback_small = processor.should_use_fallback("kyber768_keygen", 1).await;
    println!("   • Small batches (1-8): {} preferred", if should_fallback_small { "CPU" } else { "GPU" });
    
    // Medium batch characteristics
    let should_fallback_medium = processor.should_use_fallback("kyber768_keygen", 32).await;
    println!("   • Medium batches (16-64): {} preferred", if should_fallback_medium { "CPU" } else { "GPU" });
    
    // Large batch characteristics
    let should_fallback_large = processor.should_use_fallback("kyber768_keygen", 256).await;
    println!("   • Large batches (128+): {} preferred", if should_fallback_large { "CPU" } else { "GPU" });
    
    println!("\n🔧 Optimization Features:");
    println!("   • Automatic GPU/CPU selection based on batch size");
    println!("   • Parallel processing on multi-core CPUs");
    println!("   • Memory pooling for reduced allocation overhead");
    println!("   • SIMD optimizations where available");
    println!("   • Batch coalescing for improved throughput");
    
    println!("\n📈 Scaling Characteristics:");
    println!("   • GPU: High fixed cost, excellent scaling");
    println!("   • CPU: Low fixed cost, linear scaling");
    println!("   • Crossover point: ~16-32 operations");
    println!("   • Best GPU performance: 256+ operations");
    
    // Get current fallback statistics
    let stats = processor.get_fallback_metrics().await;
    println!("\n📊 Current Session Statistics:");
    println!("   • Total operations: {}", stats.total_fallbacks);
    println!("   • Success rate: {:.1}%", stats.success_rate * 100.0);
    println!("   • Average execution time: {:.2} ms", stats.average_execution_time_ms);
}

fn generate_random_seeds(count: u32) -> Vec<u8> {
    // Generate deterministic "random" seeds for demonstration
    let mut seeds = Vec::with_capacity((count * 32) as usize);
    let mut state = 0x9e3779b9u32; // Golden ratio based seed
    
    for i in 0..(count * 32) {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        state ^= state >> 16;
        state = state.wrapping_mul(0x85ebca6b);
        state ^= state >> 13;
        state = state.wrapping_mul(0xc2b2ae35);
        state ^= state >> 16;
        
        seeds.push((state ^ (i as u32)) as u8);
    }
    
    seeds
}