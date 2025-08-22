//! Substrates Observability Example
//! 
//! This example demonstrates the Humainary Substrates-inspired observability
//! framework implementation in Rust, showing proper event flow patterns.

use anyhow::Result;
use async_trait::async_trait;
use synapsed_substrates::{
    BasicCircuit, BasicChannel, Subject, Emission, Pipe,
    Channel, Circuit, Subscriber, Subscription,
    ManagedSource, ManagedSubscription,
    Queue, QueueItem, Priority as QueuePriority, Script,
    BasicSink, FilteredSink, BatchingSink, Sink,
    ValueComposer, FunctionComposer, Composer,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn};
use uuid::Uuid;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();

    info!("Starting Substrates Observability Example");
    
    // Example 1: Basic emission flow (Subject → Channel → Pipe → Emission)
    basic_emission_flow().await?;
    
    // Example 2: Subscription model with managed sources
    subscription_model().await?;
    
    // Example 3: Queue and Script execution
    queue_execution().await?;
    
    // Example 4: Sink patterns for collecting emissions
    sink_patterns().await?;
    
    // Example 5: Percepts with Composers
    percept_example().await?;
    
    // Example 6: Complex circuit with multiple channels
    complex_circuit().await?;
    
    info!("All examples completed successfully!");
    Ok(())
}

/// Example 1: Basic emission flow
async fn basic_emission_flow() -> Result<()> {
    info!("=== Example 1: Basic Emission Flow ===");
    info!("Pattern: Subject → Channel → Pipe → Emission");
    
    // Create a subject (the observable entity)
    let subject = Subject::new("user", "login");
    info!("Created subject: {:?}", subject);
    
    // Create a channel for this subject
    let channel: Arc<dyn Channel<LoginEvent>> = Arc::new(BasicChannel::new(subject.clone()));
    info!("Created channel for subject");
    
    // Create a pipe from the channel (emissions flow through pipes)
    let pipe = channel.create_pipe("auth_events");
    info!("Created pipe 'auth_events' from channel");
    
    // Emit events through the pipe (NOT directly from subject!)
    let event = LoginEvent {
        user_id: "user123".to_string(),
        timestamp: chrono::Utc::now(),
        success: true,
    };
    
    pipe.emit(Emission::new(event.clone(), subject.clone()));
    info!("✓ Emitted login event through pipe");
    
    // Wrong pattern (for demonstration - this would be incorrect):
    // subject.emit(event) // ✗ WRONG - subjects don't emit directly!
    
    Ok(())
}

/// Example 2: Subscription model
async fn subscription_model() -> Result<()> {
    info!("=== Example 2: Subscription Model ===");
    
    // Create a managed source that handles subscriptions
    let source = Arc::new(ManagedSource::<MetricEvent>::new("metrics_source"));
    
    // Create a subscriber
    let subscriber = Arc::new(MetricsSubscriber::new("dashboard"));
    
    // Subscribe to the source
    let subscription = source.subscribe(subscriber.clone()).await?;
    info!("Created subscription: {}", subscription.id());
    
    // Create channels and register with subscription
    let cpu_subject = Subject::new("system", "cpu");
    let cpu_channel = Arc::new(BasicChannel::new(cpu_subject.clone()));
    subscription.register_channel("cpu_metrics", cpu_channel.clone()).await?;
    
    let memory_subject = Subject::new("system", "memory");
    let memory_channel = Arc::new(BasicChannel::new(memory_subject.clone()));
    subscription.register_channel("memory_metrics", memory_channel.clone()).await?;
    
    info!("Registered 2 channels with subscription");
    
    // Emit metrics through the channels
    let cpu_pipe = cpu_channel.create_pipe("cpu_monitor");
    cpu_pipe.emit(Emission::new(
        MetricEvent { name: "cpu_usage".to_string(), value: 45.2 },
        cpu_subject
    ));
    
    let memory_pipe = memory_channel.create_pipe("memory_monitor");
    memory_pipe.emit(Emission::new(
        MetricEvent { name: "memory_usage".to_string(), value: 62.8 },
        memory_subject
    ));
    
    info!("✓ Emitted metrics through subscribed channels");
    
    Ok(())
}

/// Example 3: Queue and Script execution
async fn queue_execution() -> Result<()> {
    info!("=== Example 3: Queue and Script Execution ===");
    
    // Create a queue for processing events
    let mut queue = Queue::new();
    
    // Add items with different priorities
    queue.enqueue(QueueItem::new(
        "high_priority_task",
        serde_json::json!({"action": "alert", "severity": "critical"}),
        QueuePriority::Critical
    ));
    
    queue.enqueue(QueueItem::new(
        "normal_task",
        serde_json::json!({"action": "log", "level": "info"}),
        QueuePriority::Normal
    ));
    
    queue.enqueue(QueueItem::new(
        "low_priority_task",
        serde_json::json!({"action": "cleanup", "target": "cache"}),
        QueuePriority::Low
    ));
    
    info!("Enqueued 3 items with different priorities");
    
    // Create a script to process the queue
    let script = Script::new("process_queue", |item| {
        info!("Processing: {} (priority: {:?})", item.id, item.priority);
        Ok(serde_json::json!({"processed": item.id}))
    });
    
    // Execute script on queue items (processes in priority order)
    while let Some(item) = queue.dequeue() {
        let result = script.execute(item)?;
        debug!("Script result: {:?}", result);
    }
    
    info!("✓ Queue processed in priority order");
    
    Ok(())
}

/// Example 4: Sink patterns
async fn sink_patterns() -> Result<()> {
    info!("=== Example 4: Sink Patterns ===");
    
    // Create different types of sinks
    
    // 1. Basic sink - collects all emissions
    let mut basic_sink = BasicSink::<LogEvent>::new(100);
    basic_sink.collect(LogEvent {
        level: "INFO".to_string(),
        message: "Application started".to_string(),
    });
    basic_sink.collect(LogEvent {
        level: "WARN".to_string(),
        message: "High memory usage".to_string(),
    });
    
    let basic_items = basic_sink.drain();
    info!("Basic sink collected {} items", basic_items.len());
    
    // 2. Filtered sink - only collects matching emissions
    let mut filtered_sink = FilteredSink::new(50, |event: &LogEvent| {
        event.level == "ERROR" || event.level == "WARN"
    });
    
    filtered_sink.collect(LogEvent {
        level: "INFO".to_string(),
        message: "Normal operation".to_string(),
    });
    filtered_sink.collect(LogEvent {
        level: "ERROR".to_string(),
        message: "Connection failed".to_string(),
    });
    
    let filtered_items = filtered_sink.drain();
    info!("Filtered sink collected {} error/warn items", filtered_items.len());
    
    // 3. Batching sink - groups emissions into batches
    let mut batching_sink = BatchingSink::<LogEvent>::new(10, Duration::from_secs(5));
    
    for i in 0..5 {
        batching_sink.collect(LogEvent {
            level: "INFO".to_string(),
            message: format!("Event {}", i),
        });
    }
    
    if batching_sink.should_flush() {
        let batch = batching_sink.flush();
        info!("Batching sink flushed {} items", batch.len());
    }
    
    info!("✓ All sink patterns demonstrated");
    
    Ok(())
}

/// Example 5: Percepts with Composers
async fn percept_example() -> Result<()> {
    info!("=== Example 5: Percepts with Composers ===");
    
    // Create a channel
    let subject = Subject::new("sensor", "temperature");
    let channel: Arc<dyn Channel<f64>> = Arc::new(BasicChannel::new(subject.clone()));
    
    // Create a value composer that wraps the channel in a percept
    let composer = ValueComposer::new(|ch: Arc<dyn Channel<f64>>| {
        TemperaturePercept {
            channel: ch,
            unit: "celsius".to_string(),
            threshold: 25.0,
        }
    });
    
    // Compose the percept
    let percept = composer.compose(channel.clone());
    info!("Created temperature percept with threshold: {}", percept.threshold);
    
    // Use the percept
    percept.record_temperature(23.5);
    percept.record_temperature(26.8);
    percept.record_temperature(24.1);
    
    // Create a function composer for more complex percepts
    let function_composer = FunctionComposer::new(Arc::new(|ch| {
        info!("Function composer creating complex percept");
        ComplexPercept {
            channel: ch,
            processed_count: Arc::new(RwLock::new(0)),
        }
    }));
    
    let complex_percept = function_composer.compose(channel);
    info!("✓ Created percepts using composers");
    
    Ok(())
}

/// Example 6: Complex circuit with multiple channels
async fn complex_circuit() -> Result<()> {
    info!("=== Example 6: Complex Circuit ===");
    
    // Create a circuit (the computational network)
    let circuit = Arc::new(BasicCircuit::new("monitoring_circuit"));
    
    // Add multiple channels for different aspects
    let channels = vec![
        ("api", "requests"),
        ("database", "queries"),
        ("cache", "operations"),
        ("queue", "messages"),
    ];
    
    for (context, name) in channels {
        let subject = Subject::new(context, name);
        let channel: Arc<dyn Channel<String>> = Arc::new(BasicChannel::new(subject.clone()));
        circuit.add_channel(channel.clone());
        
        // Create pipes and emit sample events
        let pipe = channel.create_pipe(&format!("{}_pipe", name));
        pipe.emit(Emission::new(
            format!("{} event from {}", name, context),
            subject
        ));
    }
    
    // Get circuit statistics
    let stats = circuit.get_statistics();
    info!("Circuit statistics:");
    info!("  - Active channels: {}", stats.channels_count);
    info!("  - Total emissions: {}", stats.total_emissions);
    info!("  - Circuit uptime: {:?}", stats.uptime);
    
    // Demonstrate circuit-wide operations
    circuit.pause().await;
    info!("Circuit paused");
    
    circuit.resume().await;
    info!("Circuit resumed");
    
    info!("✓ Complex circuit demonstrated");
    
    Ok(())
}

// === Helper Types ===

#[derive(Clone, Debug)]
struct LoginEvent {
    user_id: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    success: bool,
}

#[derive(Clone, Debug)]
struct MetricEvent {
    name: String,
    value: f64,
}

#[derive(Clone, Debug)]
struct LogEvent {
    level: String,
    message: String,
}

// === Custom Subscriber ===

struct MetricsSubscriber {
    name: String,
    received_count: Arc<RwLock<usize>>,
}

impl MetricsSubscriber {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            received_count: Arc::new(RwLock::new(0)),
        }
    }
}

#[async_trait]
impl Subscriber for MetricsSubscriber {
    type Emission = MetricEvent;
    
    async fn on_emission(&self, emission: &Emission<Self::Emission>) {
        let mut count = self.received_count.write().await;
        *count += 1;
        debug!("{} received metric: {:?}", self.name, emission.data());
    }
    
    fn id(&self) -> String {
        format!("subscriber_{}", self.name)
    }
}

// === Custom Percepts ===

struct TemperaturePercept {
    channel: Arc<dyn Channel<f64>>,
    unit: String,
    threshold: f64,
}

impl TemperaturePercept {
    fn record_temperature(&self, value: f64) {
        let pipe = self.channel.create_pipe("temp_recorder");
        let subject = Subject::new("sensor", "temperature");
        
        if value > self.threshold {
            warn!("Temperature {} {} exceeds threshold {}", 
                value, self.unit, self.threshold);
        }
        
        pipe.emit(Emission::new(value, subject));
    }
}

struct ComplexPercept {
    channel: Arc<dyn Channel<f64>>,
    processed_count: Arc<RwLock<usize>>,
}