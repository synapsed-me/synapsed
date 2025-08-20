//! # Synapsed Core
//!
//! Core traits, utilities, and shared functionality for the Synapsed ecosystem.
//! This crate provides the foundational components that all other Synapsed crates depend on.
//!
//! ## Features
//!
//! - **Error Handling**: Standardized error types and result aliases
//! - **Configuration**: Unified configuration management system
//! - **Observability**: Logging, tracing, and metrics traits
//! - **Async Runtime**: Tokio-based async abstractions
//! - **Serialization**: Common serialization/deserialization helpers
//! - **Network Abstractions**: Common networking trait definitions
//! - **Security**: Cryptographic helper traits and utilities
//!
//! ## Quick Start
//!
//! ```rust
//! use synapsed_core::{SynapsedResult, SynapsedError};
//!
//! fn example_function() -> SynapsedResult<String> {
//!     Ok("Hello Synapsed!".to_string())
//! }
//! ```

#![deny(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod config;
pub mod error;
pub mod memory;
pub mod network;
pub mod observability;
pub mod runtime;
pub mod security;
pub mod serialization;
pub mod traits;
pub mod utils;

// Re-export commonly used items
pub use error::{SynapsedError, SynapsedResult};
pub use traits::{Observable, Configurable, Identifiable, Validatable};

/// Version information for the Synapsed Core library
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The name of the Synapsed Core library
pub const NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        assert!(!VERSION.is_empty());
        assert_eq!(NAME, "synapsed-core");
    }

    #[test]
    fn test_error_result_types() {
        let success: SynapsedResult<i32> = Ok(42);
        assert!(success.is_ok());
        assert_eq!(success.unwrap(), 42);

        let error: SynapsedResult<i32> = Err(SynapsedError::InvalidInput("test error".to_string()));
        assert!(error.is_err());
    }
}