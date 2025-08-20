//! Metrics collection for observability.

use crate::types::TransportMetrics;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Metrics collector for network operations.
pub struct MetricsCollector {
    transport_metrics: Arc<RwLock<HashMap<String, TransportMetrics>>>,
}

impl MetricsCollector {
    /// Creates a new metrics collector.
    pub fn new() -> Self {
        Self {
            transport_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Records transport metrics.
    pub async fn record_transport_metrics(&self, transport_name: &str, metrics: TransportMetrics) {
        let mut transport_metrics = self.transport_metrics.write().await;
        transport_metrics.insert(transport_name.to_string(), metrics);
    }
    
    /// Gets all transport metrics.
    pub async fn get_transport_metrics(&self) -> HashMap<String, TransportMetrics> {
        self.transport_metrics.read().await.clone()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}