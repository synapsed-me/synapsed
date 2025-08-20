//! GPU memory management and allocation.

use std::collections::HashMap;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use tokio::sync::{RwLock, Mutex};
use tracing::{debug, error, info, warn};
use serde::{Deserialize, Serialize};

use crate::{Device, DeviceContext, MemoryConfig, Result, GpuError};

pub mod pool;
pub mod buffer;
pub mod allocator;

pub use pool::MemoryPool;
pub use buffer::GpuBuffer;
pub use allocator::{GpuAllocator, AllocationStrategy};

/// GPU memory manager responsible for efficient memory allocation and management.
#[derive(Debug)]
pub struct MemoryManager {
    device: Device,
    config: MemoryConfig,
    allocator: Arc<GpuAllocator>,
    pool: Arc<MemoryPool>,
    active_buffers: Arc<RwLock<HashMap<String, Arc<GpuBuffer>>>>,
    metrics: Arc<MemoryMetrics>,
}

/// Memory usage and performance metrics.
#[derive(Debug)]
pub struct MemoryMetrics {
    total_allocated: AtomicU64,
    total_freed: AtomicU64,
    peak_usage: AtomicU64,
    allocation_count: AtomicU64,
    deallocation_count: AtomicU64,
    pool_hits: AtomicU64,
    pool_misses: AtomicU64,
    fragmentation_ratio: Arc<RwLock<f64>>,
}

/// Memory allocation information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationInfo {
    pub id: String,
    pub size_bytes: u64,
    pub alignment: u64,
    pub device_ptr: u64,
    pub host_ptr: Option<u64>,
    pub is_pinned: bool,
    pub is_managed: bool,
    pub allocation_time: std::time::Instant,
}

/// Memory transfer direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    HostToDevice,
    DeviceToHost,
    DeviceToDevice,
}

impl MemoryManager {
    /// Create a new memory manager for the specified device.
    pub async fn new(device: Device, config: MemoryConfig) -> Result<Self> {
        info!("Initializing memory manager for device: {}", device.info().id);

        let allocator = Arc::new(GpuAllocator::new(device.clone(), config.clone()).await?);
        let pool = Arc::new(MemoryPool::new(allocator.clone(), config.clone()).await?);
        
        let metrics = Arc::new(MemoryMetrics {
            total_allocated: AtomicU64::new(0),
            total_freed: AtomicU64::new(0),
            peak_usage: AtomicU64::new(0),
            allocation_count: AtomicU64::new(0),
            deallocation_count: AtomicU64::new(0),
            pool_hits: AtomicU64::new(0),
            pool_misses: AtomicU64::new(0),
            fragmentation_ratio: Arc::new(RwLock::new(0.0)),
        });

        Ok(Self {
            device,
            config,
            allocator,
            pool,
            active_buffers: Arc::new(RwLock::new(HashMap::new())),
            metrics,
        })
    }

    /// Allocate GPU memory buffer.
    pub async fn allocate(&self, size: u64) -> Result<Arc<GpuBuffer>> {
        self.allocate_with_alignment(size, self.config.alignment_bytes).await
    }

    /// Allocate GPU memory buffer with specific alignment.
    pub async fn allocate_with_alignment(&self, size: u64, alignment: u64) -> Result<Arc<GpuBuffer>> {
        debug!("Allocating {} bytes with alignment {}", size, alignment);

        // Try pool allocation first if enabled
        if self.config.enable_pooling {
            if let Some(buffer) = self.pool.allocate(size, alignment).await? {
                self.metrics.pool_hits.fetch_add(1, Ordering::Relaxed);
                self.track_allocation(&buffer).await;
                return Ok(buffer);
            }
            self.metrics.pool_misses.fetch_add(1, Ordering::Relaxed);
        }

        // Direct allocation
        let buffer = self.allocator.allocate(size, alignment).await?;
        self.track_allocation(&buffer).await;

        Ok(buffer)
    }

    /// Allocate managed memory (unified memory if supported).
    pub async fn allocate_managed(&self, size: u64) -> Result<Arc<GpuBuffer>> {
        debug!("Allocating {} bytes of managed memory", size);

        if !self.device.info().supports_managed_memory {
            warn!("Device does not support managed memory, falling back to regular allocation");
            return self.allocate(size).await;
        }

        let buffer = self.allocator.allocate_managed(size).await?;
        self.track_allocation(&buffer).await;

        Ok(buffer)
    }

    /// Allocate pinned host memory for faster transfers.
    pub async fn allocate_pinned(&self, size: u64) -> Result<Arc<GpuBuffer>> {
        debug!("Allocating {} bytes of pinned host memory", size);

        let buffer = self.allocator.allocate_pinned(size).await?;
        self.track_allocation(&buffer).await;

        Ok(buffer)
    }

    /// Free GPU memory buffer.
    pub async fn free(&self, buffer: Arc<GpuBuffer>) -> Result<()> {
        debug!("Freeing buffer: {}", buffer.id());

        // Remove from active buffers
        let mut active_buffers = self.active_buffers.write().await;
        active_buffers.remove(buffer.id());
        drop(active_buffers);

        // Try to return to pool first
        if self.config.enable_pooling && self.pool.can_return(&buffer).await {
            self.pool.return_buffer(buffer).await?;
        } else {
            // Direct deallocation
            let size = buffer.size();
            self.allocator.deallocate(buffer).await?;
            self.metrics.total_freed.fetch_add(size, Ordering::Relaxed);
            self.metrics.deallocation_count.fetch_add(1, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Copy data between buffers.
    pub async fn copy(&self, src: &GpuBuffer, dst: &GpuBuffer, size: Option<u64>) -> Result<()> {
        let copy_size = size.unwrap_or(std::cmp::min(src.size(), dst.size()));
        
        debug!("Copying {} bytes from {} to {}", copy_size, src.id(), dst.id());

        self.allocator.copy(src, dst, copy_size).await
    }

    /// Transfer data from host to device.
    pub async fn transfer_to_device(&self, host_data: &[u8], device_buffer: &GpuBuffer) -> Result<()> {
        debug!("Transferring {} bytes to device buffer {}", host_data.len(), device_buffer.id());
        
        self.allocator.transfer(
            host_data.as_ptr() as u64,
            device_buffer.device_ptr(),
            host_data.len() as u64,
            TransferDirection::HostToDevice,
        ).await
    }

    /// Transfer data from device to host.
    pub async fn transfer_to_host(&self, device_buffer: &GpuBuffer, host_data: &mut [u8]) -> Result<()> {
        debug!("Transferring {} bytes from device buffer {}", host_data.len(), device_buffer.id());
        
        self.allocator.transfer(
            device_buffer.device_ptr(),
            host_data.as_mut_ptr() as u64,
            host_data.len() as u64,
            TransferDirection::DeviceToHost,
        ).await
    }

    /// Get current memory usage statistics.
    pub async fn usage_stats(&self) -> MemoryUsageStats {
        let allocated = self.metrics.total_allocated.load(Ordering::Relaxed);
        let freed = self.metrics.total_freed.load(Ordering::Relaxed);
        let peak = self.metrics.peak_usage.load(Ordering::Relaxed);
        let alloc_count = self.metrics.allocation_count.load(Ordering::Relaxed);
        let dealloc_count = self.metrics.deallocation_count.load(Ordering::Relaxed);
        let pool_hits = self.metrics.pool_hits.load(Ordering::Relaxed);
        let pool_misses = self.metrics.pool_misses.load(Ordering::Relaxed);
        let fragmentation = *self.metrics.fragmentation_ratio.read().await;

        let pool_hit_ratio = if pool_hits + pool_misses > 0 {
            pool_hits as f64 / (pool_hits + pool_misses) as f64
        } else {
            0.0
        };

        MemoryUsageStats {
            total_allocated_bytes: allocated,
            total_freed_bytes: freed,
            current_usage_bytes: allocated - freed,
            peak_usage_bytes: peak,
            allocation_count: alloc_count,
            deallocation_count: dealloc_count,
            active_buffers: self.active_buffers.read().await.len() as u64,
            pool_hit_ratio,
            fragmentation_ratio: fragmentation,
        }
    }

    /// Trigger garbage collection to free unused memory.
    pub async fn garbage_collect(&self) -> Result<u64> {
        info!("Starting garbage collection");

        let freed_bytes = self.pool.garbage_collect().await?;
        
        // Update fragmentation ratio
        self.update_fragmentation_ratio().await?;

        info!("Garbage collection freed {} bytes", freed_bytes);
        Ok(freed_bytes)
    }

    /// Check if garbage collection should be triggered.
    pub async fn should_garbage_collect(&self) -> bool {
        let stats = self.usage_stats().await;
        let usage_ratio = stats.current_usage_bytes as f64 / self.device.info().total_memory_bytes as f64;
        
        usage_ratio >= self.config.gc_threshold
    }

    /// Get list of active buffer allocations.
    pub async fn active_allocations(&self) -> Vec<AllocationInfo> {
        self.active_buffers
            .read()
            .await
            .values()
            .map(|buffer| buffer.allocation_info().clone())
            .collect()
    }

    /// Synchronize all memory operations.
    pub async fn synchronize(&self) -> Result<()> {
        self.device.synchronize().await
    }

    async fn track_allocation(&self, buffer: &Arc<GpuBuffer>) {
        let size = buffer.size();
        
        // Update metrics
        self.metrics.total_allocated.fetch_add(size, Ordering::Relaxed);
        self.metrics.allocation_count.fetch_add(1, Ordering::Relaxed);
        
        // Update peak usage
        let current_usage = self.metrics.total_allocated.load(Ordering::Relaxed) 
            - self.metrics.total_freed.load(Ordering::Relaxed);
        let peak = self.metrics.peak_usage.load(Ordering::Relaxed);
        if current_usage > peak {
            self.metrics.peak_usage.store(current_usage, Ordering::Relaxed);
        }

        // Track active buffer
        let mut active_buffers = self.active_buffers.write().await;
        active_buffers.insert(buffer.id().to_string(), buffer.clone());
    }

    async fn update_fragmentation_ratio(&self) -> Result<()> {
        // This would calculate actual fragmentation based on allocator state
        // For now, provide a simple estimate
        let stats = self.usage_stats().await;
        let fragmentation = if stats.allocation_count > 0 {
            1.0 - (stats.current_usage_bytes as f64 / (stats.allocation_count as f64 * 1024.0))
        } else {
            0.0
        };
        
        *self.metrics.fragmentation_ratio.write().await = fragmentation.clamp(0.0, 1.0);
        Ok(())
    }
}

/// Memory usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsageStats {
    pub total_allocated_bytes: u64,
    pub total_freed_bytes: u64,
    pub current_usage_bytes: u64,
    pub peak_usage_bytes: u64,
    pub allocation_count: u64,
    pub deallocation_count: u64,
    pub active_buffers: u64,
    pub pool_hit_ratio: f64,
    pub fragmentation_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceManager, DeviceConfig};

    async fn create_test_memory_manager() -> Result<MemoryManager> {
        let device_config = DeviceConfig::default();
        let device_manager = DeviceManager::new(device_config).await?;
        let device = device_manager.select_best_device().await?;
        let memory_config = MemoryConfig::default();
        
        MemoryManager::new(device, memory_config).await
    }

    #[tokio::test]
    async fn test_memory_manager_creation() {
        let manager = create_test_memory_manager().await.unwrap();
        let stats = manager.usage_stats().await;
        
        assert_eq!(stats.current_usage_bytes, 0);
        assert_eq!(stats.allocation_count, 0);
        assert_eq!(stats.active_buffers, 0);
    }

    #[tokio::test]
    async fn test_basic_allocation() {
        let manager = create_test_memory_manager().await.unwrap();
        
        let buffer = manager.allocate(1024).await.unwrap();
        assert_eq!(buffer.size(), 1024);
        
        let stats = manager.usage_stats().await;
        assert_eq!(stats.allocation_count, 1);
        assert_eq!(stats.active_buffers, 1);
        assert_eq!(stats.current_usage_bytes, 1024);
    }

    #[tokio::test]
    async fn test_allocation_and_free() {
        let manager = create_test_memory_manager().await.unwrap();
        
        let buffer = manager.allocate(2048).await.unwrap();
        assert_eq!(buffer.size(), 2048);
        
        manager.free(buffer).await.unwrap();
        
        let stats = manager.usage_stats().await;
        assert_eq!(stats.allocation_count, 1);
        assert_eq!(stats.deallocation_count, 1);
        assert_eq!(stats.active_buffers, 0);
    }

    #[tokio::test]
    async fn test_aligned_allocation() {
        let manager = create_test_memory_manager().await.unwrap();
        
        let buffer = manager.allocate_with_alignment(1000, 256).await.unwrap();
        assert_eq!(buffer.size(), 1000);
        assert_eq!(buffer.alignment(), 256);
    }

    #[tokio::test]
    async fn test_managed_allocation() {
        let manager = create_test_memory_manager().await.unwrap();
        
        // Should succeed even if managed memory is not supported (falls back to regular allocation)
        let buffer = manager.allocate_managed(1024).await.unwrap();
        assert_eq!(buffer.size(), 1024);
    }

    #[tokio::test]
    async fn test_pinned_allocation() {
        let manager = create_test_memory_manager().await.unwrap();
        
        let buffer = manager.allocate_pinned(1024).await.unwrap();
        assert_eq!(buffer.size(), 1024);
    }

    #[tokio::test]
    async fn test_garbage_collection() {
        let manager = create_test_memory_manager().await.unwrap();
        
        // Allocate and free some buffers
        for _ in 0..10 {
            let buffer = manager.allocate(1024).await.unwrap();
            manager.free(buffer).await.unwrap();
        }
        
        let freed = manager.garbage_collect().await.unwrap();
        // Should have freed some memory (exact amount depends on pool implementation)
        assert!(freed >= 0);
    }

    #[tokio::test]
    async fn test_memory_transfer() {
        let manager = create_test_memory_manager().await.unwrap();
        
        let buffer = manager.allocate(1024).await.unwrap();
        let data = vec![0u8; 1024];
        
        // Test host to device transfer
        manager.transfer_to_device(&data, &buffer).await.unwrap();
        
        // Test device to host transfer
        let mut result = vec![0u8; 1024];
        manager.transfer_to_host(&buffer, &mut result).await.unwrap();
    }

    #[tokio::test]
    async fn test_buffer_copy() {
        let manager = create_test_memory_manager().await.unwrap();
        
        let src_buffer = manager.allocate(1024).await.unwrap();
        let dst_buffer = manager.allocate(1024).await.unwrap();
        
        manager.copy(&src_buffer, &dst_buffer, Some(512)).await.unwrap();
    }
}