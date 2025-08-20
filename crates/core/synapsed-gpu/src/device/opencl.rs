//! OpenCL device implementation.

#[cfg(feature = "opencl")]
use opencl3::{
    device::{Device as ClDevice, DeviceInfo as ClDeviceInfo, CL_DEVICE_TYPE_GPU},
    platform::Platform,
    context::Context,
    command_queue::CommandQueue,
    memory::ClMem,
    error_codes::ClError,
};

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::{Device, DeviceInfo, DeviceType, DeviceConfig, Result, GpuError};

/// OpenCL device implementation.
#[derive(Debug)]
pub struct OpenClDevice {
    #[cfg(feature = "opencl")]
    device: ClDevice,
    
    #[cfg(feature = "opencl")]
    context: Context,
    
    #[cfg(feature = "opencl")]
    queue: CommandQueue,
    
    info: DeviceInfo,
    
    // For when OpenCL is not available
    #[cfg(not(feature = "opencl"))]
    _phantom: std::marker::PhantomData<()>,
}

/// OpenCL context for kernel execution.
#[derive(Debug, Clone)]
pub struct OpenClContext {
    #[cfg(feature = "opencl")]
    pub device: ClDevice,
    
    #[cfg(feature = "opencl")]
    pub context: Context,
    
    #[cfg(feature = "opencl")]
    pub queue: CommandQueue,
    
    #[cfg(not(feature = "opencl"))]
    _phantom: std::marker::PhantomData<()>,
}

impl OpenClDevice {
    /// Create a new OpenCL device.
    #[cfg(feature = "opencl")]
    pub fn new(device: ClDevice, info: DeviceInfo) -> Result<Self> {
        let context = Context::from_device(&device)
            .map_err(|e| GpuError::opencl(format!("Failed to create context: {:?}", e)))?;
        
        let queue = CommandQueue::create_default_with_properties(
            &context,
            opencl3::command_queue::CL_QUEUE_PROFILING_ENABLE,
            0,
        ).map_err(|e| GpuError::opencl(format!("Failed to create command queue: {:?}", e)))?;
        
        Ok(Self {
            device,
            context,
            queue,
            info,
        })
    }

    #[cfg(not(feature = "opencl"))]
    pub fn new(_device: (), info: DeviceInfo) -> Result<Self> {
        Ok(Self {
            info,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Get memory usage information.
    pub async fn memory_usage(&self) -> Result<(u64, u64)> {
        #[cfg(feature = "opencl")]
        {
            let total = self.device.max_mem_alloc_size()
                .map_err(|e| GpuError::opencl(format!("Failed to get memory info: {:?}", e)))? as u64;
            
            // OpenCL doesn't have a direct way to get free memory
            // This is a simplified implementation
            let used = total / 4; // Assume 25% usage for now
            Ok((used, total))
        }
        
        #[cfg(not(feature = "opencl"))]
        {
            // Return mock values when OpenCL is not available
            Ok((0, self.info.total_memory_bytes))
        }
    }

    /// Synchronize device operations.
    pub async fn synchronize(&self) -> Result<()> {
        #[cfg(feature = "opencl")]
        {
            self.queue.finish()
                .map_err(|e| GpuError::opencl(format!("Failed to synchronize: {:?}", e)))?;
            Ok(())
        }
        
        #[cfg(not(feature = "opencl"))]
        {
            // Mock synchronization
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            Ok(())
        }
    }

    /// Reset device state.
    pub async fn reset(&self) -> Result<()> {
        #[cfg(feature = "opencl")]
        {
            // Finish all pending operations
            self.queue.finish()
                .map_err(|e| GpuError::opencl(format!("Failed to reset: {:?}", e)))?;
            Ok(())
        }
        
        #[cfg(not(feature = "opencl"))]
        {
            Ok(())
        }
    }

    /// Get OpenCL context for kernel operations.
    pub fn context(&self) -> OpenClContext {
        #[cfg(feature = "opencl")]
        {
            OpenClContext {
                device: self.device.clone(),
                context: self.context.clone(),
                queue: self.queue.clone(),
            }
        }
        
        #[cfg(not(feature = "opencl"))]
        {
            OpenClContext {
                _phantom: std::marker::PhantomData,
            }
        }
    }
}

/// Discover available OpenCL devices.
pub async fn discover_devices(config: &DeviceConfig) -> Result<Vec<Device>> {
    #[cfg(feature = "opencl")]
    {
        info!("Discovering OpenCL devices...");
        
        let platforms = Platform::get_platforms()
            .map_err(|e| GpuError::opencl(format!("Failed to get platforms: {:?}", e)))?;
        
        if platforms.is_empty() {
            debug!("No OpenCL platforms available");
            return Ok(Vec::new());
        }
        
        let mut devices = Vec::new();
        let mut device_id = 0;
        
        for platform in platforms {
            let platform_devices = platform.get_devices(CL_DEVICE_TYPE_GPU)
                .map_err(|e| GpuError::opencl(format!("Failed to get devices: {:?}", e)))?;
            
            for device in platform_devices {
                match create_opencl_device(device, device_id, config).await {
                    Ok(opencl_device) => {
                        devices.push(opencl_device);
                        device_id += 1;
                    }
                    Err(e) => {
                        error!("Failed to create OpenCL device {}: {}", device_id, e);
                        device_id += 1;
                    }
                }
            }
        }
        
        info!("Found {} OpenCL devices", devices.len());
        Ok(devices)
    }
    
    #[cfg(not(feature = "opencl"))]
    {
        debug!("OpenCL feature not enabled");
        Ok(Vec::new())
    }
}

#[cfg(feature = "opencl")]
async fn create_opencl_device(device: ClDevice, device_id: usize, _config: &DeviceConfig) -> Result<Device> {
    // Get device properties
    let name = device.name()
        .map_err(|e| GpuError::opencl(format!("Failed to get device name: {:?}", e)))?;
    
    let global_mem_size = device.global_mem_size()
        .map_err(|e| GpuError::opencl(format!("Failed to get memory size: {:?}", e)))? as u64;
    
    let max_work_group_size = device.max_work_group_size()
        .map_err(|e| GpuError::opencl(format!("Failed to get work group size: {:?}", e)))? as u32;
    
    let max_clock_frequency = device.max_clock_frequency()
        .unwrap_or(1000) as u32 * 1000; // Convert MHz to kHz
    
    let info = DeviceInfo {
        id: format!("opencl-{}", device_id),
        name,
        device_type: DeviceType::OpenCL,
        compute_capability: None, // OpenCL doesn't have compute capability like CUDA
        total_memory_bytes: global_mem_size,
        available_memory_bytes: global_mem_size, // Will be updated dynamically
        max_threads_per_block: max_work_group_size,
        max_blocks_per_grid: 65535, // Common limit
        warp_size: 32,              // Approximate for most GPUs
        clock_rate_khz: max_clock_frequency,
        memory_clock_rate_khz: max_clock_frequency, // Approximation
        memory_bus_width_bits: 256, // Common value
        l2_cache_size_bytes: device.global_mem_cache_size().unwrap_or(0) as u32,
        supports_unified_memory: false, // OpenCL doesn't have unified memory like CUDA
        supports_managed_memory: false,
        supports_peer_access: false,
    };
    
    let opencl_device = OpenClDevice::new(device, info.clone())?;
    let backend = crate::device::DeviceBackend::OpenCL(Arc::new(opencl_device));
    
    Ok(Device::new(info, backend))
}

#[cfg(not(feature = "opencl"))]
async fn create_opencl_device(_device: (), _device_id: usize, _config: &DeviceConfig) -> Result<Device> {
    Err(GpuError::opencl("OpenCL feature not enabled"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_opencl_device_discovery() {
        let config = DeviceConfig::default();
        let devices = discover_devices(&config).await.unwrap();
        
        // Should succeed even if no OpenCL devices are available
        // (returns empty vector in that case)
        for device in &devices {
            assert_eq!(device.device_type(), DeviceType::OpenCL);
            assert!(device.info().id.starts_with("opencl-"));
        }
    }

    #[cfg(feature = "opencl")]
    #[tokio::test]
    async fn test_opencl_device_operations() {
        let config = DeviceConfig::default();
        let devices = discover_devices(&config).await.unwrap();
        
        if let Some(device) = devices.first() {
            // Test memory usage
            let backend = match &device.backend {
                crate::device::DeviceBackend::OpenCL(opencl) => opencl,
                _ => panic!("Expected OpenCL device"),
            };
            
            let (used, total) = backend.memory_usage().await.unwrap();
            assert!(total > 0);
            assert!(used <= total);
            
            // Test synchronization
            backend.synchronize().await.unwrap();
            
            // Test reset
            backend.reset().await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_opencl_context() {
        let config = DeviceConfig::default();
        let devices = discover_devices(&config).await.unwrap();
        
        if let Some(device) = devices.first() {
            let context = device.context();
            assert!(matches!(context, crate::device::DeviceContext::OpenCL(_)));
        }
    }
}