//! # Synapsed MCP Server
//!
//! Model Context Protocol (MCP) server for intent verification in AI agents.
//! 
//! This server exposes tools to Claude and other AI agents for:
//! - Declaring intentions before actions
//! - Verifying execution against declarations
//! - Managing context boundaries
//! - Building trust through Promise Theory

pub mod server;
pub mod tools;
pub mod swarm_tools;
pub mod resources;
pub mod transport;
pub mod error;
mod intent_store;  // Internal module - not exported
mod protocol;      // Internal module - protocol handler
mod agent_spawner; // Internal module - agent spawning

pub use server::{McpServer, ServerConfig};
pub use tools::{IntentTools, VerificationTools};
pub use swarm_tools::SwarmTools;
pub use resources::ContextResources;
pub use error::{McpError, Result};

/// MCP Server metadata
pub const SERVER_NAME: &str = "synapsed-mcp";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SERVER_DESCRIPTION: &str = "Intent verification system for AI agents";
