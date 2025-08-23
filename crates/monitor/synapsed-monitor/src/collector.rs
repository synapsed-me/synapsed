//! Data collection from Substrates and Serventis frameworks
//!
//! This module subscribes to various observability sources and collects
//! data for aggregation and visualization.

use crate::{MonitorError, Result};
use synapsed_intent::{
    observability::{IntentEvent, IntentMetric, MetricType, ObservableIntent},
    IntentId, EventType,
};
use synapsed_substrates::{
    Source, Sink, Subject, Subscriber, Subscription,
    types::{Name, SubjectType, SubstratesResult},
};
use synapsed_serventis::{
    Service, Signal, Probe, Observation, Monitor, Condition, Confidence,
    Operation, Origin, Outcome,
};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use dashmap::DashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Configuration for the observability collector
#[derive(Debug, Clone)]
pub struct CollectorConfig {
    /// Size of the event buffer
    pub buffer_size: usize,
    /// How long to retain events (in seconds)
    pub retention_seconds: u64,
    /// Whether to collect metrics
    pub collect_metrics: bool,
    /// Whether to collect service signals
    pub collect_signals: bool,
    /// Whether to collect probe observations
    pub collect_observations: bool,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            buffer_size: 10000,
            retention_seconds: 3600, // 1 hour
            collect_metrics: true,
            collect_signals: true,
            collect_observations: true,
        }
    }
}

/// Unified event type combining data from both frameworks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollectedEvent {
    /// Event from Substrates
    SubstratesEvent {
        intent_id: IntentId,
        event_type: EventType,
        timestamp: DateTime<Utc>,
        data: serde_json::Value,
    },
    /// Metric from Substrates
    SubstratesMetric {
        intent_id: IntentId,
        metric_type: String,
        value: f64,
        timestamp: DateTime<Utc>,
        labels: Vec<(String, String)>,
    },
    /// Signal from Serventis Service
    ServentisSignal {
        service_id: String,
        signal: String,
        timestamp: DateTime<Utc>,
    },
    /// Observation from Serventis Probe
    ServentisObservation {
        probe_id: String,
        operation: String,
        origin: String,
        outcome: String,
        timestamp: DateTime<Utc>,
    },
    /// Status from Serventis Monitor
    ServentisStatus {
        monitor_id: String,
        condition: String,
        confidence: String,
        timestamp: DateTime<Utc>,
    },
}

/// Main collector that gathers data from all sources
pub struct ObservabilityCollector {
    config: CollectorConfig,
    /// Channel for collected events
    event_sender: mpsc::Sender<CollectedEvent>,
    event_receiver: Arc<RwLock<mpsc::Receiver<CollectedEvent>>>,
    /// Active subscriptions to observability sources
    subscriptions: Arc<RwLock<Vec<Arc<dyn Subscription + Send + Sync>>>>,
    /// Tracked observable intents
    tracked_intents: Arc<DashMap<IntentId, Arc<ObservableIntent>>>,
    /// Recent events cache
    recent_events: Arc<DashMap<String, Vec<CollectedEvent>>>,
}

impl ObservabilityCollector {
    /// Create a new collector with the given configuration
    pub fn new(config: CollectorConfig) -> Self {
        let (tx, rx) = mpsc::channel(config.buffer_size);
        
        Self {
            config,
            event_sender: tx,
            event_receiver: Arc::new(RwLock::new(rx)),
            subscriptions: Arc::new(RwLock::new(Vec::new())),
            tracked_intents: Arc::new(DashMap::new()),
            recent_events: Arc::new(DashMap::new()),
        }
    }
    
    /// Start collecting from an observable intent
    pub async fn track_intent(&self, intent: Arc<ObservableIntent>) -> Result<()> {
        let intent_id = intent.intent_id();
        
        // Store the intent for tracking
        self.tracked_intents.insert(intent_id, intent.clone());
        
        // Start periodic collection of metrics
        if self.config.collect_metrics {
            let collector = self.clone();
            let intent_clone = intent.clone();
            tokio::spawn(async move {
                collector.collect_intent_metrics(intent_clone).await;
            });
        }
        
        // Collect probe observations
        if self.config.collect_observations {
            let observations = intent.get_observations();
            for obs in observations {
                self.process_observation("intent-probe", obs).await?;
            }
        }
        
        // Collect monitor status
        let (status, confidence) = intent.get_monitor_status().await;
        self.process_status("intent-monitor", status, confidence).await?;
        
        Ok(())
    }
    
    /// Collect metrics from an intent periodically
    async fn collect_intent_metrics(&self, intent: Arc<ObservableIntent>) {
        loop {
            // Get queue statistics
            let stats = intent.queue_stats();
            let metric = CollectedEvent::SubstratesMetric {
                intent_id: intent.intent_id(),
                metric_type: "queue_depth".to_string(),
                value: (stats.total_submitted - stats.total_executed) as f64,
                timestamp: Utc::now(),
                labels: vec![
                    ("total_submitted".to_string(), stats.total_submitted.to_string()),
                    ("total_executed".to_string(), stats.total_executed.to_string()),
                ],
            };
            
            let _ = self.event_sender.send(metric).await;
            
            // Sleep for a bit before next collection
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }
    
    /// Process a Serventis observation
    async fn process_observation(&self, probe_id: &str, observation: Observation) -> Result<()> {
        let event = CollectedEvent::ServentisObservation {
            probe_id: probe_id.to_string(),
            operation: format!("{:?}", observation.operation()),
            origin: format!("{:?}", observation.origin()),
            outcome: format!("{:?}", observation.outcome()),
            timestamp: Utc::now(),
        };
        
        self.event_sender.send(event).await
            .map_err(|e| MonitorError::CollectionError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Process a Serventis status
    async fn process_status(&self, monitor_id: &str, condition: Condition, confidence: Confidence) -> Result<()> {
        let event = CollectedEvent::ServentisStatus {
            monitor_id: monitor_id.to_string(),
            condition: format!("{:?}", condition),
            confidence: format!("{:?}", confidence),
            timestamp: Utc::now(),
        };
        
        self.event_sender.send(event).await
            .map_err(|e| MonitorError::CollectionError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Get the event receiver for processing collected events
    pub fn get_receiver(&self) -> Arc<RwLock<mpsc::Receiver<CollectedEvent>>> {
        self.event_receiver.clone()
    }
    
    /// Get recent events for a specific intent
    pub fn get_recent_events(&self, intent_id: &IntentId) -> Vec<CollectedEvent> {
        self.recent_events
            .get(&intent_id.to_string())
            .map(|entry| entry.value().clone())
            .unwrap_or_default()
    }
    
    /// Clean up old events based on retention policy
    pub async fn cleanup_old_events(&self) {
        let cutoff = Utc::now() - chrono::Duration::seconds(self.config.retention_seconds as i64);
        
        for mut entry in self.recent_events.iter_mut() {
            entry.value_mut().retain(|event| {
                match event {
                    CollectedEvent::SubstratesEvent { timestamp, .. } |
                    CollectedEvent::SubstratesMetric { timestamp, .. } |
                    CollectedEvent::ServentisSignal { timestamp, .. } |
                    CollectedEvent::ServentisObservation { timestamp, .. } |
                    CollectedEvent::ServentisStatus { timestamp, .. } => {
                        *timestamp > cutoff
                    }
                }
            });
        }
    }
}

impl Clone for ObservabilityCollector {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            event_sender: self.event_sender.clone(),
            event_receiver: self.event_receiver.clone(),
            subscriptions: self.subscriptions.clone(),
            tracked_intents: self.tracked_intents.clone(),
            recent_events: self.recent_events.clone(),
        }
    }
}