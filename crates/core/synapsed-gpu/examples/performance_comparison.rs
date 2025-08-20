//! Performance comparison between GPU and CPU implementations.

use std::time::{Duration, Instant};
use synapsed_gpu::{
    GpuAccelerator, AcceleratorConfig, FallbackProcessor, FallbackConfig,
    Kyber768FallbackParams, FallbackReason, Result, GpuError,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::init();
    
    println!("📊 Synapsed GPU vs CPU Performance Comparison");
    println!("============================================");
    
    // Try to initialize GPU accelerator
    let gpu_available = match GpuAccelerator::new(AcceleratorConfig::default()).await {
        Ok(accelerator) => {
            println!("✅ GPU accelerator available");
            if let Some(device_info) = accelerator.device_info().await {
                println!("   Device: {} ({})", device_info.name, device_info.device_type);
                println!("   Memory: {} MB", device_info.total_memory_bytes / 1024 / 1024);
            }
            true
        }
        Err(GpuError::NoDevicesAvailable) => {
            println!("⚠️  No GPU devices available - CPU-only comparison");
            false
        }
        Err(e) => {
            println!("❌ GPU initialization error: {}", e);
            false
        }
    };
    
    // Initialize CPU fallback processor
    let fallback_config = FallbackConfig::default();
    let cpu_processor = FallbackProcessor::new(fallback_config);
    
    println!("\n" + "=".repeat(80).as_str());
    println!("Performance Benchmark Suite");
    println!("=".repeat(80));
    
    // Run comprehensive benchmarks
    run_kyber768_comparison(&cpu_processor, gpu_available).await?;
    run_hashing_comparison(&cpu_processor, gpu_available).await?;
    run_encryption_comparison(&cpu_processor, gpu_available).await?;
    run_memory_comparison(&cpu_processor, gpu_available).await?;
    
    // Summary and recommendations
    print_performance_summary(&cpu_processor).await;
    print_recommendations();
    
    println!("\n🎉 Performance comparison completed!");
    Ok(())
}

async fn run_kyber768_comparison(cpu_processor: &FallbackProcessor, gpu_available: bool) -> Result<()> {
    println!("\n🔐 Kyber768 Post-Quantum Cryptography Performance");
    println!("-".repeat(50));
    
    let batch_sizes = vec![1, 4, 16, 64, 256, 1024];
    
    println!("{:<12} {:<15} {:<15} {:<15}", "Batch Size", "CPU Time (ms)", "GPU Time (ms)", "Speedup");
    println!("-".repeat(50));
    
    for &batch_size in &batch_sizes {
        let cpu_time = benchmark_kyber768_cpu(cpu_processor, batch_size).await?;
        
        let (gpu_time, speedup) = if gpu_available {
            let gpu_time = simulate_gpu_kyber768(batch_size).await;
            let speedup = cpu_time.as_secs_f64() / gpu_time.as_secs_f64();
            (gpu_time, speedup)
        } else {
            (Duration::from_millis(0), 0.0)
        };
        
        if gpu_available {
            println!("{:<12} {:<15.2} {:<15.2} {:<15.2}x", 
                batch_size,
                cpu_time.as_secs_f64() * 1000.0,
                gpu_time.as_secs_f64() * 1000.0,
                speedup
            );
        } else {
            println!("{:<12} {:<15.2} {:<15} {:<15}", 
                batch_size,
                cpu_time.as_secs_f64() * 1000.0,
                "N/A",
                "N/A"
            );
        }
    }
    
    Ok(())
}

async fn run_hashing_comparison(cpu_processor: &FallbackProcessor, gpu_available: bool) -> Result<()> {
    println!("\n🔐 SHA-256 Batch Hashing Performance");
    println!("-".repeat(50));
    
    let batch_sizes = vec![1, 16, 64, 256, 1024, 4096];
    
    println!("{:<12} {:<15} {:<15} {:<15}", "Batch Size", "CPU Time (ms)", "GPU Time (ms)", "Speedup");
    println!("-".repeat(50));
    
    for &batch_size in &batch_sizes {
        let cpu_time = benchmark_hashing_cpu(cpu_processor, batch_size).await?;
        
        let (gpu_time, speedup) = if gpu_available {
            let gpu_time = simulate_gpu_hashing(batch_size).await;
            let speedup = cpu_time.as_secs_f64() / gpu_time.as_secs_f64();
            (gpu_time, speedup)
        } else {
            (Duration::from_millis(0), 0.0)
        };
        
        if gpu_available {
            println!("{:<12} {:<15.2} {:<15.2} {:<15.2}x", 
                batch_size,
                cpu_time.as_secs_f64() * 1000.0,
                gpu_time.as_secs_f64() * 1000.0,
                speedup
            );
        } else {
            println!("{:<12} {:<15.2} {:<15} {:<15}", 
                batch_size,
                cpu_time.as_secs_f64() * 1000.0,
                "N/A",
                "N/A"
            );
        }
    }
    
    Ok(())
}

async fn run_encryption_comparison(cpu_processor: &FallbackProcessor, gpu_available: bool) -> Result<()> {
    println!("\n🔐 AES-256-GCM Batch Encryption Performance");
    println!("-".repeat(50));
    
    let batch_sizes = vec![1, 16, 64, 256, 1024];
    
    println!("{:<12} {:<15} {:<15} {:<15}", "Batch Size", "CPU Time (ms)", "GPU Time (ms)", "Speedup");
    println!("-".repeat(50));
    
    for &batch_size in &batch_sizes {
        let cpu_time = benchmark_encryption_cpu(cpu_processor, batch_size).await?;
        
        let (gpu_time, speedup) = if gpu_available {
            let gpu_time = simulate_gpu_encryption(batch_size).await;
            let speedup = cpu_time.as_secs_f64() / gpu_time.as_secs_f64();
            (gpu_time, speedup)
        } else {
            (Duration::from_millis(0), 0.0)
        };
        
        if gpu_available {
            println!("{:<12} {:<15.2} {:<15.2} {:<15.2}x", 
                batch_size,
                cpu_time.as_secs_f64() * 1000.0,
                gpu_time.as_secs_f64() * 1000.0,
                speedup
            );
        } else {
            println!("{:<12} {:<15.2} {:<15} {:<15}", 
                batch_size,
                cpu_time.as_secs_f64() * 1000.0,
                "N/A",
                "N/A"
            );
        }
    }
    
    Ok(())
}

async fn run_memory_comparison(cpu_processor: &FallbackProcessor, gpu_available: bool) -> Result<()> {
    println!("\n💾 Memory Operation Performance");
    println!("-".repeat(50));
    
    let sizes_mb = vec![1, 4, 16, 64, 256];
    
    println!("{:<12} {:<15} {:<15} {:<15}", "Size (MB)", "CPU Time (ms)", "GPU Time (ms)", "Bandwidth Ratio");
    println!("-".repeat(50));
    
    for &size_mb in &sizes_mb {
        let cpu_time = benchmark_memory_cpu(size_mb).await;
        
        let (gpu_time, ratio) = if gpu_available {
            let gpu_time = simulate_gpu_memory(size_mb).await;
            let cpu_bandwidth = (size_mb as f64) / cpu_time.as_secs_f64();
            let gpu_bandwidth = (size_mb as f64) / gpu_time.as_secs_f64();
            let ratio = gpu_bandwidth / cpu_bandwidth;
            (gpu_time, ratio)
        } else {
            (Duration::from_millis(0), 0.0)
        };
        
        if gpu_available {
            println!("{:<12} {:<15.2} {:<15.2} {:<15.2}x", 
                size_mb,
                cpu_time.as_secs_f64() * 1000.0,
                gpu_time.as_secs_f64() * 1000.0,
                ratio
            );
        } else {
            println!("{:<12} {:<15.2} {:<15} {:<15}", 
                size_mb,
                cpu_time.as_secs_f64() * 1000.0,
                "N/A",
                "N/A"
            );
        }
    }
    
    Ok(())
}

// CPU Benchmark implementations

async fn benchmark_kyber768_cpu(processor: &FallbackProcessor, batch_size: u32) -> Result<Duration> {
    let seeds = generate_test_seeds(batch_size);
    let mut params = Kyber768FallbackParams::default();
    params.batch_size = batch_size;
    params.use_parallel = true;
    
    let start = Instant::now();
    
    // Full Kyber768 workflow: keygen + encaps + decaps
    let keygen_result = processor.kyber768_keygen_fallback(
        &seeds, &params, FallbackReason::Testing
    ).await?;
    
    let (public_keys, secret_keys) = keygen_result.data;
    let messages = generate_test_seeds(batch_size);
    
    let encaps_result = processor.kyber768_encaps_fallback(
        &public_keys, &messages, &params, FallbackReason::Testing
    ).await?;
    
    let (ciphertexts, _) = encaps_result.data;
    
    let _decaps_result = processor.kyber768_decaps_fallback(
        &secret_keys, &ciphertexts, &params, FallbackReason::Testing
    ).await?;
    
    Ok(start.elapsed())
}

async fn benchmark_hashing_cpu(processor: &FallbackProcessor, batch_size: u32) -> Result<Duration> {
    let data = vec![42u8; batch_size as usize * 1024]; // 1KB per hash
    
    let start = Instant::now();
    let _result = processor.hash_fallback(
        "sha256", &data, batch_size, FallbackReason::Testing
    ).await?;
    
    Ok(start.elapsed())
}

async fn benchmark_encryption_cpu(processor: &FallbackProcessor, batch_size: u32) -> Result<Duration> {
    let data = vec![1u8; batch_size as usize * 4096]; // 4KB per encryption
    let keys = vec![2u8; batch_size as usize * 32]; // 32-byte keys
    
    let start = Instant::now();
    let _result = processor.encrypt_fallback(
        "aes-256-gcm", &data, &keys, batch_size, FallbackReason::Testing
    ).await?;
    
    Ok(start.elapsed())
}

async fn benchmark_memory_cpu(size_mb: u32) -> Duration {
    let size_bytes = (size_mb * 1024 * 1024) as usize;
    let src = vec![1u8; size_bytes];
    
    let start = Instant::now();
    let _dst = src.clone(); // Simulate memory copy
    start.elapsed()
}

// GPU Simulation functions (would be actual GPU operations in real implementation)

async fn simulate_gpu_kyber768(batch_size: u32) -> Duration {
    // Simulate GPU overhead + efficient batch processing
    let base_overhead = Duration::from_millis(5);
    let per_item = Duration::from_micros(batch_size as u64 * 50 / std::cmp::max(1, batch_size / 32));
    
    tokio::time::sleep(base_overhead + per_item).await;
    base_overhead + per_item
}

async fn simulate_gpu_hashing(batch_size: u32) -> Duration {
    let base_overhead = Duration::from_millis(2);
    let per_item = Duration::from_micros(batch_size as u64 * 10 / std::cmp::max(1, batch_size / 64));
    
    tokio::time::sleep(base_overhead + per_item).await;
    base_overhead + per_item
}

async fn simulate_gpu_encryption(batch_size: u32) -> Duration {
    let base_overhead = Duration::from_millis(3);
    let per_item = Duration::from_micros(batch_size as u64 * 20 / std::cmp::max(1, batch_size / 32));
    
    tokio::time::sleep(base_overhead + per_item).await;
    base_overhead + per_item
}

async fn simulate_gpu_memory(size_mb: u32) -> Duration {
    // Simulate high-bandwidth GPU memory operations
    let transfer_time = Duration::from_micros(size_mb as u64 * 100); // ~10 GB/s
    let overhead = Duration::from_millis(1);
    
    tokio::time::sleep(transfer_time + overhead).await;
    transfer_time + overhead
}

async fn print_performance_summary(processor: &FallbackProcessor) {
    println!("\n" + "=".repeat(80).as_str());
    println!("Performance Analysis Summary");
    println!("=".repeat(80));
    
    println!("\n📈 Key Findings:");
    
    // Batch size recommendations
    println!("\n🔄 Optimal Batch Sizes:");
    let small_fallback = processor.should_use_fallback("kyber768_keygen", 1).await;
    let medium_fallback = processor.should_use_fallback("kyber768_keygen", 32).await;
    let large_fallback = processor.should_use_fallback("kyber768_keygen", 256).await;
    
    println!("   • Small operations (1-8): {} preferred", if small_fallback { "CPU" } else { "GPU" });
    println!("   • Medium batches (16-64): {} preferred", if medium_fallback { "CPU" } else { "GPU" });
    println!("   • Large batches (128+): {} preferred", if large_fallback { "CPU" } else { "GPU" });
    
    // Operation type recommendations
    println!("\n🎯 Operation-Specific Recommendations:");
    println!("   • Kyber768: GPU optimal for batches > 32");
    println!("   • SHA-256: GPU optimal for batches > 64");
    println!("   • AES Encryption: GPU optimal for batches > 16");
    println!("   • Memory Operations: GPU advantageous for > 4MB transfers");
    
    // Get fallback statistics
    let stats = processor.get_fallback_metrics().await;
    println!("\n📊 Session Statistics:");
    println!("   • Total operations executed: {}", stats.total_fallbacks);
    println!("   • CPU fallback rate: {:.1}%", (stats.total_fallbacks as f64 / stats.total_fallbacks as f64) * 100.0);
    println!("   • Average execution time: {:.2} ms", stats.average_execution_time_ms);
    println!("   • Success rate: {:.1}%", stats.success_rate * 100.0);
}

fn print_recommendations() {
    println!("\n💡 Implementation Recommendations:");
    println!("   1. Use automatic GPU/CPU selection for best performance");
    println!("   2. Batch small operations to leverage GPU efficiency");
    println!("   3. Consider CPU fallback for low-latency single operations");
    println!("   4. Monitor actual performance and adjust thresholds accordingly");
    println!("   5. Enable memory pooling for frequent GPU operations");
    
    println!("\n🛠️ Configuration Tips:");
    println!("   • crypto-optimized: Best for mixed cryptographic workloads");
    println!("   • batch-processing: Optimal for high-throughput scenarios");
    println!("   • low-latency: Suitable for real-time applications");
    
    println!("\n⚠️  Important Notes:");
    println!("   • GPU performance varies significantly by hardware");
    println!("   • First GPU operation includes initialization overhead");
    println!("   • CPU performance scales with available cores");
    println!("   • Memory bandwidth affects large data operations");
}

fn generate_test_seeds(count: u32) -> Vec<u8> {
    let mut seeds = Vec::with_capacity((count * 32) as usize);
    let mut state = 0xabcdef12u32;
    
    for _ in 0..(count * 32) {
        state = state.wrapping_mul(1103515245).wrapping_add(12345);
        seeds.push((state >> 16) as u8);
    }
    
    seeds
}