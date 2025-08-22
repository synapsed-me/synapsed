//! Transport implementations for MCP server

use crate::error::Result;
use tracing::info;

/// Transport configuration
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Buffer size for stdio
    pub buffer_size: usize,
    /// Timeout for requests in milliseconds
    pub request_timeout_ms: u64,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            buffer_size: 8192,
            request_timeout_ms: 30000,
        }
    }
}

/// Setup stdio transport for MCP
pub async fn setup_stdio() -> Result<()> {
    info!("Setting up stdio transport for MCP");
    
    // The actual transport is handled by rmcp
    // This is just a placeholder for any additional setup
    
    Ok(())
}