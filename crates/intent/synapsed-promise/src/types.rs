//! Core types for Promise Theory implementation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Unique identifier for an agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub Uuid);

impl AgentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for a promise
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PromiseId(pub Uuid);

impl PromiseId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for PromiseId {
    fn default() -> Self {
        Self::new()
    }
}

/// Type of promise being made
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromiseType {
    /// Promise to provide a service or resource
    Offer,
    /// Promise to use or accept a service
    Use,
    /// Promise to delegate to another agent
    Delegate,
    /// Promise to give information
    Give,
    /// Promise to accept information
    Accept,
}

/// Scope of a promise
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromiseScope {
    /// Promise applies to a specific agent
    Agent(AgentId),
    /// Promise applies to a group of agents
    Group(Vec<AgentId>),
    /// Promise applies to all agents
    Universal,
    /// Promise applies to agents matching criteria
    Conditional(HashMap<String, String>),
}

/// Body of a promise - what is being promised
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromiseBody {
    /// What is being promised
    pub content: String,
    /// Constraints on the promise
    pub constraints: Vec<Constraint>,
    /// Quality of service guarantees
    pub qos: Option<QualityOfService>,
    /// Metadata about the promise
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Constraint on a promise
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// Type of constraint
    pub constraint_type: ConstraintType,
    /// Constraint value
    pub value: serde_json::Value,
}

/// Types of constraints
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintType {
    /// Time-based constraint
    Temporal,
    /// Resource constraint
    Resource,
    /// Dependency on another promise
    Dependency,
    /// Security constraint
    Security,
    /// Performance constraint
    Performance,
}

/// Quality of Service guarantees
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityOfService {
    /// Availability guarantee (0.0 to 1.0)
    pub availability: f64,
    /// Response time in milliseconds
    pub response_time_ms: Option<u64>,
    /// Throughput guarantee
    pub throughput: Option<u64>,
    /// Reliability guarantee (0.0 to 1.0)
    pub reliability: f64,
}

/// Imposition - an external expectation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Imposition {
    /// ID of the imposition
    pub id: Uuid,
    /// Agent making the imposition
    pub from: AgentId,
    /// Agent receiving the imposition
    pub to: AgentId,
    /// What is being imposed
    pub body: PromiseBody,
    /// When the imposition was made
    pub timestamp: DateTime<Utc>,
}

/// Assessment of a promise outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assessment {
    /// ID of the assessment
    pub id: Uuid,
    /// Promise being assessed
    pub promise_id: PromiseId,
    /// Agent making the assessment
    pub assessor: AgentId,
    /// Whether the promise was kept
    pub kept: bool,
    /// Quality score (0.0 to 1.0)
    pub quality: f64,
    /// Evidence for the assessment
    pub evidence: Vec<Evidence>,
    /// When the assessment was made
    pub timestamp: DateTime<Utc>,
}

/// Evidence for an assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Type of evidence
    pub evidence_type: EvidenceType,
    /// Evidence data
    pub data: serde_json::Value,
    /// Cryptographic proof if available
    pub proof: Option<Vec<u8>>,
}

/// Types of evidence
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceType {
    /// Log entry evidence
    Log,
    /// Metric evidence
    Metric,
    /// State snapshot
    StateSnapshot,
    /// External verification
    ExternalVerification,
    /// Cryptographic proof
    CryptographicProof,
}

/// Communication channel between agents
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Channel {
    /// Direct peer-to-peer channel
    Direct(AgentId),
    /// Broadcast channel
    Broadcast,
    /// Topic-based pub/sub channel
    Topic(String),
    /// Hierarchical channel
    Hierarchical(Vec<AgentId>),
}

/// Message between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Message ID
    pub id: Uuid,
    /// Sender agent
    pub from: AgentId,
    /// Recipient agent(s)
    pub to: PromiseScope,
    /// Message type
    pub message_type: MessageType,
    /// Message payload
    pub payload: serde_json::Value,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Optional correlation ID for request-response
    pub correlation_id: Option<Uuid>,
}

/// Types of messages between agents
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    /// Promise declaration
    PromiseDeclaration,
    /// Promise acceptance
    PromiseAcceptance,
    /// Promise rejection
    PromiseRejection,
    /// Cooperation request
    CooperationRequest,
    /// Cooperation response
    CooperationResponse,
    /// Assessment report
    AssessmentReport,
    /// Trust update
    TrustUpdate,
    /// Heartbeat/keepalive
    Heartbeat,
}