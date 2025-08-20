//! Configuration utilities for consensus protocols

pub use crate::traits::{ConsensusConfig, TimeoutConfig};

impl ConsensusConfig {
    /// Create a new configuration with a single node (for testing)
    pub fn single_node(node_id: crate::NodeId) -> Self {
        Self::new(node_id.clone(), vec![node_id])
    }
    
    /// Add a validator to the configuration
    pub fn add_validator(mut self, validator: crate::NodeId) -> Self {
        if !self.validators.contains(&validator) {
            self.validators.push(validator);
            self.byzantine_threshold = (self.validators.len() - 1) / 3;
        }
        self
    }
    
    /// Set the Byzantine fault threshold
    pub fn with_byzantine_threshold(mut self, threshold: usize) -> Self {
        self.byzantine_threshold = threshold;
        self
    }
    
    /// Set timeout configuration
    pub fn with_timeouts(mut self, timeouts: TimeoutConfig) -> Self {
        self.timeouts = timeouts;
        self
    }
    
    /// Enable or disable fast path optimizations
    pub fn with_fast_path(mut self, enabled: bool) -> Self {
        self.enable_fast_path = enabled;
        self
    }
    
    /// Set maximum transactions per block
    pub fn with_max_transactions(mut self, max_tx: usize) -> Self {
        self.max_transactions_per_block = max_tx;
        self
    }
    
    /// Set maximum block size in bytes
    pub fn with_max_block_size(mut self, max_size: usize) -> Self {
        self.max_block_size_bytes = max_size;
        self
    }
}