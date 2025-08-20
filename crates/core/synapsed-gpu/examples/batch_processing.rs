//! Batch processing example for GPU acceleration.

use std::sync::Arc;
use synapsed_gpu::{
    GpuAccelerator, AcceleratorConfig, BatchProcessor, BatchConfig,
    BatchOperation, BatchPriority, KernelParams, KernelArg, ScalarValue,
    Result, GpuError,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::init();
    
    println!("🚀 Synapsed GPU Batch Processing Example");
    println!("========================================");
    
    // Initialize GPU accelerator for batch processing
    let config = AcceleratorConfig::for_batch_processing();
    
    match GpuAccelerator::new(config).await {
        Ok(accelerator) => {
            println!("✅ GPU accelerator initialized for batch processing!");
            
            if let Some(device_info) = accelerator.device_info().await {
                println!("   Device: {} ({})", device_info.name, device_info.device_type);
                println!("   Memory: {} MB total", device_info.total_memory_bytes / 1024 / 1024);
            }
            
            // Demonstrate batch operations
            demonstrate_batch_operations(&accelerator).await?;
        }
        
        Err(GpuError::NoDevicesAvailable) => {
            println!("⚠️  No GPU devices available, demonstrating CPU batch processing...");
            demonstrate_cpu_batch_processing().await?;
        }
        
        Err(e) => {
            eprintln!("❌ Failed to initialize GPU accelerator: {}", e);
            return Err(e);
        }
    }
    
    println!("\n🎉 Batch processing examples completed!");
    Ok(())
}

async fn demonstrate_batch_operations(accelerator: &GpuAccelerator) -> Result<()> {
    println!("\n1. Setting up batch processing...");
    
    // Note: In a real implementation, we would access internal components
    // For this example, we'll demonstrate the conceptual workflow
    
    println!("   ✅ Batch processor configured");
    println!("   - Default batch size: 1024");
    println!("   - Max concurrent batches: 16");
    println!("   - Batch timeout: 100ms");
    
    // Example 1: Cryptographic Hash Batch
    println!("\n2. Processing SHA-256 hash batch...");
    
    let data_items = generate_test_data(100);
    println!("   Generated {} data items for hashing", data_items.len());
    
    let start_time = std::time::Instant::now();
    
    // Simulate batch processing workflow
    let batch_results = simulate_hash_batch(&data_items).await?;
    
    let processing_time = start_time.elapsed();
    println!("   ✅ Processed {} hashes in {:?}", batch_results.len(), processing_time);
    println!("   - Throughput: {:.1} hashes/second", batch_results.len() as f64 / processing_time.as_secs_f64());
    
    // Example 2: AES Encryption Batch
    println!("\n3. Processing AES encryption batch...");
    
    let encryption_data = generate_encryption_data(50);
    println!("   Generated {} encryption tasks", encryption_data.len());
    
    let start_time = std::time::Instant::now();
    let encryption_results = simulate_encryption_batch(&encryption_data).await?;
    let encryption_time = start_time.elapsed();
    
    println!("   ✅ Encrypted {} items in {:?}", encryption_results.len(), encryption_time);
    println!("   - Throughput: {:.1} encryptions/second", encryption_results.len() as f64 / encryption_time.as_secs_f64());
    
    // Example 3: Mixed Workload Batch
    println!("\n4. Processing mixed workload batch...");
    
    let mixed_operations = create_mixed_batch_operations();
    println!("   Created {} mixed operations", mixed_operations.len());
    
    let start_time = std::time::Instant::now();
    let mixed_results = simulate_mixed_batch(&mixed_operations).await?;
    let mixed_time = start_time.elapsed();
    
    println!("   ✅ Completed {} mixed operations in {:?}", mixed_results.len(), mixed_time);
    
    // Performance metrics
    let metrics = accelerator.metrics().await;
    println!("\n5. Performance Summary:");
    println!("   - Total operations: {}", metrics.operations_completed);
    println!("   - GPU memory usage: {} MB", metrics.gpu_memory_usage_bytes / 1024 / 1024);
    println!("   - Average execution time: {:.2} ms", metrics.average_execution_time_ms);
    println!("   - Success rate: {:.1}%", (1.0 - metrics.error_rate) * 100.0);
    
    Ok(())
}

async fn demonstrate_cpu_batch_processing() -> Result<()> {
    use synapsed_gpu::{FallbackProcessor, FallbackConfig};
    
    println!("   Setting up CPU batch processing...");
    
    let config = FallbackConfig::default();
    let fallback_processor = FallbackProcessor::new(config);
    
    // CPU batch hashing
    println!("\n   Processing SHA-256 batch on CPU...");
    let data = vec![42u8; 10240]; // 10KB of data
    let batch_size = 32;
    
    let start_time = std::time::Instant::now();
    let hash_result = fallback_processor.hash_fallback(
        "sha256",
        &data,
        batch_size,
        synapsed_gpu::FallbackReason::NoGpuAvailable,
    ).await?;
    let cpu_time = start_time.elapsed();
    
    println!("   ✅ CPU processed {} hashes in {:?}", batch_size, cpu_time);
    println!("   - Throughput: {:.1} hashes/second", batch_size as f64 / cpu_time.as_secs_f64());
    println!("   - Output size: {} bytes", hash_result.data.len());
    
    // CPU batch encryption
    println!("\n   Processing AES encryption batch on CPU...");
    let plaintext = vec![1u8; 4096]; // 4KB plaintext
    let keys = vec![2u8; batch_size * 32]; // 32-byte keys
    
    let start_time = std::time::Instant::now();
    let encrypt_result = fallback_processor.encrypt_fallback(
        "aes-256-gcm",
        &plaintext,
        &keys,
        batch_size,
        synapsed_gpu::FallbackReason::NoGpuAvailable,
    ).await?;
    let encrypt_time = start_time.elapsed();
    
    println!("   ✅ CPU encrypted {} items in {:?}", batch_size, encrypt_time);
    println!("   - Throughput: {:.1} encryptions/second", batch_size as f64 / encrypt_time.as_secs_f64());
    
    // Get fallback statistics
    let stats = fallback_processor.get_fallback_metrics().await;
    println!("\n   CPU Batch Processing Statistics:");
    println!("   - Total fallback operations: {}", stats.total_fallbacks);
    println!("   - Success rate: {:.1}%", stats.success_rate * 100.0);
    println!("   - Average execution time: {:.2} ms", stats.average_execution_time_ms);
    
    Ok(())
}

// Helper functions for generating test data

fn generate_test_data(count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|i| {
            let mut data = vec![0u8; 256]; // 256 bytes per item
            data[0] = (i % 256) as u8;
            data[1] = ((i / 256) % 256) as u8;
            data
        })
        .collect()
}

fn generate_encryption_data(count: usize) -> Vec<(Vec<u8>, Vec<u8>)> {
    (0..count)
        .map(|i| {
            let plaintext = vec![(i % 256) as u8; 1024]; // 1KB plaintext
            let key = vec![((i * 17) % 256) as u8; 32]; // 32-byte key
            (plaintext, key)
        })
        .collect()
}

fn create_mixed_batch_operations() -> Vec<String> {
    vec![
        "sha256_hash".to_string(),
        "aes_encrypt".to_string(),
        "sha256_hash".to_string(),
        "kyber768_keygen".to_string(),
        "aes_encrypt".to_string(),
        "sha256_hash".to_string(),
        "matrix_multiply".to_string(),
        "aes_encrypt".to_string(),
    ]
}

// Simulation functions (in real implementation, these would use actual GPU operations)

async fn simulate_hash_batch(data_items: &[Vec<u8>]) -> Result<Vec<Vec<u8>>> {
    // Simulate GPU batch hashing
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    
    Ok(data_items.iter()
        .map(|_| vec![0u8; 32]) // 32-byte SHA-256 hash
        .collect())
}

async fn simulate_encryption_batch(encryption_data: &[(Vec<u8>, Vec<u8>)]) -> Result<Vec<Vec<u8>>> {
    // Simulate GPU batch encryption
    tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;
    
    Ok(encryption_data.iter()
        .map(|(plaintext, _key)| {
            let mut ciphertext = plaintext.clone();
            ciphertext.extend_from_slice(&[0u8; 16]); // Add auth tag
            ciphertext
        })
        .collect())
}

async fn simulate_mixed_batch(operations: &[String]) -> Result<Vec<String>> {
    // Simulate mixed batch operations
    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    
    Ok(operations.iter()
        .map(|op| format!("{}_result", op))
        .collect())
}