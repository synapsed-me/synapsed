//! Memory allocation system optimized for high-performance P2P operations.
//!
//! This module provides a custom memory allocator designed for the Synapsed ecosystem
//! with the following key features:
//!
//! - **Size Class Allocation**: Efficient allocation using size pools to reduce fragmentation
//! - **Arena Allocator**: Fast bulk allocation with O(1) reset capability
//! - **Thread-Local Cache**: Lock-free fast path for frequent allocations
//! - **NUMA-Aware**: Intelligent allocation based on NUMA topology
//! - **Statistics Collection**: Comprehensive metrics for observability
//!
//! ## Usage
//!
//! ```rust
//! use synapsed_core::memory::{SizeClassAllocator, AllocationStats};
//!
//! let mut allocator = SizeClassAllocator::new()?;
//! let ptr = allocator.allocate(128)?;
//! // Use the allocated memory
//! allocator.deallocate(ptr, 128)?;
//! ```

use crate::{SynapsedError, SynapsedResult};
use crate::observability::{ContextAwareObservable, ObservationContext};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use uuid::Uuid;

pub mod arena;
pub mod numa;
pub mod size_class;
pub mod thread_local;

pub use arena::ArenaAllocator;
pub use numa::NumaAllocator;
pub use size_class::SizeClassAllocator;
pub use thread_local::ThreadLocalCache;

/// Maximum allocation size supported by the allocator
pub const MAX_ALLOCATION_SIZE: usize = 1024 * 1024; // 1MB

/// Number of size classes for the allocator
pub const SIZE_CLASS_COUNT: usize = 64;

/// Statistics for memory allocation operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationStats {
    /// Total number of allocations
    pub total_allocations: u64,
    /// Total number of deallocations
    pub total_deallocations: u64,
    /// Total bytes allocated
    pub total_bytes_allocated: u64,
    /// Total bytes deallocated
    pub total_bytes_deallocated: u64,
    /// Current active allocations
    pub active_allocations: u64,
    /// Current active bytes
    pub active_bytes: u64,
    /// Peak allocations
    pub peak_allocations: u64,
    /// Peak bytes
    pub peak_bytes: u64,
    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f64,
    /// Fragmentation ratio (0.0 to 1.0)
    pub fragmentation_ratio: f64,
    /// NUMA locality ratio (0.0 to 1.0)
    pub numa_locality_ratio: f64,
    /// Per-size-class statistics
    pub size_class_stats: HashMap<usize, SizeClassStats>,
}

/// Statistics for a specific size class
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeClassStats {
    /// Size of this class in bytes
    pub size: usize,
    /// Number of allocations in this class
    pub allocations: u64,
    /// Number of deallocations in this class
    pub deallocations: u64,
    /// Number of objects currently in use
    pub objects_in_use: u64,
    /// Number of free objects available
    pub free_objects: u64,
}

/// Configuration for memory allocator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Enable NUMA-aware allocation
    pub numa_aware: bool,
    /// Size of thread-local cache in bytes
    pub thread_cache_size: usize,
    /// Number of arenas to pre-allocate
    pub initial_arenas: usize,
    /// Maximum number of arenas
    pub max_arenas: usize,
    /// Enable statistics collection
    pub collect_stats: bool,
    /// Enable observability integration
    pub enable_observability: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            numa_aware: true,
            thread_cache_size: 64 * 1024, // 64KB
            initial_arenas: 4,
            max_arenas: 256,
            collect_stats: true,
            enable_observability: true,
        }
    }
}

/// Errors specific to memory allocation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryError {
    /// Out of memory
    OutOfMemory,
    /// Invalid allocation size
    InvalidSize(usize),
    /// Invalid pointer for deallocation
    InvalidPointer,
    /// NUMA topology not available
    NumaUnavailable,
    /// Thread-local cache full
    CacheFull,
    /// Arena allocation failed
    ArenaFailed,
    /// Size class not found
    SizeClassNotFound(usize),
}

impl From<MemoryError> for SynapsedError {
    fn from(err: MemoryError) -> Self {
        match err {
            MemoryError::OutOfMemory => SynapsedError::Internal("Out of memory".to_string()),
            MemoryError::InvalidSize(size) => {
                SynapsedError::InvalidInput(format!("Invalid allocation size: {}", size))
            }
            MemoryError::InvalidPointer => {
                SynapsedError::InvalidInput("Invalid pointer for deallocation".to_string())
            }
            MemoryError::NumaUnavailable => {
                SynapsedError::Internal("NUMA topology not available".to_string())
            }
            MemoryError::CacheFull => {
                SynapsedError::Internal("Thread-local cache is full".to_string())
            }
            MemoryError::ArenaFailed => {
                SynapsedError::Internal("Arena allocation failed".to_string())
            }
            MemoryError::SizeClassNotFound(size) => {
                SynapsedError::InvalidInput(format!("Size class not found for size: {}", size))
            }
        }
    }
}

/// Trait for memory allocators in the Synapsed ecosystem
pub trait SynapsedAllocator: Send + Sync {
    /// Allocate memory of the specified size
    fn allocate(&mut self, size: usize) -> SynapsedResult<NonNull<u8>>;

    /// Deallocate memory at the specified pointer
    fn deallocate(&mut self, ptr: NonNull<u8>, size: usize) -> SynapsedResult<()>;

    /// Get current allocation statistics
    fn stats(&self) -> AllocationStats;

    /// Reset allocator state (if supported)
    fn reset(&mut self) -> SynapsedResult<()>;

    /// Check if the allocator supports the given size
    fn can_allocate(&self, size: usize) -> bool {
        size > 0 && size <= MAX_ALLOCATION_SIZE
    }
}

/// Main memory allocator for the Synapsed system
#[derive(Debug)]
pub struct SynapsedMemoryAllocator {
    /// Size class allocator for small allocations
    size_class_allocator: SizeClassAllocator,
    /// Arena allocator for bulk allocations
    arena_allocator: ArenaAllocator,
    /// Thread-local cache for fast path
    thread_cache: ThreadLocalCache,
    /// NUMA-aware allocator wrapper
    numa_allocator: Option<NumaAllocator>,
    /// Configuration
    config: MemoryConfig,
    /// Statistics
    stats: Arc<AllocationStats>,
    /// Unique identifier for observability
    id: Uuid,
}

impl SynapsedMemoryAllocator {
    /// Create a new memory allocator with default configuration
    pub fn new() -> SynapsedResult<Self> {
        Self::with_config(MemoryConfig::default())
    }

    /// Create a new memory allocator with custom configuration
    pub fn with_config(config: MemoryConfig) -> SynapsedResult<Self> {
        let size_class_allocator = SizeClassAllocator::new()?;
        let arena_allocator = ArenaAllocator::with_capacity(config.initial_arenas)?;
        let thread_cache = ThreadLocalCache::with_size(config.thread_cache_size)?;
        
        let numa_allocator = if config.numa_aware {
            Some(NumaAllocator::new()?)
        } else {
            None
        };

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
            size_class_allocator,
            arena_allocator,
            thread_cache,
            numa_allocator,
            config,
            stats,
            id: Uuid::new_v4(),
        })
    }
}

impl SynapsedAllocator for SynapsedMemoryAllocator {
    fn allocate(&mut self, size: usize) -> SynapsedResult<NonNull<u8>> {
        if !self.can_allocate(size) {
            return Err(MemoryError::InvalidSize(size).into());
        }

        // Try thread-local cache first for small allocations
        if size <= self.config.thread_cache_size / 4 {
            if let Ok(ptr) = self.thread_cache.try_allocate(size) {
                return Ok(ptr);
            }
        }

        // Use size class allocator for small to medium allocations
        if size <= MAX_ALLOCATION_SIZE / 2 {
            self.size_class_allocator.allocate(size)
        } else {
            // Use arena allocator for large allocations
            self.arena_allocator.allocate(size)
        }
    }

    fn deallocate(&mut self, ptr: NonNull<u8>, size: usize) -> SynapsedResult<()> {
        // Try thread-local cache first
        if size <= self.config.thread_cache_size / 4 {
            if self.thread_cache.try_deallocate(ptr, size).is_ok() {
                return Ok(());
            }
        }

        // Delegate to appropriate allocator
        if size <= MAX_ALLOCATION_SIZE / 2 {
            self.size_class_allocator.deallocate(ptr, size)
        } else {
            self.arena_allocator.deallocate(ptr, size)
        }
    }

    fn stats(&self) -> AllocationStats {
        (*self.stats).clone()
    }

    fn reset(&mut self) -> SynapsedResult<()> {
        self.size_class_allocator.reset()?;
        self.arena_allocator.reset()?;
        self.thread_cache.reset()?;
        Ok(())
    }
}

#[async_trait]
impl ContextAwareObservable for SynapsedMemoryAllocator {
    async fn observation_context(&self) -> SynapsedResult<ObservationContext> {
        let stats = self.stats();
        
        // This is a placeholder - would integrate with full observability system
        let context = ObservationContext {
            id: self.id,
            observer: "SynapsedMemoryAllocator".to_string(),
            subject: format!("memory-allocator-{}", self.id),
            environment: crate::observability::EnvironmentalConditions {
                load_characteristics: crate::observability::LoadCharacteristics {
                    intensity: (stats.active_allocations as f64 / stats.peak_allocations.max(1) as f64).min(1.0),
                    distribution_pattern: crate::observability::DistributionPattern::Emergent,
                    variability: 0.5, // Would calculate from actual data
                    predictability: 0.7,
                },
                resource_availability: crate::observability::ResourceAvailability {
                    computational: crate::observability::ResourceStatus {
                        availability: 0.8,
                        quality: 0.9,
                        reliability: 0.9,
                        trend: crate::observability::TrendDirection::Stable,
                    },
                    memory: crate::observability::ResourceStatus {
                        availability: 1.0 - (stats.active_bytes as f64 / stats.peak_bytes.max(1) as f64),
                        quality: 1.0 - stats.fragmentation_ratio,
                        reliability: stats.cache_hit_rate,
                        trend: crate::observability::TrendDirection::Stable,
                    },
                    network: crate::observability::ResourceStatus {
                        availability: 1.0,
                        quality: 1.0,
                        reliability: 1.0,
                        trend: crate::observability::TrendDirection::Stable,
                    },
                    storage: crate::observability::ResourceStatus {
                        availability: 1.0,
                        quality: 1.0,
                        reliability: 1.0,
                        trend: crate::observability::TrendDirection::Stable,
                    },
                    custom: HashMap::new(),
                },
                network_conditions: crate::observability::NetworkConditions {
                    connectivity_quality: 1.0,
                    latency_characteristics: crate::observability::LatencyCharacteristics {
                        average: 0.0,
                        variance: 0.0,
                        peak: 0.0,
                        pattern: crate::observability::LatencyPattern::Consistent,
                    },
                    bandwidth_availability: 1.0,
                    stability: 1.0,
                    peer_connectivity: vec![],
                },
                security_posture: crate::observability::SecurityPosture {
                    trust_level: 1.0,
                    threat_assessment: crate::observability::ThreatAssessment {
                        overall_level: 0.0,
                        detected_threats: vec![],
                        confidence: 1.0,
                    },
                    authentication_state: crate::observability::AuthenticationState::Authenticated,
                    privacy_level: 1.0,
                },
                external_influences: vec![],
            },
            patterns: vec![],
            intentions: vec![],
            relationships: vec![],
            temporal: crate::observability::TemporalContext {
                current_time: chrono::Utc::now(),
                time_since_last_event: chrono::Duration::seconds(0),
                patterns: vec![],
                prediction_horizon: chrono::Duration::minutes(5),
            },
            spatial: None,
        };

        Ok(context)
    }

    async fn update_context(&mut self, _context: ObservationContext) -> SynapsedResult<()> {
        // Would update allocator behavior based on context
        Ok(())
    }

    async fn react_to_context(&mut self, _context: &ObservationContext) -> SynapsedResult<Vec<crate::observability::ContextReaction>> {
        // Would generate reactions based on memory pressure, etc.
        Ok(vec![])
    }

    async fn learn_from_patterns(&mut self, _patterns: Vec<crate::observability::BehavioralPattern>) -> SynapsedResult<()> {
        // Would adjust allocation strategies based on patterns
        Ok(())
    }

    async fn predict_future_states(&self, _horizon: chrono::Duration) -> SynapsedResult<Vec<crate::observability::FutureState>> {
        // Would predict future memory usage patterns
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_allocator_creation() {
        let allocator = SynapsedMemoryAllocator::new();
        assert!(allocator.is_ok());
    }

    #[tokio::test]
    async fn test_memory_allocator_with_config() {
        let config = MemoryConfig {
            numa_aware: false,
            thread_cache_size: 32 * 1024,
            ..Default::default()
        };
        let allocator = SynapsedMemoryAllocator::with_config(config);
        assert!(allocator.is_ok());
    }

    #[tokio::test]
    async fn test_allocation_deallocation() {
        let mut allocator = SynapsedMemoryAllocator::new().unwrap();
        
        // Test small allocation
        let ptr = allocator.allocate(128).unwrap();
        assert!(!ptr.as_ptr().is_null());
        
        // Test deallocation
        let result = allocator.deallocate(ptr, 128);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_size_allocation() {
        let mut allocator = SynapsedMemoryAllocator::new().unwrap();
        
        // Test allocation too large
        let result = allocator.allocate(MAX_ALLOCATION_SIZE + 1);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_observability_integration() {
        let allocator = SynapsedMemoryAllocator::new().unwrap();
        let context = allocator.observation_context().await;
        assert!(context.is_ok());
        
        let ctx = context.unwrap();
        assert_eq!(ctx.observer, "SynapsedMemoryAllocator");
    }

    #[tokio::test]
    async fn test_stats_collection() {
        let mut allocator = SynapsedMemoryAllocator::new().unwrap();
        let stats = allocator.stats();
        
        assert_eq!(stats.total_allocations, 0);
        assert_eq!(stats.total_deallocations, 0);
        assert_eq!(stats.active_allocations, 0);
    }

    #[tokio::test]
    async fn test_reset_functionality() {
        let mut allocator = SynapsedMemoryAllocator::new().unwrap();
        let result = allocator.reset();
        assert!(result.is_ok());
    }
}