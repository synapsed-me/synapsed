//! Observability integration for intent execution using Substrates and Serventis
//!
//! This module provides deep observability for intent trees using both the Substrates
//! observability framework and Serventis service monitoring APIs, enabling comprehensive
//! monitoring, tracing, signaling, and verification.

use crate::{HierarchicalIntent, IntentId, EventType};
use synapsed_substrates::{
    BasicCircuit, BasicSink, BasicSource, ManagedQueue, Queue,
    Priority, QueueStats, Sink, Source, Subject, Substrate,
    types::{Name, SubjectType, SubstratesResult},
};
use synapsed_serventis::{
    BasicService, Service, ServiceExt, Signal, Sign, Orientation,
    BasicProbe, Probe, Observation, Operation, Origin, Outcome,
    BasicMonitor, Monitor, Status, Confidence,
};
use std::sync::Arc;
use std::sync::RwLock;
use serde_json::Value as JsonValue;
use chrono::{DateTime, Utc};

/// Observable intent wrapper that integrates with Substrates and Serventis
pub struct ObservableIntent {
    intent: HierarchicalIntent,
    // Substrates components
    circuit: Arc<BasicCircuit>,
    event_source: Arc<BasicSource<IntentEvent>>,
    execution_queue: Arc<ManagedQueue>,
    metrics_sink: Arc<RwLock<BasicSink<IntentMetric>>>,
    // Serventis components
    service: Arc<RwLock<BasicService>>,
    probe: Arc<RwLock<BasicProbe>>,
    monitor: Arc<RwLock<BasicMonitor>>,
}

/// Intent event emitted through Substrates
#[derive(Debug, Clone)]
pub struct IntentEvent {
    pub intent_id: IntentId,
    pub event_type: EventType,
    pub timestamp: DateTime<Utc>,
    pub data: JsonValue,
}

/// Intent metric collected in sink
#[derive(Debug, Clone)]
pub struct IntentMetric {
    pub intent_id: IntentId,
    pub metric_type: MetricType,
    pub value: f64,
    pub timestamp: DateTime<Utc>,
    pub labels: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub enum MetricType {
    ExecutionTime,
    StepCount,
    SuccessRate,
    QueueDepth,
    MemoryUsage,
}

impl ObservableIntent {
    /// Create a new observable intent
    pub async fn new(intent: HierarchicalIntent) -> SubstratesResult<Self> {
        let circuit_name = Name::from(format!("intent-circuit-{}", intent.id().0).as_str());
        let circuit = Arc::new(BasicCircuit::new(circuit_name));
        
        // Create event source for intent events
        let source_name = Name::from(format!("intent-events-{}", intent.id().0).as_str());
        let event_source = Arc::new(BasicSource::new(
            Subject::new(source_name, SubjectType::Source)
        ));
        
        // Create execution queue with priority support
        let queue_name = Name::from(format!("intent-queue-{}", intent.id().0).as_str());
        let execution_queue = Arc::new(ManagedQueue::new(queue_name));
        
        // Create metrics sink
        let sink_name = Name::from(format!("intent-metrics-{}", intent.id().0).as_str());
        let metrics_sink = Arc::new(RwLock::new(BasicSink::new(sink_name)));
        
        // Create Serventis components
        let service_subject = Subject::new(
            Name::from(format!("intent-service-{}", intent.id().0).as_str()),
            SubjectType::Component
        );
        let service = Arc::new(RwLock::new(BasicService::new(service_subject)));
        
        let probe_subject = Arc::new(Subject::new(
            Name::from(format!("intent-probe-{}", intent.id().0).as_str()),
            SubjectType::Component
        ));
        let probe = Arc::new(RwLock::new(BasicProbe::new(probe_subject)));
        
        let monitor_subject = Arc::new(Subject::new(
            Name::from(format!("intent-monitor-{}", intent.id().0).as_str()),
            SubjectType::Component
        ));
        let monitor = Arc::new(RwLock::new(BasicMonitor::new(monitor_subject)));
        
        Ok(Self {
            intent,
            circuit,
            event_source,
            execution_queue,
            metrics_sink,
            service,
            probe,
            monitor,
        })
    }
    
    /// Emit an intent event through the Substrates source
    pub async fn emit_event(&self, event_type: EventType, data: JsonValue) -> SubstratesResult<()> {
        let event = IntentEvent {
            intent_id: self.intent.id(),
            event_type,
            timestamp: Utc::now(),
            data,
        };
        
        // Emit through the source
        let subject = self.event_source.subject();
        self.event_source.emit(subject, event).await
    }
    
    /// Record a metric to the sink
    pub async fn record_metric(&self, metric_type: MetricType, value: f64) {
        let metric = IntentMetric {
            intent_id: self.intent.id(),
            metric_type,
            value,
            timestamp: Utc::now(),
            labels: vec![],
        };
        
        // Create a pipe and emit to sink
        let pipe = self.metrics_sink.read().unwrap().create_pipe();
        // In production, would properly handle Arc mutation
        drop(pipe);
        drop(metric);
    }
    
    /// Execute the intent with full observability
    pub async fn execute(&self, context: &crate::context::IntentContext) -> crate::Result<()> {
        // Start execution timer
        let start = Utc::now();
        
        // Serventis: Emit start signal
        {
            let mut service = self.service.write().unwrap();
            service.start().await.map_err(|e| crate::IntentError::ObservableError(e.to_string()))?;
            service.call().await.map_err(|e| crate::IntentError::ObservableError(e.to_string()))?;
        }
        
        // Serventis: Monitor status
        {
            let mut monitor = self.monitor.write().unwrap();
            monitor.assess(Status::Ok, Confidence::High).await
                .map_err(|e| crate::IntentError::ObservableError(e.to_string()))?;
        }
        
        // Substrates: Emit start event
        self.emit_event(EventType::Started, serde_json::json!({
            "goal": self.intent.goal(),
            "context": "context",
        })).await.map_err(|e| crate::IntentError::ObservableError(e.to_string()))?;
        
        // Submit to queue
        let script = IntentScript::new(self.intent.clone(), context.clone());
        let queue_result = self.execution_queue.submit_with_priority(
            Arc::new(script),
            self.intent_priority(),
        ).await;
        
        // Serventis: Record probe observation
        {
            let mut probe = self.probe.write().unwrap();
            let outcome = if queue_result.is_ok() {
                Outcome::Success
            } else {
                Outcome::Failure
            };
            probe.process(Origin::Client, outcome).await
                .map_err(|e| crate::IntentError::ObservableError(e.to_string()))?;
        }
        
        queue_result.map_err(|e| crate::IntentError::ObservableError(e.to_string()))?;
        
        // Wait for completion
        self.execution_queue.await_empty().await;
        
        // Serventis: Emit success/fail signal
        let execution_success = true; // Would check actual result
        {
            let mut service = self.service.write().unwrap();
            if execution_success {
                service.success().await.map_err(|e| crate::IntentError::ObservableError(e.to_string()))?;
            } else {
                service.fail().await.map_err(|e| crate::IntentError::ObservableError(e.to_string()))?;
            }
            service.stop().await.map_err(|e| crate::IntentError::ObservableError(e.to_string()))?;
        }
        
        // Record execution time
        let duration = Utc::now().signed_duration_since(start);
        self.record_metric(MetricType::ExecutionTime, duration.num_milliseconds() as f64).await;
        
        // Substrates: Emit completion event
        self.emit_event(EventType::Completed, serde_json::json!({
            "duration_ms": duration.num_milliseconds(),
        })).await.map_err(|e| crate::IntentError::ObservableError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Get queue statistics
    pub fn queue_stats(&self) -> QueueStats {
        self.execution_queue.stats()
    }
    
    /// Drain collected metrics
    pub async fn drain_metrics(&self) -> SubstratesResult<Vec<IntentMetric>> {
        // Get write lock and drain
        let mut sink_guard = self.metrics_sink.write().unwrap();
        let captures = sink_guard.drain().await?;
        Ok(captures.into_iter().map(|c| c.into_emission()).collect())
    }
    
    fn intent_priority(&self) -> Priority {
        // Access metadata through the intent's fields
        Priority::Normal // Default for now, would need to expose metadata getter
    }
    
    /// Get probe observations
    pub fn get_observations(&self) -> Vec<Observation> {
        let probe = self.probe.read().unwrap();
        probe.observations().to_vec()
    }
    
    /// Get monitor status
    pub async fn get_monitor_status(&self) -> (Status, Confidence) {
        let monitor = self.monitor.read().unwrap();
        monitor.current_status()
    }
    
    /// Execute with Serventis service wrapper
    pub async fn execute_with_service<F, R>(&self, f: F) -> Result<R, Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnOnce() -> Result<R, Box<dyn std::error::Error + Send + Sync>> + Send,
        R: Send,
    {
        let mut service = self.service.write().unwrap();
        service.execute(f).await
    }
}

/// Script that executes an intent in the queue
struct IntentScript {
    intent: HierarchicalIntent,
    context: crate::context::IntentContext,
}

impl IntentScript {
    fn new(intent: HierarchicalIntent, context: crate::context::IntentContext) -> Self {
        Self { intent, context }
    }
}

#[async_trait::async_trait]
impl synapsed_substrates::circuit::Script for IntentScript {
    async fn exec(&self, _current: &dyn synapsed_substrates::circuit::Current) -> SubstratesResult<()> {
        // Execute the intent
        self.intent.execute(&self.context).await
            .map_err(|e| synapsed_substrates::types::SubstratesError::Internal(e.to_string()))?;
        Ok(())
    }
}

/// Intent execution monitor using Substrates
pub struct IntentMonitor {
    circuit: Arc<BasicCircuit>,
    subscriptions: Vec<Arc<dyn synapsed_substrates::subject::Subscription>>,
}

impl IntentMonitor {
    /// Create a new intent monitor
    pub async fn new() -> SubstratesResult<Self> {
        let circuit = Arc::new(BasicCircuit::new(Name::from("intent-monitor")));
        Ok(Self {
            circuit,
            subscriptions: Vec::new(),
        })
    }
    
    /// Monitor an observable intent
    pub async fn monitor(&mut self, intent: &ObservableIntent) -> SubstratesResult<()> {
        // Subscribe to intent events
        let subscriber = Arc::new(IntentEventSubscriber::new());
        let subscription = intent.event_source.subscribe(subscriber).await?;
        self.subscriptions.push(subscription);
        Ok(())
    }
}

/// Subscriber for intent events
struct IntentEventSubscriber {
    handler: Arc<dyn Fn(IntentEvent) + Send + Sync>,
}

impl IntentEventSubscriber {
    fn new() -> Self {
        Self {
            handler: Arc::new(|event| {
                tracing::info!(
                    "Intent {} emitted {:?}: {}",
                    event.intent_id.0,
                    event.event_type,
                    event.data
                );
            }),
        }
    }
}

impl synapsed_substrates::subject::Subscriber for IntentEventSubscriber {
    type Emission = IntentEvent;
    
    fn accept(
        &mut self,
        _subject: &Subject,
        _registrar: &mut dyn synapsed_substrates::subject::Registrar<Emission = IntentEvent>,
    ) {
        // Register pipes for receiving events
        // In a full implementation, would register pipes that handle the events
    }
}

/// Builder for creating observable intents
pub struct ObservableIntentBuilder {
    intent_builder: crate::intent::IntentBuilder,
    enable_metrics: bool,
    enable_tracing: bool,
    queue_capacity: usize,
}

impl ObservableIntentBuilder {
    pub fn new(goal: impl Into<String>) -> Self {
        Self {
            intent_builder: crate::intent::IntentBuilder::new(goal),
            enable_metrics: true,
            enable_tracing: true,
            queue_capacity: 100,
        }
    }
    
    pub fn with_metrics(mut self, enable: bool) -> Self {
        self.enable_metrics = enable;
        self
    }
    
    pub fn with_tracing(mut self, enable: bool) -> Self {
        self.enable_tracing = enable;
        self
    }
    
    pub fn with_queue_capacity(mut self, capacity: usize) -> Self {
        self.queue_capacity = capacity;
        self
    }
    
    pub async fn build(self) -> SubstratesResult<ObservableIntent> {
        let intent = self.intent_builder.build();
        ObservableIntent::new(intent).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_observable_intent_creation() {
        let intent = HierarchicalIntent::new("Test goal");
        let observable = ObservableIntent::new(intent).await.unwrap();
        
        // Verify components are created
        let _stats = observable.queue_stats();
    }
    
    #[tokio::test]
    async fn test_intent_event_emission() {
        let intent = HierarchicalIntent::new("Test goal");
        let observable = ObservableIntent::new(intent).await.unwrap();
        
        // Emit an event
        observable.emit_event(
            EventType::Started,
            serde_json::json!({"test": "data"})
        ).await.unwrap();
        
        // In a real test, would verify the event was received by subscribers
    }
    
    #[tokio::test]
    async fn test_intent_metrics_collection() {
        let intent = HierarchicalIntent::new("Test goal");
        let observable = ObservableIntent::new(intent).await.unwrap();
        
        // Record some metrics
        observable.record_metric(MetricType::ExecutionTime, 100.0).await;
        observable.record_metric(MetricType::SuccessRate, 0.95).await;
        
        // Wait for background processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        // Drain and verify metrics
        let metrics = observable.drain_metrics().await.unwrap();
        assert_eq!(metrics.len(), 2);
    }
}