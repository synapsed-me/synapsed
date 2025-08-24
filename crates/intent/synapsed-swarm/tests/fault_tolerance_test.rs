//! Integration tests for fault tolerance mechanisms

use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use synapsed_swarm::{
    SwarmCoordinator, SwarmConfig, SwarmResult,
    FaultToleranceManager, FaultToleranceConfig, AgentHealthStatus,
    CircuitBreakerStatus, RecoveryStatistics,
    AgentId, TaskId,
    execution::ExecutionConfig,
    trust::TrustManager,
};
use synapsed_promise::AutonomousAgent;
use synapsed_intent::{HierarchicalIntent, IntentBuilder, IntentContext};

#[tokio::test]
async fn test_fault_tolerance_integration_with_coordinator() -> SwarmResult<()> {
    // Create custom config with faster timeouts for testing
    let mut config = SwarmConfig::default();
    config.fault_tolerance_config = FaultToleranceConfig {
        heartbeat_interval_ms: 1000,
        agent_timeout_ms: 3000,
        circuit_breaker_failure_threshold: 2,
        circuit_breaker_timeout_ms: 5000,
        max_restart_attempts: 2,
        restart_delay_ms: 1000,
        ..Default::default()
    };
    
    let coordinator = SwarmCoordinator::new(config);
    coordinator.initialize().await?;
    
    // Create a test agent
    let agent = std::sync::Arc::new(AutonomousAgent::new(
        Uuid::new_v4(),
        "test-agent".to_string(),
        vec!["test".to_string()],
    ));
    
    let agent_id = coordinator.add_agent(agent.clone(), synapsed_swarm::AgentRole::Worker).await?;
    
    // Check initial health status
    let health = coordinator.get_agent_health(agent_id).await;
    assert_eq!(health, Some(AgentHealthStatus::Healthy));
    
    // Get initial recovery stats
    let stats = coordinator.get_recovery_stats().await;
    assert_eq!(stats.total_recovery_attempts, 0);
    
    // Create a simple intent
    let intent = IntentBuilder::new()
        .goal("test task".to_string())
        .add_step("step1", "Test step", std::collections::HashMap::new())
        .build()
        .expect("Failed to build intent");
    
    let context = IntentContext::new();
    
    // Delegate a task
    let task_id = coordinator.delegate_intent(intent, context).await?;
    
    // Wait a bit for task execution to start
    sleep(Duration::from_millis(500)).await;
    
    // Create a checkpoint
    let checkpoint_id = coordinator.create_task_checkpoint(
        task_id,
        agent_id,
        1,
        0.5,
        std::collections::HashMap::new(),
    ).await?;
    
    assert!(!checkpoint_id.is_nil());
    
    // Clean up
    coordinator.remove_agent(agent_id).await?;
    coordinator.shutdown().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_agent_failure_detection() -> SwarmResult<()> {
    let trust_manager = std::sync::Arc::new(TrustManager::new());
    let execution_engine = std::sync::Arc::new(
        synapsed_swarm::ExecutionEngine::with_config(ExecutionConfig::default())
    );
    
    let config = FaultToleranceConfig {
        heartbeat_interval_ms: 100,
        agent_timeout_ms: 500,
        ..Default::default()
    };
    
    let manager = FaultToleranceManager::new(config, trust_manager, execution_engine);
    manager.start().await?;
    
    // Create and register an agent
    let agent = std::sync::Arc::new(AutonomousAgent::new(
        Uuid::new_v4(),
        "test-agent".to_string(),
        vec!["test".to_string()],
    ));
    let agent_id = agent.id();
    
    manager.register_agent(agent).await?;
    
    // Initially healthy
    let health = manager.get_agent_health(agent_id).await;
    assert_eq!(health, Some(AgentHealthStatus::Healthy));
    
    // Stop sending heartbeats and wait for timeout
    sleep(Duration::from_millis(1000)).await;
    
    // Agent should eventually be marked as failed
    // Note: This might take some time due to the async nature of the system
    let mut attempts = 0;
    let max_attempts = 20;
    let mut final_health = None;
    
    while attempts < max_attempts {
        final_health = manager.get_agent_health(agent_id).await;
        if final_health == Some(AgentHealthStatus::Failed) ||
           final_health == Some(AgentHealthStatus::Unresponsive) {
            break;
        }
        sleep(Duration::from_millis(200)).await;
        attempts += 1;
    }
    
    // Agent should be unresponsive or failed
    assert!(final_health == Some(AgentHealthStatus::Failed) || 
            final_health == Some(AgentHealthStatus::Unresponsive),
            "Expected agent to be failed or unresponsive, got {:?}", final_health);
    
    manager.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_circuit_breaker_functionality() -> SwarmResult<()> {
    let trust_manager = std::sync::Arc::new(TrustManager::new());
    let execution_engine = std::sync::Arc::new(
        synapsed_swarm::ExecutionEngine::with_config(ExecutionConfig::default())
    );
    
    let config = FaultToleranceConfig {
        circuit_breaker_failure_threshold: 3,
        circuit_breaker_timeout_ms: 1000,
        ..Default::default()
    };
    
    let manager = FaultToleranceManager::new(config.clone(), trust_manager, execution_engine);
    manager.start().await?;
    
    let agent = std::sync::Arc::new(AutonomousAgent::new(
        Uuid::new_v4(),
        "test-agent".to_string(),
        vec!["test".to_string()],
    ));
    let agent_id = agent.id();
    
    manager.register_agent(agent).await?;
    
    // Initially should be able to handle tasks
    assert!(manager.can_handle_task(agent_id).await);
    
    // Record failures to trigger circuit breaker
    for _ in 0..config.circuit_breaker_failure_threshold {
        manager.record_task_result(agent_id, false, 1000).await?;
    }
    
    // Circuit breaker should be open now
    let cb_status = manager.get_circuit_breaker_status(agent_id).await;
    assert_eq!(cb_status, Some(CircuitBreakerStatus::Open));
    
    // Agent should not be able to handle tasks
    assert!(!manager.can_handle_task(agent_id).await);
    
    // Wait for circuit breaker timeout
    sleep(Duration::from_millis(1200)).await;
    
    // Should be able to handle tasks again (half-open)
    assert!(manager.can_handle_task(agent_id).await);
    
    manager.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_checkpoint_creation_and_retrieval() -> SwarmResult<()> {
    let trust_manager = std::sync::Arc::new(TrustManager::new());
    let execution_engine = std::sync::Arc::new(
        synapsed_swarm::ExecutionEngine::with_config(ExecutionConfig::default())
    );
    
    let config = FaultToleranceConfig::default();
    let manager = FaultToleranceManager::new(config, trust_manager, execution_engine);
    
    let task_id = Uuid::new_v4();
    let agent_id = Uuid::new_v4();
    
    let task_state = synapsed_swarm::fault_tolerance::TaskState {
        current_step: 2,
        completed_steps: vec![],
        remaining_steps: vec![],
        metadata: std::collections::HashMap::new(),
    };
    
    let progress = synapsed_swarm::fault_tolerance::TaskProgress {
        percentage: 0.5,
        completed_steps: 2,
        total_steps: 4,
        estimated_remaining_ms: Some(5000),
    };
    
    let context = std::collections::HashMap::new();
    
    // Create checkpoint
    let checkpoint_id = manager
        .create_checkpoint(task_id, agent_id, task_state, progress, context)
        .await?;
    
    // Retrieve checkpoint
    let retrieved = manager.get_latest_checkpoint(task_id).await;
    assert!(retrieved.is_some());
    
    let checkpoint = retrieved.unwrap();
    assert_eq!(checkpoint.checkpoint_id, checkpoint_id);
    assert_eq!(checkpoint.task_id, task_id);
    assert_eq!(checkpoint.agent_id, agent_id);
    assert_eq!(checkpoint.task_state.current_step, 2);
    assert!((checkpoint.progress.percentage - 0.5).abs() < f64::EPSILON);
    
    Ok(())
}

#[tokio::test]
async fn test_recovery_statistics() -> SwarmResult<()> {
    let trust_manager = std::sync::Arc::new(TrustManager::new());
    let execution_engine = std::sync::Arc::new(
        synapsed_swarm::ExecutionEngine::with_config(ExecutionConfig::default())
    );
    
    let config = FaultToleranceConfig::default();
    let manager = FaultToleranceManager::new(config, trust_manager, execution_engine);
    
    // Initial stats should be zero
    let stats = manager.get_recovery_stats().await;
    assert_eq!(stats.total_recovery_attempts, 0);
    assert_eq!(stats.successful_recoveries, 0);
    assert_eq!(stats.failed_recoveries, 0);
    assert_eq!(stats.agent_restarts, 0);
    assert_eq!(stats.task_redistributions, 0);
    assert_eq!(stats.task_rollbacks, 0);
    assert!(stats.last_recovery.is_none());
    
    Ok(())
}

#[tokio::test]
async fn test_multiple_agent_health_monitoring() -> SwarmResult<()> {
    let trust_manager = std::sync::Arc::new(TrustManager::new());
    let execution_engine = std::sync::Arc::new(
        synapsed_swarm::ExecutionEngine::with_config(ExecutionConfig::default())
    );
    
    let config = FaultToleranceConfig::default();
    let manager = FaultToleranceManager::new(config, trust_manager, execution_engine);
    manager.start().await?;
    
    // Create multiple agents
    let agent1 = std::sync::Arc::new(AutonomousAgent::new(
        Uuid::new_v4(),
        "agent1".to_string(),
        vec!["test".to_string()],
    ));
    let agent2 = std::sync::Arc::new(AutonomousAgent::new(
        Uuid::new_v4(),
        "agent2".to_string(),
        vec!["test".to_string()],
    ));
    
    let agent1_id = agent1.id();
    let agent2_id = agent2.id();
    
    manager.register_agent(agent1).await?;
    manager.register_agent(agent2).await?;
    
    // Record heartbeats for both agents
    manager.record_heartbeat(agent1_id, None).await?;
    manager.record_heartbeat(agent2_id, Some(Uuid::new_v4())).await?;
    
    // Check all agent health statuses
    let all_health = manager.get_all_agent_health().await;
    assert_eq!(all_health.len(), 2);
    assert_eq!(all_health.get(&agent1_id), Some(&AgentHealthStatus::Healthy));
    assert_eq!(all_health.get(&agent2_id), Some(&AgentHealthStatus::Healthy));
    
    manager.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_agent_performance_metrics_tracking() -> SwarmResult<()> {
    let trust_manager = std::sync::Arc::new(TrustManager::new());
    let execution_engine = std::sync::Arc::new(
        synapsed_swarm::ExecutionEngine::with_config(ExecutionConfig::default())
    );
    
    let config = FaultToleranceConfig::default();
    let manager = FaultToleranceManager::new(config, trust_manager, execution_engine);
    
    let agent = std::sync::Arc::new(AutonomousAgent::new(
        Uuid::new_v4(),
        "test-agent".to_string(),
        vec!["test".to_string()],
    ));
    let agent_id = agent.id();
    
    manager.register_agent(agent).await?;
    
    // Record some task results
    manager.record_task_result(agent_id, true, 1000).await?;
    manager.record_task_result(agent_id, true, 1500).await?;
    manager.record_task_result(agent_id, false, 2000).await?;
    
    // The metrics are internal, but we can verify that the circuit breaker
    // doesn't open for mixed results
    assert!(manager.can_handle_task(agent_id).await);
    
    Ok(())
}

#[tokio::test] 
async fn test_fault_tolerance_config_customization() {
    let custom_config = FaultToleranceConfig {
        heartbeat_interval_ms: 2000,
        agent_timeout_ms: 6000,
        circuit_breaker_failure_threshold: 10,
        circuit_breaker_timeout_ms: 30000,
        max_restart_attempts: 5,
        restart_delay_ms: 5000,
        task_redistribution_delay_ms: 2000,
        checkpoint_interval_ms: 60000,
        max_checkpoints: 20,
        enable_auto_recovery: false,
        enable_task_redistribution: false,
        recovery_confirmation_timeout_ms: 45000,
    };
    
    let trust_manager = std::sync::Arc::new(TrustManager::new());
    let execution_engine = std::sync::Arc::new(
        synapsed_swarm::ExecutionEngine::with_config(ExecutionConfig::default())
    );
    
    let manager = FaultToleranceManager::new(
        custom_config.clone(), 
        trust_manager, 
        execution_engine
    );
    
    // Just verify that the manager can be created with custom config
    // The actual config values are used internally
    assert_eq!(custom_config.heartbeat_interval_ms, 2000);
    assert_eq!(custom_config.max_restart_attempts, 5);
    assert!(!custom_config.enable_auto_recovery);
}

#[tokio::test]
async fn test_concurrent_agent_operations() -> SwarmResult<()> {
    let trust_manager = std::sync::Arc::new(TrustManager::new());
    let execution_engine = std::sync::Arc::new(
        synapsed_swarm::ExecutionEngine::with_config(ExecutionConfig::default())
    );
    
    let config = FaultToleranceConfig::default();
    let manager = std::sync::Arc::new(FaultToleranceManager::new(
        config, 
        trust_manager, 
        execution_engine
    ));
    
    // Create multiple agents concurrently
    let mut tasks = Vec::new();
    
    for i in 0..5 {
        let manager_clone = manager.clone();
        let task = tokio::spawn(async move {
            let agent = std::sync::Arc::new(AutonomousAgent::new(
                Uuid::new_v4(),
                format!("agent-{}", i),
                vec!["test".to_string()],
            ));
            let agent_id = agent.id();
            
            manager_clone.register_agent(agent).await?;
            manager_clone.record_heartbeat(agent_id, None).await?;
            manager_clone.record_task_result(agent_id, true, 1000).await?;
            
            Ok::<(), synapsed_swarm::SwarmError>(())
        });
        
        tasks.push(task);
    }
    
    // Wait for all tasks to complete
    for task in tasks {
        task.await.unwrap()?;
    }
    
    // Verify all agents are registered
    let all_health = manager.get_all_agent_health().await;
    assert_eq!(all_health.len(), 5);
    
    Ok(())
}