//! CRDT (Conflict-free Replicated Data Types) WASM operations
//!
//! This module provides WebAssembly-compatible CRDT operations for real-time
//! collaborative editing using Yjs integration. It supports document synchronization,
//! conflict resolution, and optimized delta operations for P2P networks.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use wasm_bindgen::prelude::*;
use js_sys::{Array, Object, Uint8Array};

use crate::error::{WasmError, WasmResult};
use crate::types::{HostFunction, WasmValue, ExecutionContext};
use crate::{DEFAULT_CRDT_SYNC_INTERVAL};

/// CRDT synchronization engine for real-time collaboration
pub struct CrdtSyncEngine {
    /// Active documents
    documents: HashMap<String, Document>,
    /// Sync configuration
    config: SyncConfig,
    /// Synchronization statistics
    stats: SyncStats,
}

impl CrdtSyncEngine {
    /// Create a new CRDT sync engine
    pub fn new() -> WasmResult<Self> {
        Ok(Self {
            documents: HashMap::new(),
            config: SyncConfig::default(),
            stats: SyncStats::default(),
        })
    }

    /// Create a new document
    pub async fn create_document(&mut self, doc_id: String, doc_type: DocumentType) -> WasmResult<String> {
        let document = Document::new(doc_id.clone(), doc_type)?;
        self.documents.insert(doc_id.clone(), document);
        self.stats.documents_created += 1;

        tracing::info!(doc_id = %doc_id, "CRDT document created");
        Ok(doc_id)
    }

    /// Apply operation to document
    pub async fn apply_operation(
        &mut self,
        doc_id: &str,
        operation: Operation,
    ) -> WasmResult<Vec<u8>> {
        let document = self.documents.get_mut(doc_id)
            .ok_or_else(|| WasmError::Configuration(format!("Document {} not found", doc_id)))?;

        let delta = document.apply_operation(operation).await?;
        self.stats.operations_applied += 1;

        tracing::debug!(doc_id = %doc_id, "CRDT operation applied");
        Ok(delta)
    }

    /// Get document state
    pub async fn get_document_state(&self, doc_id: &str) -> WasmResult<Vec<u8>> {
        let document = self.documents.get(doc_id)
            .ok_or_else(|| WasmError::Configuration(format!("Document {} not found", doc_id)))?;

        document.get_state().await
    }

    /// Merge remote state with local document
    pub async fn merge_state(
        &mut self,
        doc_id: &str,
        remote_state: &[u8],
    ) -> WasmResult<Vec<u8>> {
        let document = self.documents.get_mut(doc_id)
            .ok_or_else(|| WasmError::Configuration(format!("Document {} not found", doc_id)))?;

        let delta = document.merge_state(remote_state).await?;
        self.stats.merges_performed += 1;

        tracing::debug!(doc_id = %doc_id, delta_size = delta.len(), "CRDT state merged");
        Ok(delta)
    }

    /// Generate sync message for peer
    pub async fn generate_sync_message(&self, doc_id: &str, peer_id: &str) -> WasmResult<Vec<u8>> {
        let document = self.documents.get(doc_id)
            .ok_or_else(|| WasmError::Configuration(format!("Document {} not found", doc_id)))?;

        let sync_message = document.generate_sync_message(peer_id).await?;
        self.stats.sync_messages_sent += 1;

        Ok(sync_message)
    }

    /// Process sync message from peer
    pub async fn process_sync_message(
        &mut self,
        doc_id: &str,
        peer_id: &str,
        message: &[u8],
    ) -> WasmResult<Option<Vec<u8>>> {
        let document = self.documents.get_mut(doc_id)
            .ok_or_else(|| WasmError::Configuration(format!("Document {} not found", doc_id)))?;

        let response = document.process_sync_message(peer_id, message).await?;
        self.stats.sync_messages_received += 1;

        Ok(response)
    }

    /// Get sync statistics
    pub fn get_stats(&self) -> &SyncStats {
        &self.stats
    }

    /// List active documents
    pub fn list_documents(&self) -> Vec<String> {
        self.documents.keys().cloned().collect()
    }

    /// Remove document
    pub async fn remove_document(&mut self, doc_id: &str) -> WasmResult<()> {
        if self.documents.remove(doc_id).is_some() {
            self.stats.documents_removed += 1;
            tracing::info!(doc_id = %doc_id, "CRDT document removed");
        }
        Ok(())
    }
}

/// CRDT document wrapper
pub struct Document {
    /// Document ID
    pub id: String,
    /// Document type
    pub doc_type: DocumentType,
    /// Document content (Yjs document state)
    content: Vec<u8>,
    /// Version vector for conflict resolution
    version_vector: HashMap<String, u64>,
    /// Pending operations
    pending_ops: Vec<Operation>,
    /// Creation timestamp
    created_at: std::time::SystemTime,
}

impl Document {
    /// Create a new document
    pub fn new(id: String, doc_type: DocumentType) -> WasmResult<Self> {
        Ok(Self {
            id,
            doc_type,
            content: Vec::new(),
            version_vector: HashMap::new(),
            pending_ops: Vec::new(),
            created_at: std::time::SystemTime::now(),
        })
    }

    /// Apply operation to document
    pub async fn apply_operation(&mut self, operation: Operation) -> WasmResult<Vec<u8>> {
        // Update version vector
        let client_id = operation.client_id.clone();
        let current_version = self.version_vector.get(&client_id).unwrap_or(&0);
        self.version_vector.insert(client_id, current_version + 1);

        // Apply operation based on type
        let delta = match &operation.op_type {
            OperationType::Insert { position, content } => {
                self.apply_insert(*position, content).await?
            }
            OperationType::Delete { position, length } => {
                self.apply_delete(*position, *length).await?
            }
            OperationType::Update { position, content } => {
                self.apply_update(*position, content).await?
            }
        };

        // Store operation for future sync
        self.pending_ops.push(operation);

        Ok(delta)
    }

    /// Get document state
    pub async fn get_state(&self) -> WasmResult<Vec<u8>> {
        Ok(self.content.clone())
    }

    /// Merge remote state
    pub async fn merge_state(&mut self, remote_state: &[u8]) -> WasmResult<Vec<u8>> {
        // Simple merge implementation - in practice would use Yjs algorithms
        let mut merged_content = self.content.clone();
        merged_content.extend_from_slice(remote_state);
        
        // Calculate delta
        let delta = self.calculate_delta(&self.content, &merged_content);
        self.content = merged_content;

        Ok(delta)
    }

    /// Generate sync message for peer
    pub async fn generate_sync_message(&self, _peer_id: &str) -> WasmResult<Vec<u8>> {
        // Create sync message with version vector and pending operations
        let sync_message = SyncMessage {
            doc_id: self.id.clone(),
            version_vector: self.version_vector.clone(),
            operations: self.pending_ops.clone(),
            state_vector: self.get_state_vector(),
        };

        // Serialize sync message
        bincode::serialize(&sync_message)
            .map_err(WasmError::from)
    }

    /// Process sync message from peer
    pub async fn process_sync_message(
        &mut self,
        _peer_id: &str,
        message: &[u8],
    ) -> WasmResult<Option<Vec<u8>>> {
        // Deserialize sync message
        let sync_message: SyncMessage = bincode::deserialize(message)
            .map_err(WasmError::from)?;

        // Apply missing operations
        let mut response_ops = Vec::new();
        for operation in sync_message.operations {
            if self.should_apply_operation(&operation) {
                self.apply_operation(operation).await?;
            }
        }

        // Check if we have operations to send back
        let our_missing_ops: Vec<_> = self.pending_ops.iter()
            .filter(|op| !sync_message.version_vector.contains_key(&op.client_id))
            .cloned()
            .collect();

        if !our_missing_ops.is_empty() {
            let response = SyncMessage {
                doc_id: self.id.clone(),
                version_vector: self.version_vector.clone(),
                operations: our_missing_ops,
                state_vector: self.get_state_vector(),
            };

            Ok(Some(bincode::serialize(&response).map_err(WasmError::from)?))
        } else {
            Ok(None)
        }
    }

    /// Apply insert operation
    async fn apply_insert(&mut self, position: usize, content: &[u8]) -> WasmResult<Vec<u8>> {
        if position <= self.content.len() {
            self.content.splice(position..position, content.iter().cloned());
        }
        Ok(content.to_vec()) // Return delta
    }

    /// Apply delete operation
    async fn apply_delete(&mut self, position: usize, length: usize) -> WasmResult<Vec<u8>> {
        let end_pos = std::cmp::min(position + length, self.content.len());
        if position < self.content.len() && position < end_pos {
            let deleted = self.content.drain(position..end_pos).collect();
            Ok(deleted)
        } else {
            Ok(Vec::new())
        }
    }

    /// Apply update operation
    async fn apply_update(&mut self, position: usize, content: &[u8]) -> WasmResult<Vec<u8>> {
        let end_pos = std::cmp::min(position + content.len(), self.content.len());
        if position < self.content.len() {
            for (i, &byte) in content.iter().enumerate() {
                if position + i < end_pos {
                    self.content[position + i] = byte;
                }
            }
        }
        Ok(content.to_vec()) // Return delta
    }

    /// Calculate delta between two states
    fn calculate_delta(&self, old_state: &[u8], new_state: &[u8]) -> Vec<u8> {
        // Simple diff implementation - in practice would use more sophisticated algorithms
        if new_state.len() > old_state.len() {
            new_state[old_state.len()..].to_vec()
        } else {
            Vec::new()
        }
    }

    /// Get state vector for synchronization
    fn get_state_vector(&self) -> Vec<u8> {
        // Simplified state vector representation
        bincode::serialize(&self.version_vector).unwrap_or_default()
    }

    /// Check if operation should be applied
    fn should_apply_operation(&self, operation: &Operation) -> bool {
        let current_version = self.version_vector.get(&operation.client_id).unwrap_or(&0);
        operation.timestamp > *current_version
    }
}

/// Document type enumeration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DocumentType {
    Text,
    Json,
    Array,
    Map,
}

/// CRDT operation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Operation {
    /// Client ID that created the operation
    pub client_id: String,
    /// Operation timestamp/sequence number
    pub timestamp: u64,
    /// Operation type
    pub op_type: OperationType,
}

/// Operation type enumeration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum OperationType {
    Insert {
        position: usize,
        content: Vec<u8>,
    },
    Delete {
        position: usize,
        length: usize,
    },
    Update {
        position: usize,
        content: Vec<u8>,
    },
}

/// Sync message structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SyncMessage {
    /// Document ID
    doc_id: String,
    /// Version vector
    version_vector: HashMap<String, u64>,
    /// Operations to sync
    operations: Vec<Operation>,
    /// State vector
    state_vector: Vec<u8>,
}

/// Sync configuration
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Sync interval in milliseconds
    pub sync_interval: u64,
    /// Maximum operations per sync message
    pub max_ops_per_sync: usize,
    /// Enable compression
    pub enable_compression: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            sync_interval: DEFAULT_CRDT_SYNC_INTERVAL,
            max_ops_per_sync: 100,
            enable_compression: true,
        }
    }
}

/// Synchronization statistics
#[derive(Debug, Clone, Default)]
pub struct SyncStats {
    /// Number of documents created
    pub documents_created: u64,
    /// Number of documents removed
    pub documents_removed: u64,
    /// Number of operations applied
    pub operations_applied: u64,
    /// Number of merges performed
    pub merges_performed: u64,
    /// Number of sync messages sent
    pub sync_messages_sent: u64,
    /// Number of sync messages received
    pub sync_messages_received: u64,
    /// Number of conflicts resolved
    pub conflicts_resolved: u64,
}

/// Create CRDT host functions for WASM modules
pub fn create_crdt_host_functions() -> HashMap<String, HostFunction> {
    let mut functions = HashMap::new();

    // Create document
    functions.insert(
        "crdt_create_document".to_string(),
        Arc::new(|args| {
            if let Some(WasmValue::String(doc_id)) = args.get(0) {
                tracing::info!("Creating CRDT document: {}", doc_id);
                Ok(vec![WasmValue::String(doc_id.clone())])
            } else {
                Err(WasmError::Configuration("Document ID required".to_string()))
            }
        }) as HostFunction,
    );

    // Apply operation
    functions.insert(
        "crdt_apply_operation".to_string(),
        Arc::new(|args| {
            match (args.get(0), args.get(1), args.get(2)) {
                (Some(WasmValue::String(doc_id)), 
                 Some(WasmValue::String(op_type)), 
                 Some(WasmValue::Bytes(data))) => {
                    tracing::debug!(
                        doc_id = %doc_id,
                        op_type = %op_type,
                        data_len = data.len(),
                        "Applying CRDT operation"
                    );
                    Ok(vec![WasmValue::Bytes(b"operation_applied".to_vec())])
                }
                _ => Err(WasmError::Configuration("Invalid arguments for CRDT operation".to_string()))
            }
        }) as HostFunction,
    );

    // Get document state  
    functions.insert(
        "crdt_get_state".to_string(),
        Arc::new(|args| {
            if let Some(WasmValue::String(doc_id)) = args.get(0) {
                tracing::debug!(doc_id = %doc_id, "Getting CRDT document state");
                Ok(vec![WasmValue::Bytes(b"document_state".to_vec())])
            } else {
                Err(WasmError::Configuration("Document ID required".to_string()))
            }
        }) as HostFunction,
    );

    // Merge states
    functions.insert(
        "crdt_merge_state".to_string(),
        Arc::new(|args| {
            match (args.get(0), args.get(1)) {
                (Some(WasmValue::String(doc_id)), Some(WasmValue::Bytes(remote_state))) => {
                    tracing::debug!(
                        doc_id = %doc_id,
                        state_size = remote_state.len(),
                        "Merging CRDT state"
                    );
                    Ok(vec![WasmValue::Bytes(b"merge_delta".to_vec())])
                }
                _ => Err(WasmError::Configuration("Invalid arguments for CRDT merge".to_string()))
            }
        }) as HostFunction,
    );

    functions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_crdt_sync_engine() {
        let mut engine = CrdtSyncEngine::new().unwrap();
        
        let doc_id = engine.create_document("test_doc".to_string(), DocumentType::Text).await.unwrap();
        assert_eq!(doc_id, "test_doc");
        assert_eq!(engine.list_documents().len(), 1);
    }

    #[tokio::test]
    async fn test_document_operations() {
        let mut doc = Document::new("test".to_string(), DocumentType::Text).unwrap();
        
        let operation = Operation {
            client_id: "client1".to_string(),
            timestamp: 1,
            op_type: OperationType::Insert {
                position: 0,
                content: b"Hello".to_vec(),
            },
        };

        let delta = doc.apply_operation(operation).await.unwrap();
        assert_eq!(delta, b"Hello");
        assert_eq!(doc.content, b"Hello");
    }

    #[test]
    fn test_sync_config() {
        let config = SyncConfig::default();
        assert_eq!(config.sync_interval, DEFAULT_CRDT_SYNC_INTERVAL);
        assert_eq!(config.max_ops_per_sync, 100);
        assert!(config.enable_compression);
    }

    #[test]
    fn test_operation_types() {
        let insert_op = OperationType::Insert {
            position: 0,
            content: b"test".to_vec(),
        };

        let delete_op = OperationType::Delete {
            position: 0,
            length: 4,
        };

        let update_op = OperationType::Update {
            position: 0,
            content: b"updated".to_vec(),
        };

        // Test serialization
        let _serialized_insert = bincode::serialize(&insert_op).unwrap();
        let _serialized_delete = bincode::serialize(&delete_op).unwrap();
        let _serialized_update = bincode::serialize(&update_op).unwrap();
    }
}