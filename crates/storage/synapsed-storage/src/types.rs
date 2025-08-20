//! Type definitions for the storage module

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Peer identifier for distributed systems
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(pub String);

/// Sync statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStats {
    pub items_sent: usize,
    pub items_received: usize,
    pub conflicts_resolved: usize,
    pub duration_ms: u64,
}

/// Conflict information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub key: Vec<u8>,
    pub local_value: Option<Vec<u8>>,
    pub remote_value: Option<Vec<u8>>,
    pub local_timestamp: u64,
    pub remote_timestamp: u64,
}

/// Conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Resolution {
    KeepLocal,
    KeepRemote,
    Merge(Vec<u8>),
}

/// Query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub documents: Vec<Document>,
    pub total_count: usize,
    pub has_more: bool,
}

/// Document type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub data: serde_json::Value,
    pub metadata: DocumentMetadata,
}

/// Document metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub created_at: u64,
    pub updated_at: u64,
    pub version: u64,
}

/// Stream writer for blob storage
pub struct StreamWriter {
    // Implementation details would go here
}

/// Stream reader for blob storage
pub struct StreamReader {
    // Implementation details would go here
}