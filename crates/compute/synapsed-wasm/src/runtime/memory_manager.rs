//! Memory management for WASM modules

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use parking_lot::RwLock;

use crate::error::{WasmError, WasmResult};
use crate::runtime::config::MemoryConfig;

/// Memory region information
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// Region ID
    pub id: uuid::Uuid,
    /// Starting address
    pub start: usize,
    /// Size in bytes
    pub size: usize,
    /// Whether the region is allocated
    pub allocated: bool,
    /// Module that owns this region
    pub owner: Option<String>,
    /// Creation timestamp
    pub created_at: std::time::SystemTime,
}

impl MemoryRegion {
    /// Create a new memory region
    pub fn new(start: usize, size: usize, owner: Option<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            start,
            size,
            allocated: true,
            owner,
            created_at: std::time::SystemTime::now(),
        }
    }

    /// Check if address is within this region
    pub fn contains(&self, address: usize) -> bool {
        address >= self.start && address < (self.start + self.size)
    }

    /// Get end address
    pub fn end(&self) -> usize {
        self.start + self.size
    }
}

/// Memory manager for WASM modules
pub struct MemoryManager {
    /// Memory configuration
    config: MemoryConfig,
    /// Memory regions
    regions: Arc<RwLock<HashMap<uuid::Uuid, MemoryRegion>>>,
    /// Free memory blocks
    free_blocks: Arc<Mutex<Vec<(usize, usize)>>>, // (start, size)
    /// Allocated memory counter
    allocated_memory: Arc<Mutex<usize>>,
    /// Peak memory usage
    peak_memory: Arc<Mutex<usize>>,
    /// GC statistics
    gc_stats: Arc<Mutex<GcStats>>,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new(config: MemoryConfig) -> Self {
        let mut free_blocks = Vec::new();
        free_blocks.push((0, config.memory_pool_size));

        Self {
            config,
            regions: Arc::new(RwLock::new(HashMap::new())),
            free_blocks: Arc::new(Mutex::new(free_blocks)),
            allocated_memory: Arc::new(Mutex::new(0)),
            peak_memory: Arc::new(Mutex::new(0)),
            gc_stats: Arc::new(Mutex::new(GcStats::default())),
        }
    }

    /// Allocate memory for a module
    pub fn allocate(&self, size: usize, owner: Option<String>) -> WasmResult<uuid::Uuid> {
        if size == 0 {
            return Err(WasmError::MemoryAllocation("Cannot allocate zero bytes".to_string()));
        }

        // Check if we need to run GC
        if self.should_run_gc() {
            self.garbage_collect()?;
        }

        // Find a suitable free block
        let start_address = self.find_free_block(size)?;

        // Create memory region
        let region = MemoryRegion::new(start_address, size, owner.clone());
        let region_id = region.id;

        // Update allocations
        {
            let mut regions = self.regions.write();
            regions.insert(region_id, region);
        }

        {
            let mut allocated = self.allocated_memory.lock().unwrap();
            *allocated += size;

            let mut peak = self.peak_memory.lock().unwrap();
            if *allocated > *peak {
                *peak = *allocated;
            }
        }

        tracing::debug!(
            region_id = %region_id,
            size = size,
            owner = ?owner,
            "Memory allocated"
        );

        Ok(region_id)
    }

    /// Deallocate memory region
    pub fn deallocate(&self, region_id: uuid::Uuid) -> WasmResult<()> {
        let region = {
            let mut regions = self.regions.write();
            regions.remove(&region_id)
                .ok_or_else(|| WasmError::MemoryViolation("Region not found".to_string()))?
        };

        // Add back to free blocks
        {
            let mut free_blocks = self.free_blocks.lock().unwrap();
            free_blocks.push((region.start, region.size));
            free_blocks.sort_by_key(|(start, _)| *start);
            self.coalesce_free_blocks(&mut free_blocks);
        }

        // Update allocated counter
        {
            let mut allocated = self.allocated_memory.lock().unwrap();
            *allocated = allocated.saturating_sub(region.size);
        }

        tracing::debug!(
            region_id = %region_id,
            size = region.size,
            "Memory deallocated"
        );

        Ok(())
    }

    /// Get memory region information
    pub fn get_region(&self, region_id: uuid::Uuid) -> Option<MemoryRegion> {
        let regions = self.regions.read();
        regions.get(&region_id).cloned()
    }

    /// Get memory usage statistics
    pub fn get_stats(&self) -> MemoryStats {
        let regions = self.regions.read();
        let allocated = *self.allocated_memory.lock().unwrap();
        let peak = *self.peak_memory.lock().unwrap();
        let gc_stats = self.gc_stats.lock().unwrap().clone();

        let free_blocks = self.free_blocks.lock().unwrap();
        let free_memory: usize = free_blocks.iter().map(|(_, size)| size).sum();

        MemoryStats {
            total_memory: self.config.memory_pool_size,
            allocated_memory: allocated,
            free_memory,
            peak_memory: peak,
            active_regions: regions.len(),
            gc_stats,
        }
    }

    /// Force garbage collection
    pub fn garbage_collect(&self) -> WasmResult<usize> {
        let start_time = std::time::Instant::now();
        let mut collected = 0usize;

        {
            let mut regions = self.regions.write();
            let mut to_remove = Vec::new();

            // Find regions that can be collected
            for (id, region) in regions.iter() {
                // For now, we don't have a sophisticated GC algorithm
                // In a real implementation, this would check for unreachable regions
                if !region.allocated {
                    to_remove.push(*id);
                    collected += region.size;
                }
            }

            // Remove collected regions
            for id in to_remove {
                if let Some(region) = regions.remove(&id) {
                    let mut free_blocks = self.free_blocks.lock().unwrap();
                    free_blocks.push((region.start, region.size));
                }
            }
        }

        // Update GC statistics
        {
            let mut gc_stats = self.gc_stats.lock().unwrap();
            gc_stats.total_collections += 1;
            gc_stats.total_collected += collected;
            gc_stats.last_collection_time = start_time.elapsed();
        }

        // Coalesce free blocks
        {
            let mut free_blocks = self.free_blocks.lock().unwrap();
            free_blocks.sort_by_key(|(start, _)| *start);
            self.coalesce_free_blocks(&mut free_blocks);
        }

        tracing::info!(
            collected_bytes = collected,
            duration_ms = start_time.elapsed().as_millis(),
            "Garbage collection completed"
        );

        Ok(collected)
    }

    /// Check if GC should be run
    fn should_run_gc(&self) -> bool {
        if !self.config.enable_gc {
            return false;
        }

        let allocated = *self.allocated_memory.lock().unwrap();
        allocated > self.config.gc_threshold
    }

    /// Find a free block of the requested size
    fn find_free_block(&self, size: usize) -> WasmResult<usize> {
        let mut free_blocks = self.free_blocks.lock().unwrap();

        // Find first fit
        for i in 0..free_blocks.len() {
            let (start, block_size) = free_blocks[i];
            if block_size >= size {
                // Use this block
                if block_size == size {
                    // Exact fit - remove the block
                    free_blocks.remove(i);
                } else {
                    // Partial fit - update the block
                    free_blocks[i] = (start + size, block_size - size);
                }
                return Ok(start);
            }
        }

        Err(WasmError::MemoryAllocation(format!(
            "No free block of size {} available",
            size
        )))
    }

    /// Coalesce adjacent free blocks
    fn coalesce_free_blocks(&self, free_blocks: &mut Vec<(usize, usize)>) {
        if free_blocks.len() <= 1 {
            return;
        }

        let mut i = 0;
        while i < free_blocks.len() - 1 {
            let (start1, size1) = free_blocks[i];
            let (start2, size2) = free_blocks[i + 1];

            // Check if blocks are adjacent
            if start1 + size1 == start2 {
                // Merge blocks
                free_blocks[i] = (start1, size1 + size2);
                free_blocks.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }

    /// Get all regions owned by a module
    pub fn get_regions_by_owner(&self, owner: &str) -> Vec<MemoryRegion> {
        let regions = self.regions.read();
        regions
            .values()
            .filter(|region| {
                region.owner.as_ref().map(|o| o == owner).unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    /// Deallocate all regions owned by a module
    pub fn deallocate_by_owner(&self, owner: &str) -> WasmResult<usize> {
        let regions_to_remove: Vec<uuid::Uuid> = {
            let regions = self.regions.read();
            regions
                .iter()
                .filter(|(_, region)| {
                    region.owner.as_ref().map(|o| o == owner).unwrap_or(false)
                })
                .map(|(id, _)| *id)
                .collect()
        };

        let mut total_deallocated = 0;
        for region_id in regions_to_remove {
            if let Ok(()) = self.deallocate(region_id) {
                if let Some(region) = self.get_region(region_id) {
                    total_deallocated += region.size;
                }
            }
        }

        Ok(total_deallocated)
    }

    /// Check if memory protection is enabled
    pub fn is_memory_protection_enabled(&self) -> bool {
        self.config.enable_memory_protection
    }

    /// Validate memory access
    pub fn validate_access(&self, address: usize, size: usize) -> WasmResult<()> {
        if !self.config.enable_memory_protection {
            return Ok(());
        }

        let regions = self.regions.read();
        for region in regions.values() {
            if region.contains(address) {
                if address + size <= region.end() {
                    return Ok(());
                } else {
                    return Err(WasmError::MemoryViolation(
                        "Access extends beyond region boundary".to_string(),
                    ));
                }
            }
        }

        Err(WasmError::MemoryViolation(
            "Access to unallocated memory".to_string(),
        ))
    }
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Total available memory
    pub total_memory: usize,
    /// Currently allocated memory
    pub allocated_memory: usize,
    /// Currently free memory
    pub free_memory: usize,
    /// Peak memory usage
    pub peak_memory: usize,
    /// Number of active regions
    pub active_regions: usize,
    /// Garbage collection statistics
    pub gc_stats: GcStats,
}

impl MemoryStats {
    /// Get memory utilization as percentage
    pub fn utilization(&self) -> f64 {
        if self.total_memory == 0 {
            0.0
        } else {
            (self.allocated_memory as f64 / self.total_memory as f64) * 100.0
        }
    }

    /// Get fragmentation ratio
    pub fn fragmentation(&self) -> f64 {
        if self.free_memory == 0 {
            0.0
        } else {
            // This is a simplified fragmentation metric
            self.active_regions as f64 / (self.free_memory as f64 / 1024.0)
        }
    }
}

/// Garbage collection statistics
#[derive(Debug, Clone, Default)]
pub struct GcStats {
    /// Total number of GC runs
    pub total_collections: u64,
    /// Total bytes collected
    pub total_collected: usize,
    /// Time of last collection
    pub last_collection_time: std::time::Duration,
    /// Average collection time
    pub average_collection_time: std::time::Duration,
}

impl GcStats {
    /// Update average collection time
    pub fn update_average(&mut self, duration: std::time::Duration) {
        if self.total_collections == 1 {
            self.average_collection_time = duration;
        } else {
            let total_time = self.average_collection_time * (self.total_collections - 1) as u32 + duration;
            self.average_collection_time = total_time / self.total_collections as u32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_manager_creation() {
        let config = MemoryConfig::default();
        let manager = MemoryManager::new(config);
        let stats = manager.get_stats();
        
        assert_eq!(stats.allocated_memory, 0);
        assert_eq!(stats.active_regions, 0);
        assert!(stats.free_memory > 0);
    }

    #[test]
    fn test_memory_allocation() {
        let config = MemoryConfig::default();
        let manager = MemoryManager::new(config);
        
        let region_id = manager.allocate(1024, Some("test_module".to_string())).unwrap();
        let stats = manager.get_stats();
        
        assert_eq!(stats.allocated_memory, 1024);
        assert_eq!(stats.active_regions, 1);
        
        let region = manager.get_region(region_id).unwrap();
        assert_eq!(region.size, 1024);
        assert_eq!(region.owner, Some("test_module".to_string()));
    }

    #[test]
    fn test_memory_deallocation() {
        let config = MemoryConfig::default();
        let manager = MemoryManager::new(config);
        
        let region_id = manager.allocate(1024, None).unwrap();
        assert_eq!(manager.get_stats().allocated_memory, 1024);
        
        manager.deallocate(region_id).unwrap();
        assert_eq!(manager.get_stats().allocated_memory, 0);
        assert_eq!(manager.get_stats().active_regions, 0);
    }

    #[test]
    fn test_owner_based_operations() {
        let config = MemoryConfig::default();
        let manager = MemoryManager::new(config);
        
        let _region1 = manager.allocate(1024, Some("module1".to_string())).unwrap();
        let _region2 = manager.allocate(2048, Some("module1".to_string())).unwrap();
        let _region3 = manager.allocate(512, Some("module2".to_string())).unwrap();
        
        let module1_regions = manager.get_regions_by_owner("module1");
        assert_eq!(module1_regions.len(), 2);
        
        let deallocated = manager.deallocate_by_owner("module1").unwrap();
        assert_eq!(deallocated, 3072); // 1024 + 2048
        
        let stats = manager.get_stats();
        assert_eq!(stats.allocated_memory, 512); // Only module2's allocation remains
        assert_eq!(stats.active_regions, 1);
    }

    #[test]
    fn test_memory_validation() {
        let mut config = MemoryConfig::default();
        config.enable_memory_protection = true;
        let manager = MemoryManager::new(config);
        
        let region_id = manager.allocate(1024, None).unwrap();
        let region = manager.get_region(region_id).unwrap();
        
        // Valid access
        assert!(manager.validate_access(region.start, 512).is_ok());
        
        // Invalid access - beyond region
        assert!(manager.validate_access(region.start, 2048).is_err());
        
        // Invalid access - unallocated memory
        assert!(manager.validate_access(region.end() + 1000, 100).is_err());
    }

    #[test]
    fn test_memory_stats() {
        let config = MemoryConfig::default();
        let manager = MemoryManager::new(config.clone());
        
        let _region = manager.allocate(config.memory_pool_size / 2, None).unwrap();
        let stats = manager.get_stats();
        
        assert!(stats.utilization() > 0.0);
        assert!(stats.utilization() < 100.0);
        assert_eq!(stats.total_memory, config.memory_pool_size);
    }
}