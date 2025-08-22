//! MCP Server binary for Synapsed intent verification system

use synapsed_mcp::{McpServer, ServerConfig};
use tracing::{info, error};
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .init();

    info!("Starting Synapsed MCP Server");

    // Load configuration
    let config = ServerConfig::from_env()?;
    
    // Create and run server
    let server = McpServer::new(config);
    
    match server.serve_stdio().await {
        Ok(_) => {
            info!("MCP Server shutdown gracefully");
            Ok(())
        }
        Err(e) => {
            error!("MCP Server error: {}", e);
            Err(e.into())
        }
    }
}