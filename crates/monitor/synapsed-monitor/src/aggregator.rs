//! Event correlation and aggregation
//!
//! This module correlates events from Substrates and Serventis,
//! grouping them by intent, time window, and causality.

use crate::{
    collector::CollectedEvent,
    MonitorError, Result,
};
use synapsed_intent::IntentId;
use dashmap::DashMap;
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Time window for event correlation (in milliseconds)
const CORRELATION_WINDOW_MS: i64 = 1000;

/// Correlated event grouping related events together
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelatedEvent {
    /// Primary intent ID
    pub intent_id: IntentId,
    /// Time window of the correlation
    pub time_window: TimeWindow,
    /// Events from Substrates
    pub substrates_events: Vec<CollectedEvent>,
    /// Events from Serventis
    pub serventis_events: Vec<CollectedEvent>,
    /// Detected pattern (if any)
    pub pattern: Option<EventPattern>,
    /// Human-readable summary
    pub summary: String,
}

/// Time window for event grouping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeWindow {
    pub fn new(start: DateTime<Utc>, duration_ms: i64) -> Self {
        Self {
            start,
            end: start + Duration::milliseconds(duration_ms),
        }
    }
    
    pub fn contains(&self, timestamp: &DateTime<Utc>) -> bool {
        *timestamp >= self.start && *timestamp <= self.end
    }
}

/// Detected event patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventPattern {
    /// Normal task execution: start -> process -> success -> stop
    NormalExecution,
    /// Failed execution: start -> process -> failure -> stop
    FailedExecution,
    /// Retry pattern: failure -> retry -> success
    RetrySuccess,
    /// Repeated failures: failure -> retry -> failure
    RepeatedFailure,
    /// Resource contention: queue depth spike + delays
    ResourceContention,
    /// Anomalous behavior: unexpected tool usage or permission escalation
    AnomalousBehavior,
    /// Performance degradation: increasing latencies
    PerformanceDegradation,
}

/// Event aggregator that correlates events from multiple sources
pub struct EventAggregator {
    /// Events grouped by intent ID
    events_by_intent: Arc<DashMap<IntentId, Vec<CollectedEvent>>>,
    /// Correlated event groups
    correlated_events: Arc<RwLock<Vec<CorrelatedEvent>>>,
    /// Pattern detection rules
    pattern_detectors: Vec<Box<dyn PatternDetector>>,
}

impl EventAggregator {
    pub fn new() -> Self {
        Self {
            events_by_intent: Arc::new(DashMap::new()),
            correlated_events: Arc::new(RwLock::new(Vec::new())),
            pattern_detectors: Self::create_pattern_detectors(),
        }
    }
    
    /// Process a new event
    pub async fn process_event(&self, event: CollectedEvent) -> Result<()> {
        // Extract intent ID from event
        let intent_id = self.extract_intent_id(&event)?;
        
        // Add to intent's event list
        self.events_by_intent
            .entry(intent_id)
            .or_insert_with(Vec::new)
            .push(event.clone());
        
        // Try to correlate with recent events
        self.correlate_events(intent_id).await?;
        
        Ok(())
    }
    
    /// Extract intent ID from various event types
    fn extract_intent_id(&self, event: &CollectedEvent) -> Result<IntentId> {
        match event {
            CollectedEvent::SubstratesEvent { intent_id, .. } |
            CollectedEvent::SubstratesMetric { intent_id, .. } => Ok(*intent_id),
            
            // For Serventis events, extract from service/probe/monitor ID
            CollectedEvent::ServentisSignal { service_id, .. } => {
                self.parse_intent_from_id(service_id)
            },
            CollectedEvent::ServentisObservation { probe_id, .. } => {
                self.parse_intent_from_id(probe_id)
            },
            CollectedEvent::ServentisStatus { monitor_id, .. } => {
                self.parse_intent_from_id(monitor_id)
            },
        }
    }
    
    /// Parse intent ID from component ID (e.g., "intent-service-<uuid>")
    fn parse_intent_from_id(&self, component_id: &str) -> Result<IntentId> {
        // Extract UUID from component ID
        let parts: Vec<&str> = component_id.split('-').collect();
        if parts.len() >= 3 {
            let uuid_str = parts[2..].join("-");
            uuid::Uuid::parse_str(&uuid_str)
                .map(IntentId)
                .map_err(|e| MonitorError::AggregationError(format!("Invalid UUID: {}", e)))
        } else {
            Err(MonitorError::AggregationError("Cannot extract intent ID".to_string()))
        }
    }
    
    /// Correlate events for a specific intent
    async fn correlate_events(&self, intent_id: IntentId) -> Result<()> {
        let events = self.events_by_intent.get(&intent_id);
        if let Some(events) = events {
            let events = events.clone();
            
            // Group events by time windows
            let mut windows: Vec<(TimeWindow, Vec<CollectedEvent>)> = Vec::new();
            
            for event in events {
                let timestamp = self.get_event_timestamp(&event);
                
                // Find or create appropriate window
                let mut added = false;
                for (window, window_events) in &mut windows {
                    if window.contains(&timestamp) {
                        window_events.push(event.clone());
                        added = true;
                        break;
                    }
                }
                
                if !added {
                    let window = TimeWindow::new(timestamp, CORRELATION_WINDOW_MS);
                    windows.push((window, vec![event]));
                }
            }
            
            // Create correlated events for each window
            for (window, window_events) in windows {
                let correlated = self.create_correlated_event(
                    intent_id,
                    window,
                    window_events
                ).await?;
                
                let mut correlated_list = self.correlated_events.write().await;
                correlated_list.push(correlated);
            }
        }
        
        Ok(())
    }
    
    /// Get timestamp from event
    fn get_event_timestamp(&self, event: &CollectedEvent) -> DateTime<Utc> {
        match event {
            CollectedEvent::SubstratesEvent { timestamp, .. } |
            CollectedEvent::SubstratesMetric { timestamp, .. } |
            CollectedEvent::ServentisSignal { timestamp, .. } |
            CollectedEvent::ServentisObservation { timestamp, .. } |
            CollectedEvent::ServentisStatus { timestamp, .. } => *timestamp,
        }
    }
    
    /// Create a correlated event from a group of events
    async fn create_correlated_event(
        &self,
        intent_id: IntentId,
        window: TimeWindow,
        events: Vec<CollectedEvent>,
    ) -> Result<CorrelatedEvent> {
        // Separate events by framework
        let mut substrates_events = Vec::new();
        let mut serventis_events = Vec::new();
        
        for event in events {
            match &event {
                CollectedEvent::SubstratesEvent { .. } |
                CollectedEvent::SubstratesMetric { .. } => {
                    substrates_events.push(event);
                },
                CollectedEvent::ServentisSignal { .. } |
                CollectedEvent::ServentisObservation { .. } |
                CollectedEvent::ServentisStatus { .. } => {
                    serventis_events.push(event);
                },
            }
        }
        
        // Detect patterns
        let pattern = self.detect_pattern(&substrates_events, &serventis_events);
        
        // Generate summary
        let summary = self.generate_summary(&pattern, &substrates_events, &serventis_events);
        
        Ok(CorrelatedEvent {
            intent_id,
            time_window: window,
            substrates_events,
            serventis_events,
            pattern,
            summary,
        })
    }
    
    /// Detect patterns in the event sequence
    fn detect_pattern(
        &self,
        substrates: &[CollectedEvent],
        serventis: &[CollectedEvent],
    ) -> Option<EventPattern> {
        for detector in &self.pattern_detectors {
            if let Some(pattern) = detector.detect(substrates, serventis) {
                return Some(pattern);
            }
        }
        None
    }
    
    /// Generate human-readable summary
    fn generate_summary(
        &self,
        pattern: &Option<EventPattern>,
        substrates: &[CollectedEvent],
        serventis: &[CollectedEvent],
    ) -> String {
        match pattern {
            Some(EventPattern::NormalExecution) => {
                format!("Task completed successfully ({} events)", substrates.len() + serventis.len())
            },
            Some(EventPattern::FailedExecution) => {
                "Task failed during execution".to_string()
            },
            Some(EventPattern::RetrySuccess) => {
                "Task succeeded after retry".to_string()
            },
            Some(EventPattern::RepeatedFailure) => {
                "Task failed multiple times".to_string()
            },
            Some(EventPattern::ResourceContention) => {
                "High resource contention detected".to_string()
            },
            Some(EventPattern::AnomalousBehavior) => {
                "Anomalous behavior detected - review required".to_string()
            },
            Some(EventPattern::PerformanceDegradation) => {
                "Performance degradation observed".to_string()
            },
            None => {
                format!("{} events in correlation window", substrates.len() + serventis.len())
            }
        }
    }
    
    /// Create pattern detectors
    fn create_pattern_detectors() -> Vec<Box<dyn PatternDetector>> {
        vec![
            Box::new(NormalExecutionDetector),
            Box::new(FailureDetector),
            Box::new(RetryDetector),
            Box::new(AnomalyDetector),
        ]
    }
    
    /// Get all correlated events
    pub async fn get_correlated_events(&self) -> Vec<CorrelatedEvent> {
        self.correlated_events.read().await.clone()
    }
    
    /// Get correlated events for a specific intent
    pub async fn get_intent_correlations(&self, intent_id: &IntentId) -> Vec<CorrelatedEvent> {
        self.correlated_events
            .read()
            .await
            .iter()
            .filter(|e| e.intent_id == *intent_id)
            .cloned()
            .collect()
    }
}

/// Trait for pattern detection
trait PatternDetector: Send + Sync {
    fn detect(&self, substrates: &[CollectedEvent], serventis: &[CollectedEvent]) -> Option<EventPattern>;
}

/// Detector for normal execution pattern
struct NormalExecutionDetector;
impl PatternDetector for NormalExecutionDetector {
    fn detect(&self, _substrates: &[CollectedEvent], serventis: &[CollectedEvent]) -> Option<EventPattern> {
        // Look for start -> success -> stop sequence in signals
        let has_start = serventis.iter().any(|e| {
            matches!(e, CollectedEvent::ServentisSignal { signal, .. } if signal.contains("Start"))
        });
        let has_success = serventis.iter().any(|e| {
            matches!(e, CollectedEvent::ServentisSignal { signal, .. } if signal.contains("Success"))
        });
        let has_stop = serventis.iter().any(|e| {
            matches!(e, CollectedEvent::ServentisSignal { signal, .. } if signal.contains("Stop"))
        });
        
        if has_start && has_success && has_stop {
            Some(EventPattern::NormalExecution)
        } else {
            None
        }
    }
}

/// Detector for failure patterns
struct FailureDetector;
impl PatternDetector for FailureDetector {
    fn detect(&self, _substrates: &[CollectedEvent], serventis: &[CollectedEvent]) -> Option<EventPattern> {
        let has_failure = serventis.iter().any(|e| {
            matches!(e, CollectedEvent::ServentisSignal { signal, .. } if signal.contains("Fail"))
        });
        
        if has_failure {
            Some(EventPattern::FailedExecution)
        } else {
            None
        }
    }
}

/// Detector for retry patterns
struct RetryDetector;
impl PatternDetector for RetryDetector {
    fn detect(&self, _substrates: &[CollectedEvent], serventis: &[CollectedEvent]) -> Option<EventPattern> {
        let has_retry = serventis.iter().any(|e| {
            matches!(e, CollectedEvent::ServentisSignal { signal, .. } if signal.contains("Retry"))
        });
        
        if has_retry {
            // Check if retry was successful
            let retry_index = serventis.iter().position(|e| {
                matches!(e, CollectedEvent::ServentisSignal { signal, .. } if signal.contains("Retry"))
            });
            
            if let Some(idx) = retry_index {
                let has_success_after = serventis[idx..].iter().any(|e| {
                    matches!(e, CollectedEvent::ServentisSignal { signal, .. } if signal.contains("Success"))
                });
                
                if has_success_after {
                    Some(EventPattern::RetrySuccess)
                } else {
                    Some(EventPattern::RepeatedFailure)
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Detector for anomalous behavior
struct AnomalyDetector;
impl PatternDetector for AnomalyDetector {
    fn detect(&self, substrates: &[CollectedEvent], _serventis: &[CollectedEvent]) -> Option<EventPattern> {
        // Look for anomaly events in Substrates events
        let has_anomaly = substrates.iter().any(|e| {
            matches!(e, CollectedEvent::SubstratesEvent { data, .. } if data.get("anomaly").is_some())
        });
        
        if has_anomaly {
            Some(EventPattern::AnomalousBehavior)
        } else {
            None
        }
    }
}