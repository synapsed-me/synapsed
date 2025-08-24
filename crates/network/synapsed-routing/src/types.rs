//! Core routing types

use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// Node identifier in the network
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    
    pub fn from_string(id: String) -> Self {
        Self(id)
    }
}

/// Anonymous circuit through the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Circuit {
    pub id: String,
    pub nodes: Vec<NodeId>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl Circuit {
    pub fn new(nodes: Vec<NodeId>, lifetime_secs: u64) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            nodes,
            created_at: now,
            expires_at: now + chrono::Duration::seconds(lifetime_secs as i64),
        }
    }
    
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now() > self.expires_at
    }
}

/// Message payload for routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePayload {
    pub data: Vec<u8>,
    pub destination: Option<NodeId>,
    pub reply_to: Option<NodeId>,
}