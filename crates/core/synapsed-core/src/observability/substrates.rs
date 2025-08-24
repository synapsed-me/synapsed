//! Substrates observability integration for synapsed-core
//!
//! This module provides integration with the Substrates event system for
//! comprehensive observability of core operations.

#[cfg(feature = "substrates")]
use synapsed_substrates::{
    Subject, BasicSource, BasicSink, ManagedQueue,
    types::{Name, SubjectType},
};
#[cfg(feature = "substrates")]
use synapsed_serventis::{
    BasicService, BasicProbe, BasicMonitor,
};

use crate::SynapsedResult;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Core system events emitted through Substrates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreEvent {
    /// Event ID
    pub id: Uuid,
    /// Event type
    pub event_type: CoreEventType,
    /// Component that emitted the event
    pub component: String,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Additional context
    pub context: std::collections::HashMap<String, serde_json::Value>,
}

/// Types of core system events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoreEventType {
    /// Component initialized
    ComponentInitialized,
    /// Component shutdown
    ComponentShutdown,
    /// Configuration changed
    ConfigurationChanged,
    /// Error occurred
    ErrorOccurred,
    /// Resource allocated
    ResourceAllocated,
    /// Resource released
    ResourceReleased,
    /// Operation started
    OperationStarted,
    /// Operation completed
    OperationCompleted,
    /// Health check performed
    HealthCheckPerformed,
}

/// Core metrics collected through Substrates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreMetric {
    /// Metric name
    pub name: String,
    /// Metric value
    pub value: f64,
    /// Metric unit
    pub unit: String,
    /// Collection timestamp
    pub timestamp: DateTime<Utc>,
    /// Tags for categorization
    pub tags: Vec<(String, String)>,
}

/// Substrates-based observability for core components
#[cfg(feature = "substrates")]
pub struct CoreObservability {
    /// Component ID
    id: Uuid,
    /// Component name
    name: String,
    /// Subject for event emission
    subject: Arc<Subject>,
    /// Event source
    event_source: Arc<BasicSource<CoreEvent>>,
    /// Metrics sink
    metrics_sink: Arc<RwLock<BasicSink<CoreMetric>>>,
    /// Alert queue
    alert_queue: Arc<ManagedQueue>,
    /// Service monitor
    service: Arc<RwLock<BasicService>>,
    /// Health probe
    probe: Arc<RwLock<BasicProbe>>,
}

#[cfg(feature = "substrates")]
impl CoreObservability {
    /// Create a new core observability instance
    pub fn new(component_name: &str) -> SynapsedResult<Self> {
        let id = Uuid::new_v4();
        let name = component_name.to_string();
        
        // Create Substrates components
        let subject = Arc::new(Subject::new(
            Name::from(component_name),
            SubjectType::Service,
        ));
        
        let event_source = Arc::new(BasicSource::new(
            Name::from(format!("{}_events", component_name))
        ));
        
        let metrics_sink = Arc::new(RwLock::new(BasicSink::new(
            Name::from(format!("{}_metrics", component_name))
        )));
        
        let alert_queue = Arc::new(ManagedQueue::new(
            Name::from(format!("{}_alerts", component_name))
        ));
        
        // Create Serventis components
        let service = Arc::new(RwLock::new(BasicService::new(
            Name::from(component_name)
        )));
        
        let probe = Arc::new(RwLock::new(BasicProbe::new(
            Name::from(format!("{}_probe", component_name))
        )));
        
        Ok(Self {
            id,
            name,
            subject,
            event_source,
            metrics_sink,
            alert_queue,
            service,
            probe,
        })
    }
    
    /// Emit a core event
    pub async fn emit_event(&self, event_type: CoreEventType, context: std::collections::HashMap<String, serde_json::Value>) -> SynapsedResult<()> {
        let event = CoreEvent {
            id: Uuid::new_v4(),
            event_type,
            component: self.name.clone(),
            timestamp: Utc::now(),
            context,
        };
        
        // In a real implementation, this would emit through the event source
        tracing::debug!(
            component = %self.name,
            event_type = ?event.event_type,
            "Core event emitted"
        );
        
        Ok(())
    }
    
    /// Record a metric
    pub async fn record_metric(&self, name: &str, value: f64, unit: &str, tags: Vec<(String, String)>) -> SynapsedResult<()> {
        let metric = CoreMetric {
            name: name.to_string(),
            value,
            unit: unit.to_string(),
            timestamp: Utc::now(),
            tags,
        };
        
        // In a real implementation, this would send to the metrics sink
        tracing::debug!(
            component = %self.name,
            metric_name = %name,
            metric_value = %value,
            "Metric recorded"
        );
        
        Ok(())
    }
    
    /// Send an alert
    pub async fn send_alert(&self, severity: AlertSeverity, message: &str) -> SynapsedResult<()> {
        // In a real implementation, this would enqueue to the alert queue
        tracing::warn!(
            component = %self.name,
            severity = ?severity,
            message = %message,
            "Alert sent"
        );
        
        Ok(())
    }
    
    /// Update service health status
    pub async fn update_health(&self, healthy: bool, details: Option<String>) -> SynapsedResult<()> {
        // In a real implementation, this would update the probe
        tracing::info!(
            component = %self.name,
            healthy = %healthy,
            details = ?details,
            "Health status updated"
        );
        
        Ok(())
    }
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Informational alert
    Info,
    /// Warning alert
    Warning,
    /// Error alert
    Error,
    /// Critical alert requiring immediate attention
    Critical,
}

/// Stub implementation when Substrates feature is disabled
#[cfg(not(feature = "substrates"))]
pub struct CoreObservability {
    name: String,
}

#[cfg(not(feature = "substrates"))]
impl CoreObservability {
    /// Create a stub observability instance
    pub fn new(component_name: &str) -> SynapsedResult<Self> {
        Ok(Self {
            name: component_name.to_string(),
        })
    }
    
    /// No-op event emission
    pub async fn emit_event(&self, _event_type: CoreEventType, _context: std::collections::HashMap<String, serde_json::Value>) -> SynapsedResult<()> {
        Ok(())
    }
    
    /// No-op metric recording
    pub async fn record_metric(&self, _name: &str, _value: f64, _unit: &str, _tags: Vec<(String, String)>) -> SynapsedResult<()> {
        Ok(())
    }
    
    /// No-op alert sending
    pub async fn send_alert(&self, _severity: AlertSeverity, _message: &str) -> SynapsedResult<()> {
        Ok(())
    }
    
    /// No-op health update
    pub async fn update_health(&self, _healthy: bool, _details: Option<String>) -> SynapsedResult<()> {
        Ok(())
    }
}