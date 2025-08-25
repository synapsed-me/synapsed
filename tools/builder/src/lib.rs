//! # Synapsed Builder
//! 
//! Composable application builder that allows assembling Synapsed modules
//! like building blocks without writing code.

pub mod registry;
pub mod recipe;
pub mod builder;
pub mod composer;
pub mod validator;
pub mod templates;

#[cfg(test)]
mod tests;

pub use registry::{ComponentRegistry, Component, Capability};
pub use recipe::{Recipe, RecipeStep, Connection};
pub use builder::{SynapsedBuilder, BuilderConfig};
pub use composer::{Composer, CompositionResult};
pub use validator::{Validator, ValidationResult};

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::{
        SynapsedBuilder,
        Component,
        Capability,
        Recipe,
        templates::Templates,
    };
}

use thiserror::Error;

/// Builder-specific errors
#[derive(Error, Debug)]
pub enum BuilderError {
    #[error("Component not found: {0}")]
    ComponentNotFound(String),
    
    #[error("Incompatible components: {0} and {1}")]
    IncompatibleComponents(String, String),
    
    #[error("Missing dependency: {0} requires {1}")]
    MissingDependency(String, String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    
    #[error("Composition failed: {0}")]
    CompositionFailed(String),
    
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Recipe error: {0}")]
    RecipeError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, BuilderError>;