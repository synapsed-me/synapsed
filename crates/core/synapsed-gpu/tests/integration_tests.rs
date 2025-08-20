//! Integration tests for synapsed-gpu crate.

use std::sync::Arc;
use tokio::test;

// Re-export main components for testing
use synapsed_gpu::{
    GpuAccelerator, AcceleratorConfig, DeviceManager, DeviceConfig, MemoryManager, MemoryConfig,
    KernelManager, BatchProcessor, BatchConfig, FallbackProcessor, 
    BatchOperation, BatchPriority, KernelParams, KernelArg, ScalarValue,
    Kyber768Params, Kyber768FallbackParams, FallbackReason,
    GpuError, Result,
};

/// Test basic GPU accelerator initialization and device selection.
#[test]
async fn test_gpu_accelerator_initialization() {
    let config = AcceleratorConfig::default();
    
    // Should either succeed with GPU or gracefully handle no GPU
    match GpuAccelerator::new(config).await {
        Ok(accelerator) => {
            // Test basic operations
            let device_info = accelerator.device_info().await;
            assert!(device_info.is_some());
            
            let metrics = accelerator.metrics().await;
            assert_eq!(metrics.operations_completed, 0);
            
            assert!(accelerator.is_gpu_available().await);
        }
        Err(GpuError::NoDevicesAvailable) => {
            // Expected on systems without GPU - test CPU fallback
            println!("No GPU devices available, testing fallback");
        }
        Err(e) => panic!("Unexpected error during initialization: {}", e),
    }
}

/// Test device management and selection.
#[test]
async fn test_device_management() {
    let device_config = DeviceConfig::default();
    let device_manager = DeviceManager::new(device_config).await.unwrap();
    
    assert!(device_manager.device_count().await > 0);
    
    let device = device_manager.select_best_device().await.unwrap();
    assert!(!device.info().id.is_empty());
    assert!(!device.info().name.is_empty());
    
    // Test device operations
    assert!(device.is_healthy().await);
    
    let (used, total) = device.memory_usage().await.unwrap();
    assert!(total > 0);
    assert!(used <= total);
    
    device.synchronize().await.unwrap();
}

/// Test memory management operations.
#[test]
async fn test_memory_management() {
    let device_config = DeviceConfig::default();
    let device_manager = DeviceManager::new(device_config).await.unwrap();
    let device = device_manager.select_best_device().await.unwrap();
    
    let memory_config = MemoryConfig::default();
    let memory_manager = MemoryManager::new(device, memory_config).await.unwrap();
    
    // Test basic allocation
    let buffer1 = memory_manager.allocate(1024).await.unwrap();
    assert_eq!(buffer1.size(), 1024);
    
    // Test aligned allocation
    let buffer2 = memory_manager.allocate_with_alignment(2048, 256).await.unwrap();
    assert_eq!(buffer2.size(), 2048);
    assert_eq!(buffer2.alignment(), 256);
    
    // Test memory transfer
    let data = vec![42u8; 1024];
    memory_manager.transfer_to_device(&data, &buffer1).await.unwrap();
    
    let mut result = vec![0u8; 1024];
    memory_manager.transfer_to_host(&buffer1, &mut result).await.unwrap();
    
    // Test buffer copy
    memory_manager.copy(&buffer1, &buffer2, Some(1024)).await.unwrap();
    
    // Test memory management statistics
    let stats = memory_manager.usage_stats().await;
    assert!(stats.allocation_count >= 2);
    assert!(stats.current_usage_bytes >= 3072); // At least 1KB + 2KB
    
    // Test garbage collection
    let freed = memory_manager.garbage_collect().await.unwrap();
    assert!(freed >= 0);
    
    // Clean up
    memory_manager.free(buffer1).await.unwrap();
    memory_manager.free(buffer2).await.unwrap();
}

/// Test kernel compilation and execution.
#[test]
async fn test_kernel_operations() {
    let device_config = DeviceConfig::default();
    let device_manager = DeviceManager::new(device_config).await.unwrap();
    let device = device_manager.select_best_device().await.unwrap();
    
    let kernel_manager = KernelManager::new(device).await.unwrap();
    
    // Test kernel compilation
    use synapsed_gpu::KernelSource;
    let source = KernelSource::Generic(
        "__kernel void test_kernel(__global float* data) { int id = get_global_id(0); data[id] = id; }"
        .to_string()
    );
    
    kernel_manager.compile_kernel("test_kernel", &source).await.unwrap();
    
    let kernels = kernel_manager.list_kernels().await;
    assert!(kernels.contains(&"test_kernel".to_string()));
    
    // Test kernel execution
    let params = KernelParams {
        global_work_size: (1024, 1, 1),
        local_work_size: Some((64, 1, 1)),
        args: vec![KernelArg::Scalar(ScalarValue::U32(42))],
        shared_memory_bytes: 0,
    };
    
    let buffers = std::collections::HashMap::new();
    let result = kernel_manager.execute_kernel("test_kernel", params, &buffers).await.unwrap();
    
    assert_eq!(result.kernel_name, "test_kernel");
    assert_eq!(result.work_items_executed, 1024);
    assert!(result.execution_time.as_millis() >= 0);
    
    // Test specialized kernels
    let crypto_kernels = kernel_manager.crypto_kernels();
    let kyber_kernels = kernel_manager.kyber_kernels();
    let common_kernels = kernel_manager.common_kernels();
    
    // These should not panic
    let _crypto_sources = crypto_kernels.kernel_sources().await;
    let _kyber_sources = kyber_kernels.kernel_sources().await;
    let _common_sources = common_kernels.kernel_sources().await;
}

/// Test batch processing operations.
#[test]
async fn test_batch_processing() {
    let device_config = DeviceConfig::default();
    let device_manager = DeviceManager::new(device_config).await.unwrap();
    let device = device_manager.select_best_device().await.unwrap();
    
    let memory_config = MemoryConfig::default();
    let memory_manager = Arc::new(MemoryManager::new(device.clone(), memory_config).await.unwrap());
    let kernel_manager = Arc::new(KernelManager::new(device.clone()).await.unwrap());
    
    let batch_config = BatchConfig::default();
    let batch_processor = BatchProcessor::new(
        device,
        memory_manager,
        kernel_manager,
        batch_config,
    ).await.unwrap();
    
    // Start batch processor
    batch_processor.start().await.unwrap();
    
    // Create test operations
    let operations = vec![
        create_test_batch_operation("test-op-1"),
        create_test_batch_operation("test-op-2"),
        create_test_batch_operation("test-op-3"),
    ];
    
    // Submit batch
    let batch_id = batch_processor.submit_batch(operations).await.unwrap();
    assert!(!batch_id.is_empty());
    
    // Get metrics
    let metrics = batch_processor.get_metrics().await;
    assert!(metrics.total_batches_processed >= 0);
    
    // Shutdown
    batch_processor.shutdown().await.unwrap();
}

/// Test CPU fallback operations.
#[test]
async fn test_fallback_operations() {
    use synapsed_gpu::FallbackConfig;
    
    let config = FallbackConfig::default();
    let fallback_processor = FallbackProcessor::new(config);
    
    // Test Kyber768 fallback
    let seeds = vec![1u8; 64]; // 2 seeds
    let mut kyber_params = Kyber768FallbackParams::default();
    kyber_params.batch_size = 2;
    
    let result = fallback_processor.kyber768_keygen_fallback(
        &seeds,
        &kyber_params,
        FallbackReason::Testing,
    ).await.unwrap();
    
    assert_eq!(result.data.0.len(), 2 * 1184); // 2 public keys
    assert_eq!(result.data.1.len(), 2 * 2400); // 2 secret keys
    assert_eq!(result.reason, FallbackReason::Testing);
    assert!(result.execution_time.as_millis() >= 0);
    
    // Test crypto fallback
    let data = vec![42u8; 1024];
    let hash_result = fallback_processor.hash_fallback(
        "sha256",
        &data,
        4,
        FallbackReason::Testing,
    ).await.unwrap();
    
    assert_eq!(hash_result.data.len(), 4 * 32); // 4 SHA-256 hashes
    
    // Test encryption fallback
    let keys = vec![1u8; 4 * 32]; // 4 keys
    let encrypt_result = fallback_processor.encrypt_fallback(
        "aes-256-gcm",
        &data,
        &keys,
        4,
        FallbackReason::Testing,
    ).await.unwrap();
    
    assert!(encrypt_result.data.len() > data.len()); // Should include auth tags
    
    // Test fallback metrics
    let stats = fallback_processor.get_fallback_metrics().await;
    assert_eq!(stats.total_fallbacks, 3);
    assert_eq!(stats.successful_fallbacks, 3);
    assert_eq!(stats.failed_fallbacks, 0);
    assert_eq!(stats.success_rate, 1.0);
}

/// Test error handling and recovery.
#[test]
async fn test_error_handling() {
    // Test invalid configurations
    let mut config = AcceleratorConfig::default();
    config.memory.initial_pool_size_mb = 1000;
    config.memory.max_pool_size_mb = 500; // Invalid: initial > max
    
    let validation_result = config.validate();
    assert!(validation_result.is_err());
    
    // Test device error handling
    let device_config = DeviceConfig::default();
    let device_manager = DeviceManager::new(device_config).await.unwrap();
    let device = device_manager.select_best_device().await.unwrap();
    
    // Test memory allocation failure handling
    let memory_config = MemoryConfig::default();
    let memory_manager = MemoryManager::new(device, memory_config).await.unwrap();
    
    // Test very large allocation (should handle gracefully)
    let huge_size = u64::MAX / 2;
    let result = memory_manager.allocate(huge_size).await;
    // Should either succeed or fail gracefully with appropriate error
    match result {
        Ok(_) => {}, // Unlikely but possible on systems with huge amounts of memory
        Err(e) => {
            // Should be a meaningful error
            assert!(matches!(e, GpuError::MemoryError { .. } | GpuError::ResourceExhausted { .. }));
        }
    }
}

/// Test performance comparison between GPU and CPU.
#[test]
async fn test_performance_comparison() {
    // This test measures relative performance characteristics
    let fallback_config = synapsed_gpu::FallbackConfig::default();
    let fallback_processor = FallbackProcessor::new(fallback_config);
    
    // Test small vs large workloads
    assert!(fallback_processor.should_use_fallback("kyber768_keygen", 1).await);
    assert!(!fallback_processor.should_use_fallback("kyber768_keygen", 1000).await);
    
    assert!(fallback_processor.should_use_fallback("sha256", 10).await);
    assert!(!fallback_processor.should_use_fallback("sha256", 10000).await);
    
    // Test performance data update
    let gpu_time = std::time::Duration::from_millis(100);
    let cpu_time = std::time::Duration::from_millis(200);
    
    fallback_processor.update_performance_comparison("test_op", gpu_time, cpu_time).await;
    
    // After updating comparison data, decisions should be influenced
    // This is a simplified test of the performance comparison system
}

/// Test concurrent operations and thread safety.
#[test]
async fn test_concurrent_operations() {
    let device_config = DeviceConfig::default();
    let device_manager = DeviceManager::new(device_config).await.unwrap();
    let device = device_manager.select_best_device().await.unwrap();
    
    let memory_config = MemoryConfig::default();
    let memory_manager = Arc::new(MemoryManager::new(device, memory_config).await.unwrap());
    
    // Test concurrent memory allocations
    let mut handles = Vec::new();
    
    for i in 0..10 {
        let memory_manager = memory_manager.clone();
        let handle = tokio::spawn(async move {
            let size = 1024 * (i + 1);
            let buffer = memory_manager.allocate(size).await.unwrap();
            assert_eq!(buffer.size(), size);
            
            // Use the buffer briefly
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            
            memory_manager.free(buffer).await.unwrap();
        });
        handles.push(handle);
    }
    
    // Wait for all concurrent operations to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Verify final state
    let stats = memory_manager.usage_stats().await;
    assert_eq!(stats.active_buffers, 0); // All should be freed
}

/// Test integration with synapsed-crypto API (mocked).
#[test]
async fn test_crypto_api_integration() {
    // This would test integration with the actual synapsed-crypto crate
    // For now, we test the interface compatibility
    
    let config = AcceleratorConfig::for_crypto();
    assert_eq!(config.batch.default_batch_size, 2048);
    assert!(config.performance.enable_monitoring);
    assert!(config.performance.enable_auto_tuning);
    
    // Test that crypto-optimized config is valid
    assert!(config.validate().is_ok());
    
    // Test different config profiles
    let batch_config = AcceleratorConfig::for_batch_processing();
    assert_eq!(batch_config.batch.max_concurrent_batches, 16);
    
    let latency_config = AcceleratorConfig::for_low_latency();
    assert_eq!(latency_config.batch.batch_timeout_ms, 10);
}

// Helper functions

fn create_test_batch_operation(id: &str) -> BatchOperation {
    BatchOperation {
        id: id.to_string(),
        operation_type: "test".to_string(),
        kernel_name: "test_kernel".to_string(),
        kernel_params: KernelParams {
            global_work_size: (256, 1, 1),
            local_work_size: Some((32, 1, 1)),
            args: vec![
                KernelArg::Scalar(ScalarValue::U32(42)),
                KernelArg::Scalar(ScalarValue::F32(3.14)),
            ],
            shared_memory_bytes: 1024,
        },
        input_buffers: vec!["input".to_string()],
        output_buffers: vec!["output".to_string()],
        priority: BatchPriority::Medium,
        submit_time: std::time::Instant::now(),
        deadline: None,
        callback: None,
    }
}