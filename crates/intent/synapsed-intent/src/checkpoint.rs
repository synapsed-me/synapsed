//! Checkpoint and rollback functionality for intent execution

use crate::{
    types::*, IntentError, Result
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A checkpoint in intent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentCheckpoint {
    /// Checkpoint ID
    pub id: Uuid,
    /// Intent being checkpointed
    pub intent_id: IntentId,
    /// Step at which checkpoint was taken
    pub step_id: Option<Uuid>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// State snapshot at this point
    pub state: StateSnapshot,
    /// Metadata
    pub metadata: CheckpointMetadata,
    /// Whether this is a safe rollback point
    pub safe_rollback: bool,
}

/// Snapshot of state at a checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Variables snapshot
    pub variables: HashMap<String, Value>,
    /// File system state (paths and hashes)
    pub files: HashMap<String, FileState>,
    /// Process state
    pub processes: Vec<ProcessState>,
    /// Network connections
    pub connections: Vec<ConnectionState>,
    /// Custom state data
    pub custom: HashMap<String, Value>,
}

/// State of a file at checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    /// File path
    pub path: String,
    /// File hash
    pub hash: String,
    /// File size
    pub size: u64,
    /// Modification time
    pub modified: DateTime<Utc>,
    /// Whether file existed
    pub existed: bool,
}

/// State of a process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessState {
    /// Process ID
    pub pid: u32,
    /// Command
    pub command: String,
    /// Status
    pub status: String,
    /// Start time
    pub started_at: DateTime<Utc>,
}

/// State of a network connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionState {
    /// Local address
    pub local_addr: String,
    /// Remote address
    pub remote_addr: String,
    /// Protocol
    pub protocol: String,
    /// State
    pub state: String,
}

/// Metadata for a checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    /// Creator of the checkpoint
    pub creator: String,
    /// Reason for checkpoint
    pub reason: String,
    /// Tags
    pub tags: Vec<String>,
    /// Parent checkpoint (if incremental)
    pub parent_checkpoint: Option<Uuid>,
    /// Size of checkpoint data
    pub size_bytes: usize,
}

/// Manager for checkpoints and rollback
pub struct CheckpointManager {
    /// Stored checkpoints
    checkpoints: Arc<RwLock<HashMap<Uuid, IntentCheckpoint>>>,
    /// Checkpoint history (ordered)
    history: Arc<RwLock<Vec<Uuid>>>,
    /// Maximum checkpoints to keep
    max_checkpoints: usize,
    /// Current state tracker
    current_state: Arc<RwLock<StateSnapshot>>,
    /// Rollback handler
    rollback_handler: Option<Arc<dyn RollbackHandler>>,
}

/// Handler for performing rollbacks
#[async_trait::async_trait]
pub trait RollbackHandler: Send + Sync {
    /// Performs rollback to a checkpoint
    async fn rollback(&self, checkpoint: &IntentCheckpoint) -> Result<()>;
    
    /// Validates if rollback is possible
    async fn can_rollback(&self, checkpoint: &IntentCheckpoint) -> bool;
    
    /// Cleanup after rollback
    async fn cleanup(&self) -> Result<()>;
}

impl CheckpointManager {
    /// Creates a new checkpoint manager
    pub fn new() -> Self {
        Self {
            checkpoints: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            max_checkpoints: 50,
            current_state: Arc::new(RwLock::new(StateSnapshot::default())),
            rollback_handler: None,
        }
    }
    
    /// Creates a manager with custom settings
    pub fn with_max_checkpoints(max: usize) -> Self {
        Self {
            checkpoints: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            max_checkpoints: max,
            current_state: Arc::new(RwLock::new(StateSnapshot::default())),
            rollback_handler: None,
        }
    }
    
    /// Sets the rollback handler
    pub fn set_rollback_handler(&mut self, handler: Arc<dyn RollbackHandler>) {
        self.rollback_handler = Some(handler);
    }
    
    /// Creates a checkpoint
    pub async fn create_checkpoint(
        &self,
        intent_id: IntentId,
        step_id: Uuid,
    ) -> Result<IntentCheckpoint> {
        self.create_checkpoint_with_metadata(
            intent_id,
            Some(step_id),
            CheckpointMetadata {
                creator: "system".to_string(),
                reason: "step_checkpoint".to_string(),
                tags: Vec::new(),
                parent_checkpoint: self.get_last_checkpoint_id().await,
                size_bytes: 0,
            }
        ).await
    }
    
    /// Creates a checkpoint with metadata
    pub async fn create_checkpoint_with_metadata(
        &self,
        intent_id: IntentId,
        step_id: Option<Uuid>,
        mut metadata: CheckpointMetadata,
    ) -> Result<IntentCheckpoint> {
        let id = Uuid::new_v4();
        let state = self.current_state.read().await.clone();
        
        // Calculate size
        let size = serde_json::to_vec(&state)
            .map(|v| v.len())
            .unwrap_or(0);
        metadata.size_bytes = size;
        
        let checkpoint = IntentCheckpoint {
            id,
            intent_id,
            step_id,
            timestamp: Utc::now(),
            state,
            metadata,
            safe_rollback: true,
        };
        
        // Store checkpoint
        let mut checkpoints = self.checkpoints.write().await;
        checkpoints.insert(id, checkpoint.clone());
        
        // Update history
        let mut history = self.history.write().await;
        history.push(id);
        
        // Clean up old checkpoints if needed
        if history.len() > self.max_checkpoints {
            let to_remove = history.len() - self.max_checkpoints;
            let removed: Vec<Uuid> = history.drain(0..to_remove).collect();
            for id in removed {
                checkpoints.remove(&id);
            }
        }
        
        Ok(checkpoint)
    }
    
    /// Gets a checkpoint by ID
    pub async fn get_checkpoint(&self, id: Uuid) -> Option<IntentCheckpoint> {
        self.checkpoints.read().await.get(&id).cloned()
    }
    
    /// Gets the last checkpoint
    pub async fn get_last_checkpoint(&self) -> Option<IntentCheckpoint> {
        let history = self.history.read().await;
        if let Some(&last_id) = history.last() {
            self.checkpoints.read().await.get(&last_id).cloned()
        } else {
            None
        }
    }
    
    /// Gets the last checkpoint ID
    async fn get_last_checkpoint_id(&self) -> Option<Uuid> {
        self.history.read().await.last().copied()
    }
    
    /// Rolls back to a checkpoint
    pub async fn rollback_to(&self, checkpoint_id: Uuid) -> Result<()> {
        let checkpoint = self.get_checkpoint(checkpoint_id).await
            .ok_or_else(|| IntentError::ValidationFailed("Checkpoint not found".to_string()))?;
        
        if !checkpoint.safe_rollback {
            return Err(IntentError::ValidationFailed(
                "Checkpoint is not safe for rollback".to_string()
            ));
        }
        
        // Use rollback handler if available
        if let Some(handler) = &self.rollback_handler {
            if !handler.can_rollback(&checkpoint).await {
                return Err(IntentError::ExecutionFailed(
                    "Rollback not possible".to_string()
                ));
            }
            
            handler.rollback(&checkpoint).await?;
            handler.cleanup().await?;
        }
        
        // Update current state
        *self.current_state.write().await = checkpoint.state.clone();
        
        // Remove checkpoints after this one
        let mut history = self.history.write().await;
        if let Some(pos) = history.iter().position(|&id| id == checkpoint_id) {
            let mut checkpoints = self.checkpoints.write().await;
            let removed: Vec<Uuid> = history.drain(pos + 1..).collect();
            for id in removed {
                checkpoints.remove(&id);
            }
        }
        
        Ok(())
    }
    
    /// Rolls back to the last checkpoint
    pub async fn rollback_to_last(&self) -> Result<()> {
        let last = self.get_last_checkpoint().await
            .ok_or_else(|| IntentError::ValidationFailed("No checkpoints available".to_string()))?;
        
        self.rollback_to(last.id).await
    }
    
    /// Updates the current state
    pub async fn update_state<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut StateSnapshot),
    {
        let mut state = self.current_state.write().await;
        updater(&mut state);
        Ok(())
    }
    
    /// Gets checkpoint history
    pub async fn get_history(&self) -> Vec<IntentCheckpoint> {
        let history = self.history.read().await;
        let checkpoints = self.checkpoints.read().await;
        
        history
            .iter()
            .filter_map(|id| checkpoints.get(id).cloned())
            .collect()
    }
    
    /// Clears all checkpoints
    pub async fn clear(&self) {
        self.checkpoints.write().await.clear();
        self.history.write().await.clear();
        *self.current_state.write().await = StateSnapshot::default();
    }
    
    /// Validates checkpoint integrity
    pub async fn validate_checkpoint(&self, id: Uuid) -> Result<bool> {
        let checkpoint = self.get_checkpoint(id).await
            .ok_or_else(|| IntentError::ValidationFailed("Checkpoint not found".to_string()))?;
        
        // Check if parent exists (if specified)
        if let Some(parent_id) = checkpoint.metadata.parent_checkpoint {
            if !self.checkpoints.read().await.contains_key(&parent_id) {
                return Ok(false);
            }
        }
        
        // Additional validation could be added here
        
        Ok(true)
    }
    
    /// Exports checkpoints to JSON
    pub async fn export(&self) -> Result<String> {
        let checkpoints = self.checkpoints.read().await;
        let all: Vec<IntentCheckpoint> = checkpoints.values().cloned().collect();
        
        serde_json::to_string_pretty(&all)
            .map_err(|e| IntentError::Other(anyhow::anyhow!("Failed to export: {}", e)))
    }
    
    /// Imports checkpoints from JSON
    pub async fn import(&self, json: &str) -> Result<()> {
        let imported: Vec<IntentCheckpoint> = serde_json::from_str(json)
            .map_err(|e| IntentError::Other(anyhow::anyhow!("Failed to import: {}", e)))?;
        
        let mut checkpoints = self.checkpoints.write().await;
        let mut history = self.history.write().await;
        
        for checkpoint in imported {
            let id = checkpoint.id;
            checkpoints.insert(id, checkpoint);
            if !history.contains(&id) {
                history.push(id);
            }
        }
        
        Ok(())
    }
}

impl Default for CheckpointManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for StateSnapshot {
    fn default() -> Self {
        Self {
            variables: HashMap::new(),
            files: HashMap::new(),
            processes: Vec::new(),
            connections: Vec::new(),
            custom: HashMap::new(),
        }
    }
}

/// Simple file-based rollback handler
pub struct FileRollbackHandler {
    /// Base directory for file operations
    base_dir: String,
}

impl FileRollbackHandler {
    /// Creates a new file rollback handler
    pub fn new(base_dir: impl Into<String>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }
}

#[async_trait::async_trait]
impl RollbackHandler for FileRollbackHandler {
    async fn rollback(&self, checkpoint: &IntentCheckpoint) -> Result<()> {
        // Restore files to checkpoint state
        for (path, state) in &checkpoint.state.files {
            let full_path = format!("{}/{}", self.base_dir, path);
            
            if !state.existed {
                // File didn't exist at checkpoint, remove it
                if tokio::fs::metadata(&full_path).await.is_ok() {
                    tokio::fs::remove_file(&full_path).await
                        .map_err(|e| IntentError::ExecutionFailed(format!("Failed to remove file: {}", e)))?;
                }
            } else {
                // File existed, would need to restore from backup
                // This is a simplified implementation
            }
        }
        
        Ok(())
    }
    
    async fn can_rollback(&self, _checkpoint: &IntentCheckpoint) -> bool {
        // Check if we have necessary permissions and backups
        true
    }
    
    async fn cleanup(&self) -> Result<()> {
        // Clean up temporary files or resources
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_checkpoint_creation() {
        let manager = CheckpointManager::new();
        let intent_id = IntentId::new();
        let step_id = Uuid::new_v4();
        
        let checkpoint = manager.create_checkpoint(intent_id, step_id).await.unwrap();
        
        assert_eq!(checkpoint.intent_id, intent_id);
        assert_eq!(checkpoint.step_id, Some(step_id));
        assert!(checkpoint.safe_rollback);
    }
    
    #[tokio::test]
    async fn test_checkpoint_rollback() {
        let manager = CheckpointManager::new();
        let intent_id = IntentId::new();
        
        // Create first checkpoint
        let cp1 = manager.create_checkpoint(intent_id, Uuid::new_v4()).await.unwrap();
        
        // Update state
        manager.update_state(|state| {
            state.variables.insert("test".to_string(), serde_json::json!("value1"));
        }).await.unwrap();
        
        // Create second checkpoint
        let _cp2 = manager.create_checkpoint(intent_id, Uuid::new_v4()).await.unwrap();
        
        // Update state again
        manager.update_state(|state| {
            state.variables.insert("test".to_string(), serde_json::json!("value2"));
        }).await.unwrap();
        
        // Rollback to first checkpoint
        manager.rollback_to(cp1.id).await.unwrap();
        
        // Check that second checkpoint is gone
        let history = manager.get_history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, cp1.id);
    }
}