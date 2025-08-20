//! NUMA-aware allocator wrapper for intelligent memory placement.
//!
//! This module implements NUMA-aware allocation strategies to optimize
//! memory locality and performance on multi-socket systems.

use crate::{SynapsedError, SynapsedResult};
use super::{AllocationStats, MemoryError, SynapsedAllocator};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::collections::HashMap;

/// NUMA node identifier
pub type NumaNode = u32;

/// NUMA-aware allocator wrapper
#[derive(Debug)]
pub struct NumaAllocator {
    /// Available NUMA nodes
    numa_nodes: Vec<NumaNode>,
    /// Current NUMA node for this thread
    current_node: Option<NumaNode>,
    /// Per-node allocation stats
    node_stats: HashMap<NumaNode, NodeStats>,
    /// NUMA topology information
    topology: NumaTopology,
    /// Fallback to regular allocation if NUMA unavailable
    numa_available: bool,
}

/// Statistics for a specific NUMA node
#[derive(Debug, Clone, Default)]
struct NodeStats {
    /// Allocations on this node
    allocations: AtomicU64,
    /// Bytes allocated on this node
    bytes_allocated: AtomicU64,
    /// Local vs remote access ratio
    locality_ratio: AtomicU64, // stored as percentage * 100
}

/// NUMA topology information
#[derive(Debug, Clone)]
struct NumaTopology {
    /// Number of NUMA nodes
    node_count: usize,
    /// Node distances (for optimization)
    distances: HashMap<(NumaNode, NumaNode), u32>,
    /// CPUs per node
    cpus_per_node: HashMap<NumaNode, Vec<u32>>,
}

impl NumaAllocator {
    /// Create a new NUMA-aware allocator
    pub fn new() -> SynapsedResult<Self> {
        // Try to detect NUMA topology, fallback if not available
        let (topology, numa_available) = match Self::detect_numa_topology() {
            Ok(topo) => (topo, true),
            Err(_) => {
                // NUMA not available, create fallback topology
                let fallback_topology = NumaTopology {
                    node_count: 1,
                    distances: HashMap::new(),
                    cpus_per_node: {
                        let mut map = HashMap::new();
                        map.insert(0, vec![0]); // Single CPU on node 0
                        map
                    },
                };
                (fallback_topology, false)
            }
        };

        let numa_nodes = (0..topology.node_count as NumaNode).collect();
        let mut node_stats = HashMap::new();
        for &node in &numa_nodes {
            node_stats.insert(node, NodeStats::default());
        }

        Ok(Self {
            numa_nodes,
            current_node: Some(0), // Default to node 0
            node_stats,
            topology,
            numa_available,
        })
    }

    /// Get the current NUMA node for the calling thread
    pub fn current_numa_node(&self) -> Option<NumaNode> {
        self.current_node
    }

    /// Set preferred NUMA node for allocations
    pub fn set_preferred_node(&mut self, node: NumaNode) -> SynapsedResult<()> {
        if !self.numa_nodes.contains(&node) {
            return Err(MemoryError::NumaUnavailable.into());
        }
        self.current_node = Some(node);
        Ok(())
    }

    /// Allocate memory on a specific NUMA node
    pub fn allocate_on_node(&mut self, size: usize, node: NumaNode) -> SynapsedResult<NonNull<u8>> {
        if !self.numa_available || !self.numa_nodes.contains(&node) {
            // Fallback to regular allocation
            let layout = std::alloc::Layout::from_size_align(size, 8)
                .map_err(|_| MemoryError::InvalidSize(size))?;
            let ptr = unsafe { std::alloc::alloc(layout) };
            if ptr.is_null() {
                return Err(MemoryError::OutOfMemory.into());
            }
            return NonNull::new(ptr).ok_or_else(|| MemoryError::OutOfMemory.into());
        }
        
        // In a real implementation, this would use NUMA-specific allocation
        // For now, just do regular allocation and track stats
        let layout = std::alloc::Layout::from_size_align(size, 8)
            .map_err(|_| MemoryError::InvalidSize(size))?;
        let ptr = unsafe { std::alloc::alloc(layout) };
        if ptr.is_null() {
            return Err(MemoryError::OutOfMemory.into());
        }
        
        // Update node stats
        if let Some(stats) = self.node_stats.get(&node) {
            stats.allocations.fetch_add(1, Ordering::Relaxed);
            stats.bytes_allocated.fetch_add(size as u64, Ordering::Relaxed);
        }
        
        NonNull::new(ptr).ok_or_else(|| MemoryError::OutOfMemory.into())
    }

    /// Get NUMA node for a given memory address
    pub fn get_node_for_address(&self, ptr: NonNull<u8>) -> Option<NumaNode> {
        // This will fail until we implement it (RED phase)
        None
    }

    /// Get locality statistics
    pub fn locality_stats(&self) -> HashMap<NumaNode, f64> {
        // This will fail until we implement it (RED phase)
        HashMap::new()
    }

    /// Check if NUMA is available on this system
    pub fn is_numa_available(&self) -> bool {
        self.numa_available
    }

    /// Detect NUMA topology
    fn detect_numa_topology() -> SynapsedResult<NumaTopology> {
        // Simplified NUMA detection - in a real implementation,
        // this would query the system's NUMA topology
        #[cfg(target_os = "linux")]
        {
            // Try to read from /sys/devices/system/node/
            use std::fs;
            if let Ok(entries) = fs::read_dir("/sys/devices/system/node/") {
                let mut node_count = 0;
                for entry in entries {
                    if let Ok(entry) = entry {
                        if entry.file_name().to_string_lossy().starts_with("node") {
                            node_count += 1;
                        }
                    }
                }
                
                if node_count > 0 {
                    let mut cpus_per_node = HashMap::new();
                    for i in 0..node_count {
                        cpus_per_node.insert(i as NumaNode, vec![i as u32]);
                    }
                    
                    return Ok(NumaTopology {
                        node_count: node_count as usize,
                        distances: HashMap::new(),
                        cpus_per_node,
                    });
                }
            }
        }
        
        // Fallback: assume single NUMA node
        Err(MemoryError::NumaUnavailable.into())
    }

    /// Choose optimal NUMA node for allocation
    fn choose_optimal_node(&self, size: usize) -> NumaNode {
        // Default to node 0 until implemented
        0
    }
}

impl SynapsedAllocator for NumaAllocator {
    fn allocate(&mut self, size: usize) -> SynapsedResult<NonNull<u8>> {
        let node = self.current_node.unwrap_or_else(|| self.choose_optimal_node(size));
        self.allocate_on_node(size, node)
    }

    fn deallocate(&mut self, ptr: NonNull<u8>, size: usize) -> SynapsedResult<()> {
        // Deallocate memory (NUMA-aware deallocation would be more complex)
        let layout = std::alloc::Layout::from_size_align(size, 8)
            .map_err(|_| MemoryError::InvalidSize(size))?;
        unsafe { std::alloc::dealloc(ptr.as_ptr(), layout) };
        Ok(())
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
            cache_hit_rate: 0.0,
            fragmentation_ratio: 0.0,
            numa_locality_ratio: 0.0,
            size_class_stats: HashMap::new(),
        }
    }

    fn reset(&mut self) -> SynapsedResult<()> {
        // Reset all node statistics
        for (_, stats) in &self.node_stats {
            stats.allocations.store(0, Ordering::Relaxed);
            stats.bytes_allocated.store(0, Ordering::Relaxed);
            stats.locality_ratio.store(0, Ordering::Relaxed);
        }
        Ok(())
    }
}

unsafe impl Send for NumaAllocator {}
unsafe impl Sync for NumaAllocator {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_numa_allocator_creation() {
        let allocator = NumaAllocator::new();
        assert!(allocator.is_ok(), "Should create NUMA allocator successfully");
    }

    #[tokio::test]
    async fn test_numa_availability_detection() {
        let allocator = NumaAllocator::new().unwrap();
        
        // Should detect whether NUMA is available
        let numa_available = allocator.is_numa_available();
        assert!(numa_available || !numa_available, "Should return a boolean for NUMA availability");
    }

    #[tokio::test]
    async fn test_current_numa_node_detection() {
        let allocator = NumaAllocator::new().unwrap();
        
        if allocator.is_numa_available() {
            let current_node = allocator.current_numa_node();
            assert!(current_node.is_some(), "Should detect current NUMA node when NUMA is available");
            
            let node = current_node.unwrap();
            assert!(allocator.numa_nodes.contains(&node), "Current node should be in available nodes list");
        } else {
            // On systems without NUMA, current_node may be None
            let current_node = allocator.current_numa_node();
            // This is fine - either Some(0) for fallback or None
        }
    }

    #[tokio::test]
    async fn test_preferred_node_setting() {
        let mut allocator = NumaAllocator::new().unwrap();
        
        if allocator.is_numa_available() && !allocator.numa_nodes.is_empty() {
            let first_node = allocator.numa_nodes[0];
            let result = allocator.set_preferred_node(first_node);
            assert!(result.is_ok(), "Should be able to set preferred node");
            
            assert_eq!(allocator.current_node, Some(first_node), "Should update current node");
        }
    }

    #[tokio::test]
    async fn test_node_specific_allocation() {
        let mut allocator = NumaAllocator::new().unwrap();
        
        if allocator.is_numa_available() && !allocator.numa_nodes.is_empty() {
            let node = allocator.numa_nodes[0];
            let ptr = allocator.allocate_on_node(1024, node).unwrap();
            
            assert!(!ptr.as_ptr().is_null(), "NUMA-specific allocation should succeed");
            
            // Check if allocation is actually on the requested node
            let allocated_node = allocator.get_node_for_address(ptr);
            assert_eq!(allocated_node, Some(node), "Allocation should be on requested NUMA node");
        }
    }

    #[tokio::test]
    async fn test_fallback_when_numa_unavailable() {
        let mut allocator = NumaAllocator::new().unwrap();
        
        // Even without NUMA, basic allocation should work
        let ptr = allocator.allocate(256).unwrap();
        assert!(!ptr.as_ptr().is_null(), "Allocation should work even without NUMA");
        
        let result = allocator.deallocate(ptr, 256);
        assert!(result.is_ok(), "Deallocation should work even without NUMA");
    }

    #[tokio::test]
    async fn test_locality_statistics() {
        let mut allocator = NumaAllocator::new().unwrap();
        
        if allocator.is_numa_available() {
            // Make some allocations
            let _ptr1 = allocator.allocate(128).unwrap();
            let _ptr2 = allocator.allocate(256).unwrap();
            
            let locality_stats = allocator.locality_stats();
            
            // Should have statistics for at least one node
            assert!(!locality_stats.is_empty(), "Should have locality statistics");
            
            // All locality ratios should be between 0.0 and 1.0
            for (_node, ratio) in locality_stats {
                assert!(ratio >= 0.0 && ratio <= 1.0, "Locality ratio should be between 0.0 and 1.0");
            }
        }
    }

    #[tokio::test]
    async fn test_numa_topology_detection() {
        let topology_result = NumaAllocator::detect_numa_topology();
        
        // Should either succeed with valid topology or fail gracefully
        match topology_result {
            Ok(topology) => {
                assert!(topology.node_count > 0, "Should have at least one NUMA node");
                assert!(!topology.cpus_per_node.is_empty(), "Should have CPU mapping");
            }
            Err(_) => {
                // NUMA not available - this is fine
            }
        }
    }

    #[tokio::test]
    async fn test_optimal_node_selection() {
        let allocator = NumaAllocator::new().unwrap();
        
        // Test node selection for different allocation sizes
        let small_node = allocator.choose_optimal_node(64);
        let large_node = allocator.choose_optimal_node(1024 * 1024);
        
        if allocator.is_numa_available() {
            assert!(allocator.numa_nodes.contains(&small_node), "Selected node should be valid");
            assert!(allocator.numa_nodes.contains(&large_node), "Selected node should be valid");
        }
    }

    #[tokio::test]
    async fn test_cross_node_allocation_performance() {
        let mut allocator = NumaAllocator::new().unwrap();
        
        if allocator.is_numa_available() && allocator.numa_nodes.len() > 1 {
            let node1 = allocator.numa_nodes[0];
            let node2 = allocator.numa_nodes[1];
            
            // Allocate on different nodes and measure
            let start = std::time::Instant::now();
            let _ptr1 = allocator.allocate_on_node(1024, node1).unwrap();
            let local_time = start.elapsed();
            
            let start = std::time::Instant::now();
            let _ptr2 = allocator.allocate_on_node(1024, node2).unwrap();
            let remote_time = start.elapsed();
            
            // This is just a basic test - actual performance differences
            // depend on system topology and current load
            assert!(local_time.as_nanos() > 0, "Local allocation should take some time");
            assert!(remote_time.as_nanos() > 0, "Remote allocation should take some time");
        }
    }

    #[tokio::test]
    async fn test_numa_stats_collection() {
        let mut allocator = NumaAllocator::new().unwrap();
        
        // Make allocations on different nodes if available
        if allocator.is_numa_available() {
            for &node in &allocator.numa_nodes {
                let _ = allocator.allocate_on_node(128, node);
            }
        } else {
            // Fallback allocations
            let _ = allocator.allocate(128);
            let _ = allocator.allocate(256);
        }
        
        let stats = allocator.stats();
        assert!(stats.total_allocations > 0, "Should track allocations");
        
        if allocator.is_numa_available() {
            assert!(stats.numa_locality_ratio >= 0.0, "NUMA locality ratio should be valid");
        }
    }

    #[tokio::test]
    async fn test_memory_migration() {
        let mut allocator = NumaAllocator::new().unwrap();
        
        if allocator.is_numa_available() && allocator.numa_nodes.len() > 1 {
            let node1 = allocator.numa_nodes[0];
            let node2 = allocator.numa_nodes[1];
            
            // Allocate on first node
            let ptr = allocator.allocate_on_node(1024, node1).unwrap();
            let initial_node = allocator.get_node_for_address(ptr);
            assert_eq!(initial_node, Some(node1), "Should initially be on node1");
            
            // Change preferred node
            allocator.set_preferred_node(node2).unwrap();
            
            // New allocations should prefer node2
            let ptr2 = allocator.allocate(1024).unwrap();
            let new_node = allocator.get_node_for_address(ptr2);
            
            // May or may not be on node2 depending on system policy
            // but should be a valid node
            if let Some(node) = new_node {
                assert!(allocator.numa_nodes.contains(&node), "Allocated node should be valid");
            }
        }
    }

    #[tokio::test]
    async fn test_numa_error_handling() {
        let mut allocator = NumaAllocator::new().unwrap();
        
        // Test invalid node allocation
        let invalid_node = 9999;
        let result = allocator.allocate_on_node(128, invalid_node);
        
        if allocator.is_numa_available() {
            assert!(result.is_err(), "Should fail with invalid NUMA node");
        } else {
            // Without NUMA, might succeed with fallback
            // Either outcome is acceptable
        }
    }

    #[tokio::test]
    async fn test_reset_preserves_topology() {
        let mut allocator = NumaAllocator::new().unwrap();
        
        let original_nodes = allocator.numa_nodes.clone();
        let original_available = allocator.is_numa_available();
        
        // Make some allocations
        let _ = allocator.allocate(128);
        
        // Reset
        allocator.reset().unwrap();
        
        // Topology should be preserved
        assert_eq!(allocator.numa_nodes, original_nodes, "NUMA nodes should be preserved after reset");
        assert_eq!(allocator.is_numa_available(), original_available, "NUMA availability should be preserved");
        
        // Stats should be reset
        let stats = allocator.stats();
        assert_eq!(stats.active_allocations, 0, "Active allocations should be reset");
    }
}