//! FIPA ACL (Agent Communication Language) implementation
//! 
//! Based on FIPA standards for agent communication with speech act theory,
//! integrated with Promise Theory for voluntary cooperation.

use crate::{types::*, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// FIPA ACL Performative - the type of communicative act
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Performative {
    // Assertives - commit to truth of proposition
    Inform,              // Inform about a fact
    InformIf,           // Inform whether proposition is true
    InformRef,          // Inform value of reference
    Confirm,            // Confirm truth of proposition
    Disconfirm,         // Disconfirm truth of proposition
    
    // Directives - attempt to get receiver to do something
    Request,            // Request an action
    RequestWhen,        // Request action when condition holds
    RequestWhenever,    // Request action whenever condition holds
    QueryIf,            // Query whether proposition is true
    QueryRef,           // Query value of reference
    Subscribe,          // Subscribe to information updates
    
    // Commissives - commit to future action
    Agree,              // Agree to perform requested action
    Refuse,             // Refuse to perform requested action
    
    // Expressives - express psychological state
    Failure,            // Action failed
    NotUnderstood,      // Message not understood
    
    // Declarations - change institutional state
    AcceptProposal,     // Accept a proposal
    RejectProposal,     // Reject a proposal
    Propose,            // Make a proposal
    
    // Control
    Cancel,             // Cancel a previous request
    CallForProposal,    // Request proposals
    Propagate,          // Propagate message to others
}

/// ACL Message structure based on FIPA specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ACLMessage {
    /// Unique message ID
    pub id: MessageId,
    /// The performative (required field)
    pub performative: Performative,
    /// Sender agent
    pub sender: AgentId,
    /// Receiver agent(s)
    pub receiver: Vec<AgentId>,
    /// Reply-to agent(s)
    pub reply_to: Option<Vec<AgentId>>,
    /// Content of the message
    pub content: MessageContent,
    /// Language used for content
    pub language: Option<String>,
    /// Encoding of content
    pub encoding: Option<String>,
    /// Ontology reference
    pub ontology: Option<String>,
    /// Interaction protocol
    pub protocol: Option<InteractionProtocol>,
    /// Conversation ID for threading
    pub conversation_id: Option<ConversationId>,
    /// Message this replies to
    pub in_reply_to: Option<MessageId>,
    /// Reply deadline
    pub reply_by: Option<DateTime<Utc>>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Message ID wrapper
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub Uuid);

impl MessageId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Conversation ID for message threading
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConversationId(pub String);

/// Message content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    /// Plain text content
    Text(String),
    /// Structured JSON content
    Json(serde_json::Value),
    /// Binary content
    Binary(Vec<u8>),
    /// Promise-based content
    Promise(PromiseBody),
    /// Query expression
    Query(QueryExpression),
    /// Proposal content
    Proposal(ProposalContent),
}

/// Query expression for QueryIf/QueryRef
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExpression {
    pub query_type: QueryType,
    pub expression: String,
    pub constraints: Vec<QueryConstraint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryType {
    Boolean,    // Yes/no query
    Reference,  // Value query
    Set,        // Set of values
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryConstraint {
    pub constraint_type: String,
    pub value: serde_json::Value,
}

/// Proposal content for CFP/Propose
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalContent {
    pub proposal_id: Uuid,
    pub description: String,
    pub conditions: Vec<ProposalCondition>,
    pub validity: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalCondition {
    pub condition_type: String,
    pub requirement: String,
    pub negotiable: bool,
}

/// Standard interaction protocols
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractionProtocol {
    /// Simple request-response
    RequestResponse,
    /// Contract Net Protocol
    ContractNet,
    /// Iterated Contract Net
    IteratedContractNet,
    /// English Auction
    EnglishAuction,
    /// Dutch Auction
    DutchAuction,
    /// Subscribe/Notify
    SubscribeNotify,
    /// Custom protocol
    Custom(String),
}

/// Conversation state for managing multi-turn dialogues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationState {
    pub conversation_id: ConversationId,
    pub protocol: InteractionProtocol,
    pub participants: Vec<AgentId>,
    pub initiator: AgentId,
    pub state: ConversationPhase,
    pub messages: Vec<ACLMessage>,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub context: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversationPhase {
    Initiated,
    Negotiating,
    Agreed,
    Executing,
    Completed,
    Failed,
    Cancelled,
}

/// ACL message builder for ergonomic message creation
pub struct ACLMessageBuilder {
    performative: Option<Performative>,
    sender: Option<AgentId>,
    receiver: Vec<AgentId>,
    content: Option<MessageContent>,
    conversation_id: Option<ConversationId>,
    protocol: Option<InteractionProtocol>,
    in_reply_to: Option<MessageId>,
    reply_by: Option<DateTime<Utc>>,
}

impl ACLMessageBuilder {
    pub fn new() -> Self {
        Self {
            performative: None,
            sender: None,
            receiver: Vec::new(),
            content: None,
            conversation_id: None,
            protocol: None,
            in_reply_to: None,
            reply_by: None,
        }
    }
    
    pub fn performative(mut self, p: Performative) -> Self {
        self.performative = Some(p);
        self
    }
    
    pub fn sender(mut self, s: AgentId) -> Self {
        self.sender = Some(s);
        self
    }
    
    pub fn receiver(mut self, r: AgentId) -> Self {
        self.receiver.push(r);
        self
    }
    
    pub fn receivers(mut self, rs: Vec<AgentId>) -> Self {
        self.receiver = rs;
        self
    }
    
    pub fn content(mut self, c: MessageContent) -> Self {
        self.content = Some(c);
        self
    }
    
    pub fn text_content(mut self, text: String) -> Self {
        self.content = Some(MessageContent::Text(text));
        self
    }
    
    pub fn conversation(mut self, id: ConversationId) -> Self {
        self.conversation_id = Some(id);
        self
    }
    
    pub fn protocol(mut self, p: InteractionProtocol) -> Self {
        self.protocol = Some(p);
        self
    }
    
    pub fn reply_to(mut self, msg_id: MessageId) -> Self {
        self.in_reply_to = Some(msg_id);
        self
    }
    
    pub fn reply_by(mut self, deadline: DateTime<Utc>) -> Self {
        self.reply_by = Some(deadline);
        self
    }
    
    pub fn build(self) -> Result<ACLMessage> {
        Ok(ACLMessage {
            id: MessageId::new(),
            performative: self.performative.ok_or_else(|| {
                crate::PromiseError::ValidationFailed("Performative is required".to_string())
            })?,
            sender: self.sender.ok_or_else(|| {
                crate::PromiseError::ValidationFailed("Sender is required".to_string())
            })?,
            receiver: self.receiver,
            reply_to: None,
            content: self.content.ok_or_else(|| {
                crate::PromiseError::ValidationFailed("Content is required".to_string())
            })?,
            language: None,
            encoding: None,
            ontology: None,
            protocol: self.protocol,
            conversation_id: self.conversation_id,
            in_reply_to: self.in_reply_to,
            reply_by: self.reply_by,
            timestamp: Utc::now(),
        })
    }
}

/// Convert Promise types to ACL performatives
impl From<&PromiseType> for Performative {
    fn from(promise_type: &PromiseType) -> Self {
        match promise_type {
            PromiseType::Offer => Performative::Propose,
            PromiseType::Use => Performative::Request,
            PromiseType::Delegate => Performative::RequestWhen,
            PromiseType::Give => Performative::Inform,
            PromiseType::Accept => Performative::Agree,
        }
    }
}

/// Conversation manager for handling multi-turn dialogues
pub struct ConversationManager {
    conversations: HashMap<ConversationId, ConversationState>,
}

impl ConversationManager {
    pub fn new() -> Self {
        Self {
            conversations: HashMap::new(),
        }
    }
    
    /// Start a new conversation
    pub fn start_conversation(
        &mut self,
        initiator: AgentId,
        participants: Vec<AgentId>,
        protocol: InteractionProtocol,
    ) -> ConversationId {
        let id = ConversationId(Uuid::new_v4().to_string());
        let state = ConversationState {
            conversation_id: id.clone(),
            protocol,
            participants,
            initiator,
            state: ConversationPhase::Initiated,
            messages: Vec::new(),
            started_at: Utc::now(),
            last_activity: Utc::now(),
            context: HashMap::new(),
        };
        self.conversations.insert(id.clone(), state);
        id
    }
    
    /// Add message to conversation
    pub fn add_message(
        &mut self,
        conversation_id: &ConversationId,
        message: ACLMessage,
    ) -> Result<()> {
        if let Some(conversation) = self.conversations.get_mut(conversation_id) {
            conversation.messages.push(message);
            conversation.last_activity = Utc::now();
            Ok(())
        } else {
            Err(crate::PromiseError::ValidationFailed(
                "Conversation not found".to_string()
            ))
        }
    }
    
    /// Update conversation phase
    pub fn update_phase(
        &mut self,
        conversation_id: &ConversationId,
        phase: ConversationPhase,
    ) -> Result<()> {
        if let Some(conversation) = self.conversations.get_mut(conversation_id) {
            conversation.state = phase;
            Ok(())
        } else {
            Err(crate::PromiseError::ValidationFailed(
                "Conversation not found".to_string()
            ))
        }
    }
    
    /// Get conversation state
    pub fn get_conversation(&self, id: &ConversationId) -> Option<&ConversationState> {
        self.conversations.get(id)
    }
    
    /// Clean up old conversations
    pub fn cleanup_old_conversations(&mut self, max_age: chrono::Duration) {
        let cutoff = Utc::now() - max_age;
        self.conversations.retain(|_, conv| {
            conv.last_activity > cutoff || 
            !matches!(conv.state, ConversationPhase::Completed | ConversationPhase::Failed | ConversationPhase::Cancelled)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_message_builder() {
        let sender = AgentId::new();
        let receiver = AgentId::new();
        
        let message = ACLMessageBuilder::new()
            .performative(Performative::Request)
            .sender(sender)
            .receiver(receiver)
            .text_content("Please perform task X".to_string())
            .protocol(InteractionProtocol::RequestResponse)
            .build()
            .unwrap();
        
        assert_eq!(message.performative, Performative::Request);
        assert_eq!(message.sender, sender);
        assert!(message.receiver.contains(&receiver));
    }
    
    #[test]
    fn test_conversation_manager() {
        let mut manager = ConversationManager::new();
        let initiator = AgentId::new();
        let participant = AgentId::new();
        
        let conv_id = manager.start_conversation(
            initiator,
            vec![participant],
            InteractionProtocol::RequestResponse,
        );
        
        let conversation = manager.get_conversation(&conv_id).unwrap();
        assert_eq!(conversation.state, ConversationPhase::Initiated);
        assert_eq!(conversation.initiator, initiator);
    }
}