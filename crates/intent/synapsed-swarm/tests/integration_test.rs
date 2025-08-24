//! Integration tests for swarm coordination

use synapsed_swarm::prelude::*;
use synapsed_intent::{IntentBuilder, Step, StepAction};
use synapsed_promise::{AutonomousAgent, AgentConfig, AgentCapabilities, QualityOfService};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_swarm_initialization() {
    let config = SwarmConfig::default();
    let coordinator = SwarmCoordinator::new(config);
    
    let result = coordinator.initialize().await;
    assert!(result.is_ok(), "Swarm should initialize successfully");
    
    let state = coordinator.state().await;
    assert_eq!(state.active_agents, 0);
    assert_eq!(state.phase, SwarmPhase::Ready);
}

#[tokio::test]
async fn test_agent_registration() {
    let coordinator = SwarmCoordinator::new(SwarmConfig::default());
    coordinator.initialize().await.unwrap();
    
    // Create test agent
    let agent = create_test_agent("test_agent");
    
    // Add agent to swarm
    let agent_id = coordinator.add_agent(agent, AgentRole::Worker).await;
    assert!(agent_id.is_ok(), "Should add agent successfully");
    
    let state = coordinator.state().await;
    assert_eq!(state.active_agents, 1);
}

#[tokio::test]
async fn test_intent_delegation() {
    let coordinator = SwarmCoordinator::new(SwarmConfig::default());
    coordinator.initialize().await.unwrap();
    
    // Add agents
    let agent1 = create_test_agent("agent_1");
    let agent2 = create_test_agent("agent_2");
    
    coordinator.add_agent(agent1, AgentRole::Worker).await.unwrap();
    coordinator.add_agent(agent2, AgentRole::Worker).await.unwrap();
    
    // Create intent
    let intent = IntentBuilder::new("Test task")
        .add_step(Step::new(
            "Execute test",
            StepAction::Custom(serde_json::json!({"test": true}))
        ))
        .build()
        .unwrap();
    
    // Create context
    let context = synapsed_intent::ContextBuilder::new()
        .variable("test", serde_json::json!(true))
        .build()
        .await;
    
    // Delegate intent
    let task_id = coordinator.delegate_intent(intent, context).await;
    assert!(task_id.is_ok(), "Should delegate intent successfully");
    
    // Wait a bit for async execution
    sleep(Duration::from_millis(100)).await;
    
    let state = coordinator.state().await;
    assert!(state.pending_tasks > 0 || state.running_tasks > 0);
}

#[tokio::test]
async fn test_trust_management() {
    use synapsed_swarm::trust::TrustManager;
    
    let trust_manager = TrustManager::new();
    trust_manager.initialize().await.unwrap();
    
    let agent_id = uuid::Uuid::new_v4();
    
    // Initialize agent trust
    trust_manager.initialize_agent(agent_id, 0.5).await.unwrap();
    
    // Get initial trust
    let initial_trust = trust_manager.get_trust(agent_id).await.unwrap();
    assert_eq!(initial_trust, 0.5);
    
    // Update trust for success
    trust_manager.update_trust(agent_id, true, true).await.unwrap();
    
    // Check trust increased
    let updated_trust = trust_manager.get_trust(agent_id).await.unwrap();
    assert!(updated_trust > initial_trust, "Trust should increase after success");
}

#[tokio::test]
async fn test_agent_protocol() {
    use synapsed_swarm::protocol::{AgentProtocol, AgentMessage, MessageType, MessagePayload};
    use uuid::Uuid;
    
    let mut protocol = AgentProtocol::new();
    
    // Create hello message
    let message = AgentProtocol::create_hello(
        Uuid::new_v4(),
        vec!["capability1".to_string()]
    );
    
    assert_eq!(message.message_type, MessageType::Hello);
    
    // Process message (no handler registered, should return None)
    let response = protocol.process_message(&message).unwrap();
    assert!(response.is_none());
}

#[tokio::test]
async fn test_claude_agent() {
    use synapsed_swarm::claude_agent::{ClaudeAgent, ClaudeAgentConfig};
    
    let config = ClaudeAgentConfig {
        name: "test_claude".to_string(),
        capabilities: vec!["test".to_string()],
        ..Default::default()
    };
    
    let agent = ClaudeAgent::new(config);
    let agent_id = agent.id();
    
    // Initialize agent
    let result = agent.initialize().await;
    assert!(result.is_ok());
    
    // Check initial trust score
    let trust = agent.trust_score().await;
    assert_eq!(trust.value, crate::DEFAULT_TRUST_SCORE);
}

// Helper function to create test agent
fn create_test_agent(name: &str) -> Arc<AutonomousAgent> {
    let config = AgentConfig {
        name: name.to_string(),
        capabilities: AgentCapabilities {
            services: vec!["test".to_string()],
            resources: vec!["cpu".to_string()],
            protocols: vec!["promise".to_string()],
            quality: QualityOfService::default(),
        },
        trust_model: synapsed_promise::TrustModel::new(),
        cooperation_protocol: synapsed_promise::CooperationProtocol::new(),
        max_promises: 5,
        promise_timeout_secs: 60,
    };
    
    Arc::new(AutonomousAgent::new(config))
}