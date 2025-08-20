//! Basic GPU acceleration usage example.

use std::sync::Arc;
use synapsed_gpu::{
    GpuAccelerator, AcceleratorConfig, 
    Kyber768Params, FallbackReason,
    Result, GpuError,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::init();
    
    println!("🚀 Synapsed GPU Acceleration Example");
    println!("=====================================");
    
    // Example 1: Basic GPU Accelerator Setup
    println!("\n1. Setting up GPU accelerator...");
    
    let config = AcceleratorConfig::for_crypto();
    
    match GpuAccelerator::new(config).await {
        Ok(accelerator) => {
            println!("✅ GPU accelerator initialized successfully!");
            
            if let Some(device_info) = accelerator.device_info().await {
                println!("   Device: {} ({})", device_info.name, device_info.device_type);
                println!("   Memory: {} MB total", device_info.total_memory_bytes / 1024 / 1024);
                println!("   Compute Capability: {:?}", device_info.compute_capability);
            }
            
            // Example 2: Basic Operations
            println!("\n2. Running basic operations...");
            
            let metrics = accelerator.metrics().await;
            println!("   Initial metrics:");
            println!("   - Operations completed: {}", metrics.operations_completed);
            println!("   - GPU memory usage: {} bytes", metrics.gpu_memory_usage_bytes);
            
            // Example 3: Performance Testing
            println!("\n3. Performance testing...");
            
            // Simulate some work
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            let final_metrics = accelerator.metrics().await;
            println!("   Final metrics:");
            println!("   - Total execution time: {} ms", final_metrics.total_execution_time_ms);
            println!("   - Error rate: {:.2}%", final_metrics.error_rate * 100.0);
            
            println!("✅ GPU operations completed successfully!");
        }
        
        Err(GpuError::NoDevicesAvailable) => {
            println!("⚠️  No GPU devices available, demonstrating CPU fallback...");
            
            // Example 4: CPU Fallback
            demonstrate_cpu_fallback().await?;
        }
        
        Err(e) => {
            eprintln!("❌ Failed to initialize GPU accelerator: {}", e);
            return Err(e);
        }
    }
    
    // Example 5: Kyber768 Operations
    println!("\n5. Kyber768 post-quantum cryptography...");
    demonstrate_kyber768_operations().await?;
    
    println!("\n🎉 All examples completed successfully!");
    Ok(())
}

async fn demonstrate_cpu_fallback() -> Result<()> {
    use synapsed_gpu::{FallbackProcessor, FallbackConfig, Kyber768FallbackParams};
    
    println!("   Setting up CPU fallback processor...");
    
    let config = FallbackConfig::default();
    let fallback_processor = FallbackProcessor::new(config);
    
    // Test small workload (should prefer CPU)
    let should_fallback = fallback_processor.should_use_fallback("kyber768_keygen", 1).await;
    println!("   Should use CPU for single Kyber768 keygen: {}", should_fallback);
    
    // Test large workload (should prefer GPU if available)
    let should_fallback_large = fallback_processor.should_use_fallback("kyber768_keygen", 1000).await;
    println!("   Should use CPU for 1000 Kyber768 keygens: {}", should_fallback_large);
    
    // Demonstrate Kyber768 CPU fallback
    println!("   Running Kyber768 key generation on CPU...");
    
    let seeds = vec![1u8; 64]; // 2 seeds of 32 bytes each
    let mut params = Kyber768FallbackParams::default();
    params.batch_size = 2;
    params.use_parallel = true;
    
    let start_time = std::time::Instant::now();
    let result = fallback_processor.kyber768_keygen_fallback(
        &seeds,
        &params,
        FallbackReason::NoGpuAvailable,
    ).await?;
    let execution_time = start_time.elapsed();
    
    println!("   ✅ Generated {} key pairs in {:?}", params.batch_size, execution_time);
    println!("   - Public keys: {} bytes", result.data.0.len());
    println!("   - Secret keys: {} bytes", result.data.1.len());
    println!("   - Performance score: {:.2}", result.performance_score);
    
    // Demonstrate cryptographic hashing fallback
    println!("   Running SHA-256 batch hashing on CPU...");
    
    let data = vec![42u8; 4096]; // 4KB of data
    let batch_size = 16;
    
    let start_time = std::time::Instant::now();
    let hash_result = fallback_processor.hash_fallback(
        "sha256",
        &data,
        batch_size,
        FallbackReason::NoGpuAvailable,
    ).await?;
    let hash_time = start_time.elapsed();
    
    println!("   ✅ Computed {} SHA-256 hashes in {:?}", batch_size, hash_time);
    println!("   - Output size: {} bytes ({} bytes per hash)", hash_result.data.len(), hash_result.data.len() / batch_size as usize);
    
    // Get fallback statistics
    let stats = fallback_processor.get_fallback_metrics().await;
    println!("   Fallback statistics:");
    println!("   - Total fallbacks: {}", stats.total_fallbacks);
    println!("   - Success rate: {:.1}%", stats.success_rate * 100.0);
    println!("   - Average execution time: {:.2} ms", stats.average_execution_time_ms);
    
    Ok(())
}

async fn demonstrate_kyber768_operations() -> Result<()> {
    use synapsed_gpu::{FallbackProcessor, FallbackConfig, Kyber768FallbackParams};
    
    println!("   Kyber768 is a post-quantum key encapsulation mechanism");
    println!("   that provides security against both classical and quantum attacks.");
    
    let config = FallbackConfig::default();
    let processor = FallbackProcessor::new(config);
    
    // Key Generation
    println!("\n   Step 1: Key Generation");
    let seeds = generate_random_seeds(4); // 4 key pairs
    let mut params = Kyber768FallbackParams::default();
    params.batch_size = 4;
    params.use_parallel = true;
    
    let keygen_result = processor.kyber768_keygen_fallback(
        &seeds,
        &params,
        FallbackReason::Testing,
    ).await?;
    
    println!("   ✅ Generated {} Kyber768 key pairs", params.batch_size);
    
    let public_keys = keygen_result.data.0;
    let secret_keys = keygen_result.data.1;
    
    // Encapsulation
    println!("\n   Step 2: Encapsulation");
    let messages = generate_random_seeds(4); // 4 messages to encapsulate
    
    let encaps_result = processor.kyber768_encaps_fallback(
        &public_keys,
        &messages,
        &params,
        FallbackReason::Testing,
    ).await?;
    
    println!("   ✅ Performed {} Kyber768 encapsulations", params.batch_size);
    
    let ciphertexts = encaps_result.data.0;
    let shared_secrets_alice = encaps_result.data.1;
    
    // Decapsulation
    println!("\n   Step 3: Decapsulation");
    
    let decaps_result = processor.kyber768_decaps_fallback(
        &secret_keys,
        &ciphertexts,
        &params,
        FallbackReason::Testing,
    ).await?;
    
    println!("   ✅ Performed {} Kyber768 decapsulations", params.batch_size);
    
    let shared_secrets_bob = decaps_result.data;
    
    // Verify shared secrets match
    println!("\n   Step 4: Verification");
    if shared_secrets_alice == shared_secrets_bob {
        println!("   ✅ All shared secrets match! Kyber768 KEM working correctly.");
        println!("   🔐 Secure communication channel established using post-quantum cryptography.");
    } else {
        println!("   ❌ Shared secret mismatch - this shouldn't happen!");
    }
    
    // Performance summary
    let total_time = keygen_result.execution_time + encaps_result.execution_time + decaps_result.execution_time;
    println!("\n   Performance Summary:");
    println!("   - Key generation: {:?}", keygen_result.execution_time);
    println!("   - Encapsulation: {:?}", encaps_result.execution_time);
    println!("   - Decapsulation: {:?}", decaps_result.execution_time);
    println!("   - Total time: {:?}", total_time);
    
    let ops_per_sec = (params.batch_size * 3) as f64 / total_time.as_secs_f64();
    println!("   - Throughput: {:.1} operations/second", ops_per_sec);
    
    Ok(())
}

fn generate_random_seeds(count: u32) -> Vec<u8> {
    // Generate deterministic "random" seeds for demonstration
    let mut seeds = Vec::with_capacity((count * 32) as usize);
    let mut state = 0x12345678u32;
    
    for _ in 0..(count * 32) {
        state = state.wrapping_mul(1103515245).wrapping_add(12345);
        seeds.push((state >> 24) as u8);
    }
    
    seeds
}