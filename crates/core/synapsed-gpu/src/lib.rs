//! # Synapsed GPU Acceleration
//!
//! High-performance GPU acceleration for Synapsed cryptographic operations,
//! providing transparent CUDA and OpenCL backends with automatic fallback.
//!
//! ## Features
//!
//! - **Multi-backend Support**: CUDA and OpenCL with runtime selection
//! - **Transparent Acceleration**: Drop-in replacement for CPU operations
//! - **Automatic Fallback**: Seamless CPU fallback when GPU unavailable
//! - **Batch Processing**: Efficient batch operations for throughput
//! - **Memory Management**: Optimized GPU memory allocation and transfers
//! - **Error Recovery**: Robust error handling and recovery mechanisms
//!
//! ## Usage
//!
//! ```rust
//! use synapsed_gpu::{GpuAccelerator, AcceleratorConfig};
//!
//! // Initialize GPU accelerator with automatic device selection
//! let config = AcceleratorConfig::default();
//! let accelerator = GpuAccelerator::new(config).await?;
//!
//! // Accelerate Kyber768 operations
//! let result = accelerator.kyber768_keygen_batch(&seeds).await?;
//! ```

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

pub mod device;
pub mod kernels;
pub mod memory;
pub mod batch;
pub mod fallback;
pub mod error;
pub mod config;

pub use device::{Device, DeviceManager, DeviceType, DeviceInfo};
pub use kernels::{KernelManager, CryptoKernels};
pub use memory::{MemoryManager, GpuBuffer, MemoryPool};
pub use batch::{BatchProcessor, BatchOperation, BatchResult};
pub use fallback::{FallbackProcessor, FallbackReason};
pub use error::{GpuError, Result};
pub use config::{AcceleratorConfig, DeviceConfig, MemoryConfig};

/// Main GPU acceleration interface providing transparent GPU acceleration
/// for Synapsed cryptographic operations.
#[derive(Debug)]
pub struct GpuAccelerator {
    device_manager: Arc<DeviceManager>,
    memory_manager: Arc<MemoryManager>,
    kernel_manager: Arc<KernelManager>,
    batch_processor: Arc<BatchProcessor>,
    fallback_processor: Arc<FallbackProcessor>,
    config: AcceleratorConfig,
    state: Arc<RwLock<AcceleratorState>>,
}

#[derive(Debug, Clone)]
struct AcceleratorState {
    active_device: Option<Device>,
    performance_metrics: PerformanceMetrics,
    error_count: u64,
    fallback_count: u64,
}

#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    pub operations_completed: u64,
    pub total_execution_time_ms: u64,
    pub gpu_memory_usage_bytes: u64,
    pub batch_efficiency: f64,
    pub error_rate: f64,
}

impl GpuAccelerator {
    /// Create a new GPU accelerator with the specified configuration.
    ///
    /// This will automatically detect available GPU devices and select
    /// the best one based on the configuration preferences.
    pub async fn new(config: AcceleratorConfig) -> Result<Self> {
        info!("Initializing GPU accelerator with config: {:?}", config);

        let device_manager = Arc::new(DeviceManager::new(config.device.clone()).await?);
        let active_device = device_manager.select_best_device().await?;
        
        info!("Selected GPU device: {:?}", active_device);

        let memory_manager = Arc::new(
            MemoryManager::new(active_device.clone(), config.memory.clone()).await?
        );
        
        let kernel_manager = Arc::new(
            KernelManager::new(active_device.clone()).await?
        );
        
        let batch_processor = Arc::new(
            BatchProcessor::new(
                active_device.clone(),
                memory_manager.clone(),
                kernel_manager.clone(),
                config.batch.clone(),
            ).await?
        );
        
        let fallback_processor = Arc::new(
            FallbackProcessor::new(config.fallback.clone())
        );

        let state = Arc::new(RwLock::new(AcceleratorState {
            active_device: Some(active_device),
            performance_metrics: PerformanceMetrics::default(),
            error_count: 0,
            fallback_count: 0,
        }));

        Ok(Self {
            device_manager,
            memory_manager,
            kernel_manager,
            batch_processor,
            fallback_processor,
            config,
            state,
        })
    }

    /// Initialize with automatic device detection and optimal configuration.
    pub async fn with_auto_config() -> Result<Self> {
        let config = AcceleratorConfig::auto_detect().await?;
        Self::new(config).await
    }

    /// Get current performance metrics.
    pub async fn metrics(&self) -> PerformanceMetrics {
        self.state.read().await.performance_metrics.clone()
    }

    /// Get information about the active GPU device.
    pub async fn device_info(&self) -> Option<DeviceInfo> {
        self.state.read().await.active_device.as_ref()
            .map(|device| device.info().clone())
    }

    /// Check if GPU acceleration is currently available.
    pub async fn is_gpu_available(&self) -> bool {
        self.state.read().await.active_device.is_some()
    }

    /// Force fallback to CPU processing for testing or recovery.
    pub async fn force_fallback(&self, reason: FallbackReason) {
        warn!("Forcing fallback to CPU: {:?}", reason);
        let mut state = self.state.write().await;
        state.active_device = None;
        state.fallback_count += 1;
    }

    /// Attempt to recover GPU processing after fallback.
    pub async fn recover_gpu(&self) -> Result<bool> {
        info!("Attempting GPU recovery");
        
        match self.device_manager.select_best_device().await {
            Ok(device) => {
                let mut state = self.state.write().await;
                state.active_device = Some(device);
                info!("GPU recovery successful");
                Ok(true)
            }
            Err(e) => {
                error!("GPU recovery failed: {}", e);
                Ok(false)
            }
        }
    }
}

// Re-export commonly used types
pub use synapsed_crypto::{Kyber768, Signature, KeyPair};

/// Convenient type alias for GPU-accelerated results
pub type GpuResult<T> = std::result::Result<T, GpuError>;

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_accelerator_creation() {
        let config = AcceleratorConfig::default();
        let result = GpuAccelerator::new(config).await;
        
        // Should either succeed with GPU or gracefully handle no GPU
        match result {
            Ok(accelerator) => {
                assert!(accelerator.device_manager.device_count().await > 0);
            }
            Err(GpuError::NoDevicesAvailable) => {
                // Expected on systems without GPU
            }
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    async fn test_auto_config_creation() {
        let result = GpuAccelerator::with_auto_config().await;
        
        // Should handle both GPU and no-GPU scenarios gracefully
        match result {
            Ok(_) => {}, // Success case
            Err(GpuError::NoDevicesAvailable) => {}, // Expected on no-GPU systems
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    async fn test_metrics_retrieval() {
        if let Ok(accelerator) = GpuAccelerator::with_auto_config().await {
            let metrics = accelerator.metrics().await;
            assert_eq!(metrics.operations_completed, 0);
            assert_eq!(metrics.error_rate, 0.0);
        }
    }

    #[test]
    async fn test_fallback_mechanism() {
        if let Ok(accelerator) = GpuAccelerator::with_auto_config().await {
            let was_gpu_available = accelerator.is_gpu_available().await;
            
            accelerator.force_fallback(FallbackReason::Testing).await;
            assert!(!accelerator.is_gpu_available().await);
            
            if was_gpu_available {
                let recovered = accelerator.recover_gpu().await.unwrap();
                assert!(recovered);
            }
        }
    }
}