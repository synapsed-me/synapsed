//! Router configuration

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    pub hop_count: usize,
    pub circuit_lifetime: u64,
    pub mix_delay_ms: u64,
    pub use_cover_traffic: bool,
}

impl RouterConfig {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_hop_count(mut self, count: usize) -> Self {
        self.hop_count = count;
        self
    }
    
    pub fn with_circuit_lifetime(mut self, lifetime: u64) -> Self {
        self.circuit_lifetime = lifetime;
        self
    }
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            hop_count: 3,
            circuit_lifetime: 600,
            mix_delay_ms: 100,
            use_cover_traffic: true,
        }
    }
}