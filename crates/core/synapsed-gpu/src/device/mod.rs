//! GPU device management and selection.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use serde::{Deserialize, Serialize};

use crate::{Result, GpuError, DeviceConfig};

pub mod cuda;
pub mod opencl;
pub mod health;

pub use cuda::CudaDevice;
pub use opencl::OpenClDevice;
pub use health::DeviceHealthMonitor;

/// Supported GPU device types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceType {
    /// NVIDIA CUDA device.
    Cuda,
    
    /// OpenCL compatible device.
    OpenCL,
    
    /// Automatically select best available device.
    Auto,
}

/// Information about a GPU device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub compute_capability: Option<(u32, u32)>,
    pub total_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub max_threads_per_block: u32,
    pub max_blocks_per_grid: u32,
    pub warp_size: u32,
    pub clock_rate_khz: u32,
    pub memory_clock_rate_khz: u32,
    pub memory_bus_width_bits: u32,
    pub l2_cache_size_bytes: u32,
    pub supports_unified_memory: bool,
    pub supports_managed_memory: bool,
    pub supports_peer_access: bool,
}

/// GPU device abstraction supporting multiple backends.
#[derive(Debug, Clone)]
pub struct Device {
    info: DeviceInfo,
    backend: DeviceBackend,
    health_monitor: Arc<DeviceHealthMonitor>,
}

#[derive(Debug, Clone)]
enum DeviceBackend {
    #[cfg(feature = "cuda")]
    Cuda(Arc<CudaDevice>),
    
    #[cfg(feature = "opencl")]
    OpenCL(Arc<OpenClDevice>),
    
    /// Mock device for testing.
    Mock(Arc<MockDevice>),
}

/// Mock device implementation for testing.
#[derive(Debug)]
pub struct MockDevice {
    info: DeviceInfo,
    should_fail: Arc<RwLock<bool>>,
}

/// Device manager responsible for device discovery and selection.
#[derive(Debug)]
pub struct DeviceManager {
    config: DeviceConfig,
    devices: Arc<RwLock<HashMap<String, Device>>>,
    selected_device: Arc<RwLock<Option<Device>>>,
}

impl Device {
    /// Create a new device from backend-specific implementation.
    pub fn new(info: DeviceInfo, backend: DeviceBackend) -> Self {
        let health_monitor = Arc::new(DeviceHealthMonitor::new(info.clone()));
        
        Self {
            info,
            backend,
            health_monitor,
        }
    }

    /// Get device information.
    pub fn info(&self) -> &DeviceInfo {
        &self.info
    }

    /// Get device type.
    pub fn device_type(&self) -> DeviceType {
        self.info.device_type
    }

    /// Check if device is healthy and available.
    pub async fn is_healthy(&self) -> bool {
        self.health_monitor.is_healthy().await
    }

    /// Get current memory usage.
    pub async fn memory_usage(&self) -> Result<(u64, u64)> {
        match &self.backend {
            #[cfg(feature = "cuda")]
            DeviceBackend::Cuda(cuda) => cuda.memory_usage().await,
            
            #[cfg(feature = "opencl")]
            DeviceBackend::OpenCL(opencl) => opencl.memory_usage().await,
            
            DeviceBackend::Mock(mock) => mock.memory_usage().await,
        }
    }

    /// Synchronize device operations.
    pub async fn synchronize(&self) -> Result<()> {
        match &self.backend {
            #[cfg(feature = "cuda")]
            DeviceBackend::Cuda(cuda) => cuda.synchronize().await,
            
            #[cfg(feature = "opencl")]
            DeviceBackend::OpenCL(opencl) => opencl.synchronize().await,
            
            DeviceBackend::Mock(mock) => mock.synchronize().await,
        }
    }

    /// Reset device state.
    pub async fn reset(&self) -> Result<()> {
        match &self.backend {
            #[cfg(feature = "cuda")]
            DeviceBackend::Cuda(cuda) => cuda.reset().await,
            
            #[cfg(feature = "opencl")]
            DeviceBackend::OpenCL(opencl) => opencl.reset().await,
            
            DeviceBackend::Mock(mock) => mock.reset().await,
        }
    }

    /// Get device-specific context for kernel execution.
    pub fn context(&self) -> DeviceContext {
        match &self.backend {
            #[cfg(feature = "cuda")]
            DeviceBackend::Cuda(cuda) => DeviceContext::Cuda(cuda.context()),
            
            #[cfg(feature = "opencl")]
            DeviceBackend::OpenCL(opencl) => DeviceContext::OpenCL(opencl.context()),
            
            DeviceBackend::Mock(_) => DeviceContext::Mock,
        }
    }
}

/// Device context for backend-specific operations.
#[derive(Debug, Clone)]
pub enum DeviceContext {
    #[cfg(feature = "cuda")]
    Cuda(cuda::CudaContext),
    
    #[cfg(feature = "opencl")]
    OpenCL(opencl::OpenClContext),
    
    Mock,
}

impl DeviceManager {
    /// Create a new device manager with the given configuration.
    pub async fn new(config: DeviceConfig) -> Result<Self> {
        let manager = Self {
            config,
            devices: Arc::new(RwLock::new(HashMap::new())),
            selected_device: Arc::new(RwLock::new(None)),
        };

        manager.discover_devices().await?;
        
        Ok(manager)
    }

    /// Discover all available GPU devices.
    pub async fn discover_devices(&self) -> Result<()> {
        info!("Discovering GPU devices...");
        
        let mut devices = self.devices.write().await;
        devices.clear();

        // Discover CUDA devices
        #[cfg(feature = "cuda")]
        {
            match self.discover_cuda_devices().await {
                Ok(cuda_devices) => {
                    for device in cuda_devices {
                        devices.insert(device.info().id.clone(), device);
                    }
                }
                Err(e) => {
                    warn!("Failed to discover CUDA devices: {}", e);
                }
            }
        }

        // Discover OpenCL devices
        #[cfg(feature = "opencl")]
        {
            match self.discover_opencl_devices().await {
                Ok(opencl_devices) => {
                    for device in opencl_devices {
                        devices.insert(device.info().id.clone(), device);
                    }
                }
                Err(e) => {
                    warn!("Failed to discover OpenCL devices: {}", e);
                }
            }
        }

        // Add mock device for testing if no real devices found
        if devices.is_empty() {
            warn!("No GPU devices found, creating mock device for testing");
            let mock_device = self.create_mock_device().await?;
            devices.insert(mock_device.info().id.clone(), mock_device);
        }

        info!("Discovered {} GPU devices", devices.len());
        
        Ok(())
    }

    /// Select the best available device based on configuration.
    pub async fn select_best_device(&self) -> Result<Device> {
        let devices = self.devices.read().await;
        
        if devices.is_empty() {
            return Err(GpuError::NoDevicesAvailable);
        }

        let candidates: Vec<&Device> = devices
            .values()
            .filter(|device| self.meets_requirements(device))
            .collect();

        if candidates.is_empty() {
            return Err(GpuError::NoDevicesAvailable);
        }

        let selected = match self.config.selection_strategy {
            crate::config::DeviceSelectionStrategy::Fastest => {
                self.select_fastest_device(&candidates).await?
            }
            crate::config::DeviceSelectionStrategy::MostMemory => {
                self.select_most_memory_device(&candidates).await?
            }
            crate::config::DeviceSelectionStrategy::LeastUtilized => {
                self.select_least_utilized_device(&candidates).await?
            }
            crate::config::DeviceSelectionStrategy::RoundRobin => {
                candidates[0].clone() // Simple implementation
            }
            crate::config::DeviceSelectionStrategy::Custom => {
                candidates[0].clone() // Placeholder for custom logic
            }
        };

        // Update selected device
        let mut selected_device = self.selected_device.write().await;
        *selected_device = Some(selected.clone());

        info!("Selected device: {} ({})", selected.info().name, selected.info().id);
        
        Ok(selected)
    }

    /// Get the currently selected device.
    pub async fn selected_device(&self) -> Option<Device> {
        self.selected_device.read().await.clone()
    }

    /// Get the number of available devices.
    pub async fn device_count(&self) -> usize {
        self.devices.read().await.len()
    }

    /// Get information about all available devices.
    pub async fn device_info_list(&self) -> Vec<DeviceInfo> {
        self.devices
            .read()
            .await
            .values()
            .map(|device| device.info().clone())
            .collect()
    }

    #[cfg(feature = "cuda")]
    async fn discover_cuda_devices(&self) -> Result<Vec<Device>> {
        cuda::discover_devices(&self.config).await
    }

    #[cfg(feature = "opencl")]
    async fn discover_opencl_devices(&self) -> Result<Vec<Device>> {
        opencl::discover_devices(&self.config).await
    }

    async fn create_mock_device(&self) -> Result<Device> {
        let info = DeviceInfo {
            id: "mock-device-0".to_string(),
            name: "Mock GPU Device".to_string(),
            device_type: DeviceType::Auto,
            compute_capability: Some((8, 0)),
            total_memory_bytes: 1024 * 1024 * 1024, // 1GB
            available_memory_bytes: 1024 * 1024 * 1024,
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
        };

        let mock = Arc::new(MockDevice {
            info: info.clone(),
            should_fail: Arc::new(RwLock::new(false)),
        });

        Ok(Device::new(info, DeviceBackend::Mock(mock)))
    }

    fn meets_requirements(&self, device: &Device) -> bool {
        let info = device.info();

        // Check memory requirement
        if info.available_memory_bytes < self.config.min_memory_mb * 1024 * 1024 {
            return false;
        }

        // Check compute capability requirement
        if let Some(required_capability) = self.config.min_compute_capability {
            if let Some(device_capability) = info.compute_capability {
                if device_capability < required_capability {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check device type preference
        match self.config.preferred_type {
            DeviceType::Cuda => info.device_type == DeviceType::Cuda,
            DeviceType::OpenCL => info.device_type == DeviceType::OpenCL,
            DeviceType::Auto => true,
        }
    }

    async fn select_fastest_device(&self, candidates: &[&Device]) -> Result<Device> {
        // For now, select based on clock rate
        // In a real implementation, this would run benchmarks
        candidates
            .iter()
            .max_by_key(|device| device.info().clock_rate_khz)
            .map(|&device| device.clone())
            .ok_or(GpuError::NoDevicesAvailable)
    }

    async fn select_most_memory_device(&self, candidates: &[&Device]) -> Result<Device> {
        candidates
            .iter()
            .max_by_key(|device| device.info().available_memory_bytes)
            .map(|&device| device.clone())
            .ok_or(GpuError::NoDevicesAvailable)
    }

    async fn select_least_utilized_device(&self, candidates: &[&Device]) -> Result<Device> {
        // For now, just return the first one
        // In a real implementation, this would check device utilization
        candidates
            .first()
            .map(|&device| device.clone())
            .ok_or(GpuError::NoDevicesAvailable)
    }
}

// Mock device implementation
impl MockDevice {
    pub async fn memory_usage(&self) -> Result<(u64, u64)> {
        if *self.should_fail.read().await {
            return Err(GpuError::device("Mock device failure"));
        }
        
        Ok((
            self.info.total_memory_bytes - self.info.available_memory_bytes,
            self.info.total_memory_bytes,
        ))
    }

    pub async fn synchronize(&self) -> Result<()> {
        if *self.should_fail.read().await {
            return Err(GpuError::device("Mock device failure"));
        }
        
        // Simulate synchronization delay
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        Ok(())
    }

    pub async fn reset(&self) -> Result<()> {
        if *self.should_fail.read().await {
            return Err(GpuError::device("Mock device failure"));
        }
        
        // Reset failure state
        *self.should_fail.write().await = false;
        Ok(())
    }

    pub async fn set_should_fail(&self, should_fail: bool) {
        *self.should_fail.write().await = should_fail;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DeviceSelectionStrategy;

    #[tokio::test]
    async fn test_device_manager_creation() {
        let config = DeviceConfig::default();
        let manager = DeviceManager::new(config).await.unwrap();
        
        // Should have at least the mock device
        assert!(manager.device_count().await > 0);
    }

    #[tokio::test]
    async fn test_device_selection() {
        let config = DeviceConfig::default();
        let manager = DeviceManager::new(config).await.unwrap();
        
        let device = manager.select_best_device().await.unwrap();
        assert!(!device.info().id.is_empty());
        assert!(!device.info().name.is_empty());
    }

    #[tokio::test]
    async fn test_device_requirements() {
        let mut config = DeviceConfig::default();
        config.min_memory_mb = 2048; // 2GB requirement
        
        let manager = DeviceManager::new(config).await.unwrap();
        
        // Mock device has 1GB, so selection might fail
        let result = manager.select_best_device().await;
        // Test should handle both success and failure cases
        match result {
            Ok(_) => {}, // Device meets requirements
            Err(GpuError::NoDevicesAvailable) => {}, // No devices meet requirements
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_device_health() {
        let config = DeviceConfig::default();
        let manager = DeviceManager::new(config).await.unwrap();
        let device = manager.select_best_device().await.unwrap();
        
        assert!(device.is_healthy().await);
    }

    #[tokio::test]
    async fn test_mock_device_operations() {
        let config = DeviceConfig::default();
        let manager = DeviceManager::new(config).await.unwrap();
        let device = manager.select_best_device().await.unwrap();
        
        // Test memory usage
        let (used, total) = device.memory_usage().await.unwrap();
        assert!(total > 0);
        assert!(used <= total);
        
        // Test synchronization
        device.synchronize().await.unwrap();
        
        // Test reset
        device.reset().await.unwrap();
    }
}