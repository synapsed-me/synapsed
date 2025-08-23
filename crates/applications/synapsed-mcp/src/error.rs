//! Error types for MCP server

use thiserror::Error;

/// Result type for MCP operations
pub type Result<T> = std::result::Result<T, McpError>;

/// MCP server errors
#[derive(Error, Debug)]
pub enum McpError {
    /// Intent-related errors
    #[error("Intent error: {0}")]
    Intent(#[from] synapsed_intent::IntentError),
    
    /// Promise-related errors
    #[error("Promise error: {0}")]
    Promise(#[from] synapsed_promise::PromiseError),
    
    /// Verification errors
    #[error("Verification error: {0}")]
    Verification(#[from] synapsed_verify::VerifyError),
    
    /// Transport errors
    #[error("Transport error: {0}")]
    Transport(String),
    
    /// Protocol errors
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    /// Tool not found
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    
    /// Invalid parameters
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
    
    /// Internal server error
    #[error("Internal error: {0}")]
    Internal(String),
    
    /// Storage errors
    #[error("Storage error: {0}")]
    StorageError(String),
    
    /// Serialization errors
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    /// Not found error
    #[error("Not found: {0}")]
    NotFound(String),
    
    /// Other errors
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}