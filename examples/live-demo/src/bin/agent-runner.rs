//! Agent runner executable
//! 
//! This binary is spawned by the MCP server to run agents with intents.
//! It simulates how Claude would spawn sub-agents to execute tasks.

use anyhow::Result;
use clap::Parser;
use serde_json::Value;
use std::path::PathBuf;
use synapsed_intent::{ContextBuilder, IntentContext};
use tracing::{info, error};

mod agents {
    pub use live_demo::agents::*;
}

mod project {
    pub use live_demo::project::*;
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Agent type to run
    #[arg(long)]
    agent_type: String,

    /// Workspace directory
    #[arg(long)]
    workspace: String,

    /// Intent ID associated with this agent
    #[arg(long)]
    intent_id: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let args = Args::parse();
    
    // Get environment variables set by MCP
    let agent_id = std::env::var("AGENT_ID").unwrap_or_else(|_| "unknown".to_string());
    let intent_id = args.intent_id.or_else(|| std::env::var("INTENT_ID").ok());
    
    info!("🤖 Agent runner started");
    info!("  Agent ID: {}", agent_id);
    info!("  Agent Type: {}", args.agent_type);
    info!("  Workspace: {}", args.workspace);
    if let Some(ref id) = intent_id {
        info!("  Intent ID: {}", id);
    }
    
    // Create workspace wrapper
    let workspace = project::ProjectWorkspace::from_path(PathBuf::from(&args.workspace));
    
    // Create execution context
    let context = ContextBuilder::new()
        .allow_commands(vec!["cargo".to_string(), "rustc".to_string()])
        .variable("workspace", Value::String(args.workspace.clone()))
        .variable("agent_id", Value::String(agent_id.clone()))
        .build()
        .await;
    
    // Connect to MCP server to update intent status
    if let Some(intent_id) = &intent_id {
        update_intent_status(intent_id, "executing").await?;
    }
    
    // Execute the appropriate agent
    let result = match args.agent_type.as_str() {
        "architect" => {
            info!("🏗️ Architect agent: Designing API structure...");
            agents::architect::execute(&workspace, &context).await
        }
        "backend" => {
            info!("⚙️ Backend agent: Implementing API endpoints...");
            agents::backend::execute(&workspace, &context).await
        }
        "tester" => {
            info!("🧪 Tester agent: Writing and running tests...");
            agents::tester::execute(&workspace, &context).await
        }
        "documenter" => {
            info!("📚 Documenter agent: Creating API documentation...");
            agents::documenter::execute(&workspace, &context).await
        }
        "reviewer" => {
            info!("🔍 Reviewer agent: Analyzing code quality...");
            agents::reviewer::execute(&workspace, &context).await
        }
        _ => {
            error!("❌ Unknown agent type: {}", args.agent_type);
            return Err(anyhow::anyhow!("Unknown agent type"));
        }
    };
    
    // Update intent status based on result
    if let Some(intent_id) = &intent_id {
        match result {
            Ok(_) => {
                info!("✅ Agent completed successfully");
                update_intent_status(intent_id, "completed").await?;
            }
            Err(ref e) => {
                error!("❌ Agent failed: {}", e);
                update_intent_status(intent_id, "failed").await?;
            }
        }
    }
    
    result
}

/// Update intent status by calling back to MCP server
async fn update_intent_status(intent_id: &str, status: &str) -> Result<()> {
    // In a real implementation, this would make an HTTP/RPC call to the MCP server
    // For now, we'll just log the status update
    info!("📝 Updating intent {} status to: {}", intent_id, status);
    
    // TODO: Implement actual MCP client call to update intent status
    // This would involve:
    // 1. Connecting to MCP server (via stdio or HTTP)
    // 2. Sending intent/update request
    // 3. Handling response
    
    Ok(())
}