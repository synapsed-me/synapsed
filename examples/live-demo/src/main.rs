//! Live demonstration of the Synapsed Intent System with real agents
//!
//! This demo shows multiple agents collaborating to build a TODO REST API
//! while being monitored in real-time through the observability system.

mod project;
mod mcp_client;

use synapsed_intent::agent_parser::AgentMarkdownParser;
use synapsed_monitor::{
    ObservabilityCollector, CollectorConfig,
    EventAggregator, narrator::{EventNarrator, NarrativeStyle},
    MonitorServer, ServerConfig,
};
use synapsed_verify::{CommandVerifier, FileSystemVerifier};
use crate::mcp_client::McpClient;
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tracing::{info, error};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();
    
    // Set persistent storage path for intents
    std::env::set_var("SYNAPSED_INTENT_STORAGE_PATH", "/tmp/synapsed-intents.db");
    info!("💾 Using persistent storage at: /tmp/synapsed-intents.db");
    
    info!("🚀 Starting Synapsed Live Demo - Multi-Agent API Builder");
    
    // Start MCP server
    info!("🔌 Starting MCP server...");
    let mut mcp_client = McpClient::new().await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await; // Give server time to start
    info!("✅ MCP server ready");
    
    // Start the monitoring system
    let (monitor_handle, collector) = start_monitoring_system().await?;
    
    // Create project workspace
    let workspace = project::ProjectWorkspace::new("todo-api").await?;
    info!("📁 Created workspace at: {}", workspace.root().display());
    
    // Parse agent definitions
    let agents = load_agents().await?;
    info!("🤖 Loaded {} agent definitions", agents.len());
    
    // Declare main intent through MCP
    let main_intent_id = mcp_client.declare_intent(
        "orchestrator",
        "Build TODO REST API - Complete REST API for TODO application with tests and documentation",
        Some(serde_json::json!({
            "workspace": workspace.root().to_string_lossy().to_string(),
            "type": "main_intent"
        })),
    ).await?;
    info!("🎯 Created main intent via MCP: {}", main_intent_id);
    
    // Create sub-intent IDs for each phase through MCP
    let mut sub_intent_ids = Vec::new();
    let phases = [
        ("architect", "Design and structure the API"),
        ("backend", "Implement backend functionality"),
        ("tester", "Write comprehensive tests"),
        ("documenter", "Create API documentation"),
        ("reviewer", "Review and finalize the implementation"),
    ];
    
    for (agent, description) in &phases {
        let intent_id = mcp_client.declare_intent(
            agent,
            description,
            Some(serde_json::json!({
                "parent_intent": &main_intent_id,
                "workspace": workspace.root().to_string_lossy().to_string(),
            })),
        ).await?;
        info!("📋 Created sub-intent for {}: {}", agent, &intent_id);
        sub_intent_ids.push((agent.to_string(), intent_id));
    }
    
    info!("📋 Created {} sub-intents via MCP", sub_intent_ids.len());
    
    // Execute the demonstration
    info!("▶️ Starting execution...");
    info!("📊 Monitor dashboard available at http://localhost:3000");
    info!("🔌 Monitor API available at http://localhost:8080");
    
    execute_demo(main_intent_id, sub_intent_ids, &workspace, &collector, &mut mcp_client).await?;
    
    info!("✅ Demo completed successfully!");
    info!("📁 API built in: {}", workspace.root().display());
    
    // List all intents stored through MCP
    info!("📋 Listing all stored intents...");
    let intents = mcp_client.list_intents().await?;
    info!("  Found {} intents in storage", intents.len());
    
    // Shutdown MCP server
    mcp_client.shutdown().await?;
    info!("🔌 MCP server shutdown");
    
    // Keep server running for monitoring
    info!("Press Ctrl+C to stop the monitoring server...");
    monitor_handle.await?;
    
    Ok(())
}

/// Start the monitoring system
async fn start_monitoring_system() -> Result<(tokio::task::JoinHandle<()>, Arc<ObservabilityCollector>)> {
    // Create collector
    let collector = Arc::new(ObservabilityCollector::new(CollectorConfig::default()));
    
    // Create aggregator and narrator
    let aggregator = Arc::new(RwLock::new(EventAggregator::new()));
    let narrator = Arc::new(EventNarrator::new(NarrativeStyle::Conversational));
    
    // Create and start server
    let server_config = ServerConfig::default();
    let monitor_server = MonitorServer::new(
        server_config,
        collector.clone(),
        aggregator,
        narrator,
    );
    
    let handle = tokio::spawn(async move {
        if let Err(e) = monitor_server.start().await {
            error!("Monitor server error: {}", e);
        }
    });
    
    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    Ok((handle, collector))
}

/// Load agent definitions from markdown files
async fn load_agents() -> Result<Vec<synapsed_intent::dynamic_agents::SubAgentDefinition>> {
    let parser = AgentMarkdownParser::new();
    let agent_dir = PathBuf::from("agent-definitions");
    
    let mut agents = Vec::new();
    
    // Load each agent definition
    for agent_file in ["architect-agent.md", "backend-agent.md", "test-agent.md", 
                       "documentation-agent.md", "review-agent.md"] {
        let path = agent_dir.join(agent_file);
        let agent = parser.parse_file(&path).await?;
        info!("  ✓ Loaded agent: {}", agent.name);
        agents.push(agent);
    }
    
    Ok(agents)
}


/// Execute the demonstration
async fn execute_demo(
    main_intent_id: String,
    sub_intent_ids: Vec<(String, String)>,
    workspace: &project::ProjectWorkspace,
    collector: &Arc<ObservabilityCollector>,
    mcp_client: &mut McpClient,
) -> Result<()> {
    // Execute each phase
    for (i, (agent_name, intent_id)) in sub_intent_ids.iter().enumerate() {
        info!("📍 Phase {}/{}: Agent '{}' with intent {}", 
              i + 1, sub_intent_ids.len(), agent_name, intent_id);
        
        // Update intent status to executing
        mcp_client.update_intent(&intent_id, "executing", None).await?;
        
        // Spawn agent through MCP with the already-declared intent
        let agent_id = mcp_client.spawn_agent(
            agent_name,
            Some(serde_json::json!({
                "workspace": workspace.root().to_string_lossy().to_string(),
                "phase": i + 1,
            })),
            Some(intent_id.clone()),
        ).await?;
        info!("  🤖 Agent '{}' spawned with ID: {}", agent_name, agent_id);
        
        // Wait for agent to complete
        info!("  ⏳ Waiting for agent to complete its work...");
        
        // Poll agent status until it completes
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            
            let status = mcp_client.get_agent_status(&agent_id).await?;
            let agent_status = status["status"].as_str().unwrap_or("Unknown");
            
            match agent_status {
                "Completed" => {
                    info!("  ✅ Agent completed successfully");
                    break;
                }
                "Failed" => {
                    error!("  ❌ Agent failed");
                    break;
                }
                "Running" | "Spawning" => {
                    // Still running, continue waiting
                }
                _ => {
                    info!("  Agent status: {}", agent_status);
                }
            }
        }
        
        // Update intent status to completed
        mcp_client.update_intent(&intent_id, "completed", Some(serde_json::json!({
            "agent": agent_name,
            "phase": i + 1,
        }))).await?;
        
        // Verify the intent
        let verified = mcp_client.verify_intent(&intent_id).await?;
        if verified {
            info!("  ✅ Phase complete and verified!");
        } else {
            info!("  ✅ Phase complete!");
        }
        
        // Small delay between phases for visibility
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
    
    // Update main intent status to completed
    mcp_client.update_intent(&main_intent_id, "completed", Some(serde_json::json!({
        "message": "All phases completed successfully",
        "sub_intents": sub_intent_ids.len(),
    }))).await?;
    
    // Verify main intent
    let verified = mcp_client.verify_intent(&main_intent_id).await?;
    if verified {
        info!("🎯 Main intent completed and verified!");
    } else {
        info!("🎯 Main intent completed!");
    }
    
    Ok(())
}