//! Device health monitoring and diagnostics.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use serde::{Deserialize, Serialize};

use crate::{DeviceInfo, Result, GpuError};

/// Device health status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Device is healthy and fully functional.
    Healthy,
    
    /// Device is functional but showing warning signs.
    Warning,
    
    /// Device is critical but still operational.
    Critical,
    
    /// Device is unhealthy and should not be used.
    Unhealthy,
    
    /// Device status is unknown.
    Unknown,
}

/// Device health metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    pub status: HealthStatus,
    pub last_check: Instant,
    pub temperature_celsius: Option<f32>,
    pub memory_usage_percent: f32,
    pub utilization_percent: f32,
    pub error_count: u64,
    pub warning_count: u64,
    pub uptime_seconds: u64,
    pub last_error_time: Option<Instant>,
    pub last_error_message: Option<String>,
}

/// Device health monitor for tracking device status and performance.
#[derive(Debug)]
pub struct DeviceHealthMonitor {
    device_info: DeviceInfo,
    metrics: Arc<RwLock<HealthMetrics>>,
    config: HealthConfig,
    start_time: Instant,
}

/// Configuration for device health monitoring.
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Temperature threshold for warnings (°C).
    pub temperature_warning_threshold: f32,
    
    /// Temperature threshold for critical status (°C).
    pub temperature_critical_threshold: f32,
    
    /// Memory usage threshold for warnings (%).
    pub memory_warning_threshold: f32,
    
    /// Memory usage threshold for critical status (%).
    pub memory_critical_threshold: f32,
    
    /// Maximum error count before marking as unhealthy.
    pub max_error_count: u64,
    
    /// Time window for error counting.
    pub error_window: Duration,
    
    /// Health check interval.
    pub check_interval: Duration,
    
    /// Enable automatic recovery attempts.
    pub enable_auto_recovery: bool,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            temperature_warning_threshold: 75.0,
            temperature_critical_threshold: 85.0,
            memory_warning_threshold: 80.0,
            memory_critical_threshold: 95.0,
            max_error_count: 10,
            error_window: Duration::from_secs(300), // 5 minutes
            check_interval: Duration::from_secs(30),
            enable_auto_recovery: true,
        }
    }
}

impl DeviceHealthMonitor {
    /// Create a new health monitor for the specified device.
    pub fn new(device_info: DeviceInfo) -> Self {
        let start_time = Instant::now();
        
        let metrics = Arc::new(RwLock::new(HealthMetrics {
            status: HealthStatus::Unknown,
            last_check: start_time,
            temperature_celsius: None,
            memory_usage_percent: 0.0,
            utilization_percent: 0.0,
            error_count: 0,
            warning_count: 0,
            uptime_seconds: 0,
            last_error_time: None,
            last_error_message: None,
        }));

        Self {
            device_info,
            metrics,
            config: HealthConfig::default(),
            start_time,
        }
    }

    /// Create a health monitor with custom configuration.
    pub fn with_config(device_info: DeviceInfo, config: HealthConfig) -> Self {
        let mut monitor = Self::new(device_info);
        monitor.config = config;
        monitor
    }

    /// Check if the device is currently healthy.
    pub async fn is_healthy(&self) -> bool {
        let metrics = self.metrics.read().await;
        matches!(metrics.status, HealthStatus::Healthy | HealthStatus::Warning)
    }

    /// Get current health status.
    pub async fn status(&self) -> HealthStatus {
        self.metrics.read().await.status
    }

    /// Get current health metrics.
    pub async fn metrics(&self) -> HealthMetrics {
        self.metrics.read().await.clone()
    }

    /// Perform a health check and update metrics.
    pub async fn check_health(&self) -> Result<HealthStatus> {
        debug!("Performing health check for device: {}", self.device_info.id);
        
        let now = Instant::now();
        let uptime_seconds = now.duration_since(self.start_time).as_secs();
        
        // Get current device metrics
        let temperature = self.get_temperature().await?;
        let memory_usage = self.get_memory_usage().await?;
        let utilization = self.get_utilization().await?;
        
        // Determine health status
        let status = self.evaluate_health_status(
            temperature,
            memory_usage,
            utilization,
        ).await;
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.status = status;
            metrics.last_check = now;
            metrics.temperature_celsius = temperature;
            metrics.memory_usage_percent = memory_usage;
            metrics.utilization_percent = utilization;
            metrics.uptime_seconds = uptime_seconds;
        }
        
        debug!("Health check complete: {:?} for device {}", status, self.device_info.id);
        
        Ok(status)
    }

    /// Record an error for health tracking.
    pub async fn record_error(&self, error: &GpuError) {
        warn!("Recording error for device {}: {}", self.device_info.id, error);
        
        let now = Instant::now();
        let mut metrics = self.metrics.write().await;
        
        metrics.error_count += 1;
        metrics.last_error_time = Some(now);
        metrics.last_error_message = Some(error.to_string());
        
        // Update status based on error count
        if metrics.error_count >= self.config.max_error_count {
            metrics.status = HealthStatus::Unhealthy;
            error!("Device {} marked as unhealthy due to error count: {}", 
                   self.device_info.id, metrics.error_count);
        } else if metrics.error_count >= self.config.max_error_count / 2 {
            metrics.status = HealthStatus::Critical;
            warn!("Device {} marked as critical due to error count: {}", 
                  self.device_info.id, metrics.error_count);
        }
    }

    /// Record a warning for health tracking.
    pub async fn record_warning(&self, message: &str) {
        debug!("Recording warning for device {}: {}", self.device_info.id, message);
        
        let mut metrics = self.metrics.write().await;
        metrics.warning_count += 1;
        
        // Update status if we have too many warnings
        if metrics.status == HealthStatus::Healthy && metrics.warning_count > 5 {
            metrics.status = HealthStatus::Warning;
        }
    }

    /// Attempt to recover an unhealthy device.
    pub async fn attempt_recovery(&self) -> Result<bool> {
        if !self.config.enable_auto_recovery {
            return Ok(false);
        }
        
        info!("Attempting recovery for device: {}", self.device_info.id);
        
        // Reset error counters
        {
            let mut metrics = self.metrics.write().await;
            metrics.error_count = 0;
            metrics.warning_count = 0;
            metrics.last_error_time = None;
            metrics.last_error_message = None;
        }
        
        // Perform device-specific recovery
        self.perform_device_recovery().await?;
        
        // Re-check health
        let status = self.check_health().await?;
        let recovered = matches!(status, HealthStatus::Healthy | HealthStatus::Warning);
        
        if recovered {
            info!("Device recovery successful: {}", self.device_info.id);
        } else {
            error!("Device recovery failed: {}", self.device_info.id);
        }
        
        Ok(recovered)
    }

    /// Reset health monitoring state.
    pub async fn reset(&self) {
        info!("Resetting health monitor for device: {}", self.device_info.id);
        
        let mut metrics = self.metrics.write().await;
        metrics.status = HealthStatus::Unknown;
        metrics.error_count = 0;
        metrics.warning_count = 0;
        metrics.last_error_time = None;
        metrics.last_error_message = None;
    }

    async fn get_temperature(&self) -> Result<Option<f32>> {
        // This would query actual device temperature
        // For now, simulate based on device type
        match self.device_info.device_type {
            crate::DeviceType::Cuda => {
                // Simulate CUDA temperature query
                Ok(Some(45.0 + (rand::random::<f32>() * 20.0)))
            }
            crate::DeviceType::OpenCL => {
                // Simulate OpenCL temperature query
                Ok(Some(40.0 + (rand::random::<f32>() * 25.0)))
            }
            crate::DeviceType::Auto => {
                // Mock device
                Ok(Some(35.0))
            }
        }
    }

    async fn get_memory_usage(&self) -> Result<f32> {
        // This would query actual memory usage
        // For now, simulate based on available memory
        let total = self.device_info.total_memory_bytes as f32;
        let available = self.device_info.available_memory_bytes as f32;
        let used = total - available;
        
        Ok((used / total) * 100.0)
    }

    async fn get_utilization(&self) -> Result<f32> {
        // This would query actual GPU utilization
        // For now, simulate some utilization
        Ok(rand::random::<f32>() * 50.0)
    }

    async fn evaluate_health_status(
        &self,
        temperature: Option<f32>,
        memory_usage: f32,
        _utilization: f32,
    ) -> HealthStatus {
        let metrics = self.metrics.read().await;
        
        // Check if we have too many errors
        if metrics.error_count >= self.config.max_error_count {
            return HealthStatus::Unhealthy;
        }
        
        // Check temperature thresholds
        if let Some(temp) = temperature {
            if temp >= self.config.temperature_critical_threshold {
                return HealthStatus::Critical;
            }
            if temp >= self.config.temperature_warning_threshold {
                return HealthStatus::Warning;
            }
        }
        
        // Check memory usage thresholds
        if memory_usage >= self.config.memory_critical_threshold {
            return HealthStatus::Critical;
        }
        if memory_usage >= self.config.memory_warning_threshold {
            return HealthStatus::Warning;
        }
        
        // Check error rate
        if metrics.error_count >= self.config.max_error_count / 2 {
            return HealthStatus::Warning;
        }
        
        HealthStatus::Healthy
    }

    async fn perform_device_recovery(&self) -> Result<()> {
        match self.device_info.device_type {
            crate::DeviceType::Cuda => {
                // Perform CUDA-specific recovery
                debug!("Performing CUDA device recovery");
                // This would reset CUDA context, clear memory, etc.
            }
            crate::DeviceType::OpenCL => {
                // Perform OpenCL-specific recovery
                debug!("Performing OpenCL device recovery");
                // This would reset OpenCL context, clear command queues, etc.
            }
            crate::DeviceType::Auto => {
                // Mock recovery
                debug!("Performing mock device recovery");
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceType, DeviceInfo};

    fn create_test_device_info() -> DeviceInfo {
        DeviceInfo {
            id: "test-device".to_string(),
            name: "Test GPU".to_string(),
            device_type: DeviceType::Auto,
            compute_capability: Some((8, 0)),
            total_memory_bytes: 1024 * 1024 * 1024,
            available_memory_bytes: 512 * 1024 * 1024,
            max_threads_per_block: 1024,
            max_blocks_per_grid: 65535,
            warp_size: 32,
            clock_rate_khz: 1500000,
            memory_clock_rate_khz: 6000000,
            memory_bus_width_bits: 256,
            l2_cache_size_bytes: 1024 * 1024,
            supports_unified_memory: true,
            supports_managed_memory: true,
            supports_peer_access: false,
        }
    }

    #[tokio::test]
    async fn test_health_monitor_creation() {
        let device_info = create_test_device_info();
        let monitor = DeviceHealthMonitor::new(device_info);
        
        let status = monitor.status().await;
        assert_eq!(status, HealthStatus::Unknown);
    }

    #[tokio::test]
    async fn test_health_check() {
        let device_info = create_test_device_info();
        let monitor = DeviceHealthMonitor::new(device_info);
        
        let status = monitor.check_health().await.unwrap();
        assert!(matches!(status, HealthStatus::Healthy | HealthStatus::Warning));
        assert!(monitor.is_healthy().await);
    }

    #[tokio::test]
    async fn test_error_recording() {
        let device_info = create_test_device_info();
        let monitor = DeviceHealthMonitor::new(device_info);
        
        // Record some errors
        let error = GpuError::memory("Test error");
        monitor.record_error(&error).await;
        
        let metrics = monitor.metrics().await;
        assert_eq!(metrics.error_count, 1);
        assert!(metrics.last_error_message.is_some());
    }

    #[tokio::test]
    async fn test_warning_recording() {
        let device_info = create_test_device_info();
        let monitor = DeviceHealthMonitor::new(device_info);
        
        monitor.record_warning("Test warning").await;
        
        let metrics = monitor.metrics().await;
        assert_eq!(metrics.warning_count, 1);
    }

    #[tokio::test]
    async fn test_recovery_attempt() {
        let device_info = create_test_device_info();
        let mut config = HealthConfig::default();
        config.enable_auto_recovery = true;
        
        let monitor = DeviceHealthMonitor::with_config(device_info, config);
        
        // Force unhealthy status by recording many errors
        for _ in 0..15 {
            monitor.record_error(&GpuError::memory("Test error")).await;
        }
        
        assert!(!monitor.is_healthy().await);
        
        // Attempt recovery
        let recovered = monitor.attempt_recovery().await.unwrap();
        assert!(recovered);
        assert!(monitor.is_healthy().await);
    }

    #[tokio::test]
    async fn test_health_reset() {
        let device_info = create_test_device_info();
        let monitor = DeviceHealthMonitor::new(device_info);
        
        // Record some errors and warnings
        monitor.record_error(&GpuError::memory("Test error")).await;
        monitor.record_warning("Test warning").await;
        
        // Reset
        monitor.reset().await;
        
        let metrics = monitor.metrics().await;
        assert_eq!(metrics.error_count, 0);
        assert_eq!(metrics.warning_count, 0);
        assert_eq!(metrics.status, HealthStatus::Unknown);
    }
}