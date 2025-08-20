//! Observability integration for the networking layer.
//!
//! This module provides comprehensive observability through:
//! - Direct Substrates integration for real-time event streaming
//! - Edge Serventis integration for service monitoring
//! - Privacy-preserving metrics collection
//! - Extensible architecture for future observability APIs

pub mod context;
pub mod metrics;
pub mod privacy;
pub mod unified;

pub use context::{ObservabilityContext, OBSERVABILITY_CONTEXT};
pub use privacy::PrivacyPreservingObserver;
pub use unified::UnifiedObservability;

use synapsed_substrates::Cortex;
use synapsed_serventis::{Service, State};

/// Observable trait for components that support observability.
pub trait Observable {
    /// Returns the observability handle for this component.
    fn observability(&self) -> ObservabilityHandle;
}

/// Handle for observability operations.
#[derive(Clone)]
pub struct ObservabilityHandle {
    cortex: std::sync::Arc<dyn Cortex>,
    service: std::sync::Arc<dyn Service>,
}

impl ObservabilityHandle {
    /// Creates a new observability handle.
    pub fn new(cortex: std::sync::Arc<dyn Cortex>, service: std::sync::Arc<dyn Service>) -> Self {
        Self { cortex, service }
    }
    
    /// Emits an event through Substrates.
    pub fn emit_event<E: Into<SubstrateEvent>>(&self, event: E) {
        let _event = event.into();
        // Note: Actual event emission would require async context
        // For now, this is a placeholder for the event emission API
        // TODO: Implement proper event emission when cortex API is finalized
    }
    
    /// Updates service state through Serventis.
    pub fn update_state(&self, _state: State) {
        // Note: Actual state update would require async context
        // For now, this is a placeholder for the state update API
        // TODO: Implement proper state update when service API is finalized
    }
}

/// Event types that can be emitted through Substrates.
#[derive(Debug, Clone)]
pub enum SubstrateEvent {
    /// Transport-related events
    Transport(TransportEvent),
    
    /// Security-related events
    Security(SecurityEvent),
    
    /// Privacy-related events
    Privacy(PrivacyEvent),
    
    /// Connection-related events
    Connection(ConnectionEvent),
}

impl SubstrateEvent {
    /// Returns the circuit name for this event.
    pub fn circuit_name(&self) -> &str {
        match self {
            SubstrateEvent::Transport(_) => "net.transport",
            SubstrateEvent::Security(_) => "net.security",
            SubstrateEvent::Privacy(_) => "net.privacy",
            SubstrateEvent::Connection(_) => "net.connection",
        }
    }
    
    /// Returns the channel name for this event.
    pub fn channel_name(&self) -> &str {
        match self {
            SubstrateEvent::Transport(e) => e.channel_name(),
            SubstrateEvent::Security(e) => e.channel_name(),
            SubstrateEvent::Privacy(e) => e.channel_name(),
            SubstrateEvent::Connection(e) => e.channel_name(),
        }
    }
    
    /// Converts the event to a Substrate value.
    pub fn into_value(self) -> serde_json::Value {
        match self {
            SubstrateEvent::Transport(e) => serde_json::to_value(e).unwrap_or_default(),
            SubstrateEvent::Security(e) => serde_json::to_value(e).unwrap_or_default(),
            SubstrateEvent::Privacy(e) => serde_json::to_value(e).unwrap_or_default(),
            SubstrateEvent::Connection(e) => serde_json::to_value(e).unwrap_or_default(),
        }
    }
}

/// Transport events.
#[derive(Debug, Clone, serde::Serialize)]
pub enum TransportEvent {
    /// Transport registered
    TransportRegistered {
        transport_type: String,
    },
    
    /// Connection attempt started
    ConnectionAttempt {
        peer: crate::types::AnonymizedPeerInfo,
        #[serde(skip)]
        timestamp: std::time::Instant,
        transport_type: String,
    },
    
    /// Connection established
    ConnectionEstablished {
        duration: std::time::Duration,
        protocol_version: String,
    },
    
    /// Connection failed
    ConnectionFailed {
        duration: std::time::Duration,
        error_type: crate::error::ErrorClass,
    },
    
    /// Data sent
    DataSent {
        bytes: usize,
        duration: std::time::Duration,
    },
    
    /// Data received
    DataReceived {
        bytes: usize,
    },
}

impl TransportEvent {
    fn channel_name(&self) -> &str {
        match self {
            TransportEvent::TransportRegistered { .. } => "transport.registered",
            TransportEvent::ConnectionAttempt { .. } => "connection.attempts",
            TransportEvent::ConnectionEstablished { .. } => "connection.established",
            TransportEvent::ConnectionFailed { .. } => "connection.failed",
            TransportEvent::DataSent { .. } => "data.sent",
            TransportEvent::DataReceived { .. } => "data.received",
        }
    }
}

/// Security events.
#[derive(Debug, Clone, serde::Serialize)]
pub enum SecurityEvent {
    /// Handshake started
    HandshakeStarted {
        protocol: String,
        #[serde(skip)]
        timestamp: std::time::Instant,
    },
    
    /// Handshake completed
    HandshakeCompleted {
        duration: std::time::Duration,
        cipher_suite: String,
    },
    
    /// Authentication attempt
    AuthenticationAttempt {
        method: String,
        #[serde(skip)]
        timestamp: std::time::Instant,
    },
    
    /// Authentication result
    AuthenticationResult {
        success: bool,
        duration: std::time::Duration,
    },
}

impl SecurityEvent {
    fn channel_name(&self) -> &str {
        match self {
            SecurityEvent::HandshakeStarted { .. } => "handshake.started",
            SecurityEvent::HandshakeCompleted { .. } => "handshake.completed",
            SecurityEvent::AuthenticationAttempt { .. } => "auth.attempt",
            SecurityEvent::AuthenticationResult { .. } => "auth.result",
        }
    }
}

/// Privacy events.
#[derive(Debug, Clone, serde::Serialize)]
pub enum PrivacyEvent {
    /// Anonymous route created
    AnonymousRouteCreated {
        hop_count: usize,
        #[serde(skip)]
        timestamp: std::time::Instant,
    },
    
    /// Mix packet sent
    MixPacketSent {
        layer_count: usize,
    },
    
    /// Cover traffic generated
    CoverTrafficGenerated {
        bytes: usize,
    },
}

impl PrivacyEvent {
    fn channel_name(&self) -> &str {
        match self {
            PrivacyEvent::AnonymousRouteCreated { .. } => "route.created",
            PrivacyEvent::MixPacketSent { .. } => "mix.sent",
            PrivacyEvent::CoverTrafficGenerated { .. } => "cover.generated",
        }
    }
}

/// Connection events.
#[derive(Debug, Clone, serde::Serialize)]
pub enum ConnectionEvent {
    /// Connection opened
    Opened {
        connection_id: String,
        transport: crate::types::TransportType,
    },
    
    /// Connection closed
    Closed {
        connection_id: String,
        reason: String,
        duration: std::time::Duration,
    },
    
    /// Connection metrics update
    MetricsUpdate {
        connection_id: String,
        bytes_sent: u64,
        bytes_received: u64,
        rtt_ms: Option<u64>,
    },
}

impl ConnectionEvent {
    fn channel_name(&self) -> &str {
        match self {
            ConnectionEvent::Opened { .. } => "opened",
            ConnectionEvent::Closed { .. } => "closed",
            ConnectionEvent::MetricsUpdate { .. } => "metrics",
        }
    }
}