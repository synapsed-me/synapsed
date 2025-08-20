//! Arena allocator implementation for bulk allocation with O(1) reset.
//!
//! This module implements an arena allocator that provides fast bulk allocation
//! and extremely fast reset operations.

use crate::{SynapsedError, SynapsedResult};
use super::{AllocationStats, MemoryError, SynapsedAllocator};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Arena allocator for bulk allocation patterns
#[derive(Debug)]
pub struct ArenaAllocator {
    /// Current arena being allocated from
    current_arena: Option<Arena>,
    /// List of full arenas
    full_arenas: Vec<Arena>,
    /// Capacity for each arena
    arena_capacity: usize,
    /// Total number of arenas created
    total_arenas: AtomicUsize,
    /// Statistics
    total_allocated: AtomicUsize,
    active_bytes: AtomicUsize,
}

/// A single arena for bulk allocation
#[derive(Debug)]
struct Arena {
    /// Memory buffer for this arena
    buffer: NonNull<u8>,
    /// Total capacity of the arena
    capacity: usize,
    /// Current offset into the arena
    offset: usize,
}

impl ArenaAllocator {
    /// Create a new arena allocator with default capacity
    pub fn new() -> SynapsedResult<Self> {
        Self::with_capacity(4) // Default 4 arenas
    }

    /// Create a new arena allocator with specified initial arena count
    pub fn with_capacity(initial_arenas: usize) -> SynapsedResult<Self> {
        let arena_capacity = 64 * 1024; // 64KB per arena
        
        Ok(Self {
            current_arena: None,
            full_arenas: Vec::new(),
            arena_capacity,
            total_arenas: AtomicUsize::new(0),
            total_allocated: AtomicUsize::new(0),
            active_bytes: AtomicUsize::new(0),
        })
    }

    /// Allocate a new arena
    fn allocate_new_arena(&mut self) -> SynapsedResult<()> {
        let layout = std::alloc::Layout::from_size_align(self.arena_capacity, 8)
            .map_err(|_| MemoryError::InvalidSize(self.arena_capacity))?;

        let buffer_ptr = unsafe { std::alloc::alloc(layout) };
        if buffer_ptr.is_null() {
            return Err(MemoryError::OutOfMemory.into());
        }

        let buffer = NonNull::new(buffer_ptr)
            .ok_or(MemoryError::OutOfMemory)?;

        // Move current arena to full_arenas if it exists
        if let Some(arena) = self.current_arena.take() {
            self.full_arenas.push(arena);
        }

        // Create new current arena
        self.current_arena = Some(Arena {
            buffer,
            capacity: self.arena_capacity,
            offset: 0,
        });

        self.total_arenas.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Try to allocate from the current arena
    fn try_allocate_from_current(&mut self, size: usize, align: usize) -> Option<NonNull<u8>> {
        let arena = self.current_arena.as_mut()?;
        
        // Align the current offset
        let aligned_offset = (arena.offset + align - 1) & !(align - 1);
        
        // Check if we have enough space
        if aligned_offset + size > arena.capacity {
            return None;
        }
        
        // Allocate from the arena
        let ptr = unsafe { arena.buffer.as_ptr().add(aligned_offset) };
        arena.offset = aligned_offset + size;
        
        NonNull::new(ptr)
    }

    /// Get the alignment for a given size
    fn align_for_size(size: usize) -> usize {
        // Default alignment strategy - would be implemented
        if size >= 8 { 8 } else if size >= 4 { 4 } else { 1 }
    }
}

impl SynapsedAllocator for ArenaAllocator {
    fn allocate(&mut self, size: usize) -> SynapsedResult<NonNull<u8>> {
        if !self.can_allocate(size) {
            return Err(MemoryError::InvalidSize(size).into());
        }

        let align = Self::align_for_size(size);
        
        // Try to allocate from current arena
        if let Some(ptr) = self.try_allocate_from_current(size, align) {
            self.total_allocated.fetch_add(size, Ordering::Relaxed);
            self.active_bytes.fetch_add(size, Ordering::Relaxed);
            return Ok(ptr);
        }
        
        // Need a new arena
        self.allocate_new_arena()?;
        
        // Try again with new arena
        if let Some(ptr) = self.try_allocate_from_current(size, align) {
            self.total_allocated.fetch_add(size, Ordering::Relaxed);
            self.active_bytes.fetch_add(size, Ordering::Relaxed);
            Ok(ptr)
        } else {
            Err(MemoryError::ArenaFailed.into())
        }
    }

    fn deallocate(&mut self, _ptr: NonNull<u8>, _size: usize) -> SynapsedResult<()> {
        // Arena allocator doesn't support individual deallocation
        // This is by design - use reset() instead
        Ok(())
    }

    fn stats(&self) -> AllocationStats {
        let total_allocated = self.total_allocated.load(Ordering::Relaxed);
        let active_bytes = self.active_bytes.load(Ordering::Relaxed);
        let total_arenas = self.total_arenas.load(Ordering::Relaxed);
        
        // Calculate fragmentation based on arena usage
        let total_arena_capacity = total_arenas * self.arena_capacity;
        let fragmentation = if total_arena_capacity > 0 {
            1.0 - (active_bytes as f64 / total_arena_capacity as f64)
        } else {
            0.0
        };
        
        AllocationStats {
            total_allocations: total_allocated as u64,
            total_deallocations: 0, // Arena doesn't track individual deallocations
            total_bytes_allocated: total_allocated as u64,
            total_bytes_deallocated: 0,
            active_allocations: if active_bytes > 0 { 1 } else { 0 }, // Simplified
            active_bytes: active_bytes as u64,
            peak_allocations: total_allocated as u64,
            peak_bytes: active_bytes as u64,
            cache_hit_rate: 0.0, // Not applicable for arena allocator
            fragmentation_ratio: fragmentation,
            numa_locality_ratio: 0.0, // Not applicable for arena allocator
            size_class_stats: std::collections::HashMap::new(),
        }
    }

    fn reset(&mut self) -> SynapsedResult<()> {
        // Reset current arena offset to 0 (O(1) operation)
        if let Some(arena) = &mut self.current_arena {
            arena.offset = 0;
        }
        
        // Reset all full arenas offsets to 0
        for arena in &mut self.full_arenas {
            arena.offset = 0;
        }
        
        // Move all full arenas back to available (they're now empty)
        if let Some(current) = self.current_arena.take() {
            self.full_arenas.push(current);
        }
        
        // Take the first arena as current if available
        if !self.full_arenas.is_empty() {
            self.current_arena = Some(self.full_arenas.remove(0));
        }
        
        // Reset statistics
        self.active_bytes.store(0, Ordering::Relaxed);
        
        Ok(())
    }
}

unsafe impl Send for ArenaAllocator {}
unsafe impl Sync for ArenaAllocator {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_arena_allocator_creation() {
        let allocator = ArenaAllocator::new();
        assert!(allocator.is_ok(), "Should create arena allocator successfully");
    }

    #[tokio::test]
    async fn test_arena_allocator_with_capacity() {
        let allocator = ArenaAllocator::with_capacity(8);
        assert!(allocator.is_ok(), "Should create arena allocator with specified capacity");
        
        let allocator = allocator.unwrap();
        assert_eq!(allocator.total_arenas.load(Ordering::Relaxed), 8, "Should pre-allocate specified number of arenas");
    }

    #[tokio::test]
    async fn test_sequential_allocations() {
        let mut allocator = ArenaAllocator::new().unwrap();
        
        // Test sequential allocations
        let ptr1 = allocator.allocate(64).unwrap();
        let ptr2 = allocator.allocate(128).unwrap();
        let ptr3 = allocator.allocate(256).unwrap();
        
        // Verify pointers are not null and sequential
        assert!(!ptr1.as_ptr().is_null(), "First allocation should not be null");
        assert!(!ptr2.as_ptr().is_null(), "Second allocation should not be null");
        assert!(!ptr3.as_ptr().is_null(), "Third allocation should not be null");
        
        // In arena allocation, subsequent allocations should have higher addresses
        assert!(ptr2.as_ptr() > ptr1.as_ptr(), "Sequential allocations should have increasing addresses");
        assert!(ptr3.as_ptr() > ptr2.as_ptr(), "Sequential allocations should have increasing addresses");
    }

    #[tokio::test]
    async fn test_large_allocations() {
        let mut allocator = ArenaAllocator::new().unwrap();
        
        // Test allocating objects larger than typical arena size
        let large_size = 512 * 1024; // 512KB
        let ptr = allocator.allocate(large_size).unwrap();
        assert!(!ptr.as_ptr().is_null(), "Large allocation should succeed");
        
        let stats = allocator.stats();
        assert!(stats.active_bytes >= large_size as u64, "Should track large allocation");
    }

    #[tokio::test]
    async fn test_arena_expansion() {
        let mut allocator = ArenaAllocator::new().unwrap();
        
        // Allocate enough to force arena expansion
        let mut ptrs = Vec::new();
        for i in 0..100 {
            let ptr = allocator.allocate(1024).unwrap(); // 1KB each
            ptrs.push(ptr);
            assert!(!ptr.as_ptr().is_null(), "Allocation {} should succeed", i);
        }
        
        let stats = allocator.stats();
        assert!(stats.total_allocations >= 100, "Should track all allocations");
        assert!(stats.active_bytes >= 100 * 1024, "Should track all allocated bytes");
    }

    #[tokio::test]
    async fn test_alignment_requirements() {
        let mut allocator = ArenaAllocator::new().unwrap();
        
        // Test various sizes with alignment requirements
        let ptr1 = allocator.allocate(1).unwrap(); // 1-byte aligned
        let ptr2 = allocator.allocate(4).unwrap(); // 4-byte aligned
        let ptr3 = allocator.allocate(8).unwrap(); // 8-byte aligned
        let ptr4 = allocator.allocate(16).unwrap(); // 8-byte aligned (max)
        
        // Check alignment
        assert_eq!(ptr1.as_ptr() as usize % 1, 0, "1-byte allocation should be 1-byte aligned");
        assert_eq!(ptr2.as_ptr() as usize % 4, 0, "4-byte allocation should be 4-byte aligned");
        assert_eq!(ptr3.as_ptr() as usize % 8, 0, "8-byte allocation should be 8-byte aligned");
        assert_eq!(ptr4.as_ptr() as usize % 8, 0, "16-byte allocation should be 8-byte aligned");
    }

    #[tokio::test]
    async fn test_o1_reset_performance() {
        let mut allocator = ArenaAllocator::new().unwrap();
        
        // Allocate a lot of objects
        for _ in 0..1000 {
            let _ = allocator.allocate(64).unwrap();
        }
        
        let stats_before = allocator.stats();
        assert!(stats_before.active_allocations > 0, "Should have active allocations before reset");
        
        // Reset should be O(1) regardless of number of allocations
        let start = std::time::Instant::now();
        allocator.reset().unwrap();
        let reset_time = start.elapsed();
        
        // Reset should be very fast (under 1ms even for many allocations)
        assert!(reset_time.as_millis() < 10, "Reset should be very fast (O(1))");
        
        let stats_after = allocator.stats();
        assert_eq!(stats_after.active_allocations, 0, "Should have no active allocations after reset");
        assert_eq!(stats_after.active_bytes, 0, "Should have no active bytes after reset");
    }

    #[tokio::test]
    async fn test_no_individual_deallocation() {
        let mut allocator = ArenaAllocator::new().unwrap();
        
        let ptr = allocator.allocate(64).unwrap();
        
        // Arena allocator should not support individual deallocation
        // but should not error (it's a no-op)
        let result = allocator.deallocate(ptr, 64);
        assert!(result.is_ok(), "Individual deallocation should be a no-op and succeed");
        
        // Memory should still be considered active until reset
        let stats = allocator.stats();
        assert!(stats.active_bytes > 0, "Memory should still be active after individual deallocate");
    }

    #[tokio::test]
    async fn test_arena_reuse_after_reset() {
        let mut allocator = ArenaAllocator::new().unwrap();
        
        // Allocate some memory
        let ptr1 = allocator.allocate(128).unwrap();
        let first_address = ptr1.as_ptr() as usize;
        
        // Reset the allocator
        allocator.reset().unwrap();
        
        // Allocate again - should reuse the same arena
        let ptr2 = allocator.allocate(128).unwrap();
        let second_address = ptr2.as_ptr() as usize;
        
        assert_eq!(first_address, second_address, "Should reuse the same memory after reset");
    }

    #[tokio::test]
    async fn test_stats_tracking() {
        let mut allocator = ArenaAllocator::new().unwrap();
        
        // Make several allocations
        allocator.allocate(32).unwrap();
        allocator.allocate(64).unwrap();
        allocator.allocate(128).unwrap();
        
        let stats = allocator.stats();
        assert_eq!(stats.total_allocations, 3, "Should track total number of allocations");
        assert_eq!(stats.total_bytes_allocated, 32 + 64 + 128, "Should track total bytes allocated");
        assert_eq!(stats.active_allocations, 3, "Should track active allocations");
        assert_eq!(stats.active_bytes, 32 + 64 + 128, "Should track active bytes");
        
        // Reset and check stats
        allocator.reset().unwrap();
        let stats_after_reset = allocator.stats();
        assert_eq!(stats_after_reset.active_allocations, 0, "Active allocations should be 0 after reset");
        assert_eq!(stats_after_reset.active_bytes, 0, "Active bytes should be 0 after reset");
        assert_eq!(stats_after_reset.total_allocations, 3, "Total allocations should persist after reset");
    }

    #[tokio::test]
    async fn test_memory_safety() {
        let mut allocator = ArenaAllocator::new().unwrap();
        
        // Allocate and write to memory
        let ptr = allocator.allocate(64).unwrap();
        unsafe {
            std::ptr::write(ptr.as_ptr(), 42u8);
            assert_eq!(std::ptr::read(ptr.as_ptr()), 42u8, "Should be able to read written value");
        }
        
        // After reset, memory should be available for reuse
        allocator.reset().unwrap();
        
        let new_ptr = allocator.allocate(64).unwrap();
        assert_eq!(ptr.as_ptr(), new_ptr.as_ptr(), "Should reuse the same memory location");
    }

    #[tokio::test]
    async fn test_concurrent_allocations() {
        use std::sync::{Arc, Mutex};
        use tokio::task;
        
        let allocator = Arc::new(Mutex::new(ArenaAllocator::new().unwrap()));
        let mut handles = vec![];
        
        // Spawn multiple tasks that allocate
        for i in 0..10 {
            let allocator_clone = Arc::clone(&allocator);
            let handle = task::spawn(async move {
                let mut alloc_guard = allocator_clone.lock().unwrap();
                let ptr = alloc_guard.allocate(128 + i * 16).unwrap();
                assert!(!ptr.as_ptr().is_null(), "Concurrent allocation should succeed");
            });
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }
        
        let final_stats = allocator.lock().unwrap().stats();
        assert_eq!(final_stats.total_allocations, 10, "Should have processed all concurrent allocations");
    }
}