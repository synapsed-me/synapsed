//! Observable storage for monitoring and reactive storage operations
//!
//! This module provides simple observability for storage operations without
//! external dependencies on substrate or serventis frameworks.

use crate::Storage;
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Storage event for observability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageEvent {
    /// Event type
    pub event_type: EventType,
    /// Associated key
    pub key: Option<Vec<u8>>,
    /// Timestamp in milliseconds
    pub timestamp: u64,
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Types of storage events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    /// Get operation
    Get,
    /// Put operation
    Put,
    /// Delete operation
    Delete,
    /// Flush operation
    Flush,
    /// Health check
    HealthCheck,
    /// Performance metric
    PerformanceMetric,
}

/// Health status for storage systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    /// System is operating normally
    Healthy,
    /// System has minor issues
    Warning,
    /// System has major issues
    Critical,
    /// System is not operational
    Failed,
}

/// Configuration for monitoring features
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    /// Enable event streaming
    pub enable_events: bool,
    /// Enable performance monitoring
    pub enable_performance: bool,
    /// Enable health checks
    pub enable_health_checks: bool,
    /// Monitoring interval in milliseconds
    pub interval_ms: u64,
    /// Maximum event buffer size
    pub max_buffer_size: usize,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_events: true,
            enable_performance: true,
            enable_health_checks: true,
            interval_ms: 1000,
            max_buffer_size: 10000,
        }
    }
}

/// Metrics export formats
#[derive(Debug, Clone)]
pub enum MetricsFormat {
    /// JSON format
    Json,
    /// Prometheus format
    Prometheus,
    /// InfluxDB format
    InfluxDb,
    /// Custom format
    Custom(String),
}

/// Simple observable storage wrapper
pub struct ObservableStorage<S: Storage + ?Sized> {
    inner: Arc<S>,
    event_sender: broadcast::Sender<StorageEvent>,
    monitoring_config: MonitoringConfig,
}

impl<S: Storage + ?Sized> ObservableStorage<S> {
    /// Create a new observable storage
    pub fn new(storage: Arc<S>, monitoring_config: MonitoringConfig) -> Self {
        let (event_sender, _) = broadcast::channel(monitoring_config.max_buffer_size);
        
        Self {
            inner: storage,
            event_sender,
            monitoring_config,
        }
    }

    /// Subscribe to storage events
    pub fn subscribe(&self) -> broadcast::Receiver<StorageEvent> {
        self.event_sender.subscribe()
    }

    /// Emit an event
    fn emit_event(&self, event_type: EventType, key: Option<&[u8]>) {
        if self.monitoring_config.enable_events {
            let event = StorageEvent {
                event_type,
                key: key.map(|k| k.to_vec()),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
                metadata: None,
            };
            
            // Ignore send errors (no receivers)
            let _ = self.event_sender.send(event);
        }
    }
}

#[async_trait]
impl<S: Storage + ?Sized> Storage for ObservableStorage<S> {
    type Error = S::Error;

    async fn get(&self, key: &[u8]) -> Result<Option<Bytes>, Self::Error> {
        self.emit_event(EventType::Get, Some(key));
        self.inner.get(key).await
    }

    async fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Self::Error> {
        self.emit_event(EventType::Put, Some(key));
        self.inner.put(key, value).await
    }

    async fn delete(&self, key: &[u8]) -> Result<(), Self::Error> {
        self.emit_event(EventType::Delete, Some(key));
        self.inner.delete(key).await
    }

    async fn exists(&self, key: &[u8]) -> Result<bool, Self::Error> {
        self.inner.exists(key).await
    }

    async fn flush(&self) -> Result<(), Self::Error> {
        self.emit_event(EventType::Flush, None);
        self.inner.flush().await
    }
}

/// Builder for creating observable storage
pub struct ObservableStorageBuilder {
    monitoring_config: MonitoringConfig,
}

impl ObservableStorageBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            monitoring_config: MonitoringConfig::default(),
        }
    }

    /// Set monitoring configuration
    pub fn with_monitoring_config(mut self, config: MonitoringConfig) -> Self {
        self.monitoring_config = config;
        self
    }

    /// Build the observable storage
    pub fn build<S: Storage + ?Sized>(self, storage: Arc<S>) -> ObservableStorage<S> {
        ObservableStorage::new(storage, self.monitoring_config)
    }
}

impl Default for ObservableStorageBuilder {
    fn default() -> Self {
        Self::new()
    }
}