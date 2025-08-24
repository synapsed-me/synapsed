//! Simple recovery integration example
//! 
//! This example demonstrates how to integrate the recovery system
//! without all the complex swarm coordinator dependencies.

use synapsed_swarm::recovery::*;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tracing::{info, warn, error};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("🚀 Simple Recovery Integration Demo");

    // Create recovery manager
    let recovery_manager = RecoveryManager::new();

    // Start background monitoring
    recovery_manager.start_monitoring().await;
    info!("✅ Recovery system monitoring started");

    // Add a custom recovery strategy
    let custom_strategy = Arc::new(CustomRecoveryStrategy::new());
    recovery_manager.add_strategy(custom_strategy).await;
    info!("✅ Custom recovery strategy added");

    // Simulate some errors and recovery attempts
    simulate_error_scenarios(&recovery_manager).await?;

    // Show final statistics
    show_recovery_statistics(&recovery_manager).await;

    Ok(())
}

async fn simulate_error_scenarios(recovery_manager: &RecoveryManager) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🔥 Simulating Error Scenarios");

    // Create a mock coordinator context
    let mock_coordinator = Arc::new(MockCoordinator::new());

    let error_scenarios = vec![
        ("Network failure", synapsed_swarm::SwarmError::CommunicationError("Connection timeout".to_string())),
        ("Resource exhaustion", synapsed_swarm::SwarmError::SwarmSizeLimitExceeded { current: 10, max: 5 }),
        ("Coordination timeout", synapsed_swarm::SwarmError::CoordinationTimeout(30)),
        ("Custom error", synapsed_swarm::SwarmError::Other(anyhow::anyhow!("CustomTestError: Simulation"))),
        ("Concurrency issue", synapsed_swarm::SwarmError::ConcurrencyError("Race condition detected".to_string())),
    ];

    for (scenario_name, error) in error_scenarios {
        info!("\n--- {} ---", scenario_name);
        info!("Error: {}", error);

        let task_id = Some(Uuid::new_v4());
        let agent_id = Some(Uuid::new_v4());

        let start_time = std::time::Instant::now();
        
        match recovery_manager.recover(mock_coordinator.clone(), error, task_id, agent_id).await {
            Ok(result) => {
                let duration = start_time.elapsed();
                if result.success {
                    info!("✅ Recovery successful in {:?}", duration);
                    info!("   Action: {}", result.action_taken);
                    info!("   Confidence: {:.2}", result.confidence);
                } else {
                    warn!("⚠️  Recovery attempted but failed");
                    warn!("   Action: {}", result.action_taken);
                }
            }
            Err(e) => {
                error!("❌ Recovery system error: {}", e);
            }
        }

        // Small delay between scenarios
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    Ok(())
}

async fn show_recovery_statistics(recovery_manager: &RecoveryManager) {
    info!("\n📊 Recovery System Statistics");

    let history = recovery_manager.get_recovery_history().await;
    
    if history.is_empty() {
        info!("No recovery attempts recorded");
        return;
    }

    info!("Total recovery attempts: {}", history.len());
    
    let successful_count = history.iter().filter(|a| a.result.success).count();
    let success_rate = (successful_count as f64 / history.len() as f64) * 100.0;
    info!("Success rate: {:.1}% ({}/{})", success_rate, successful_count, history.len());

    // Strategy usage statistics
    let mut strategy_usage: HashMap<String, usize> = HashMap::new();
    let mut total_confidence = 0.0;
    let mut total_duration = Duration::from_millis(0);

    for attempt in &history {
        *strategy_usage.entry(attempt.strategy_used.clone()).or_insert(0) += 1;
        total_confidence += attempt.result.confidence;
        total_duration += attempt.result.recovery_duration;
    }

    info!("Average confidence: {:.2}", total_confidence / history.len() as f64);
    info!("Average recovery time: {:?}", total_duration / history.len() as u32);

    info!("Strategy usage:");
    for (strategy, count) in strategy_usage {
        let percentage = (count as f64 / history.len() as f64) * 100.0;
        info!("  {}: {} times ({:.1}%)", strategy, count, percentage);
    }

    // Resource monitor statistics
    let resource_monitor = recovery_manager.resource_monitor();
    let usage = resource_monitor.get_usage().await;
    info!("Current resource usage:");
    info!("  CPU: {:.1}%", usage.cpu_percent * 100.0);
    info!("  Memory: {:.1}%", usage.memory_percent * 100.0);
    info!("  Connections: {}", usage.active_connections);
    info!("  Tasks: {}", usage.active_tasks);
}

/// Mock coordinator for testing without complex dependencies
struct MockCoordinator {
    id: Uuid,
    state: synapsed_swarm::SwarmState,
}

impl MockCoordinator {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            state: synapsed_swarm::SwarmState {
                swarm_id: Uuid::new_v4(),
                active_agents: 3,
                pending_tasks: 2,
                running_tasks: 1,
                phase: synapsed_swarm::SwarmPhase::Coordinating,
                metrics: synapsed_swarm::SwarmMetrics::default(),
            },
        }
    }

    async fn state(&self) -> synapsed_swarm::SwarmState {
        self.state.clone()
    }
}

/// Custom recovery strategy for demonstration
struct CustomRecoveryStrategy {
    strategy_id: String,
}

impl CustomRecoveryStrategy {
    fn new() -> Self {
        Self {
            strategy_id: "custom_demo_strategy".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl RecoveryStrategy for CustomRecoveryStrategy {
    fn strategy_id(&self) -> &str {
        &self.strategy_id
    }

    fn description(&self) -> &str {
        "Custom recovery strategy for demo purposes"
    }

    async fn can_handle(&self, error: &synapsed_swarm::SwarmError) -> bool {
        matches!(error, synapsed_swarm::SwarmError::Other(e) if e.to_string().contains("CustomTestError"))
    }

    async fn recover(
        &self,
        _context: &RecoveryContext,
        error: &synapsed_swarm::SwarmError,
    ) -> RecoveryResult {
        info!("🔧 Custom recovery strategy handling: {}", error);
        
        // Simulate some recovery work
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        RecoveryResult {
            success: true,
            action_taken: "Applied custom recovery logic for test error".to_string(),
            recovery_duration: Duration::from_millis(100),
            new_state: None,
            confidence: 0.85,
            continue_recovery: false,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("custom_strategy".to_string(), serde_json::json!(true));
                meta.insert("error_pattern".to_string(), serde_json::json!("CustomTestError"));
                meta
            },
        }
    }

    async fn cost_estimate(&self) -> f64 {
        0.15 // Low-medium cost
    }

    async fn requires_external_resources(&self) -> bool {
        false
    }
}

/// Demonstrate specific recovery strategy testing
async fn demo_strategy_specific_testing() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🧪 Testing Individual Recovery Strategies");

    // Test exponential backoff
    let backoff_config = BackoffConfig::default();
    let backoff_strategy = ExponentialBackoffStrategy::new(backoff_config);
    
    info!("Testing exponential backoff delays:");
    for retry in 0..6 {
        let delay = backoff_strategy.calculate_delay(retry);
        info!("  Retry {}: {:?}", retry, delay);
    }

    // Test resource monitoring
    let resource_monitor = Arc::new(ResourceMonitor::new());
    
    // Simulate increasing load
    for i in 1..=5 {
        resource_monitor.update_cpu_usage(i as f64 * 0.15);
        resource_monitor.update_memory_usage(i as f64 * 0.12);
        
        for _ in 0..i {
            resource_monitor.increment_connections();
            resource_monitor.increment_tasks();
        }

        let usage = resource_monitor.get_usage().await;
        info!("Load level {}: CPU={:.1}%, Mem={:.1}%, Conn={}, Tasks={}", 
              i, usage.cpu_percent * 100.0, usage.memory_percent * 100.0,
              usage.active_connections, usage.active_tasks);
        
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    Ok(())
}