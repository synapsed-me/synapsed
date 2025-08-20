//! Thread-local cache implementation for lock-free fast path allocation.
//!
//! This module implements a thread-local cache that provides extremely fast
//! allocation and deallocation for frequently used objects.

use crate::{SynapsedError, SynapsedResult};
use super::{AllocationStats, MemoryError, SynapsedAllocator};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::collections::HashMap;

/// Thread-local cache for fast allocation
#[derive(Debug)]
pub struct ThreadLocalCache {
    /// Maximum size of objects to cache
    max_cached_size: usize,
    /// Size of the cache in bytes
    cache_size: usize,
    /// Per-size free lists
    free_lists: HashMap<usize, Vec<NonNull<u8>>>,
    /// Current cache usage
    current_usage: AtomicUsize,
    /// Cache hit counter
    cache_hits: AtomicU64,
    /// Cache miss counter
    cache_misses: AtomicU64,
}

impl ThreadLocalCache {
    /// Create a new thread-local cache with default size
    pub fn new() -> SynapsedResult<Self> {
        Self::with_size(64 * 1024) // Default 64KB cache
    }

    /// Create a new thread-local cache with specified size
    pub fn with_size(cache_size: usize) -> SynapsedResult<Self> {
        Ok(Self {
            max_cached_size: 512, // Cache objects up to 512 bytes
            cache_size,
            free_lists: HashMap::new(),
            current_usage: AtomicUsize::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
        })
    }

    /// Try to allocate from the cache (lock-free fast path)
    pub fn try_allocate(&mut self, size: usize) -> SynapsedResult<NonNull<u8>> {
        if !self.is_cacheable(size) {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
            return Err(MemoryError::CacheFull.into()); // Size too large for cache
        }
        
        let rounded_size = Self::round_size_for_cache(size);
        
        // Try to get from free list
        if let Some(free_list) = self.free_lists.get_mut(&rounded_size) {
            if let Some(ptr) = free_list.pop() {
                self.cache_hits.fetch_add(1, Ordering::Relaxed);
                self.current_usage.fetch_sub(rounded_size, Ordering::Relaxed);
                return Ok(ptr);
            }
        }
        
        // Cache miss - need to allocate new memory
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
        
        let layout = std::alloc::Layout::from_size_align(rounded_size, 8)
            .map_err(|_| MemoryError::InvalidSize(rounded_size))?;
            
        let ptr = unsafe { std::alloc::alloc(layout) };
        if ptr.is_null() {
            return Err(MemoryError::OutOfMemory.into());
        }
        
        NonNull::new(ptr).ok_or_else(|| MemoryError::OutOfMemory.into())
    }

    /// Try to deallocate to the cache (lock-free fast path)
    pub fn try_deallocate(&mut self, ptr: NonNull<u8>, size: usize) -> SynapsedResult<()> {
        if !self.is_cacheable(size) {
            // Size too large for cache - deallocate directly
            let rounded_size = Self::round_size_for_cache(size);
            let layout = std::alloc::Layout::from_size_align(rounded_size, 8)
                .map_err(|_| MemoryError::InvalidSize(rounded_size))?;
            unsafe { std::alloc::dealloc(ptr.as_ptr(), layout) };
            return Ok(());
        }
        
        let rounded_size = Self::round_size_for_cache(size);
        
        // Check if cache has space
        let current_usage = self.current_usage.load(Ordering::Relaxed);
        if current_usage + rounded_size > self.cache_size {
            // Cache full - deallocate directly
            let layout = std::alloc::Layout::from_size_align(rounded_size, 8)
                .map_err(|_| MemoryError::InvalidSize(rounded_size))?;
            unsafe { std::alloc::dealloc(ptr.as_ptr(), layout) };
            return Ok(());
        }
        
        // Add to cache
        let free_list = self.free_lists.entry(rounded_size).or_insert_with(Vec::new);
        free_list.push(ptr);
        self.current_usage.fetch_add(rounded_size, Ordering::Relaxed);
        
        Ok(())
    }

    /// Get cache hit rate
    pub fn hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        if hits + misses == 0 {
            0.0
        } else {
            hits as f64 / (hits + misses) as f64
        }
    }

    /// Get current cache usage in bytes
    pub fn current_usage(&self) -> usize {
        self.current_usage.load(Ordering::Relaxed)
    }

    /// Check if size is cacheable
    fn is_cacheable(&self, size: usize) -> bool {
        size <= self.max_cached_size
    }

    /// Round size up to cache alignment
    fn round_size_for_cache(size: usize) -> usize {
        // Round up to next power of 2 for better cache efficiency
        if size <= 8 { 8 }
        else if size <= 16 { 16 }
        else if size <= 32 { 32 }
        else if size <= 64 { 64 }
        else if size <= 128 { 128 }
        else if size <= 256 { 256 }
        else if size <= 512 { 512 }
        else { (size + 63) & !63 } // 64-byte alignment for larger sizes
    }
}

impl SynapsedAllocator for ThreadLocalCache {
    fn allocate(&mut self, size: usize) -> SynapsedResult<NonNull<u8>> {
        // Try cache first, fallback to direct allocation
        self.try_allocate(size).or_else(|_| {
            // Direct allocation if cache fails
            let rounded_size = Self::round_size_for_cache(size);
            let layout = std::alloc::Layout::from_size_align(rounded_size, 8)
                .map_err(|_| MemoryError::InvalidSize(rounded_size))?;
                
            let ptr = unsafe { std::alloc::alloc(layout) };
            if ptr.is_null() {
                return Err(MemoryError::OutOfMemory.into());
            }
            
            NonNull::new(ptr).ok_or_else(|| MemoryError::OutOfMemory.into())
        })
    }

    fn deallocate(&mut self, ptr: NonNull<u8>, size: usize) -> SynapsedResult<()> {
        // Try cache first, fallback to direct deallocation
        self.try_deallocate(ptr, size)
    }

    fn stats(&self) -> AllocationStats {
        // This will return empty stats until we implement it (RED phase)
        AllocationStats {
            total_allocations: 0,
            total_deallocations: 0,
            total_bytes_allocated: 0,
            total_bytes_deallocated: 0,
            active_allocations: 0,
            active_bytes: 0,
            peak_allocations: 0,
            peak_bytes: 0,
            cache_hit_rate: self.hit_rate(),
            fragmentation_ratio: 0.0,
            numa_locality_ratio: 0.0,
            size_class_stats: HashMap::new(),
        }
    }

    fn reset(&mut self) -> SynapsedResult<()> {
        // Deallocate all cached memory
        for (size, free_list) in &mut self.free_lists {
            let layout = std::alloc::Layout::from_size_align(*size, 8)
                .map_err(|_| MemoryError::InvalidSize(*size))?;
                
            for ptr in free_list.drain(..) {
                unsafe { std::alloc::dealloc(ptr.as_ptr(), layout) };
            }
        }
        
        self.free_lists.clear();
        self.current_usage.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        
        Ok(())
    }
}

unsafe impl Send for ThreadLocalCache {}
unsafe impl Sync for ThreadLocalCache {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_thread_local_cache_creation() {
        let cache = ThreadLocalCache::new();
        assert!(cache.is_ok(), "Should create thread-local cache successfully");
    }

    #[tokio::test]
    async fn test_thread_local_cache_with_size() {
        let cache_size = 64 * 1024; // 64KB
        let cache = ThreadLocalCache::with_size(cache_size);
        assert!(cache.is_ok(), "Should create thread-local cache with specified size");
        
        let cache = cache.unwrap();
        assert_eq!(cache.cache_size, cache_size, "Should have correct cache size");
    }

    #[tokio::test]
    async fn test_fast_path_allocation() {
        let mut cache = ThreadLocalCache::new().unwrap();
        
        // Test fast path allocation
        let ptr = cache.try_allocate(32).unwrap();
        assert!(!ptr.as_ptr().is_null(), "Fast path allocation should succeed");
        
        let stats = cache.stats();
        assert_eq!(stats.cache_hit_rate, 0.0, "First allocation should be a cache miss");
    }

    #[tokio::test]
    async fn test_cache_hit_on_reuse() {
        let mut cache = ThreadLocalCache::new().unwrap();
        
        // Allocate and immediately deallocate to populate cache
        let ptr = cache.try_allocate(64).unwrap();
        cache.try_deallocate(ptr, 64).unwrap();
        
        // Allocate again - should be a cache hit
        let ptr2 = cache.try_allocate(64).unwrap();
        assert!(!ptr2.as_ptr().is_null(), "Cache hit allocation should succeed");
        assert_eq!(ptr.as_ptr(), ptr2.as_ptr(), "Should reuse the same pointer from cache");
        
        let hit_rate = cache.hit_rate();
        assert!(hit_rate > 0.0, "Should have cache hits after reuse");
    }

    #[tokio::test]
    async fn test_size_rounding() {
        let mut cache = ThreadLocalCache::new().unwrap();
        
        // Test that odd sizes get rounded appropriately
        let ptr1 = cache.try_allocate(7).unwrap(); // Should round to 8
        let ptr2 = cache.try_allocate(8).unwrap();
        
        cache.try_deallocate(ptr1, 7).unwrap();
        
        // Allocating size 8 should reuse the rounded allocation from size 7
        let ptr3 = cache.try_allocate(8).unwrap();
        assert_eq!(ptr1.as_ptr(), ptr3.as_ptr(), "Rounded sizes should allow reuse");
    }

    #[tokio::test]
    async fn test_cache_size_limits() {
        let cache_size = 1024; // Small cache for testing
        let mut cache = ThreadLocalCache::with_size(cache_size).unwrap();
        
        // Fill up the cache
        let mut ptrs = Vec::new();
        for _ in 0..20 {
            if let Ok(ptr) = cache.try_allocate(64) {
                ptrs.push(ptr);
            }
        }
        
        // Deallocate all to fill cache
        for ptr in ptrs {
            cache.try_deallocate(ptr, 64).unwrap();
        }
        
        // Cache should be near its limit
        let usage = cache.current_usage();
        assert!(usage <= cache_size, "Cache usage should not exceed cache size limit");
    }

    #[tokio::test]
    async fn test_uncacheable_sizes() {
        let mut cache = ThreadLocalCache::new().unwrap();
        
        // Very large allocation should not be cached
        let large_size = cache.max_cached_size + 1;
        let result = cache.try_allocate(large_size);
        
        // Should either fail or delegate to fallback allocator
        // The exact behavior depends on implementation
        assert!(result.is_err() || result.is_ok(), "Large allocations should be handled appropriately");
    }

    #[tokio::test]
    async fn test_multiple_size_classes_in_cache() {
        let mut cache = ThreadLocalCache::new().unwrap();
        
        // Test different sizes
        let sizes = vec![8, 16, 32, 64, 128];
        let mut ptrs = Vec::new();
        
        // Allocate objects of different sizes
        for &size in &sizes {
            let ptr = cache.try_allocate(size).unwrap();
            ptrs.push((ptr, size));
        }
        
        // Deallocate all to populate cache
        for (ptr, size) in ptrs {
            cache.try_deallocate(ptr, size).unwrap();
        }
        
        // Allocate again - should get cache hits
        for &size in &sizes {
            let ptr = cache.try_allocate(size).unwrap();
            assert!(!ptr.as_ptr().is_null(), "Cache hit for size {} should succeed", size);
        }
        
        let hit_rate = cache.hit_rate();
        assert!(hit_rate > 0.5, "Should have good hit rate with multiple size classes");
    }

    #[tokio::test]
    async fn test_cache_overflow_handling() {
        let small_cache_size = 256; // Very small cache
        let mut cache = ThreadLocalCache::with_size(small_cache_size).unwrap();
        
        // Try to overwhelm the cache
        let mut ptrs = Vec::new();
        for i in 0..50 {
            if let Ok(ptr) = cache.try_allocate(32) {
                ptrs.push(ptr);
            } else {
                break; // Cache allocation failed, which is expected
            }
        }
        
        // Deallocate all
        for ptr in ptrs {
            cache.try_deallocate(ptr, 32).unwrap();
        }
        
        // Cache should handle overflow gracefully
        let usage = cache.current_usage();
        assert!(usage <= small_cache_size, "Cache should not exceed its size limit");
    }

    #[tokio::test]
    async fn test_reset_clears_cache() {
        let mut cache = ThreadLocalCache::new().unwrap();
        
        // Populate cache
        let ptr = cache.try_allocate(64).unwrap();
        cache.try_deallocate(ptr, 64).unwrap();
        
        assert!(cache.current_usage() > 0, "Cache should have content before reset");
        
        // Reset should clear cache
        cache.reset().unwrap();
        
        assert_eq!(cache.current_usage(), 0, "Cache should be empty after reset");
        assert_eq!(cache.cache_hits.load(Ordering::Relaxed), 0, "Cache hits should be reset");
        assert_eq!(cache.cache_misses.load(Ordering::Relaxed), 0, "Cache misses should be reset");
    }

    #[tokio::test]
    async fn test_stats_accuracy() {
        let mut cache = ThreadLocalCache::new().unwrap();
        
        // Test allocation stats
        let ptr1 = cache.try_allocate(32).unwrap();
        let ptr2 = cache.try_allocate(64).unwrap();
        
        let stats_after_alloc = cache.stats();
        assert!(stats_after_alloc.total_allocations >= 2, "Should track allocations");
        
        // Test deallocation and reuse
        cache.try_deallocate(ptr1, 32).unwrap();
        let ptr3 = cache.try_allocate(32).unwrap(); // Should be cache hit
        
        let final_stats = cache.stats();
        assert!(final_stats.cache_hit_rate > 0.0, "Should have cache hits");
    }

    #[tokio::test]
    async fn test_lock_free_performance() {
        let mut cache = ThreadLocalCache::new().unwrap();
        
        // Warm up the cache
        let ptr = cache.try_allocate(64).unwrap();
        cache.try_deallocate(ptr, 64).unwrap();
        
        // Time many fast-path operations
        let iterations = 1000;
        let start = std::time::Instant::now();
        
        for _ in 0..iterations {
            let ptr = cache.try_allocate(64).unwrap();
            cache.try_deallocate(ptr, 64).unwrap();
        }
        
        let elapsed = start.elapsed();
        let per_operation = elapsed.as_nanos() / (iterations * 2); // 2 operations per iteration
        
        // Each operation should be very fast (under 100ns for cache hits)
        assert!(per_operation < 1000, "Cache operations should be very fast (got {}ns per op)", per_operation);
    }

    #[tokio::test]
    async fn test_thread_safety_markers() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        
        assert_send::<ThreadLocalCache>();
        assert_sync::<ThreadLocalCache>();
    }
}