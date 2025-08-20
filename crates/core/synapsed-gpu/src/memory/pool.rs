//! GPU memory pool for efficient allocation and reuse.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use tracing::{debug, info, warn};

use crate::{GpuBuffer, GpuAllocator, MemoryConfig, Result, GpuError};

/// Memory pool for efficient GPU buffer allocation and reuse.
#[derive(Debug)]
pub struct MemoryPool {
    allocator: Arc<GpuAllocator>,
    config: MemoryConfig,
    pools: Arc<RwLock<HashMap<PoolKey, VecDeque<Arc<GpuBuffer>>>>>,
    total_pooled_bytes: Arc<Mutex<u64>>,
    allocation_stats: Arc<Mutex<PoolStats>>,
}

/// Pool key for categorizing buffers by size and alignment.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PoolKey {
    size_class: u64,
    alignment: u64,
}

/// Pool statistics for monitoring and optimization.
#[derive(Debug, Clone, Default)]
struct PoolStats {
    total_allocations: u64,
    pool_hits: u64,
    pool_misses: u64,
    total_returns: u64,
    successful_returns: u64,
    garbage_collections: u64,
    bytes_freed_by_gc: u64,
}

/// Memory pool configuration constants.
const MIN_POOL_SIZE_CLASS: u64 = 256;       // 256 bytes
const MAX_POOL_SIZE_CLASS: u64 = 256 * 1024 * 1024; // 256 MB
const POOL_SIZE_CLASS_MULTIPLIER: u64 = 2;  // Powers of 2
const MAX_BUFFERS_PER_POOL: usize = 64;     // Maximum buffers per size class
const BUFFER_AGE_THRESHOLD_SECS: u64 = 300;  // 5 minutes

impl MemoryPool {
    /// Create a new memory pool.
    pub async fn new(allocator: Arc<GpuAllocator>, config: MemoryConfig) -> Result<Self> {
        info!("Creating memory pool with config: {:?}", config);

        Ok(Self {
            allocator,
            config,
            pools: Arc::new(RwLock::new(HashMap::new())),
            total_pooled_bytes: Arc::new(Mutex::new(0)),
            allocation_stats: Arc::new(Mutex::new(PoolStats::default())),
        })
    }

    /// Attempt to allocate from pool, returns None if not available.
    pub async fn allocate(&self, size: u64, alignment: u64) -> Result<Option<Arc<GpuBuffer>>> {
        if !self.config.enable_pooling {
            return Ok(None);
        }

        let mut stats = self.allocation_stats.lock().await;
        stats.total_allocations += 1;
        drop(stats);

        let pool_key = self.get_pool_key(size, alignment);
        let mut pools = self.pools.write().await;
        
        if let Some(pool) = pools.get_mut(&pool_key) {
            while let Some(buffer) = pool.pop_front() {
                if buffer.is_available() && buffer.can_reuse(size, alignment) {
                    debug!("Pool hit for size {} alignment {}", size, alignment);
                    
                    // Update stats
                    let mut stats = self.allocation_stats.lock().await;
                    stats.pool_hits += 1;
                    drop(stats);
                    
                    // Update total pooled bytes
                    let mut total_bytes = self.total_pooled_bytes.lock().await;
                    *total_bytes = total_bytes.saturating_sub(buffer.size());
                    drop(total_bytes);
                    
                    return Ok(Some(buffer));
                }
            }
        }

        // No suitable buffer found in pool
        debug!("Pool miss for size {} alignment {}", size, alignment);
        let mut stats = self.allocation_stats.lock().await;
        stats.pool_misses += 1;
        
        Ok(None)
    }

    /// Return a buffer to the pool for reuse.
    pub async fn return_buffer(&self, buffer: Arc<GpuBuffer>) -> Result<()> {
        if !self.config.enable_pooling {
            return Ok(());
        }

        let mut stats = self.allocation_stats.lock().await;
        stats.total_returns += 1;
        drop(stats);

        let pool_key = self.get_pool_key(buffer.size(), buffer.alignment());
        let mut pools = self.pools.write().await;
        
        let pool = pools.entry(pool_key).or_insert_with(VecDeque::new);
        
        // Don't exceed maximum buffers per pool
        if pool.len() >= MAX_BUFFERS_PER_POOL {
            debug!("Pool full, deallocating buffer instead");
            return self.allocator.deallocate(buffer).await;
        }

        // Check if we exceed memory limits
        let total_bytes = *self.total_pooled_bytes.lock().await;
        let max_pool_bytes = if self.config.max_pool_size_mb > 0 {
            self.config.max_pool_size_mb * 1024 * 1024
        } else {
            u64::MAX
        };

        if total_bytes + buffer.size() > max_pool_bytes {
            debug!("Pool memory limit exceeded, deallocating buffer");
            return self.allocator.deallocate(buffer).await;
        }

        debug!("Returning buffer {} to pool", buffer.id());
        pool.push_back(buffer.clone());
        
        // Update stats and total bytes
        let mut stats = self.allocation_stats.lock().await;
        stats.successful_returns += 1;
        drop(stats);
        
        let mut total_bytes = self.total_pooled_bytes.lock().await;
        *total_bytes += buffer.size();
        
        Ok(())
    }

    /// Check if a buffer can be returned to the pool.
    pub async fn can_return(&self, buffer: &GpuBuffer) -> bool {
        if !self.config.enable_pooling {
            return false;
        }

        // Check size limits
        if buffer.size() < MIN_POOL_SIZE_CLASS || buffer.size() > MAX_POOL_SIZE_CLASS {
            return false;
        }

        // Check if buffer is in a usable state
        if !buffer.is_available() {
            return false;
        }

        // Check age (don't pool very old buffers)
        if buffer.age().as_secs() > BUFFER_AGE_THRESHOLD_SECS {
            return false;
        }

        true
    }

    /// Perform garbage collection to free unused buffers.
    pub async fn garbage_collect(&self) -> Result<u64> {
        if !self.config.enable_pooling {
            return Ok(0);
        }

        info!("Starting memory pool garbage collection");
        
        let mut freed_bytes = 0u64;
        let mut pools = self.pools.write().await;
        
        for (pool_key, pool) in pools.iter_mut() {
            let initial_count = pool.len();
            
            // Remove old or unusable buffers
            pool.retain(|buffer| {
                let should_keep = buffer.is_available() && 
                    buffer.age().as_secs() <= BUFFER_AGE_THRESHOLD_SECS;
                
                if !should_keep {
                    freed_bytes += buffer.size();
                }
                
                should_keep
            });
            
            let removed_count = initial_count - pool.len();
            if removed_count > 0 {
                debug!("Removed {} buffers from pool {:?}", removed_count, pool_key);
            }
        }

        // Remove empty pools
        pools.retain(|_, pool| !pool.is_empty());
        drop(pools);

        // Update total pooled bytes
        let mut total_bytes = self.total_pooled_bytes.lock().await;
        *total_bytes = total_bytes.saturating_sub(freed_bytes);
        drop(total_bytes);

        // Update stats
        let mut stats = self.allocation_stats.lock().await;
        stats.garbage_collections += 1;
        stats.bytes_freed_by_gc += freed_bytes;
        
        info!("Garbage collection freed {} bytes", freed_bytes);
        
        Ok(freed_bytes)
    }

    /// Get current pool statistics.
    pub async fn stats(&self) -> PoolStatistics {
        let stats = self.allocation_stats.lock().await.clone();
        let total_pooled_bytes = *self.total_pooled_bytes.lock().await;
        let pools = self.pools.read().await;
        
        let total_buffers: usize = pools.values().map(|pool| pool.len()).sum();
        let pool_count = pools.len();
        
        let hit_ratio = if stats.total_allocations > 0 {
            stats.pool_hits as f64 / stats.total_allocations as f64
        } else {
            0.0
        };
        
        let return_success_ratio = if stats.total_returns > 0 {
            stats.successful_returns as f64 / stats.total_returns as f64
        } else {
            0.0
        };

        PoolStatistics {
            total_allocations: stats.total_allocations,
            pool_hits: stats.pool_hits,
            pool_misses: stats.pool_misses,
            hit_ratio,
            total_returns: stats.total_returns,
            successful_returns: stats.successful_returns,
            return_success_ratio,
            total_pooled_bytes,
            total_pooled_buffers: total_buffers as u64,
            pool_count: pool_count as u64,
            garbage_collections: stats.garbage_collections,
            bytes_freed_by_gc: stats.bytes_freed_by_gc,
        }
    }

    /// Clear all pools and free memory.
    pub async fn clear(&self) -> Result<u64> {
        info!("Clearing memory pool");
        
        let mut pools = self.pools.write().await;
        let mut freed_bytes = 0u64;
        
        for pool in pools.values_mut() {
            for buffer in pool.drain(..) {
                freed_bytes += buffer.size();
                self.allocator.deallocate(buffer).await?;
            }
        }
        
        pools.clear();
        drop(pools);
        
        // Reset counters
        *self.total_pooled_bytes.lock().await = 0;
        
        info!("Cleared memory pool, freed {} bytes", freed_bytes);
        Ok(freed_bytes)
    }

    fn get_pool_key(&self, size: u64, alignment: u64) -> PoolKey {
        let size_class = self.get_size_class(size);
        PoolKey {
            size_class,
            alignment,
        }
    }

    fn get_size_class(&self, size: u64) -> u64 {
        // Round up to next power of 2, with minimum size
        let size = size.max(MIN_POOL_SIZE_CLASS);
        
        if size > MAX_POOL_SIZE_CLASS {
            return MAX_POOL_SIZE_CLASS;
        }
        
        // Find the smallest power of 2 that is >= size
        let mut size_class = MIN_POOL_SIZE_CLASS;
        while size_class < size {
            size_class *= POOL_SIZE_CLASS_MULTIPLIER;
        }
        
        size_class.min(MAX_POOL_SIZE_CLASS)
    }
}

/// Pool statistics for monitoring.
#[derive(Debug, Clone)]
pub struct PoolStatistics {
    pub total_allocations: u64,
    pub pool_hits: u64,
    pub pool_misses: u64,
    pub hit_ratio: f64,
    pub total_returns: u64,
    pub successful_returns: u64,
    pub return_success_ratio: f64,
    pub total_pooled_bytes: u64,
    pub total_pooled_buffers: u64,
    pub pool_count: u64,
    pub garbage_collections: u64,
    pub bytes_freed_by_gc: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Device, DeviceManager, DeviceConfig};

    async fn create_test_pool() -> Result<MemoryPool> {
        let device_config = DeviceConfig::default();
        let device_manager = DeviceManager::new(device_config).await?;
        let device = device_manager.select_best_device().await?;
        let memory_config = MemoryConfig::default();
        let allocator = Arc::new(GpuAllocator::new(device, memory_config.clone()).await?);
        
        MemoryPool::new(allocator, memory_config).await
    }

    #[tokio::test]
    async fn test_pool_creation() {
        let pool = create_test_pool().await.unwrap();
        let stats = pool.stats().await;
        
        assert_eq!(stats.total_allocations, 0);
        assert_eq!(stats.pool_hits, 0);
        assert_eq!(stats.total_pooled_bytes, 0);
    }

    #[tokio::test]
    async fn test_pool_miss_on_empty() {
        let pool = create_test_pool().await.unwrap();
        
        let result = pool.allocate(1024, 256).await.unwrap();
        assert!(result.is_none()); // Should miss on empty pool
        
        let stats = pool.stats().await;
        assert_eq!(stats.pool_misses, 1);
        assert_eq!(stats.total_allocations, 1);
    }

    #[tokio::test]
    async fn test_pool_return_and_hit() {
        let pool = create_test_pool().await.unwrap();
        
        // Create a buffer to return to pool
        let buffer = Arc::new(GpuBuffer::new_mock("test-buffer".to_string(), 1024, 256));
        
        // Return to pool
        pool.return_buffer(buffer.clone()).await.unwrap();
        
        let stats = pool.stats().await;
        assert_eq!(stats.successful_returns, 1);
        assert_eq!(stats.total_pooled_buffers, 1);
        
        // Try to allocate from pool
        let result = pool.allocate(512, 256).await.unwrap();
        assert!(result.is_some()); // Should hit
        
        let stats = pool.stats().await;
        assert_eq!(stats.pool_hits, 1);
    }

    #[test]
    fn test_size_class_calculation() {
        let pool = create_test_pool().await.unwrap();
        
        assert_eq!(pool.get_size_class(100), 256);     // Rounds up to min
        assert_eq!(pool.get_size_class(256), 256);     // Exact match
        assert_eq!(pool.get_size_class(300), 512);     // Rounds up to next power of 2
        assert_eq!(pool.get_size_class(1024), 1024);   // Exact match
        assert_eq!(pool.get_size_class(1500), 2048);   // Rounds up
    }

    #[tokio::test]
    async fn test_pool_key_generation() {
        let pool = create_test_pool().await.unwrap();
        
        let key1 = pool.get_pool_key(1024, 256);
        let key2 = pool.get_pool_key(1024, 256);
        let key3 = pool.get_pool_key(2048, 256);
        let key4 = pool.get_pool_key(1024, 512);
        
        assert_eq!(key1, key2); // Same size and alignment
        assert_ne!(key1, key3); // Different size
        assert_ne!(key1, key4); // Different alignment
    }

    #[tokio::test]
    async fn test_pool_buffer_age_limit() {
        let pool = create_test_pool().await.unwrap();
        
        // Create an old buffer (simulate by manually setting age)
        let mut buffer = GpuBuffer::new_mock("old-buffer".to_string(), 1024, 256);
        
        // For testing, we check if can_return works with age
        assert!(pool.can_return(&buffer).await);
        
        // In a real scenario, we'd test with actual old buffers
        // but that would require time manipulation or mocking
    }

    #[tokio::test]
    async fn test_garbage_collection() {
        let pool = create_test_pool().await.unwrap();
        
        // Add some buffers to pool
        for i in 0..5 {
            let buffer = Arc::new(GpuBuffer::new_mock(
                format!("buffer-{}", i), 
                1024, 
                256
            ));
            pool.return_buffer(buffer).await.unwrap();
        }
        
        let stats_before = pool.stats().await;
        assert_eq!(stats_before.total_pooled_buffers, 5);
        
        // Run garbage collection
        let freed = pool.garbage_collect().await.unwrap();
        
        let stats_after = pool.stats().await;
        assert_eq!(stats_after.garbage_collections, 1);
        // Note: In this test, buffers are new so they might not be freed
        // In real usage, old buffers would be freed
    }

    #[tokio::test]
    async fn test_pool_clear() {
        let pool = create_test_pool().await.unwrap();
        
        // Add some buffers
        for i in 0..3 {
            let buffer = Arc::new(GpuBuffer::new_mock(
                format!("buffer-{}", i), 
                1024, 
                256
            ));
            pool.return_buffer(buffer).await.unwrap();
        }
        
        let stats_before = pool.stats().await;
        assert_eq!(stats_before.total_pooled_buffers, 3);
        
        // Clear pool
        let freed = pool.clear().await.unwrap();
        assert!(freed > 0);
        
        let stats_after = pool.stats().await;
        assert_eq!(stats_after.total_pooled_buffers, 0);
        assert_eq!(stats_after.total_pooled_bytes, 0);
    }

    #[tokio::test]
    async fn test_pool_size_limits() {
        let mut config = MemoryConfig::default();
        config.max_pool_size_mb = 1; // 1 MB limit
        
        let device_config = DeviceConfig::default();
        let device_manager = DeviceManager::new(device_config).await.unwrap();
        let device = device_manager.select_best_device().await.unwrap();
        let allocator = Arc::new(GpuAllocator::new(device, config.clone()).await.unwrap());
        let pool = MemoryPool::new(allocator, config).await.unwrap();
        
        // Try to add buffers that exceed the limit
        for i in 0..10 {
            let buffer = Arc::new(GpuBuffer::new_mock(
                format!("buffer-{}", i), 
                200 * 1024, // 200 KB each
                256
            ));
            pool.return_buffer(buffer).await.unwrap();
        }
        
        let stats = pool.stats().await;
        // Should not have all 10 buffers due to size limit
        assert!(stats.total_pooled_bytes <= 1024 * 1024); // 1 MB
    }
}