//! Routing error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RoutingError {
    #[error("Circuit creation failed: {0}")]
    CircuitCreation(String),
    
    #[error("No available nodes for routing")]
    NoAvailableNodes,
    
    #[error("Message encryption failed: {0}")]
    EncryptionError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Timeout waiting for response")]
    Timeout,
}

pub type Result<T> = std::result::Result<T, RoutingError>;