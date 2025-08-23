//! # Synapsed Monitor
//!
//! Human-centric monitoring interface that combines data from Substrates and Serventis
//! to provide meaningful, task-focused insights into synapsed-intent operations.
//!
//! ## Key Features
//! - Task-centric view of intent execution
//! - Agent behavior monitoring with trust metrics
//! - Natural language event narration
//! - Real-time WebSocket updates
//! - Visual task journey mapping
//! - System health interpretation

pub mod collector;
pub mod aggregator;
pub mod views;
pub mod narrator;
// pub mod visualizations; // TODO: Add visualization support
pub mod server;

// Re-export main types
pub use collector::{ObservabilityCollector, CollectorConfig};
pub use aggregator::{EventAggregator, CorrelatedEvent};
pub use views::{TaskView, AgentView, SystemHealthView};
pub use narrator::{EventNarrator, Narrative};
pub use server::{MonitorServer, ServerConfig};

use thiserror::Error;

/// Monitor-specific error types
#[derive(Error, Debug)]
pub enum MonitorError {
    #[error("Collection error: {0}")]
    CollectionError(String),
    
    #[error("Aggregation error: {0}")]
    AggregationError(String),
    
    #[error("Narration error: {0}")]
    NarrationError(String),
    
    #[error("Server error: {0}")]
    ServerError(String),
    
    #[error("Substrates error: {0}")]
    SubstratesError(#[from] synapsed_substrates::types::SubstratesError),
    
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, MonitorError>;