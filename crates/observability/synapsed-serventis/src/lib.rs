//! # Synapsed Serventis
//!
//! Rust implementation of the Serventis Java API.
//! 
//! Serventis is a semiotic-inspired observability framework designed to provide structured sensing and
//! sense-making for distributed systems. It defines a contract for monitoring system states and service 
//! interactions through a standardized language of signals and assessments.
//!
//! ## Core Modules
//!
//! - **Services API** - Captures service-to-service interactions using structured signals and orientations
//! - **Monitors API** - Monitors operational condition of services with confidence levels
//! - **Reporters API** - Reports situational assessments
//! - **Probes API** - Monitors communication outcomes in distributed systems
//! - **Resources API** - Emits signals describing interactions with shared resources
//! - **Queues API** - Emits signals describing interactions with queue-like systems

pub mod monitors;
pub mod probes;
pub mod queues;
pub mod reporters;
pub mod resources;
pub mod services;
pub mod service_ext;

// Re-export main interfaces
pub use monitors::*;
pub use probes::*;
pub use queues::*;
pub use reporters::*;
pub use resources::*;
pub use services::*;
pub use service_ext::*;

// Re-export substrate types
pub use synapsed_substrates::{
    async_trait, Composer, Pipe, Subject, Substrate, 
    Name, State, Arc, mpsc, oneshot, watch
};