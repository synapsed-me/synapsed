//! Recovery system demonstration
//! 
//! This example shows how the recovery system works with different types of errors
//! and recovery strategies.

use synapsed_swarm::{
    prelude::*,
    recovery::*,
    error::SwarmError,
};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("🔧 Starting Recovery System Demo");

    // Create recovery manager
    let recovery_manager = RecoveryManager::new();
    
    // Demonstrate individual recovery strategies
    demo_exponential_backoff().await?;
    demo_checkpoint_recovery().await?;
    demo_graceful_degradation().await?;
    demo_self_healing().await?;
    
    // Demonstrate comprehensive recovery scenario
    demo_comprehensive_recovery(&recovery_manager).await?;
    
    info!("✅ Recovery System Demo Complete");
    Ok(())
}

async fn demo_exponential_backoff() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🔄 Demonstrating Exponential Backoff Strategy");
    
    let config = BackoffConfig {
        initial_delay: Duration::from_millis(50),
        max_delay: Duration::from_secs(2),
        multiplier: 2.0,
        max_retries: 4,
        jitter_factor: 0.1,
    };
    
    let strategy = ExponentialBackoffStrategy::new(config);
    
    // Show delay progression
    for retry_count in 0..5 {
        let delay = strategy.calculate_delay(retry_count);
        info!("Retry {}: delay = {:?}", retry_count, delay);
    }
    
    // Simulate recovery attempt
    let swarm_config = SwarmConfig::default();
    let coordinator = Arc::new(SwarmCoordinator::new(swarm_config));
    
    let context = RecoveryContext {
        coordinator: coordinator.clone(),
        swarm_state: coordinator.state().await,
        failed_task_id: None,
        failed_agent_id: None,
        retry_count: 1,
        error_timestamp: chrono::Utc::now(),
        metadata: std::collections::HashMap::new(),
    };
    
    let error = SwarmError::CommunicationError("Simulated network timeout".to_string());
    let result = strategy.recover(&context, &error).await;
    
    info!("Recovery result: success={}, action={}, confidence={:.2}", 
          result.success, result.action_taken, result.confidence);
    
    Ok(())
}

async fn demo_checkpoint_recovery() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n💾 Demonstrating Checkpoint Recovery Strategy");
    
    let strategy = CheckpointRecoveryStrategy::new(3);
    let swarm_config = SwarmConfig::default();
    let coordinator = SwarmCoordinator::new(swarm_config);
    
    // Create a few checkpoints
    for i in 1..=3 {
        let checkpoint_id = strategy.create_checkpoint(&coordinator).await?;
        info!("Created checkpoint {}: {}", i, checkpoint_id);
        sleep(Duration::from_millis(100)).await;
    }
    
    // Simulate recovery from checkpoint
    let context = RecoveryContext {
        coordinator: Arc::new(coordinator),
        swarm_state: SwarmState {
            swarm_id: uuid::Uuid::new_v4(),
            active_agents: 2,
            pending_tasks: 1,
            running_tasks: 0,
            phase: SwarmPhase::Coordinating,
            metrics: SwarmMetrics::default(),
        },
        failed_task_id: None,
        failed_agent_id: None,
        retry_count: 0,
        error_timestamp: chrono::Utc::now(),
        metadata: std::collections::HashMap::new(),
    };
    
    let error = SwarmError::ConcurrencyError("State corruption detected".to_string());
    let result = strategy.recover(&context, &error).await;
    
    info!("Checkpoint recovery result: success={}, action={}", 
          result.success, result.action_taken);
    
    Ok(())
}

async fn demo_graceful_degradation() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n📉 Demonstrating Graceful Degradation Strategy");
    
    let resource_monitor = Arc::new(ResourceMonitor::new());
    let strategy = GracefulDegradationStrategy::new(resource_monitor.clone());
    
    // Simulate high resource usage
    resource_monitor.update_cpu_usage(0.85);
    resource_monitor.update_memory_usage(0.90);
    for _ in 0..10 {
        resource_monitor.increment_connections();
        resource_monitor.increment_tasks();
    }
    
    let usage = resource_monitor.get_usage().await;
    info!("Resource usage: CPU={:.1}%, Memory={:.1}%, Connections={}, Tasks={}", 
          usage.cpu_percent * 100.0, usage.memory_percent * 100.0, 
          usage.active_connections, usage.active_tasks);
    
    let swarm_config = SwarmConfig::default();
    let coordinator = Arc::new(SwarmCoordinator::new(swarm_config));
    
    let context = RecoveryContext {
        coordinator: coordinator.clone(),
        swarm_state: coordinator.state().await,
        failed_task_id: None,
        failed_agent_id: None,
        retry_count: 0,
        error_timestamp: chrono::Utc::now(),
        metadata: std::collections::HashMap::new(),
    };
    
    let error = SwarmError::SwarmSizeLimitExceeded { current: 12, max: 8 };
    let result = strategy.recover(&context, &error).await;
    
    info!("Graceful degradation result: success={}, action={}", 
          result.success, result.action_taken);
    
    Ok(())
}

async fn demo_self_healing() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🩹 Demonstrating Self-Healing Strategy");
    
    let strategy = SelfHealingStrategy::new();
    
    // Add custom healing rule
    let custom_rule = HealingRule {
        rule_id: "demo_rule".to_string(),
        error_pattern: "DemoError".to_string(),
        healing_action: HealingAction::AdjustConfig({
            let mut config = std::collections::HashMap::new();
            config.insert("timeout".to_string(), serde_json::json!(300));
            config.insert("retries".to_string(), serde_json::json!(5));
            config
        }),
        cooldown_duration: Duration::from_secs(60),
        last_applied: None,
    };
    
    strategy.add_healing_rule(custom_rule).await;
    info!("Added custom healing rule for DemoError pattern");
    
    let swarm_config = SwarmConfig::default();
    let coordinator = Arc::new(SwarmCoordinator::new(swarm_config));
    
    let context = RecoveryContext {
        coordinator: coordinator.clone(),
        swarm_state: coordinator.state().await,
        failed_task_id: None,
        failed_agent_id: None,
        retry_count: 0,
        error_timestamp: chrono::Utc::now(),
        metadata: std::collections::HashMap::new(),
    };
    
    // Test built-in rule
    let comm_error = SwarmError::CommunicationError("Agent communication failed".to_string());
    if strategy.can_handle(&comm_error).await {
        let result = strategy.recover(&context, &comm_error).await;
        info!("Self-healing (communication): success={}, action={}", 
              result.success, result.action_taken);
    }
    
    // Test custom rule
    let demo_error = SwarmError::Other(anyhow::anyhow!("DemoError: Custom failure condition"));
    if strategy.can_handle(&demo_error).await {
        let result = strategy.recover(&context, &demo_error).await;
        info!("Self-healing (custom): success={}, action={}", 
              result.success, result.action_taken);
    }
    
    Ok(())
}

async fn demo_comprehensive_recovery(recovery_manager: &RecoveryManager) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🎯 Demonstrating Comprehensive Recovery Scenario");
    
    let swarm_config = SwarmConfig::default();
    let coordinator = Arc::new(SwarmCoordinator::new(swarm_config));
    
    // Initialize coordinator (this may fail due to missing dependencies, but we'll demo anyway)
    if let Err(e) = coordinator.initialize().await {
        info!("Coordinator initialization failed (expected): {}", e);
    }
    
    // Create checkpoint before potential failures
    if let Ok(checkpoint_id) = recovery_manager.create_checkpoint(&coordinator).await {
        info!("Created safety checkpoint: {}", checkpoint_id);
    }
    
    // Simulate different types of failures and recovery
    let test_scenarios = vec![
        SwarmError::CommunicationError("Network partition detected".to_string()),
        SwarmError::CoordinationTimeout(45),
        SwarmError::SwarmSizeLimitExceeded { current: 15, max: 10 },
        SwarmError::ResourceConflict { resource: "memory_pool".to_string() },
        SwarmError::ConcurrencyError("Deadlock in task scheduler".to_string()),
    ];
    
    for (i, error) in test_scenarios.into_iter().enumerate() {
        info!("\n--- Scenario {} ---", i + 1);
        info!("Simulating error: {}", error);
        
        let task_id = Some(uuid::Uuid::new_v4());
        let agent_id = Some(uuid::Uuid::new_v4());
        
        match recovery_manager.recover(coordinator.clone(), error.clone(), task_id, agent_id).await {
            Ok(result) => {
                info!("✅ Recovery successful: {}", result.action_taken);
                info!("   Confidence: {:.2}, Duration: {:?}", 
                      result.confidence, result.recovery_duration);
                
                if let Some(metadata) = result.metadata.get("strategy_cost") {
                    info!("   Strategy cost: {}", metadata);
                }
            }
            Err(e) => {
                error!("❌ Recovery failed: {}", e);
            }
        }
        
        // Small delay between scenarios
        sleep(Duration::from_millis(200)).await;
    }
    
    // Show recovery history
    let history = recovery_manager.get_recovery_history().await;
    info!("\n📊 Recovery Statistics:");
    info!("   Total attempts: {}", history.len());
    
    let successful = history.iter().filter(|a| a.result.success).count();
    info!("   Successful: {} ({:.1}%)", successful, 
          (successful as f64 / history.len() as f64) * 100.0);
    
    // Show strategy usage
    let mut strategy_usage = std::collections::HashMap::new();
    for attempt in &history {
        *strategy_usage.entry(attempt.strategy_used.clone()).or_insert(0) += 1;
    }
    
    info!("   Strategy usage:");
    for (strategy, count) in strategy_usage {
        info!("     {}: {} times", strategy, count);
    }
    
    // Show current resource usage
    let resource_monitor = recovery_manager.resource_monitor();
    let usage = resource_monitor.get_usage().await;
    info!("   Current resource usage:");
    info!("     CPU: {:.1}%", usage.cpu_percent * 100.0);
    info!("     Memory: {:.1}%", usage.memory_percent * 100.0);
    info!("     Connections: {}", usage.active_connections);
    info!("     Tasks: {}", usage.active_tasks);
    
    Ok(())
}

/// Helper function to demonstrate recovery strategy cost analysis
async fn demo_cost_analysis(recovery_manager: &RecoveryManager) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n💰 Recovery Strategy Cost Analysis");
    
    let strategies = recovery_manager.strategies.read().await;
    let mut costs = Vec::new();
    
    for strategy in strategies.iter() {
        let cost = strategy.cost_estimate().await;
        let requires_external = strategy.requires_external_resources().await;
        costs.push((strategy.strategy_id().to_string(), cost, requires_external));
    }
    
    // Sort by cost
    costs.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    
    info!("Strategies ordered by cost (low to high):");
    for (strategy, cost, external) in costs {
        let external_marker = if external { " (ext)" } else { "" };
        info!("  {:<20} Cost: {:.2}{}", strategy, cost, external_marker);
    }
    
    Ok(())
}