//! CUDA device implementation.

#[cfg(feature = "cuda")]
use cudarc::driver::{CudaDevice as CudaDriverDevice, CudaStream, DriverError};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::{Device, DeviceInfo, DeviceType, DeviceConfig, Result, GpuError};

/// CUDA device implementation.
#[derive(Debug)]
pub struct CudaDevice {
    #[cfg(feature = "cuda")]
    device: Arc<CudaDriverDevice>,
    
    #[cfg(feature = "cuda")]
    stream: CudaStream,
    
    info: DeviceInfo,
    
    // For when CUDA is not available
    #[cfg(not(feature = "cuda"))]
    _phantom: std::marker::PhantomData<()>,
}

/// CUDA context for kernel execution.
#[derive(Debug, Clone)]
pub struct CudaContext {
    #[cfg(feature = "cuda")]
    pub device: Arc<CudaDriverDevice>,
    
    #[cfg(feature = "cuda")]
    pub stream: CudaStream,
    
    #[cfg(not(feature = "cuda"))]
    _phantom: std::marker::PhantomData<()>,
}

impl CudaDevice {
    /// Create a new CUDA device.
    #[cfg(feature = "cuda")]
    pub fn new(device: Arc<CudaDriverDevice>, info: DeviceInfo) -> Result<Self> {
        let stream = device.fork_default_stream()?;
        
        Ok(Self {
            device,
            stream,
            info,
        })
    }

    #[cfg(not(feature = "cuda"))]
    pub fn new(_device: (), info: DeviceInfo) -> Result<Self> {
        Ok(Self {
            info,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Get memory usage information.
    pub async fn memory_usage(&self) -> Result<(u64, u64)> {
        #[cfg(feature = "cuda")]
        {
            let (free, total) = self.device.memory_info()?;
            let used = total - free;
            Ok((used, total))
        }
        
        #[cfg(not(feature = "cuda"))]
        {
            // Return mock values when CUDA is not available
            Ok((0, self.info.total_memory_bytes))
        }
    }

    /// Synchronize device operations.
    pub async fn synchronize(&self) -> Result<()> {
        #[cfg(feature = "cuda")]
        {
            self.device.synchronize()?;
            Ok(())
        }
        
        #[cfg(not(feature = "cuda"))]
        {
            // Mock synchronization
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            Ok(())
        }
    }

    /// Reset device state.
    pub async fn reset(&self) -> Result<()> {
        #[cfg(feature = "cuda")]
        {
            // In a real implementation, this would reset the CUDA context
            self.device.synchronize()?;
            Ok(())
        }
        
        #[cfg(not(feature = "cuda"))]
        {
            Ok(())
        }
    }

    /// Get CUDA context for kernel operations.
    pub fn context(&self) -> CudaContext {
        #[cfg(feature = "cuda")]
        {
            CudaContext {
                device: self.device.clone(),
                stream: self.stream.clone(),
            }
        }
        
        #[cfg(not(feature = "cuda"))]
        {
            CudaContext {
                _phantom: std::marker::PhantomData,
            }
        }
    }
}

/// Discover available CUDA devices.
pub async fn discover_devices(config: &DeviceConfig) -> Result<Vec<Device>> {
    #[cfg(feature = "cuda")]
    {
        info!("Discovering CUDA devices...");
        
        match cudarc::driver::CudaDevice::new(0) {
            Ok(device) => {
                let device = Arc::new(device);
                let device_count = device.num_devices().map_err(|e| GpuError::cuda(format!("{:?}", e)))?;
                
                let mut devices = Vec::new();
                
                for device_id in 0..device_count {
                    match create_cuda_device(device_id, config).await {
                        Ok(cuda_device) => devices.push(cuda_device),
                        Err(e) => {
                            error!("Failed to create CUDA device {}: {}", device_id, e);
                        }
                    }
                }
                
                info!("Found {} CUDA devices", devices.len());
                Ok(devices)
            }
            Err(e) => {
                debug!("CUDA not available: {:?}", e);
                Ok(Vec::new())
            }
        }
    }
    
    #[cfg(not(feature = "cuda"))]
    {
        debug!("CUDA feature not enabled");
        Ok(Vec::new())
    }
}

#[cfg(feature = "cuda")]
async fn create_cuda_device(device_id: usize, _config: &DeviceConfig) -> Result<Device> {
    let device = Arc::new(CudaDriverDevice::new(device_id)?);
    
    // Get device properties
    let name = device.name()?;
    let (total_memory, _) = device.memory_info()?;
    let (major, minor) = device.compute_capability()?;
    
    let info = DeviceInfo {
        id: format!("cuda-{}", device_id),
        name,
        device_type: DeviceType::Cuda,
        compute_capability: Some((major as u32, minor as u32)),
        total_memory_bytes: total_memory,
        available_memory_bytes: total_memory, // Will be updated dynamically
        max_threads_per_block: 1024, // Common CUDA limit
        max_blocks_per_grid: 65535,  // Common CUDA limit
        warp_size: 32,               // CUDA warp size
        clock_rate_khz: 1500000,     // Would query actual clock rate
        memory_clock_rate_khz: 6000000,
        memory_bus_width_bits: 256,
        l2_cache_size_bytes: 1024 * 1024,
        supports_unified_memory: true,
        supports_managed_memory: true,
        supports_peer_access: false, // Would check actual capability
    };
    
    let cuda_device = CudaDevice::new(device, info.clone())?;
    let backend = crate::device::DeviceBackend::Cuda(Arc::new(cuda_device));
    
    Ok(Device::new(info, backend))
}

#[cfg(not(feature = "cuda"))]
async fn create_cuda_device(_device_id: usize, _config: &DeviceConfig) -> Result<Device> {
    Err(GpuError::cuda("CUDA feature not enabled"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cuda_device_discovery() {
        let config = DeviceConfig::default();
        let devices = discover_devices(&config).await.unwrap();
        
        // Should succeed even if no CUDA devices are available
        // (returns empty vector in that case)
        for device in &devices {
            assert_eq!(device.device_type(), DeviceType::Cuda);
            assert!(device.info().id.starts_with("cuda-"));
        }
    }

    #[cfg(feature = "cuda")]
    #[tokio::test]
    async fn test_cuda_device_operations() {
        let config = DeviceConfig::default();
        let devices = discover_devices(&config).await.unwrap();
        
        if let Some(device) = devices.first() {
            // Test memory usage
            let backend = match &device.backend {
                crate::device::DeviceBackend::Cuda(cuda) => cuda,
                _ => panic!("Expected CUDA device"),
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
    async fn test_cuda_context() {
        let config = DeviceConfig::default();
        let devices = discover_devices(&config).await.unwrap();
        
        if let Some(device) = devices.first() {
            let context = device.context();
            assert!(matches!(context, crate::device::DeviceContext::Cuda(_)));
        }
    }
}