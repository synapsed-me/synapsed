//! Internal intent storage module for MCP server
//! This module is NOT exposed to MCP clients - it's used internally only

use crate::error::{McpError, Result};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use synapsed_intent::{HierarchicalIntent, IntentId};
use synapsed_storage::{Storage, StorageConfig, backends::{MemoryStorage, FileStorage, SqliteStorage}};
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Intent execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IntentStatus {
    Declared,
    Executing,
    Completed,
    Failed,
    Verified,
}

/// Step execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepStatus {
    pub name: String,
    pub action: String,
    pub status: IntentStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// Stored intent record with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentRecord {
    pub id: String,
    pub goal: String,
    pub description: Option<String>,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub steps: Vec<StepStatus>,
    pub status: IntentStatus,
    pub agent_id: Option<String>,
    pub parent_intent_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub verification_results: HashMap<String, bool>,
    pub context_bounds: Option<serde_json::Value>,
}

impl From<&HierarchicalIntent> for IntentRecord {
    fn from(intent: &HierarchicalIntent) -> Self {
        IntentRecord {
            id: intent.id.to_string(),
            goal: intent.goal.clone(),
            description: intent.description.clone(),
            preconditions: Vec::new(), // HierarchicalIntent doesn't have this field directly
            postconditions: Vec::new(), // HierarchicalIntent doesn't have this field directly
            steps: intent.steps.iter().map(|s| StepStatus {
                name: s.name.clone(),
                action: format!("{:?}", s.action), // Convert StepAction to string
                status: IntentStatus::Declared,
                started_at: None,
                completed_at: None,
                error: None,
            }).collect(),
            status: IntentStatus::Declared,
            agent_id: None,
            parent_intent_id: None, // HierarchicalIntent doesn't have parent_id field
            created_at: Utc::now(),
            updated_at: Utc::now(),
            verification_results: HashMap::new(),
            context_bounds: None, // HierarchicalIntent doesn't have context_bounds field
        }
    }
}

/// Internal intent storage manager
pub(crate) struct IntentStore {
    storage: Arc<RwLock<Box<dyn Storage<Error = synapsed_storage::StorageError>>>>,
}

impl IntentStore {
    /// Create new intent store with memory backend
    pub fn new() -> Result<Self> {
        let backend = Box::new(MemoryStorage::default());
        Ok(Self {
            storage: Arc::new(RwLock::new(backend)),
        })
    }
    
    /// Create intent store with file-based persistence
    pub fn with_file_storage(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let backend = Box::new(FileStorage::new(path)
            .map_err(|e| McpError::StorageError(e.to_string()))?);
        Ok(Self {
            storage: Arc::new(RwLock::new(backend)),
        })
    }
    
    /// Create intent store with SQLite persistence
    pub fn with_sqlite_storage(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let backend = Box::new(SqliteStorage::new(path)
            .map_err(|e| McpError::StorageError(e.to_string()))?);
        Ok(Self {
            storage: Arc::new(RwLock::new(backend)),
        })
    }

    /// Store an intent (called internally by intent_declare)
    pub async fn store_intent(&self, intent: &HierarchicalIntent) -> Result<String> {
        let record = IntentRecord::from(intent);
        let id = record.id.clone();
        
        let json = serde_json::to_vec(&record)
            .map_err(|e| McpError::SerializationError(e.to_string()))?;
        
        let key = format!("intent:{}", id);
        let mut storage = self.storage.write().await;
        storage.put(key.as_bytes(), &json).await
            .map_err(|e| McpError::StorageError(e.to_string()))?;
        
        Ok(id)
    }

    /// Retrieve an intent by ID (called internally by intent_get)
    pub async fn get_intent(&self, id: &str) -> Result<Option<IntentRecord>> {
        let storage = self.storage.read().await;
        
        let key = format!("intent:{}", id);
        match storage.get(key.as_bytes()).await {
            Ok(Some(data)) => {
                let record: IntentRecord = serde_json::from_slice(&data)
                    .map_err(|e| McpError::SerializationError(e.to_string()))?;
                Ok(Some(record))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(McpError::StorageError(e.to_string())),
        }
    }

    /// Update intent status (called internally by intent_status, intent_complete)
    pub async fn update_status(&self, id: &str, status: IntentStatus) -> Result<()> {
        let mut record = self.get_intent(id).await?
            .ok_or_else(|| McpError::NotFound(format!("Intent {} not found", id)))?;
        
        record.status = status;
        record.updated_at = Utc::now();
        
        let json = serde_json::to_vec(&record)
            .map_err(|e| McpError::SerializationError(e.to_string()))?;
        
        let mut storage = self.storage.write().await;
        storage.put(id.as_bytes(), &json).await
            .map_err(|e| McpError::StorageError(e.to_string()))?;
        
        Ok(())
    }

    /// Update step status (called internally when steps execute)
    pub async fn update_step_status(
        &self, 
        intent_id: &str, 
        step_name: &str, 
        status: IntentStatus,
        error: Option<String>
    ) -> Result<()> {
        let mut record = self.get_intent(intent_id).await?
            .ok_or_else(|| McpError::NotFound(format!("Intent {} not found", intent_id)))?;
        
        for step in &mut record.steps {
            if step.name == step_name {
                step.status = status.clone();
                if status == IntentStatus::Executing {
                    step.started_at = Some(Utc::now());
                } else if status == IntentStatus::Completed || status == IntentStatus::Failed {
                    step.completed_at = Some(Utc::now());
                }
                if let Some(err) = error {
                    step.error = Some(err);
                }
                break;
            }
        }
        
        record.updated_at = Utc::now();
        
        let json = serde_json::to_vec(&record)
            .map_err(|e| McpError::SerializationError(e.to_string()))?;
        
        let mut storage = self.storage.write().await;
        storage.put(intent_id.as_bytes(), &json).await
            .map_err(|e| McpError::StorageError(e.to_string()))?;
        
        Ok(())
    }

    /// List intents with optional filters (called internally by intent_list)
    pub async fn list_intents(
        &self,
        status_filter: Option<IntentStatus>,
        agent_filter: Option<String>,
        parent_filter: Option<String>,
    ) -> Result<Vec<IntentRecord>> {
        let storage = self.storage.read().await;
        
        // Get all keys with "intent:" prefix
        let prefix = b"intent:";
        let keys = storage.list(prefix).await
            .map_err(|e| McpError::StorageError(e.to_string()))?;
        
        let mut intents = Vec::new();
        for key in keys {
            if let Ok(Some(data)) = storage.get(&key).await {
                if let Ok(record) = serde_json::from_slice::<IntentRecord>(&data) {
                    // Apply filters
                    if let Some(ref status) = status_filter {
                        if record.status != *status {
                            continue;
                        }
                    }
                    if let Some(ref agent) = agent_filter {
                        if record.agent_id.as_ref() != Some(agent) {
                            continue;
                        }
                    }
                    if let Some(ref parent) = parent_filter {
                        if record.parent_intent_id.as_ref() != Some(parent) {
                            continue;
                        }
                    }
                    intents.push(record);
                }
            }
        }
        
        // Sort by creation time
        intents.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(intents)
    }
    
    /// Get all stored intents (no filters)
    pub async fn get_all_intents(&self) -> Result<Vec<IntentRecord>> {
        self.list_intents(None, None, None).await
    }
    
    /// Store an intent from UUID and HierarchicalIntent (for rmcp adapter)
    pub fn store_intent(&self, id: Uuid, intent: HierarchicalIntent) -> Result<()> {
        // Convert to async operation synchronously since rmcp adapter calls are async
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let mut record = IntentRecord::from(&intent);
            record.id = id.to_string();
            
            let json = serde_json::to_vec(&record)
                .map_err(|e| McpError::SerializationError(e.to_string()))?;
            
            let key = format!("intent:{}", id);
            let mut storage = self.storage.write().await;
            storage.put(key.as_bytes(), &json).await
                .map_err(|e| McpError::StorageError(e.to_string()))?;
            
            Ok(())
        })
    }
    
    /// Get intent by UUID (for rmcp adapter)
    pub fn get_intent(&self, id: &Uuid) -> Result<HierarchicalIntent> {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let storage = self.storage.read().await;
            
            let key = format!("intent:{}", id);
            match storage.get(key.as_bytes()).await {
                Ok(Some(data)) => {
                    let record: IntentRecord = serde_json::from_slice(&data)
                        .map_err(|e| McpError::SerializationError(e.to_string()))?;
                    
                    // Convert IntentRecord back to HierarchicalIntent
                    let mut intent = HierarchicalIntent::new(record.goal);
                    if let Some(desc) = record.description {
                        intent = intent.with_description(desc);
                    }
                    
                    Ok(intent)
                }
                Ok(None) => Err(McpError::NotFound(format!("Intent {} not found", id))),
                Err(e) => Err(McpError::StorageError(e.to_string())),
            }
        })
    }
    
    /// List active intents (for rmcp adapter)
    pub fn list_active_intents(&self) -> Result<Vec<serde_json::Value>> {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let intents = self.list_intents(Some(IntentStatus::Executing), None, None).await?;
            Ok(intents.into_iter().map(|i| serde_json::json!({
                "id": i.id,
                "goal": i.goal,
                "status": format!("{:?}", i.status),
                "created_at": i.created_at.to_rfc3339(),
            })).collect())
        })
    }
    
    /// Get verification results (for rmcp adapter)
    pub fn get_verification_results(&self, limit: usize) -> Result<Vec<serde_json::Value>> {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let intents = self.list_intents(Some(IntentStatus::Verified), None, None).await?;
            Ok(intents.into_iter()
                .take(limit)
                .map(|i| serde_json::json!({
                    "id": i.id,
                    "goal": i.goal,
                    "verified": true,
                    "timestamp": i.updated_at.to_rfc3339(),
                }))
                .collect())
        })
    }

    /// Get child intents (called internally by intent_children)
    pub async fn get_children(&self, parent_id: &str) -> Result<Vec<IntentRecord>> {
        // In real implementation, would query by parent_intent_id
        // For now, return empty list
        Ok(Vec::new())
    }

    /// Store verification results (called internally by intent_verify)
    pub async fn store_verification(
        &self,
        intent_id: &str,
        results: HashMap<String, bool>
    ) -> Result<()> {
        let mut record = self.get_intent(intent_id).await?
            .ok_or_else(|| McpError::NotFound(format!("Intent {} not found", intent_id)))?;
        
        record.verification_results = results;
        
        // Update status to Verified if all postconditions pass
        let all_verified = record.verification_results.values().all(|v| *v);
        if all_verified && !record.postconditions.is_empty() {
            record.status = IntentStatus::Verified;
        }
        
        record.updated_at = Utc::now();
        
        let json = serde_json::to_vec(&record)
            .map_err(|e| McpError::SerializationError(e.to_string()))?;
        
        let mut storage = self.storage.write().await;
        storage.put(intent_id.as_bytes(), &json).await
            .map_err(|e| McpError::StorageError(e.to_string()))?;
        
        Ok(())
    }
}