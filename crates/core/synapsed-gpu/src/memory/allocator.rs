//! GPU memory allocator implementations for different backends.

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{RwLock, Mutex};
use tracing::{debug, error, info, warn};

use crate::{Device, DeviceContext, GpuBuffer, MemoryConfig, TransferDirection, Result, GpuError};

/// GPU memory allocator supporting multiple backends.
#[derive(Debug)]
pub struct GpuAllocator {
    device: Device,
    config: MemoryConfig,
    backend: AllocatorBackend,
    allocation_tracker: Arc<RwLock<HashMap<String, AllocationRecord>>>,
    next_id: Arc<Mutex<u64>>,
}

/// Backend-specific allocator implementations.
#[derive(Debug)]
enum AllocatorBackend {
    #[cfg(feature = "cuda")]
    Cuda(CudaAllocator),
    
    #[cfg(feature = "opencl")]
    OpenCL(OpenClAllocator),
    
    Mock(MockAllocator),
}

/// CUDA allocator implementation.
#[cfg(feature = "cuda")]
#[derive(Debug)]
struct CudaAllocator {
    device: Arc<cudarc::driver::CudaDevice>,
    stream: cudarc::driver::CudaStream,
}

/// OpenCL allocator implementation.
#[cfg(feature = "opencl")]
#[derive(Debug)]
struct OpenClAllocator {
    context: opencl3::context::Context,
    queue: opencl3::command_queue::CommandQueue,
}

/// Mock allocator for testing.
#[derive(Debug)]
struct MockAllocator {
    allocations: Arc<RwLock<HashMap<u64, Vec<u8>>>>,
    next_ptr: Arc<Mutex<u64>>,
}

/// Allocation tracking record.
#[derive(Debug, Clone)]
struct AllocationRecord {
    id: String,
    size: u64,
    alignment: u64,
    device_ptr: u64,
    allocation_time: std::time::Instant,
    strategy: AllocationStrategy,
}

/// Memory allocation strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationStrategy {
    /// Standard device memory allocation.
    Standard,
    
    /// Pinned host memory for faster transfers.
    Pinned,
    
    /// Managed/unified memory (CUDA only).
    Managed,
    
    /// Device-local memory with specific alignment.
    Aligned,
}

impl GpuAllocator {
    /// Create a new GPU allocator for the specified device.
    pub async fn new(device: Device, config: MemoryConfig) -> Result<Self> {
        info!("Creating GPU allocator for device: {}", device.info().id);

        let backend = match device.context() {
            #[cfg(feature = "cuda")]
            DeviceContext::Cuda(cuda_context) => {
                AllocatorBackend::Cuda(CudaAllocator {
                    device: cuda_context.device,
                    stream: cuda_context.stream,
                })
            }
            
            #[cfg(feature = "opencl")]
            DeviceContext::OpenCL(opencl_context) => {
                AllocatorBackend::OpenCL(OpenClAllocator {
                    context: opencl_context.context,
                    queue: opencl_context.queue,
                })
            }
            
            DeviceContext::Mock => {
                AllocatorBackend::Mock(MockAllocator {
                    allocations: Arc::new(RwLock::new(HashMap::new())),
                    next_ptr: Arc::new(Mutex::new(0x1000_0000)), // Start at some non-zero address
                })
            }
        };

        Ok(Self {
            device,
            config,
            backend,
            allocation_tracker: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(0)),
        })
    }

    /// Allocate device memory with specified size and alignment.
    pub async fn allocate(&self, size: u64, alignment: u64) -> Result<Arc<GpuBuffer>> {
        self.allocate_with_strategy(size, alignment, AllocationStrategy::Standard).await
    }

    /// Allocate managed/unified memory.
    pub async fn allocate_managed(&self, size: u64) -> Result<Arc<GpuBuffer>> {
        self.allocate_with_strategy(size, self.config.alignment_bytes, AllocationStrategy::Managed).await
    }

    /// Allocate pinned host memory.
    pub async fn allocate_pinned(&self, size: u64) -> Result<Arc<GpuBuffer>> {
        self.allocate_with_strategy(size, self.config.alignment_bytes, AllocationStrategy::Pinned).await
    }

    /// Allocate memory with specific strategy.
    pub async fn allocate_with_strategy(
        &self,
        size: u64,
        alignment: u64,
        strategy: AllocationStrategy,
    ) -> Result<Arc<GpuBuffer>> {
        debug!("Allocating {} bytes with alignment {} using strategy {:?}", size, alignment, strategy);

        // Generate unique ID
        let id = {
            let mut next_id = self.next_id.lock().await;
            *next_id += 1;
            format!("gpu-buffer-{}", *next_id)
        };

        // Perform backend-specific allocation
        let buffer = match &self.backend {
            #[cfg(feature = "cuda")]
            AllocatorBackend::Cuda(cuda) => {
                self.cuda_allocate(cuda, &id, size, alignment, strategy).await?
            }
            
            #[cfg(feature = "opencl")]
            AllocatorBackend::OpenCL(opencl) => {
                self.opencl_allocate(opencl, &id, size, alignment, strategy).await?
            }
            
            AllocatorBackend::Mock(mock) => {
                self.mock_allocate(mock, &id, size, alignment, strategy).await?
            }
        };

        // Track allocation
        let record = AllocationRecord {
            id: id.clone(),
            size,
            alignment,
            device_ptr: buffer.device_ptr(),
            allocation_time: std::time::Instant::now(),
            strategy,
        };

        let mut tracker = self.allocation_tracker.write().await;
        tracker.insert(id, record);

        Ok(buffer)
    }

    /// Deallocate GPU memory buffer.
    pub async fn deallocate(&self, buffer: Arc<GpuBuffer>) -> Result<()> {
        debug!("Deallocating buffer: {}", buffer.id());

        // Remove from tracker
        let mut tracker = self.allocation_tracker.write().await;
        tracker.remove(buffer.id());
        drop(tracker);

        // Perform backend-specific deallocation
        match &self.backend {
            #[cfg(feature = "cuda")]
            AllocatorBackend::Cuda(cuda) => {
                self.cuda_deallocate(cuda, &buffer).await
            }
            
            #[cfg(feature = "opencl")]
            AllocatorBackend::OpenCL(opencl) => {
                self.opencl_deallocate(opencl, &buffer).await
            }
            
            AllocatorBackend::Mock(mock) => {
                self.mock_deallocate(mock, &buffer).await
            }
        }
    }

    /// Copy data between GPU buffers.
    pub async fn copy(&self, src: &GpuBuffer, dst: &GpuBuffer, size: u64) -> Result<()> {
        debug!("Copying {} bytes from {} to {}", size, src.id(), dst.id());

        match &self.backend {
            #[cfg(feature = "cuda")]
            AllocatorBackend::Cuda(cuda) => {
                self.cuda_copy(cuda, src, dst, size).await
            }
            
            #[cfg(feature = "opencl")]
            AllocatorBackend::OpenCL(opencl) => {
                self.opencl_copy(opencl, src, dst, size).await
            }
            
            AllocatorBackend::Mock(mock) => {
                self.mock_copy(mock, src, dst, size).await
            }
        }
    }

    /// Transfer data between host and device.
    pub async fn transfer(
        &self,
        src_ptr: u64,
        dst_ptr: u64,
        size: u64,
        direction: TransferDirection,
    ) -> Result<()> {
        debug!("Transferring {} bytes {:?}", size, direction);

        match &self.backend {
            #[cfg(feature = "cuda")]
            AllocatorBackend::Cuda(cuda) => {
                self.cuda_transfer(cuda, src_ptr, dst_ptr, size, direction).await
            }
            
            #[cfg(feature = "opencl")]
            AllocatorBackend::OpenCL(opencl) => {
                self.opencl_transfer(opencl, src_ptr, dst_ptr, size, direction).await
            }
            
            AllocatorBackend::Mock(mock) => {
                self.mock_transfer(mock, src_ptr, dst_ptr, size, direction).await
            }
        }
    }

    /// Get allocation statistics.
    pub async fn stats(&self) -> AllocationStats {
        let tracker = self.allocation_tracker.read().await;
        
        let total_allocations = tracker.len() as u64;
        let total_bytes: u64 = tracker.values().map(|r| r.size).sum();
        
        let strategy_counts = tracker.values().fold(
            HashMap::new(),
            |mut acc, record| {
                *acc.entry(record.strategy).or_insert(0u64) += 1;
                acc
            }
        );

        AllocationStats {
            total_allocations,
            total_bytes,
            strategy_counts,
            average_allocation_size: if total_allocations > 0 {
                total_bytes / total_allocations
            } else {
                0
            },
        }
    }

    // Backend-specific implementation methods

    #[cfg(feature = "cuda")]
    async fn cuda_allocate(
        &self,
        cuda: &CudaAllocator,
        id: &str,
        size: u64,
        alignment: u64,
        strategy: AllocationStrategy,
    ) -> Result<Arc<GpuBuffer>> {
        use cudarc::driver::DevicePtr;

        let device_ptr = match strategy {
            AllocationStrategy::Standard | AllocationStrategy::Aligned => {
                cuda.device.alloc_zeros::<u8>(size as usize)?
            }
            AllocationStrategy::Managed => {
                // CUDA managed memory allocation would go here
                cuda.device.alloc_zeros::<u8>(size as usize)?
            }
            AllocationStrategy::Pinned => {
                // CUDA pinned memory allocation would go here
                cuda.device.alloc_zeros::<u8>(size as usize)?
            }
        };

        let buffer = GpuBuffer::new_cuda(
            id.to_string(),
            device_ptr,
            size,
            alignment,
            strategy == AllocationStrategy::Pinned,
            strategy == AllocationStrategy::Managed,
        );

        Ok(Arc::new(buffer))
    }

    #[cfg(feature = "opencl")]
    async fn opencl_allocate(
        &self,
        opencl: &OpenClAllocator,
        id: &str,
        size: u64,
        alignment: u64,
        strategy: AllocationStrategy,
    ) -> Result<Arc<GpuBuffer>> {
        use opencl3::memory::{Buffer, CL_MEM_READ_WRITE};

        let buffer = Buffer::<u8>::create(
            &opencl.context,
            CL_MEM_READ_WRITE,
            size as usize,
            std::ptr::null_mut(),
        ).map_err(|e| GpuError::opencl(format!("Buffer allocation failed: {:?}", e)))?;

        let gpu_buffer = GpuBuffer::new_opencl(
            id.to_string(),
            buffer,
            size,
            alignment,
        );

        Ok(Arc::new(gpu_buffer))
    }

    async fn mock_allocate(
        &self,
        mock: &MockAllocator,
        id: &str,
        size: u64,
        alignment: u64,
        _strategy: AllocationStrategy,
    ) -> Result<Arc<GpuBuffer>> {
        let ptr = {
            let mut next_ptr = mock.next_ptr.lock().await;
            let ptr = *next_ptr;
            *next_ptr += ((size + alignment - 1) / alignment) * alignment; // Align to next boundary
            ptr
        };

        // Store the allocation
        let mut allocations = mock.allocations.write().await;
        allocations.insert(ptr, vec![0u8; size as usize]);

        let buffer = GpuBuffer::new_mock(id.to_string(), size, alignment);
        Ok(Arc::new(buffer))
    }

    #[cfg(feature = "cuda")]
    async fn cuda_deallocate(&self, _cuda: &CudaAllocator, _buffer: &GpuBuffer) -> Result<()> {
        // CUDA memory is automatically freed when DevicePtr is dropped
        Ok(())
    }

    #[cfg(feature = "opencl")]
    async fn opencl_deallocate(&self, _opencl: &OpenClAllocator, _buffer: &GpuBuffer) -> Result<()> {
        // OpenCL memory is automatically freed when Buffer is dropped
        Ok(())
    }

    async fn mock_deallocate(&self, mock: &MockAllocator, buffer: &GpuBuffer) -> Result<()> {
        let mut allocations = mock.allocations.write().await;
        allocations.remove(&buffer.device_ptr());
        Ok(())
    }

    #[cfg(feature = "cuda")]
    async fn cuda_copy(&self, cuda: &CudaAllocator, src: &GpuBuffer, dst: &GpuBuffer, size: u64) -> Result<()> {
        // CUDA device-to-device copy would go here
        // For now, just synchronize
        cuda.device.synchronize()?;
        Ok(())
    }

    #[cfg(feature = "opencl")]
    async fn opencl_copy(&self, opencl: &OpenClAllocator, src: &GpuBuffer, dst: &GpuBuffer, size: u64) -> Result<()> {
        // OpenCL buffer copy would go here
        opencl.queue.finish()?;
        Ok(())
    }

    async fn mock_copy(&self, mock: &MockAllocator, src: &GpuBuffer, dst: &GpuBuffer, size: u64) -> Result<()> {
        let allocations = mock.allocations.read().await;
        
        if let (Some(src_data), Some(_dst_data)) = 
            (allocations.get(&src.device_ptr()), allocations.get(&dst.device_ptr())) {
            // Mock copy operation
            debug!("Mock copy {} bytes", size);
        }
        
        Ok(())
    }

    #[cfg(feature = "cuda")]
    async fn cuda_transfer(
        &self,
        cuda: &CudaAllocator,
        _src_ptr: u64,
        _dst_ptr: u64,
        _size: u64,
        _direction: TransferDirection,
    ) -> Result<()> {
        // CUDA memory transfer would go here
        cuda.device.synchronize()?;
        Ok(())
    }

    #[cfg(feature = "opencl")]
    async fn opencl_transfer(
        &self,
        opencl: &OpenClAllocator,
        _src_ptr: u64,
        _dst_ptr: u64,
        _size: u64,
        _direction: TransferDirection,
    ) -> Result<()> {
        // OpenCL memory transfer would go here
        opencl.queue.finish()?;
        Ok(())
    }

    async fn mock_transfer(
        &self,
        _mock: &MockAllocator,
        _src_ptr: u64,
        _dst_ptr: u64,
        size: u64,
        direction: TransferDirection,
    ) -> Result<()> {
        debug!("Mock transfer {} bytes {:?}", size, direction);
        // Simulate transfer time
        tokio::time::sleep(tokio::time::Duration::from_micros(size / 1000)).await;
        Ok(())
    }
}

/// Allocation statistics.
#[derive(Debug, Clone)]
pub struct AllocationStats {
    pub total_allocations: u64,
    pub total_bytes: u64,
    pub strategy_counts: HashMap<AllocationStrategy, u64>,
    pub average_allocation_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceManager, DeviceConfig};

    async fn create_test_allocator() -> Result<GpuAllocator> {
        let device_config = DeviceConfig::default();
        let device_manager = DeviceManager::new(device_config).await?;
        let device = device_manager.select_best_device().await?;
        let memory_config = MemoryConfig::default();
        
        GpuAllocator::new(device, memory_config).await
    }

    #[tokio::test]
    async fn test_allocator_creation() {
        let allocator = create_test_allocator().await.unwrap();
        let stats = allocator.stats().await;
        
        assert_eq!(stats.total_allocations, 0);
        assert_eq!(stats.total_bytes, 0);
    }

    #[tokio::test]
    async fn test_standard_allocation() {
        let allocator = create_test_allocator().await.unwrap();
        
        let buffer = allocator.allocate(1024, 256).await.unwrap();
        
        assert_eq!(buffer.size(), 1024);
        assert_eq!(buffer.alignment(), 256);
        assert!(!buffer.is_pinned());
        assert!(!buffer.is_managed());
        
        let stats = allocator.stats().await;
        assert_eq!(stats.total_allocations, 1);
        assert_eq!(stats.total_bytes, 1024);
    }

    #[tokio::test]
    async fn test_managed_allocation() {
        let allocator = create_test_allocator().await.unwrap();
        
        let buffer = allocator.allocate_managed(2048).await.unwrap();
        
        assert_eq!(buffer.size(), 2048);
        // Note: For mock device, managed memory might not be truly managed
        
        let stats = allocator.stats().await;
        assert_eq!(stats.total_allocations, 1);
        assert_eq!(stats.total_bytes, 2048);
    }

    #[tokio::test]
    async fn test_pinned_allocation() {
        let allocator = create_test_allocator().await.unwrap();
        
        let buffer = allocator.allocate_pinned(1024).await.unwrap();
        
        assert_eq!(buffer.size(), 1024);
        // Note: For mock device, pinned memory might not be truly pinned
        
        let stats = allocator.stats().await;
        assert_eq!(stats.total_allocations, 1);
    }

    #[tokio::test]
    async fn test_allocation_strategies() {
        let allocator = create_test_allocator().await.unwrap();
        
        // Test different strategies
        let _standard = allocator.allocate_with_strategy(1024, 256, AllocationStrategy::Standard).await.unwrap();
        let _aligned = allocator.allocate_with_strategy(1024, 512, AllocationStrategy::Aligned).await.unwrap();
        let _managed = allocator.allocate_with_strategy(1024, 256, AllocationStrategy::Managed).await.unwrap();
        let _pinned = allocator.allocate_with_strategy(1024, 256, AllocationStrategy::Pinned).await.unwrap();
        
        let stats = allocator.stats().await;
        assert_eq!(stats.total_allocations, 4);
        assert_eq!(stats.total_bytes, 4096);
        
        // Check strategy counts
        assert!(stats.strategy_counts.len() > 0);
    }

    #[tokio::test]
    async fn test_deallocation() {
        let allocator = create_test_allocator().await.unwrap();
        
        let buffer = allocator.allocate(1024, 256).await.unwrap();
        let buffer_id = buffer.id().to_string();
        
        let stats_before = allocator.stats().await;
        assert_eq!(stats_before.total_allocations, 1);
        
        allocator.deallocate(buffer).await.unwrap();
        
        let stats_after = allocator.stats().await;
        assert_eq!(stats_after.total_allocations, 0); // Should be removed from tracker
    }

    #[tokio::test]
    async fn test_buffer_copy() {
        let allocator = create_test_allocator().await.unwrap();
        
        let src = allocator.allocate(1024, 256).await.unwrap();
        let dst = allocator.allocate(1024, 256).await.unwrap();
        
        allocator.copy(&src, &dst, 512).await.unwrap();
        
        // Copy should succeed without error
    }

    #[tokio::test]
    async fn test_memory_transfer() {
        let allocator = create_test_allocator().await.unwrap();
        
        let buffer = allocator.allocate(1024, 256).await.unwrap();
        let host_data = vec![0u8; 1024];
        
        // Test host to device transfer
        allocator.transfer(
            host_data.as_ptr() as u64,
            buffer.device_ptr(),
            1024,
            TransferDirection::HostToDevice,
        ).await.unwrap();
        
        // Test device to host transfer
        let mut result = vec![0u8; 1024];
        allocator.transfer(
            buffer.device_ptr(),
            result.as_mut_ptr() as u64,
            1024,
            TransferDirection::DeviceToHost,
        ).await.unwrap();
    }

    #[tokio::test]
    async fn test_allocation_tracking() {
        let allocator = create_test_allocator().await.unwrap();
        
        // Allocate multiple buffers
        let _buf1 = allocator.allocate(1024, 256).await.unwrap();
        let _buf2 = allocator.allocate(2048, 512).await.unwrap();
        let _buf3 = allocator.allocate_managed(1024).await.unwrap();
        
        let stats = allocator.stats().await;
        assert_eq!(stats.total_allocations, 3);
        assert_eq!(stats.total_bytes, 4096);
        assert_eq!(stats.average_allocation_size, 4096 / 3);
        
        // Check that different strategies are tracked
        assert!(stats.strategy_counts.len() > 0);
    }
}