//! Test-Driven Development for Substrate Runtime Integration with Serventis
//!
//! This test suite follows TDD methodology by implementing failing tests first,
//! then implementing the minimal code to make them pass.

use synapsed_substrates::*;
use synapsed_serventis::*;
use tokio::time::{sleep, Duration};
use std::sync::Arc;
use uuid::Uuid;

/// Test suite for substrate circuit integration with serventis observability
#[cfg(test)]
mod runtime_integration_tests {
    use super::*;

    /// Test 1: Circuit creation should automatically register observability hooks
    /// RED PHASE: This test will fail because ObservableCircuit doesn't exist yet
    #[tokio::test]
    async fn test_circuit_creates_with_observability_hooks() {
        // Arrange
        let cortex = create_cortex();
        let circuit_name = Name::from("test-circuit");
        
        // Act - This should fail because ObservableCircuit doesn't exist
        let observable_circuit = ObservableCircuit::new(&cortex, circuit_name.clone()).await;
        
        // Assert - Circuit should be created with observability enabled
        assert!(observable_circuit.is_ok());
        let circuit = observable_circuit.unwrap();
        assert!(circuit.has_observability_enabled());
        assert_eq!(circuit.name(), &circuit_name);
        
        // Circuit should have serventis monitors attached
        let monitors = circuit.get_monitors().await;
        assert!(!monitors.is_empty());
        assert!(monitors.iter().any(|m| m.monitor_type() == MonitorSignal::Confidence));
    }

    /// Test 2: Circuit execution should emit serventis signals
    /// RED PHASE: This test will fail because circuit execution doesn't emit signals yet
    #[tokio::test]
    async fn test_circuit_execution_emits_serventis_signals() {
        // Arrange
        let cortex = create_cortex();
        let circuit = ObservableCircuit::new(&cortex, Name::from("execution-test")).await.unwrap();
        let mut signal_receiver = circuit.get_signal_receiver().await;
        
        // Create a simple pipe that processes data
        let test_pipe = FunctionPipe::new(|value: i32| async move { value * 2 });
        circuit.add_pipe("doubler", test_pipe).await;
        
        // Act - Execute circuit with input data
        let input_data = 42;
        circuit.execute_with_input("doubler", input_data).await.unwrap();
        
        // Assert - Should receive execution signals
        let signal = tokio::time::timeout(Duration::from_millis(100), signal_receiver.recv())
            .await
            .expect("Should receive signal within timeout")
            .expect("Signal should be present");
            
        match signal {
            ServentisSignal::Service(service_signal) => {
                assert_eq!(service_signal.service_name(), "execution-test");
                assert_eq!(service_signal.operation(), "execute_with_input");
                assert!(service_signal.is_successful());
            }
            _ => panic!("Expected ServiceSignal but got {:?}", signal),
        }
    }

    /// Test 3: Circuit errors should be reported through serventis probes
    /// RED PHASE: This test will fail because error reporting doesn't exist yet
    #[tokio::test]
    async fn test_circuit_errors_reported_through_probes() {
        // Arrange
        let cortex = create_cortex();
        let circuit = ObservableCircuit::new(&cortex, Name::from("error-test")).await.unwrap();
        let mut probe_receiver = circuit.get_probe_receiver().await;
        
        // Create a pipe that always fails
        let failing_pipe = FunctionPipe::new(|_value: i32| async move { 
            Err(SubstratesError::RuntimeError("Intentional test failure".to_string()))
        });
        circuit.add_pipe("failer", failing_pipe).await;
        
        // Act - Execute circuit which should fail
        let result = circuit.execute_with_input("failer", 42).await;
        
        // Assert - Execution should fail and probe should report it
        assert!(result.is_err());
        
        let probe_signal = tokio::time::timeout(Duration::from_millis(100), probe_receiver.recv())
            .await
            .expect("Should receive probe signal within timeout")
            .expect("Probe signal should be present");
            
        match probe_signal {
            ProbeSignal::Outcome(outcome) => {
                assert_eq!(outcome.operation(), "execute_with_input");
                assert!(outcome.is_failure());
                assert!(outcome.error_message().contains("Intentional test failure"));
            }
            _ => panic!("Expected ProbeSignal::Outcome but got {:?}", probe_signal),
        }
    }

    /// Test 4: Multiple circuits should coordinate through shared observability
    /// RED PHASE: This test will fail because circuit coordination doesn't exist yet
    #[tokio::test]
    async fn test_multi_circuit_coordination() {
        // Arrange
        let cortex = create_cortex();
        let circuit_a = ObservableCircuit::new(&cortex, Name::from("circuit-a")).await.unwrap();
        let circuit_b = ObservableCircuit::new(&cortex, Name::from("circuit-b")).await.unwrap();
        
        // Set up coordination between circuits
        let coordination_channel = circuit_a.create_coordination_channel("circuit-b").await.unwrap();
        circuit_b.connect_coordination_channel(coordination_channel).await.unwrap();
        
        // Add processing pipes
        let pipe_a = FunctionPipe::new(|value: i32| async move { value + 10 });
        let pipe_b = FunctionPipe::new(|value: i32| async move { value * 3 });
        
        circuit_a.add_pipe("adder", pipe_a).await;
        circuit_b.add_pipe("multiplier", pipe_b).await;
        
        // Act - Execute coordinated processing
        let initial_value = 5;
        let intermediate_result = circuit_a.execute_with_input("adder", initial_value).await.unwrap();
        let final_result = circuit_b.execute_with_input("multiplier", intermediate_result).await.unwrap();
        
        // Assert - Both circuits should have executed and reported coordination
        assert_eq!(final_result, 45); // (5 + 10) * 3 = 45
        
        // Check coordination was reported
        let coordination_events = cortex.get_coordination_events().await;
        assert_eq!(coordination_events.len(), 2); // One event per circuit execution
        
        let event_a = &coordination_events[0];
        let event_b = &coordination_events[1];
        
        assert_eq!(event_a.source_circuit(), "circuit-a");
        assert_eq!(event_b.source_circuit(), "circuit-b");
        assert!(event_b.depends_on(&event_a.id()));
    }

    /// Test 5: Performance metrics should be collected during circuit execution
    /// RED PHASE: This test will fail because performance metrics collection doesn't exist yet
    #[tokio::test]
    async fn test_performance_metrics_collection() {
        // Arrange
        let cortex = create_cortex();
        let circuit = ObservableCircuit::new(&cortex, Name::from("perf-test")).await.unwrap();
        
        // Add a pipe with artificial delay to measure performance
        let slow_pipe = FunctionPipe::new(|value: i32| async move {
            sleep(Duration::from_millis(50)).await;
            value * 2
        });
        circuit.add_pipe("slow_processor", slow_pipe).await;
        
        // Act - Execute circuit multiple times
        for i in 0..5 {
            circuit.execute_with_input("slow_processor", i).await.unwrap();
        }
        
        // Assert - Performance metrics should be available
        let metrics = circuit.get_performance_metrics().await;
        assert!(metrics.is_some());
        
        let perf_data = metrics.unwrap();
        assert_eq!(perf_data.execution_count(), 5);
        assert!(perf_data.average_execution_time() >= Duration::from_millis(50));
        assert!(perf_data.total_execution_time() >= Duration::from_millis(250));
        
        // Metrics should also be available through serventis reporters
        let reporter_metrics = circuit.get_reporter_metrics().await;
        assert!(reporter_metrics.contains_key("execution_count"));
        assert!(reporter_metrics.contains_key("average_latency"));
        assert!(reporter_metrics.contains_key("total_processing_time"));
    }
}

// Additional types and traits that need to be implemented (currently don't exist)

/// Observable circuit that integrates substrate circuits with serventis observability
pub struct ObservableCircuit {
    inner_circuit: Arc<dyn Circuit>,
    name: Name,
    monitors: Vec<Arc<dyn Monitor>>,
    signal_sender: mpsc::UnboundedSender<ServentisSignal>,
    probe_sender: mpsc::UnboundedSender<ProbeSignal>,
}

/// Serventis signal types for substrate integration
#[derive(Debug, Clone)]
pub enum ServentisSignal {
    Service(ServiceSignal),
    Monitor(MonitorSignal),
    Resource(ResourceSignal),
    Queue(QueueSignal),
}

/// Probe signal for error and outcome reporting
#[derive(Debug, Clone)]
pub enum ProbeSignal {
    Outcome(ProbeOutcome),
    Communication(CommunicationProbe),
}

/// Service signal for tracking circuit operations
#[derive(Debug, Clone)]
pub struct ServiceSignal {
    service_name: String,
    operation: String,
    successful: bool,
    timestamp: chrono::DateTime<chrono::Utc>,
}

/// Monitor signal for confidence-based reporting
#[derive(Debug, Clone)]
pub enum MonitorSignal {
    Confidence,
    Health,
    Performance,
}

/// Resource signal for substrate resource interactions
#[derive(Debug, Clone)]
pub struct ResourceSignal {
    resource_id: String,
    interaction_type: String,
    outcome: bool,
}

/// Queue signal for substrate queue operations
#[derive(Debug, Clone)]
pub struct QueueSignal {
    queue_id: String,
    operation: String,
    size: usize,
    health: QueueHealth,
}

/// Queue health assessment
#[derive(Debug, Clone)]
pub enum QueueHealth {
    Healthy,
    Degraded,
    Critical,
}

/// Probe outcome for error reporting
#[derive(Debug, Clone)]
pub struct ProbeOutcome {
    operation: String,
    successful: bool,
    error_message: Option<String>,
    timestamp: chrono::DateTime<chrono::Utc>,
}

/// Communication probe for distributed system monitoring
#[derive(Debug, Clone)]
pub struct CommunicationProbe {
    source: String,
    target: String,
    outcome: bool,
    latency: Duration,
}

/// Coordination event for multi-circuit tracking
#[derive(Debug, Clone)]
pub struct CoordinationEvent {
    id: Uuid,
    source_circuit: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    dependencies: Vec<Uuid>,
}

/// Performance metrics for circuit execution
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    execution_count: usize,
    total_execution_time: Duration,
    average_execution_time: Duration,
    min_execution_time: Duration,
    max_execution_time: Duration,
}

// Trait definitions that need to be implemented

/// Monitor trait for observability hooks
pub trait Monitor: Send + Sync {
    fn monitor_type(&self) -> MonitorSignal;
    async fn assess_condition(&self) -> f64; // confidence level 0.0 to 1.0
}

/// Extension trait for cortex coordination
pub trait CortexCoordination {
    async fn get_coordination_events(&self) -> Vec<CoordinationEvent>;
    async fn register_coordination_event(&self, event: CoordinationEvent);
}