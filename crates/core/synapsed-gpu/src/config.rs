//! Configuration types for GPU acceleration.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::{DeviceType, Result, GpuError};

/// Main configuration for GPU accelerator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceleratorConfig {
    pub device: DeviceConfig,
    pub memory: MemoryConfig,
    pub batch: BatchConfig,
    pub fallback: FallbackConfig,
    pub performance: PerformanceConfig,
}

/// Device selection and management configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// Preferred device type (CUDA, OpenCL, or Auto).
    pub preferred_type: DeviceType,
    
    /// Minimum compute capability required.
    pub min_compute_capability: Option<(u32, u32)>,
    
    /// Minimum memory required in MB.
    pub min_memory_mb: u64,
    
    /// Maximum devices to use concurrently.
    pub max_concurrent_devices: u32,
    
    /// Device selection strategy.
    pub selection_strategy: DeviceSelectionStrategy,
    
    /// Enable device health monitoring.
    pub enable_health_monitoring: bool,
    
    /// Health check interval.
    pub health_check_interval: Duration,
}

/// Memory management configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Initial memory pool size in MB.
    pub initial_pool_size_mb: u64,
    
    /// Maximum memory pool size in MB (0 = unlimited).
    pub max_pool_size_mb: u64,
    
    /// Memory allocation alignment in bytes.
    pub alignment_bytes: u64,
    
    /// Enable memory pooling for better performance.
    pub enable_pooling: bool,
    
    /// Garbage collection threshold (0.0-1.0).
    pub gc_threshold: f64,
    
    /// Maximum memory fragmentation allowed.
    pub max_fragmentation: f64,
    
    /// Enable memory usage tracking.
    pub enable_tracking: bool,
}

/// Batch processing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Default batch size for operations.
    pub default_batch_size: u32,
    
    /// Maximum batch size allowed.
    pub max_batch_size: u32,
    
    /// Minimum batch size for GPU efficiency.
    pub min_batch_size: u32,
    
    /// Batch timeout in milliseconds.
    pub batch_timeout_ms: u64,
    
    /// Maximum concurrent batches.
    pub max_concurrent_batches: u32,
    
    /// Enable dynamic batch sizing.
    pub enable_dynamic_sizing: bool,
    
    /// Batch queue capacity.
    pub queue_capacity: u32,
}

/// Fallback processing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackConfig {
    /// Enable automatic fallback to CPU.
    pub enable_auto_fallback: bool,
    
    /// Maximum GPU errors before forcing fallback.
    pub max_gpu_errors: u32,
    
    /// Fallback retry interval.
    pub retry_interval: Duration,
    
    /// Maximum retry attempts.
    pub max_retry_attempts: u32,
    
    /// Enable fallback performance monitoring.
    pub monitor_fallback_performance: bool,
    
    /// CPU thread pool size for fallback.
    pub cpu_thread_pool_size: Option<u32>,
}

/// Performance monitoring and optimization configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable performance monitoring.
    pub enable_monitoring: bool,
    
    /// Metrics collection interval.
    pub metrics_interval: Duration,
    
    /// Enable automatic performance tuning.
    pub enable_auto_tuning: bool,
    
    /// Performance history size.
    pub history_size: u32,
    
    /// Enable profiling (may impact performance).
    pub enable_profiling: bool,
    
    /// Profiling output directory.
    pub profiling_output_dir: Option<String>,
}

/// Device selection strategies.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DeviceSelectionStrategy {
    /// Select fastest device based on benchmarks.
    Fastest,
    
    /// Select device with most memory.
    MostMemory,
    
    /// Select least utilized device.
    LeastUtilized,
    
    /// Round-robin selection.
    RoundRobin,
    
    /// Custom selection based on application requirements.
    Custom,
}

impl Default for AcceleratorConfig {
    fn default() -> Self {
        Self {
            device: DeviceConfig::default(),
            memory: MemoryConfig::default(),
            batch: BatchConfig::default(),
            fallback: FallbackConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            preferred_type: DeviceType::Auto,
            min_compute_capability: None,
            min_memory_mb: 512, // 512 MB minimum
            max_concurrent_devices: 4,
            selection_strategy: DeviceSelectionStrategy::Fastest,
            enable_health_monitoring: true,
            health_check_interval: Duration::from_secs(30),
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            initial_pool_size_mb: 256, // 256 MB initial pool
            max_pool_size_mb: 2048,    // 2 GB maximum
            alignment_bytes: 256,      // 256-byte alignment
            enable_pooling: true,
            gc_threshold: 0.8,         // Trigger GC at 80% usage
            max_fragmentation: 0.3,    // 30% fragmentation limit
            enable_tracking: true,
        }
    }
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            default_batch_size: 1024,
            max_batch_size: 16384,
            min_batch_size: 32,
            batch_timeout_ms: 100,     // 100ms timeout
            max_concurrent_batches: 8,
            enable_dynamic_sizing: true,
            queue_capacity: 10000,
        }
    }
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            enable_auto_fallback: true,
            max_gpu_errors: 5,
            retry_interval: Duration::from_secs(10),
            max_retry_attempts: 3,
            monitor_fallback_performance: true,
            cpu_thread_pool_size: None, // Use system default
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_monitoring: true,
            metrics_interval: Duration::from_secs(1),
            enable_auto_tuning: false,  // Conservative default
            history_size: 1000,
            enable_profiling: false,    // Disabled by default
            profiling_output_dir: None,
        }
    }
}

impl AcceleratorConfig {
    /// Create configuration optimized for cryptographic workloads.
    pub fn for_crypto() -> Self {
        let mut config = Self::default();
        
        // Optimize for crypto operations
        config.batch.default_batch_size = 2048;
        config.batch.max_batch_size = 32768;
        config.memory.initial_pool_size_mb = 512;
        config.memory.max_pool_size_mb = 4096;
        
        // Enable performance monitoring for crypto
        config.performance.enable_monitoring = true;
        config.performance.enable_auto_tuning = true;
        
        config
    }

    /// Create configuration optimized for high-throughput batch processing.
    pub fn for_batch_processing() -> Self {
        let mut config = Self::default();
        
        // Optimize for batch processing
        config.batch.default_batch_size = 4096;
        config.batch.max_batch_size = 65536;
        config.batch.max_concurrent_batches = 16;
        config.batch.queue_capacity = 50000;
        
        // Larger memory pools
        config.memory.initial_pool_size_mb = 1024;
        config.memory.max_pool_size_mb = 8192;
        
        config
    }

    /// Create configuration optimized for low-latency operations.
    pub fn for_low_latency() -> Self {
        let mut config = Self::default();
        
        // Optimize for low latency
        config.batch.default_batch_size = 128;
        config.batch.batch_timeout_ms = 10; // 10ms timeout
        config.batch.enable_dynamic_sizing = false;
        
        // Smaller memory pools for faster allocation
        config.memory.initial_pool_size_mb = 128;
        config.memory.enable_pooling = true;
        
        // Disable some monitoring for lower overhead
        config.performance.enable_profiling = false;
        config.performance.metrics_interval = Duration::from_secs(5);
        
        config
    }

    /// Auto-detect optimal configuration based on system capabilities.
    pub async fn auto_detect() -> Result<Self> {
        // This would typically probe the system for capabilities
        // For now, return a sensible default based on common scenarios
        
        let available_memory = Self::detect_available_gpu_memory().await?;
        let compute_capability = Self::detect_compute_capability().await?;
        
        let mut config = if available_memory > 4096 {
            Self::for_batch_processing()
        } else if available_memory > 1024 {
            Self::for_crypto()
        } else {
            Self::for_low_latency()
        };
        
        // Adjust based on compute capability
        if let Some((major, minor)) = compute_capability {
            if major >= 8 {
                // Modern GPU, enable advanced features
                config.performance.enable_auto_tuning = true;
                config.batch.enable_dynamic_sizing = true;
            }
        }
        
        Ok(config)
    }

    /// Validate configuration parameters.
    pub fn validate(&self) -> Result<()> {
        // Validate memory config
        if self.memory.initial_pool_size_mb > self.memory.max_pool_size_mb && self.memory.max_pool_size_mb > 0 {
            return Err(GpuError::config("Initial pool size cannot exceed maximum pool size"));
        }
        
        if self.memory.gc_threshold < 0.0 || self.memory.gc_threshold > 1.0 {
            return Err(GpuError::config("GC threshold must be between 0.0 and 1.0"));
        }
        
        // Validate batch config
        if self.batch.min_batch_size > self.batch.max_batch_size {
            return Err(GpuError::config("Minimum batch size cannot exceed maximum batch size"));
        }
        
        if self.batch.default_batch_size < self.batch.min_batch_size || 
           self.batch.default_batch_size > self.batch.max_batch_size {
            return Err(GpuError::config("Default batch size must be within min/max range"));
        }
        
        // Validate device config
        if self.device.max_concurrent_devices == 0 {
            return Err(GpuError::config("Maximum concurrent devices must be at least 1"));
        }
        
        Ok(())
    }

    // Helper methods for auto-detection
    async fn detect_available_gpu_memory() -> Result<u64> {
        // This would query actual GPU memory
        // For now, return a reasonable default
        Ok(2048) // 2GB default
    }

    async fn detect_compute_capability() -> Result<Option<(u32, u32)>> {
        // This would query actual compute capability
        // For now, return None (unknown)
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_configs() {
        let config = AcceleratorConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_crypto_config() {
        let config = AcceleratorConfig::for_crypto();
        assert!(config.validate().is_ok());
        assert_eq!(config.batch.default_batch_size, 2048);
        assert!(config.performance.enable_auto_tuning);
    }

    #[test]
    fn test_batch_processing_config() {
        let config = AcceleratorConfig::for_batch_processing();
        assert!(config.validate().is_ok());
        assert_eq!(config.batch.default_batch_size, 4096);
        assert_eq!(config.batch.max_concurrent_batches, 16);
    }

    #[test]
    fn test_low_latency_config() {
        let config = AcceleratorConfig::for_low_latency();
        assert!(config.validate().is_ok());
        assert_eq!(config.batch.default_batch_size, 128);
        assert_eq!(config.batch.batch_timeout_ms, 10);
    }

    #[test]
    fn test_config_validation() {
        let mut config = AcceleratorConfig::default();
        
        // Test invalid memory config
        config.memory.initial_pool_size_mb = 1000;
        config.memory.max_pool_size_mb = 500;
        assert!(config.validate().is_err());
        
        // Test invalid batch config
        config = AcceleratorConfig::default();
        config.batch.min_batch_size = 1000;
        config.batch.max_batch_size = 500;
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_auto_detect() {
        let result = AcceleratorConfig::auto_detect().await;
        // Should succeed with reasonable defaults
        assert!(result.is_ok());
        let config = result.unwrap();
        assert!(config.validate().is_ok());
    }
}