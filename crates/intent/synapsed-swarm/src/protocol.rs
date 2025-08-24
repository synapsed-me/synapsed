//! Agent communication protocol for swarm coordination

use crate::{error::SwarmResult, types::*};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::{debug, trace};

/// Version of the agent protocol
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

impl ProtocolVersion {
    pub fn current() -> Self {
        Self {
            major: 1,
            minor: 0,
            patch: 0,
        }
    }
    
    pub fn is_compatible(&self, other: &ProtocolVersion) -> bool {
        self.major == other.major
    }
}

impl std::fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Type of message in the protocol
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    // Coordination messages
    Hello,
    Goodbye,
    Heartbeat,
    
    // Task messages
    TaskRequest,
    TaskAccept,
    TaskReject,
    TaskUpdate,
    TaskComplete,
    TaskFailed,
    
    // Promise messages
    PromiseProposal,
    PromiseAccept,
    PromiseReject,
    PromiseFulfilled,
    PromiseBroken,
    
    // Verification messages
    VerificationRequest,
    VerificationResponse,
    ProofRequest,
    ProofResponse,
    
    // Consensus messages
    ConsensusProposal,
    ConsensusVote,
    ConsensusResult,
    
    // Context messages
    ContextRequest,
    ContextResponse,
    ContextUpdate,
    
    // Trust messages
    TrustQuery,
    TrustReport,
    TrustUpdate,
    
    // Error messages
    Error,
    
    // Custom messages
    Custom(String),
}

/// Message exchanged between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Unique message ID
    pub id: Uuid,
    /// Protocol version
    pub version: ProtocolVersion,
    /// Message type
    pub message_type: MessageType,
    /// Sender agent ID
    pub sender: AgentId,
    /// Recipient agent ID (None for broadcast)
    pub recipient: Option<AgentId>,
    /// Message payload
    pub payload: MessagePayload,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Correlation ID for request/response pairs
    pub correlation_id: Option<Uuid>,
    /// Message metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl AgentMessage {
    /// Create a new message
    pub fn new(
        message_type: MessageType,
        sender: AgentId,
        recipient: Option<AgentId>,
        payload: MessagePayload,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            version: ProtocolVersion::current(),
            message_type,
            sender,
            recipient,
            payload,
            timestamp: Utc::now(),
            correlation_id: None,
            metadata: HashMap::new(),
        }
    }
    
    /// Create a response to a message
    pub fn response_to(
        original: &AgentMessage,
        message_type: MessageType,
        payload: MessagePayload,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            version: ProtocolVersion::current(),
            message_type,
            sender: original.recipient.unwrap_or(Uuid::new_v4()),
            recipient: Some(original.sender),
            payload,
            timestamp: Utc::now(),
            correlation_id: Some(original.id),
            metadata: HashMap::new(),
        }
    }
    
    /// Check if this is a response to another message
    pub fn is_response_to(&self, message_id: Uuid) -> bool {
        self.correlation_id == Some(message_id)
    }
}

/// Payload of a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    /// Empty payload
    Empty,
    
    /// Text payload
    Text(String),
    
    /// JSON payload
    Json(serde_json::Value),
    
    /// Task assignment
    TaskAssignment(TaskAssignment),
    
    /// Task result
    TaskResult(TaskResult),
    
    /// Promise contract
    PromiseContract(synapsed_promise::PromiseContract),
    
    /// Promise
    Promise(synapsed_promise::Promise),
    
    /// Intent
    Intent(synapsed_intent::HierarchicalIntent),
    
    /// Context
    Context(HashMap<String, serde_json::Value>),
    
    /// Verification result
    VerificationResult(synapsed_verify::VerificationResult),
    
    /// Verification proof
    VerificationProof(synapsed_verify::VerificationProof),
    
    /// Trust score
    TrustScore {
        agent_id: AgentId,
        score: f64,
        confidence: f64,
    },
    
    /// Consensus proposal
    ConsensusProposal {
        proposal_id: Uuid,
        topic: String,
        options: Vec<String>,
        deadline: DateTime<Utc>,
    },
    
    /// Consensus vote
    ConsensusVote {
        proposal_id: Uuid,
        vote: String,
        justification: Option<String>,
    },
    
    /// Error information
    Error {
        code: String,
        message: String,
        details: Option<serde_json::Value>,
    },
}

/// Agent protocol handler
pub struct AgentProtocol {
    /// Message handlers
    handlers: HashMap<MessageType, Box<dyn Fn(&AgentMessage) -> SwarmResult<Option<AgentMessage>> + Send + Sync>>,
    /// Message history
    history: Vec<AgentMessage>,
    /// Maximum history size
    max_history: usize,
}

impl AgentProtocol {
    /// Create a new protocol handler
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            history: Vec::new(),
            max_history: 1000,
        }
    }
    
    /// Register a message handler
    pub fn register_handler<F>(&mut self, message_type: MessageType, handler: F)
    where
        F: Fn(&AgentMessage) -> SwarmResult<Option<AgentMessage>> + Send + Sync + 'static,
    {
        self.handlers.insert(message_type, Box::new(handler));
    }
    
    /// Process an incoming message
    pub fn process_message(&mut self, message: &AgentMessage) -> SwarmResult<Option<AgentMessage>> {
        debug!(
            "Processing {} message from {} to {:?}",
            message.message_type.to_string(),
            message.sender,
            message.recipient
        );
        
        // Check protocol version
        if !message.version.is_compatible(&ProtocolVersion::current()) {
            return Ok(Some(AgentMessage::new(
                MessageType::Error,
                message.recipient.unwrap_or(Uuid::new_v4()),
                Some(message.sender),
                MessagePayload::Error {
                    code: "PROTOCOL_VERSION_MISMATCH".to_string(),
                    message: format!(
                        "Incompatible protocol version: {} (expected {})",
                        message.version,
                        ProtocolVersion::current()
                    ),
                    details: None,
                },
            )));
        }
        
        // Store in history
        self.add_to_history(message.clone());
        
        // Find and execute handler
        if let Some(handler) = self.handlers.get(&message.message_type) {
            handler(message)
        } else {
            trace!("No handler for message type: {:?}", message.message_type);
            Ok(None)
        }
    }
    
    /// Create a hello message
    pub fn create_hello(sender: AgentId, capabilities: Vec<String>) -> AgentMessage {
        AgentMessage::new(
            MessageType::Hello,
            sender,
            None,
            MessagePayload::Json(serde_json::json!({
                "capabilities": capabilities,
                "protocol_version": ProtocolVersion::current(),
            })),
        )
    }
    
    /// Create a task request message
    pub fn create_task_request(
        sender: AgentId,
        recipient: AgentId,
        assignment: TaskAssignment,
    ) -> AgentMessage {
        AgentMessage::new(
            MessageType::TaskRequest,
            sender,
            Some(recipient),
            MessagePayload::TaskAssignment(assignment),
        )
    }
    
    /// Create a task complete message
    pub fn create_task_complete(
        sender: AgentId,
        recipient: AgentId,
        result: TaskResult,
        original_request: &AgentMessage,
    ) -> AgentMessage {
        let mut message = AgentMessage::new(
            MessageType::TaskComplete,
            sender,
            Some(recipient),
            MessagePayload::TaskResult(result),
        );
        message.correlation_id = Some(original_request.id);
        message
    }
    
    /// Create a promise proposal message
    pub fn create_promise_proposal(
        sender: AgentId,
        recipient: AgentId,
        contract: synapsed_promise::PromiseContract,
    ) -> AgentMessage {
        AgentMessage::new(
            MessageType::PromiseProposal,
            sender,
            Some(recipient),
            MessagePayload::PromiseContract(contract),
        )
    }
    
    /// Create a verification request message
    pub fn create_verification_request(
        sender: AgentId,
        recipient: AgentId,
        task_id: TaskId,
        intent: synapsed_intent::HierarchicalIntent,
    ) -> AgentMessage {
        AgentMessage::new(
            MessageType::VerificationRequest,
            sender,
            Some(recipient),
            MessagePayload::Json(serde_json::json!({
                "task_id": task_id,
                "intent": intent,
            })),
        )
    }
    
    /// Get message history
    pub fn history(&self) -> &[AgentMessage] {
        &self.history
    }
    
    /// Add message to history
    fn add_to_history(&mut self, message: AgentMessage) {
        self.history.push(message);
        
        // Trim history if too large
        if self.history.len() > self.max_history {
            self.history.drain(0..100);
        }
    }
}

impl MessageType {
    /// Convert to string representation
    pub fn to_string(&self) -> String {
        match self {
            MessageType::Hello => "HELLO",
            MessageType::Goodbye => "GOODBYE",
            MessageType::Heartbeat => "HEARTBEAT",
            MessageType::TaskRequest => "TASK_REQUEST",
            MessageType::TaskAccept => "TASK_ACCEPT",
            MessageType::TaskReject => "TASK_REJECT",
            MessageType::TaskUpdate => "TASK_UPDATE",
            MessageType::TaskComplete => "TASK_COMPLETE",
            MessageType::TaskFailed => "TASK_FAILED",
            MessageType::PromiseProposal => "PROMISE_PROPOSAL",
            MessageType::PromiseAccept => "PROMISE_ACCEPT",
            MessageType::PromiseReject => "PROMISE_REJECT",
            MessageType::PromiseFulfilled => "PROMISE_FULFILLED",
            MessageType::PromiseBroken => "PROMISE_BROKEN",
            MessageType::VerificationRequest => "VERIFICATION_REQUEST",
            MessageType::VerificationResponse => "VERIFICATION_RESPONSE",
            MessageType::ProofRequest => "PROOF_REQUEST",
            MessageType::ProofResponse => "PROOF_RESPONSE",
            MessageType::ConsensusProposal => "CONSENSUS_PROPOSAL",
            MessageType::ConsensusVote => "CONSENSUS_VOTE",
            MessageType::ConsensusResult => "CONSENSUS_RESULT",
            MessageType::ContextRequest => "CONTEXT_REQUEST",
            MessageType::ContextResponse => "CONTEXT_RESPONSE",
            MessageType::ContextUpdate => "CONTEXT_UPDATE",
            MessageType::TrustQuery => "TRUST_QUERY",
            MessageType::TrustReport => "TRUST_REPORT",
            MessageType::TrustUpdate => "TRUST_UPDATE",
            MessageType::Error => "ERROR",
            MessageType::Custom(s) => s,
        }.to_string()
    }
}