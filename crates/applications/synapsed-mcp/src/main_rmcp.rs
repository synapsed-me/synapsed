//! Synapsed MCP Server - Using rmcp transport layer

use anyhow::Result;
use rmcp::transport::StdioTransport;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod error;
mod intent_store;
mod agent_spawner;
mod observability;
mod rmcp_adapter;

use crate::{
    intent_store::IntentStore,
    agent_spawner::AgentSpawner,
    rmcp_adapter::SynapsedMcpAdapter,
};

// Server metadata
pub const SERVER_NAME: &str = "synapsed-mcp";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SERVER_DESCRIPTION: &str = "Intent verification and Promise Theory MCP server for AI agents";

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    info!("Starting Synapsed MCP Server v{}", SERVER_VERSION);
    info!("Using rmcp transport layer for protocol handling");
    
    // Initialize intent store
    let intent_store = if let Ok(storage_path) = std::env::var("SYNAPSED_INTENT_STORAGE_PATH") {
        info!("Using persistent storage at: {}", storage_path);
        if storage_path.ends_with(".db") {
            Arc::new(RwLock::new(IntentStore::with_sqlite_storage(&storage_path)?))
        } else {
            Arc::new(RwLock::new(IntentStore::with_file_storage(&storage_path)?))
        }
    } else {
        info!("Using in-memory storage");
        Arc::new(RwLock::new(IntentStore::new()?))
    };
    
    // Initialize agent spawner
    let agent_spawner = Arc::new(AgentSpawner::new());
    
    // Initialize observability
    let log_path = std::env::var("SYNAPSED_SUBSTRATES_LOG")
        .unwrap_or_else(|_| "/tmp/synapsed_substrates.log".to_string());
    tokio::spawn(async move {
        if let Err(e) = observability::EVENT_CIRCUIT.initialize(log_path).await {
            tracing::error!("Failed to initialize event circuit: {}", e);
        }
    });
    
    // Create the adapter
    let adapter = SynapsedMcpAdapter::new(intent_store, agent_spawner);
    
    info!("MCP server ready - accepting requests via stdio");
    info!("Available tools:");
    info!("  - intent_declare: Declare intentions before actions");
    info!("  - intent_verify: Verify execution against declarations");
    info!("  - intent_status: Get intent status");
    info!("  - context_inject: Inject context for sub-agents");
    
    // Use rmcp's stdio transport
    let transport = StdioTransport::new();
    let server = rmcp::server::Server::new(adapter);
    
    // Run the server
    server.run(transport).await?;
    
    info!("Synapsed MCP server shutting down");
    Ok(())
}