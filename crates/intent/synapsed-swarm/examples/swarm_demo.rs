//! Demonstration of swarm coordination with intent, promise, and verification

use synapsed_swarm::prelude::*;
use synapsed_intent::{IntentBuilder, Step, StepAction};
use synapsed_promise::{AutonomousAgent, AgentConfig, AgentCapabilities};
use tracing::{info, error};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    info!("🚀 Starting Swarm Coordination Demo");
    
    // Create swarm coordinator
    let config = SwarmConfig::default();
    let coordinator = Arc::new(SwarmCoordinator::new(config));
    
    // Initialize swarm
    coordinator.initialize().await?;
    info!("✅ Swarm initialized");
    
    // Create and add agents
    let agent1 = create_agent("agent_1", vec!["code_generation".to_string()]);
    let agent2 = create_agent("agent_2", vec!["code_review".to_string()]);
    let agent3 = create_agent("agent_3", vec!["testing".to_string()]);
    
    let agent1_id = coordinator.add_agent(agent1, AgentRole::Worker).await?;
    let agent2_id = coordinator.add_agent(agent2, AgentRole::Worker).await?;
    let agent3_id = coordinator.add_agent(agent3, AgentRole::Verifier).await?;
    
    info!("✅ Added 3 agents to swarm");
    
    // Create an intent to delegate
    let intent = IntentBuilder::new("Build REST API")
        .add_step(Step::new(
            "Design API",
            StepAction::Custom(serde_json::json!({
                "action": "design",
                "target": "REST API"
            }))
        ))
        .add_step(Step::new(
            "Implement endpoints",
            StepAction::Custom(serde_json::json!({
                "action": "implement",
                "target": "endpoints"
            }))
        ))
        .add_step(Step::new(
            "Write tests",
            StepAction::Custom(serde_json::json!({
                "action": "test",
                "target": "API"
            }))
        ))
        .build()?;
    
    // Create execution context
    let context = synapsed_intent::ContextBuilder::new()
        .variable("project", serde_json::json!("demo_api"))
        .variable("language", serde_json::json!("rust"))
        .build()
        .await;
    
    info!("📋 Delegating intent to swarm");
    
    // Delegate intent to swarm
    let task_id = coordinator.delegate_intent(intent, context).await?;
    
    info!("✅ Task {} delegated to swarm", task_id);
    
    // Wait for task completion
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    
    // Get task result
    if let Some(result) = coordinator.get_task_result(task_id).await {
        if result.success {
            info!("✅ Task completed successfully!");
            if let Some(output) = result.output {
                info!("📊 Output: {}", serde_json::to_string_pretty(&output)?);
            }
        } else {
            error!("❌ Task failed: {:?}", result.error);
        }
    }
    
    // Get swarm metrics
    let metrics = coordinator.metrics().await;
    info!("📊 Swarm Metrics:");
    info!("  Total agents: {}", metrics.total_agents);
    info!("  Tasks succeeded: {}", metrics.tasks_succeeded);
    info!("  Tasks failed: {}", metrics.tasks_failed);
    info!("  Promises fulfilled: {}", metrics.promises_fulfilled);
    info!("  Average trust score: {:.2}", metrics.avg_trust_score);
    
    info!("🎉 Demo completed!");
    
    Ok(())
}

fn create_agent(name: &str, capabilities: Vec<String>) -> Arc<AutonomousAgent> {
    let config = AgentConfig {
        name: name.to_string(),
        capabilities: AgentCapabilities {
            services: capabilities,
            resources: vec!["memory".to_string(), "cpu".to_string()],
            protocols: vec!["promise".to_string()],
            quality: synapsed_promise::QualityOfService::default(),
        },
        trust_model: synapsed_promise::TrustModel::new(),
        cooperation_protocol: synapsed_promise::CooperationProtocol::new(),
        max_promises: 10,
        promise_timeout_secs: 60,
    };
    
    Arc::new(AutonomousAgent::new(config))
}