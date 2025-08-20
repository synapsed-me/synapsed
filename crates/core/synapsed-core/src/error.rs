//! Error handling types and utilities for the Synapsed ecosystem.
//!
//! This module provides standardized error types that are used throughout
//! all Synapsed crates to ensure consistent error handling patterns.

use std::fmt;
use thiserror::Error;
use serde::{Deserialize, Serialize};

/// The main error type for the Synapsed ecosystem.
///
/// This enum provides a comprehensive set of error variants that cover
/// common error scenarios across all Synapsed components.
#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SynapsedError {
    /// Configuration related errors
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Network related errors
    #[error("Network error: {0}")]
    Network(String),

    /// Cryptographic operation errors
    #[error("Cryptographic error: {0}")]
    Cryptographic(String),

    /// Storage/persistence errors
    #[error("Storage error: {0}")]
    Storage(String),

    /// Authentication/authorization errors
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Invalid input parameters
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Operation not permitted
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Timeout errors
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Internal system errors
    #[error("Internal error: {0}")]
    Internal(String),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// DID (Decentralized Identity) related errors
    #[error("DID error: {0}")]
    Did(String),

    /// P2P networking errors
    #[error("P2P error: {0}")]
    P2P(String),

    /// WASM runtime errors
    #[error("WASM error: {0}")]
    Wasm(String),

    /// Payment processing errors
    #[error("Payment error: {0}")]
    Payment(String),

    /// Generic application errors with context
    #[error("Application error: {message} (context: {context})")]
    Application { 
        /// Error message
        message: String, 
        /// Error context
        context: String 
    },
}

impl SynapsedError {
    /// Create a new configuration error
    pub fn config<T: fmt::Display>(msg: T) -> Self {
        Self::Configuration(msg.to_string())
    }

    /// Create a new network error
    pub fn network<T: fmt::Display>(msg: T) -> Self {
        Self::Network(msg.to_string())
    }

    /// Create a new cryptographic error
    pub fn crypto<T: fmt::Display>(msg: T) -> Self {
        Self::Cryptographic(msg.to_string())
    }

    /// Create a new storage error
    pub fn storage<T: fmt::Display>(msg: T) -> Self {
        Self::Storage(msg.to_string())
    }

    /// Create a new authentication error
    pub fn auth<T: fmt::Display>(msg: T) -> Self {
        Self::Authentication(msg.to_string())
    }

    /// Create a new invalid input error
    pub fn invalid_input<T: fmt::Display>(msg: T) -> Self {
        Self::InvalidInput(msg.to_string())
    }

    /// Create a new not found error
    pub fn not_found<T: fmt::Display>(msg: T) -> Self {
        Self::NotFound(msg.to_string())
    }

    /// Create a new permission denied error
    pub fn permission_denied<T: fmt::Display>(msg: T) -> Self {
        Self::PermissionDenied(msg.to_string())
    }

    /// Create a new timeout error
    pub fn timeout<T: fmt::Display>(msg: T) -> Self {
        Self::Timeout(msg.to_string())
    }

    /// Create a new internal error
    pub fn internal<T: fmt::Display>(msg: T) -> Self {
        Self::Internal(msg.to_string())
    }

    /// Create a new serialization error
    pub fn serialization<T: fmt::Display>(msg: T) -> Self {
        Self::Serialization(msg.to_string())
    }

    /// Create a new DID error
    pub fn did<T: fmt::Display>(msg: T) -> Self {
        Self::Did(msg.to_string())
    }

    /// Create a new P2P error
    pub fn p2p<T: fmt::Display>(msg: T) -> Self {
        Self::P2P(msg.to_string())
    }

    /// Create a new WASM error
    pub fn wasm<T: fmt::Display>(msg: T) -> Self {
        Self::Wasm(msg.to_string())
    }

    /// Create a new payment error
    pub fn payment<T: fmt::Display>(msg: T) -> Self {
        Self::Payment(msg.to_string())
    }

    /// Create a new application error with context
    pub fn application<T: fmt::Display, U: fmt::Display>(message: T, context: U) -> Self {
        Self::Application {
            message: message.to_string(),
            context: context.to_string(),
        }
    }

    /// Check if this error is retryable
    #[must_use] pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Network(_) | Self::Timeout(_) | Self::Internal(_)
        )
    }

    /// Check if this error is a client error (4xx-style)
    #[must_use] pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            Self::InvalidInput(_) | Self::NotFound(_) | Self::PermissionDenied(_) | Self::Authentication(_)
        )
    }

    /// Check if this error is a server error (5xx-style)
    #[must_use] pub fn is_server_error(&self) -> bool {
        matches!(
            self,
            Self::Internal(_) | Self::Storage(_) | Self::Configuration(_)
        )
    }
}

/// Result type alias for Synapsed operations
pub type SynapsedResult<T> = Result<T, SynapsedError>;

/// Trait for converting external errors to `SynapsedError`
pub trait IntoSynapsedError {
    /// Convert this error into a `SynapsedError`
    fn into_synapsed_error(self) -> SynapsedError;
}

// Standard error conversions
impl From<std::io::Error> for SynapsedError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

impl From<serde_json::Error> for SynapsedError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

impl From<bincode::Error> for SynapsedError {
    fn from(err: bincode::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

impl From<uuid::Error> for SynapsedError {
    fn from(err: uuid::Error) -> Self {
        Self::InvalidInput(err.to_string())
    }
}

impl From<chrono::ParseError> for SynapsedError {
    fn from(err: chrono::ParseError) -> Self {
        Self::InvalidInput(err.to_string())
    }
}

#[cfg(feature = "config")]
impl From<config::ConfigError> for SynapsedError {
    fn from(err: config::ConfigError) -> Self {
        Self::Configuration(err.to_string())
    }
}

impl From<toml::de::Error> for SynapsedError {
    fn from(err: toml::de::Error) -> Self {
        Self::Configuration(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = SynapsedError::config("test config error");
        assert_eq!(err, SynapsedError::Configuration("test config error".to_string()));
    }

    #[test]
    fn test_error_classification() {
        let client_err = SynapsedError::invalid_input("bad input");
        assert!(client_err.is_client_error());
        assert!(!client_err.is_server_error());
        assert!(!client_err.is_retryable());

        let server_err = SynapsedError::internal("server problem");
        assert!(server_err.is_server_error());
        assert!(!server_err.is_client_error());
        assert!(server_err.is_retryable());

        let network_err = SynapsedError::network("connection failed");
        assert!(!network_err.is_client_error());
        assert!(!network_err.is_server_error());
        assert!(network_err.is_retryable());
    }

    #[test]
    fn test_application_error() {
        let err = SynapsedError::application("failed to process", "user_id=123");
        match err {
            SynapsedError::Application { message, context } => {
                assert_eq!(message, "failed to process");
                assert_eq!(context, "user_id=123");
            }
            _ => panic!("Expected Application error"),
        }
    }

    #[test]
    fn test_error_conversions() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let synapsed_err: SynapsedError = io_err.into();
        matches!(synapsed_err, SynapsedError::Internal(_));
    }
}