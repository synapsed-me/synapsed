//! # Synapsed Substrates
//!
//! Rust implementation of the Substrates Java API.
//! 
//! The Substrates API provides a flexible framework for building event-driven and observability systems
//! by combining concepts of circuits, conduits, channels, pipes, subscribers, subscriptions, and subjects.
//!
//! ## Key Components
//!
//! - **Circuit**: Central processing engine that manages data flow across channels and conduits
//! - **Conduit**: Routes emitted values by channels (producers) to pipes (consumers)  
//! - **Channel**: Subject-based port into an owning conduit's pipeline
//! - **Pipe**: Abstraction for passing typed values along a pipeline
//! - **Subscriber**: Dynamically subscribes to a source and registers pipes with subjects
//! - **Subject**: Hierarchical reference system for observing and addressing entities
//! - **Cortex**: Bootstrap entry point into the Substrates runtime

pub mod circuit;
pub mod circuit_ext;
pub mod cortex;
pub mod cortex_ext;
pub mod pipe;
pub mod path_ext;
pub mod scope_ext;
pub mod source;
pub mod subject;
pub mod subject_ext;
pub mod types;

// Re-export main interfaces - avoiding conflicts
pub use circuit::{
    BasicChannel, BasicCircuit, BasicCurrent, BasicQueue, BasicScope, Channel, Circuit, Clock, 
    ClockCycle, Closure, Composer, Conduit, Container, Current, IdentityComposer, Inlet, 
    PipeComposer, Pool, Queue, Scope, Script, Sink, Tap,
};
pub use circuit_ext::{CircuitExt, ClosureExt, CurrentExt};
pub use cortex::{create_cortex, Cortex, DefaultCortex};
pub use cortex_ext::CortexExt;
pub use path_ext::PathExt;
pub use scope_ext::ScopeExt;
pub use pipe::{
    Assembly, Capture, EmptyPipe, FunctionPipe, Path, Pipe, Sequencer, Sift,
};
pub use source::{BasicRegistrar, BasicSource, BasicSubscription, FunctionSubscriber, Source};
pub use subject::{
    Component, Context, DynComponent, DynContext, DynRegistrar, DynSubscriber,
    Registrar, Resource, Subject, Subscriber, Subscription, Substrate,
};
pub use subject_ext::{ComponentExt, ContextExt, RegistrarExt, SubscriberExt};
pub use types::{
    Id, Name, Slot, State, StateValue, SubjectType, SubstratesError, SubstratesResult,
};

// Re-export common traits and types
pub use async_trait::async_trait;
pub use std::sync::Arc;
pub use tokio::sync::{mpsc, oneshot, watch};
pub use uuid::Uuid;

// Re-export core types for better integration
pub use synapsed_core::{SynapsedError, SynapsedResult};
pub use synapsed_core::traits::{Observable, Configurable, Identifiable, Validatable};
pub use synapsed_core::observability::*;

// Map SubstratesError to SynapsedError
impl From<types::SubstratesError> for SynapsedError {
    fn from(err: types::SubstratesError) -> Self {
        match err {
            types::SubstratesError::NotFound(msg) => SynapsedError::NotFound(msg),
            types::SubstratesError::Closed(msg) => SynapsedError::Internal(format!("Resource closed: {}", msg)),
            types::SubstratesError::InvalidOperation(msg) => SynapsedError::InvalidInput(msg),
            types::SubstratesError::ChannelError(msg) => SynapsedError::Internal(format!("Channel error: {}", msg)),
            types::SubstratesError::Internal(msg) => SynapsedError::Internal(msg),
        }
    }
}