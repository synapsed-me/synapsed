//! Size class allocator implementation for efficient small object allocation.
//!
//! This module implements a size class allocator that groups allocation requests
//! into size classes to reduce fragmentation and improve performance.

use crate::{SynapsedError, SynapsedResult};
use super::{AllocationStats, MemoryError, SizeClassStats, SynapsedAllocator, SIZE_CLASS_COUNT};
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

/// Size class allocator with pre-defined size pools
#[derive(Debug)]
pub struct SizeClassAllocator {
    /// Size classes with their free lists
    size_classes: Vec<SizeClass>,
    /// Statistics for observability
    stats: Arc<AllocationStats>,
}

/// A single size class with its free list
#[derive(Debug)]
struct SizeClass {
    /// Size of objects in this class
    size: usize,
    /// Free list of available objects
    free_list: Vec<NonNull<u8>>,
    /// Total objects allocated for this class
    total_objects: AtomicUsize,
    /// Objects currently in use
    objects_in_use: AtomicUsize,
}

impl SizeClassAllocator {
    /// Create a new size class allocator
    pub fn new() -> SynapsedResult<Self> {
        let mut size_classes = Vec::with_capacity(SIZE_CLASS_COUNT);
        
        // Create size classes with exponentially increasing sizes
        // Size class 0: 8 bytes, 1: 16 bytes, 2: 32 bytes, etc.
        for i in 0..SIZE_CLASS_COUNT {
            let size = 8 << (i / 8); // Exponential growth with steps
            let adjusted_size = if i % 8 != 0 {
                size + (size / 8) * (i % 8) // Add intermediate steps
            } else {
                size
            };
            
            size_classes.push(SizeClass {
                size: adjusted_size.min(super::MAX_ALLOCATION_SIZE),
                free_list: Vec::new(),
                total_objects: AtomicUsize::new(0),
                objects_in_use: AtomicUsize::new(0),
            });
        }

        let stats = Arc::new(AllocationStats {
            total_allocations: 0,
            total_deallocations: 0,
            total_bytes_allocated: 0,
            total_bytes_deallocated: 0,
            active_allocations: 0,
            active_bytes: 0,
            peak_allocations: 0,
            peak_bytes: 0,
            cache_hit_rate: 0.0,
            fragmentation_ratio: 0.0,
            numa_locality_ratio: 0.0,
            size_class_stats: HashMap::new(),
        });

        Ok(Self {
            size_classes,
            stats,
        })
    }

    /// Get size class index for a given size
    fn size_class_index(&self, size: usize) -> Option<usize> {
        if size == 0 || size > super::MAX_ALLOCATION_SIZE {
            return None;
        }

        // Find the smallest size class that can accommodate this size
        for (index, size_class) in self.size_classes.iter().enumerate() {
            if size_class.size >= size {
                return Some(index);
            }
        }

        None
    }

    /// Allocate an object from the appropriate size class
    fn allocate_from_size_class(&mut self, class_index: usize) -> SynapsedResult<NonNull<u8>> {
        if class_index >= self.size_classes.len() {
            return Err(MemoryError::SizeClassNotFound(class_index).into());
        }

        let size_class = &mut self.size_classes[class_index];
        
        // Try to get from free list first
        if let Some(ptr) = size_class.free_list.pop() {
            size_class.objects_in_use.fetch_add(1, Ordering::Relaxed);
            return Ok(ptr);
        }

        // Need to allocate new objects for this size class
        self.refill_size_class(class_index)?;
        
        // Try again after refill
        if let Some(ptr) = size_class.free_list.pop() {
            size_class.objects_in_use.fetch_add(1, Ordering::Relaxed);
            Ok(ptr)
        } else {
            Err(MemoryError::OutOfMemory.into())
        }
    }

    /// Return an object to its size class
    fn deallocate_to_size_class(&mut self, ptr: NonNull<u8>, class_index: usize) -> SynapsedResult<()> {
        if class_index >= self.size_classes.len() {
            return Err(MemoryError::SizeClassNotFound(class_index).into());
        }

        let size_class = &mut self.size_classes[class_index];
        
        // Add to free list
        size_class.free_list.push(ptr);
        size_class.objects_in_use.fetch_sub(1, Ordering::Relaxed);
        
        Ok(())
    }

    /// Refill a size class when it runs out of objects
    fn refill_size_class(&mut self, class_index: usize) -> SynapsedResult<()> {
        if class_index >= self.size_classes.len() {
            return Err(MemoryError::SizeClassNotFound(class_index).into());
        }

        let size_class = &mut self.size_classes[class_index];
        let object_size = size_class.size;
        let objects_to_allocate = 64; // Batch allocate 64 objects

        // Allocate a chunk of memory for multiple objects
        let total_size = object_size * objects_to_allocate;
        let layout = std::alloc::Layout::from_size_align(total_size, 8)
            .map_err(|_| MemoryError::InvalidSize(total_size))?;

        let chunk_ptr = unsafe { std::alloc::alloc(layout) };
        if chunk_ptr.is_null() {
            return Err(MemoryError::OutOfMemory.into());
        }

        // Split the chunk into individual objects and add to free list
        for i in 0..objects_to_allocate {
            let object_ptr = unsafe { chunk_ptr.add(i * object_size) };
            if let Some(non_null_ptr) = NonNull::new(object_ptr) {
                size_class.free_list.push(non_null_ptr);
            }
        }

        size_class.total_objects.fetch_add(objects_to_allocate, Ordering::Relaxed);
        Ok(())
    }
}

impl SizeClassAllocator {
    /// Calculate fragmentation ratio
    fn calculate_fragmentation(&self) -> f64 {
        let mut total_allocated = 0;
        let mut total_used = 0;
        
        for size_class in &self.size_classes {
            let objects_in_use = size_class.objects_in_use.load(Ordering::Relaxed);
            let total_objects = size_class.total_objects.load(Ordering::Relaxed);
            
            total_allocated += total_objects * size_class.size;
            total_used += objects_in_use * size_class.size;
        }
        
        if total_allocated == 0 {
            0.0
        } else {
            1.0 - (total_used as f64 / total_allocated as f64)
        }
    }
}

impl SynapsedAllocator for SizeClassAllocator {
    fn allocate(&mut self, size: usize) -> SynapsedResult<NonNull<u8>> {
        if !self.can_allocate(size) {
            return Err(MemoryError::InvalidSize(size).into());
        }

        let class_index = self.size_class_index(size)
            .ok_or_else(|| MemoryError::SizeClassNotFound(size))?;

        self.allocate_from_size_class(class_index)
    }

    fn deallocate(&mut self, ptr: NonNull<u8>, size: usize) -> SynapsedResult<()> {
        let class_index = self.size_class_index(size)
            .ok_or_else(|| MemoryError::SizeClassNotFound(size))?;

        self.deallocate_to_size_class(ptr, class_index)
    }

    fn stats(&self) -> AllocationStats {
        let mut size_class_stats = HashMap::new();
        let mut total_active_allocations = 0;
        let mut total_active_bytes = 0;
        
        for size_class in &self.size_classes {
            let objects_in_use = size_class.objects_in_use.load(Ordering::Relaxed);
            let total_objects = size_class.total_objects.load(Ordering::Relaxed);
            
            size_class_stats.insert(size_class.size, SizeClassStats {
                size: size_class.size,
                allocations: total_objects as u64,
                deallocations: (total_objects - objects_in_use) as u64,
                objects_in_use: objects_in_use as u64,
                free_objects: size_class.free_list.len() as u64,
            });
            
            total_active_allocations += objects_in_use;
            total_active_bytes += objects_in_use * size_class.size;
        }
        
        AllocationStats {
            total_allocations: total_active_allocations as u64,
            total_deallocations: 0, // Would be tracked separately
            total_bytes_allocated: total_active_bytes as u64,
            total_bytes_deallocated: 0,
            active_allocations: total_active_allocations as u64,
            active_bytes: total_active_bytes as u64,
            peak_allocations: total_active_allocations as u64, // Simplified
            peak_bytes: total_active_bytes as u64,
            cache_hit_rate: 0.0, // Would be calculated from actual cache hits
            fragmentation_ratio: self.calculate_fragmentation(),
            numa_locality_ratio: 0.0, // Not applicable for size class allocator
            size_class_stats,
        }
    }

    fn reset(&mut self) -> SynapsedResult<()> {
        for size_class in &mut self.size_classes {
            // Deallocate all memory chunks
            // Note: This is a simplified reset - in a real implementation,
            // we'd need to track and properly deallocate the original chunks
            size_class.free_list.clear();
            size_class.total_objects.store(0, Ordering::Relaxed);
            size_class.objects_in_use.store(0, Ordering::Relaxed);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_size_class_allocator_creation() {
        let allocator = SizeClassAllocator::new();
        assert!(allocator.is_ok(), "Should create size class allocator successfully");
        
        let allocator = allocator.unwrap();
        assert_eq!(allocator.size_classes.len(), SIZE_CLASS_COUNT, "Should have correct number of size classes");
    }

    #[tokio::test]
    async fn test_size_class_index_calculation() {
        let allocator = SizeClassAllocator::new().unwrap();
        
        // Test various sizes map to appropriate size classes
        assert_eq!(allocator.size_class_index(8), Some(0), "Size 8 should map to class 0");
        assert_eq!(allocator.size_class_index(16), Some(1), "Size 16 should map to class 1");
        assert_eq!(allocator.size_class_index(32), Some(2), "Size 32 should map to class 2");
        assert_eq!(allocator.size_class_index(64), Some(3), "Size 64 should map to class 3");
        
        // Test sizes that need rounding up
        assert_eq!(allocator.size_class_index(9), Some(1), "Size 9 should round up to class 1 (16 bytes)");
        assert_eq!(allocator.size_class_index(17), Some(2), "Size 17 should round up to class 2 (32 bytes)");
        
        // Test maximum size
        assert!(allocator.size_class_index(super::MAX_ALLOCATION_SIZE).is_some(), "Max size should have a class");
        
        // Test oversized allocation
        assert_eq!(allocator.size_class_index(super::MAX_ALLOCATION_SIZE + 1), None, "Oversized allocation should return None");
    }

    #[tokio::test]
    async fn test_small_allocation() {
        let mut allocator = SizeClassAllocator::new().unwrap();
        
        // Test allocating small objects
        let ptr1 = allocator.allocate(8).unwrap();
        assert!(!ptr1.as_ptr().is_null(), "Allocated pointer should not be null");
        
        let ptr2 = allocator.allocate(16).unwrap();
        assert!(!ptr2.as_ptr().is_null(), "Second allocated pointer should not be null");
        assert_ne!(ptr1.as_ptr(), ptr2.as_ptr(), "Different allocations should have different pointers");
        
        // Test allocation from same size class
        let ptr3 = allocator.allocate(8).unwrap();
        assert!(!ptr3.as_ptr().is_null(), "Third allocated pointer should not be null");
        assert_ne!(ptr1.as_ptr(), ptr3.as_ptr(), "Same size class allocations should have different pointers");
    }

    #[tokio::test]
    async fn test_allocation_deallocation_cycle() {
        let mut allocator = SizeClassAllocator::new().unwrap();
        
        // Allocate memory
        let ptr = allocator.allocate(32).unwrap();
        let stats_after_alloc = allocator.stats();
        assert_eq!(stats_after_alloc.active_allocations, 1, "Should have 1 active allocation");
        assert_eq!(stats_after_alloc.active_bytes, 32, "Should have 32 active bytes");
        
        // Deallocate memory
        allocator.deallocate(ptr, 32).unwrap();
        let stats_after_dealloc = allocator.stats();
        assert_eq!(stats_after_dealloc.active_allocations, 0, "Should have 0 active allocations after deallocation");
        assert_eq!(stats_after_dealloc.active_bytes, 0, "Should have 0 active bytes after deallocation");
        assert_eq!(stats_after_dealloc.total_allocations, 1, "Should track total allocations");
        assert_eq!(stats_after_dealloc.total_deallocations, 1, "Should track total deallocations");
    }

    #[tokio::test]
    async fn test_size_class_reuse() {
        let mut allocator = SizeClassAllocator::new().unwrap();
        
        // Allocate and deallocate to populate free list
        let ptr1 = allocator.allocate(64).unwrap();
        allocator.deallocate(ptr1, 64).unwrap();
        
        // Allocate again - should reuse the freed object
        let ptr2 = allocator.allocate(64).unwrap();
        assert_eq!(ptr1.as_ptr(), ptr2.as_ptr(), "Should reuse freed object from same size class");
    }

    #[tokio::test]
    async fn test_multiple_size_classes() {
        let mut allocator = SizeClassAllocator::new().unwrap();
        
        // Allocate from different size classes
        let small_ptrs: Vec<_> = (0..10).map(|_| allocator.allocate(8).unwrap()).collect();
        let medium_ptrs: Vec<_> = (0..10).map(|_| allocator.allocate(128).unwrap()).collect();
        let large_ptrs: Vec<_> = (0..10).map(|_| allocator.allocate(1024).unwrap()).collect();
        
        let stats = allocator.stats();
        assert_eq!(stats.active_allocations, 30, "Should have 30 total allocations");
        
        // Verify size class specific stats
        let size_class_stats = &stats.size_class_stats;
        assert!(size_class_stats.contains_key(&8), "Should have stats for 8-byte class");
        assert!(size_class_stats.contains_key(&128), "Should have stats for 128-byte class");
        assert!(size_class_stats.contains_key(&1024), "Should have stats for 1024-byte class");
        
        assert_eq!(size_class_stats[&8].objects_in_use, 10, "Should have 10 objects in use for 8-byte class");
        assert_eq!(size_class_stats[&128].objects_in_use, 10, "Should have 10 objects in use for 128-byte class");
        assert_eq!(size_class_stats[&1024].objects_in_use, 10, "Should have 10 objects in use for 1024-byte class");
    }

    #[tokio::test]
    async fn test_fragmentation_handling() {
        let mut allocator = SizeClassAllocator::new().unwrap();
        
        // Allocate many objects
        let ptrs: Vec<_> = (0..100).map(|i| {
            let size = if i % 2 == 0 { 16 } else { 64 };
            allocator.allocate(size).unwrap()
        }).collect();
        
        // Deallocate every other object to create fragmentation
        for (i, ptr) in ptrs.iter().enumerate() {
            if i % 2 == 0 {
                let size = if i % 2 == 0 { 16 } else { 64 };
                allocator.deallocate(*ptr, size).unwrap();
            }
        }
        
        let stats = allocator.stats();
        assert!(stats.fragmentation_ratio < 0.5, "Fragmentation should be manageable with size classes");
    }

    #[tokio::test]
    async fn test_reset_functionality() {
        let mut allocator = SizeClassAllocator::new().unwrap();
        
        // Allocate some memory
        let _ptr1 = allocator.allocate(32).unwrap();
        let _ptr2 = allocator.allocate(64).unwrap();
        
        let stats_before = allocator.stats();
        assert!(stats_before.active_allocations > 0, "Should have active allocations before reset");
        
        // Reset the allocator
        allocator.reset().unwrap();
        
        let stats_after = allocator.stats();
        assert_eq!(stats_after.active_allocations, 0, "Should have no active allocations after reset");
        assert_eq!(stats_after.active_bytes, 0, "Should have no active bytes after reset");
    }

    #[tokio::test]
    async fn test_invalid_deallocation() {
        let mut allocator = SizeClassAllocator::new().unwrap();
        
        // Try to deallocate an invalid pointer
        let invalid_ptr = NonNull::new(0x1000 as *mut u8).unwrap();
        
        let result = allocator.deallocate(invalid_ptr, 32);
        assert!(result.is_err(), "Should fail to deallocate invalid pointer");
        
        match result.unwrap_err() {
            SynapsedError::InvalidInput(_) => {}, // Expected
            _ => panic!("Should return InvalidInput error for invalid pointer"),
        }
    }

    #[tokio::test]
    async fn test_size_class_overflow() {
        let mut allocator = SizeClassAllocator::new().unwrap();
        
        // Try to allocate more than maximum supported size
        let result = allocator.allocate(super::MAX_ALLOCATION_SIZE + 1);
        assert!(result.is_err(), "Should fail to allocate oversized object");
        
        match result.unwrap_err() {
            SynapsedError::InvalidInput(_) => {}, // Expected
            _ => panic!("Should return InvalidInput error for oversized allocation"),
        }
    }

    #[tokio::test]
    async fn test_concurrent_allocations() {
        use std::sync::{Arc, Mutex};
        use tokio::task;
        
        let allocator = Arc::new(Mutex::new(SizeClassAllocator::new().unwrap()));
        let mut handles = vec![];
        
        // Spawn multiple tasks that allocate and deallocate
        for i in 0..10 {
            let allocator_clone = Arc::clone(&allocator);
            let handle = task::spawn(async move {
                let mut alloc_guard = allocator_clone.lock().unwrap();
                
                // Allocate
                let size = 32 + (i * 16); // Different sizes
                let ptr = alloc_guard.allocate(size).unwrap();
                
                // Immediately deallocate
                alloc_guard.deallocate(ptr, size).unwrap();
            });
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }
        
        let final_stats = allocator.lock().unwrap().stats();
        assert_eq!(final_stats.active_allocations, 0, "Should have no active allocations after concurrent test");
        assert_eq!(final_stats.total_allocations, 10, "Should have processed all allocations");
        assert_eq!(final_stats.total_deallocations, 10, "Should have processed all deallocations");
    }
}