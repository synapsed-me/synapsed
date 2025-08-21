//! State verification and snapshot management for AI agent claims

use crate::{types::*, Result, VerifyError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};

/// State snapshot for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Snapshot ID
    pub id: Uuid,
    /// When the snapshot was taken
    pub timestamp: DateTime<Utc>,
    /// State data
    pub state: HashMap<String, Value>,
    /// Hash of the state
    pub hash: String,
    /// Metadata about the snapshot
    pub metadata: StateMetadata,
}

/// Metadata about a state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMetadata {
    /// Agent that created the snapshot
    pub agent_id: Option<String>,
    /// Context in which snapshot was taken
    pub context: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Parent snapshot ID (for chaining)
    pub parent_snapshot: Option<Uuid>,
    /// Whether this is a checkpoint
    pub is_checkpoint: bool,
}

/// Difference between two states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDiff {
    /// Fields that were added
    pub added: HashMap<String, Value>,
    /// Fields that were modified
    pub modified: HashMap<String, (Value, Value)>, // (old, new)
    /// Fields that were removed
    pub removed: HashMap<String, Value>,
    /// Whether states are identical
    pub identical: bool,
    /// Similarity score (0.0 to 1.0)
    pub similarity: f64,
}

/// State verifier for tracking and verifying state changes
pub struct StateVerifier {
    /// Stored snapshots
    snapshots: HashMap<Uuid, StateSnapshot>,
    /// Checkpoint snapshots (for rollback)
    checkpoints: Vec<Uuid>,
    /// Current state
    current_state: HashMap<String, Value>,
    /// Maximum snapshots to keep
    max_snapshots: usize,
}

impl StateVerifier {
    /// Creates a new state verifier
    pub fn new() -> Self {
        Self {
            snapshots: HashMap::new(),
            checkpoints: Vec::new(),
            current_state: HashMap::new(),
            max_snapshots: 100,
        }
    }
    
    /// Creates a verifier with custom settings
    pub fn with_max_snapshots(max: usize) -> Self {
        Self {
            snapshots: HashMap::new(),
            checkpoints: Vec::new(),
            current_state: HashMap::new(),
            max_snapshots: max,
        }
    }
    
    /// Takes a snapshot of the current state
    pub async fn take_snapshot(&mut self) -> Result<StateSnapshot> {
        self.take_snapshot_with_metadata(StateMetadata {
            agent_id: None,
            context: None,
            tags: Vec::new(),
            parent_snapshot: None,
            is_checkpoint: false,
        }).await
    }
    
    /// Takes a snapshot with metadata
    pub async fn take_snapshot_with_metadata(
        &mut self,
        metadata: StateMetadata,
    ) -> Result<StateSnapshot> {
        let id = Uuid::new_v4();
        let timestamp = Utc::now();
        let state = self.current_state.clone();
        let hash = Self::calculate_state_hash(&state);
        
        let snapshot = StateSnapshot {
            id,
            timestamp,
            state,
            hash,
            metadata: metadata.clone(),
        };
        
        // Store snapshot
        self.snapshots.insert(id, snapshot.clone());
        
        // Add to checkpoints if marked
        if metadata.is_checkpoint {
            self.checkpoints.push(id);
        }
        
        // Clean up old snapshots if needed
        if self.snapshots.len() > self.max_snapshots {
            self.cleanup_old_snapshots();
        }
        
        Ok(snapshot)
    }
    
    /// Updates the current state
    pub fn update_state(&mut self, key: String, value: Value) {
        self.current_state.insert(key, value);
    }
    
    /// Sets the entire state
    pub fn set_state(&mut self, state: HashMap<String, Value>) {
        self.current_state = state;
    }
    
    /// Gets the current state
    pub fn get_state(&self) -> &HashMap<String, Value> {
        &self.current_state
    }
    
    /// Verifies current state against a snapshot
    pub async fn verify_against_snapshot(
        &self,
        snapshot: &StateSnapshot,
    ) -> Result<StateDiff> {
        self.compare_states(&snapshot.state, &self.current_state)
    }
    
    /// Compares two states
    pub fn compare_states(
        &self,
        expected: &HashMap<String, Value>,
        actual: &HashMap<String, Value>,
    ) -> Result<StateDiff> {
        let mut added = HashMap::new();
        let mut modified = HashMap::new();
        let mut removed = HashMap::new();
        
        // Check for added and modified fields
        for (key, actual_value) in actual {
            if let Some(expected_value) = expected.get(key) {
                if expected_value != actual_value {
                    modified.insert(
                        key.clone(),
                        (expected_value.clone(), actual_value.clone())
                    );
                }
            } else {
                added.insert(key.clone(), actual_value.clone());
            }
        }
        
        // Check for removed fields
        for (key, expected_value) in expected {
            if !actual.contains_key(key) {
                removed.insert(key.clone(), expected_value.clone());
            }
        }
        
        let identical = added.is_empty() && modified.is_empty() && removed.is_empty();
        
        // Calculate similarity score
        let total_fields = expected.len().max(actual.len()) as f64;
        let changed_fields = (added.len() + modified.len() + removed.len()) as f64;
        let similarity = if total_fields > 0.0 {
            1.0 - (changed_fields / total_fields)
        } else {
            1.0
        };
        
        Ok(StateDiff {
            added,
            modified,
            removed,
            identical,
            similarity,
        })
    }
    
    /// Verifies a state transition
    pub async fn verify_transition(
        &self,
        before_id: Uuid,
        after_id: Uuid,
        expected_changes: &HashMap<String, Value>,
    ) -> Result<VerificationResult> {
        let start = Utc::now();
        
        let before = self.snapshots.get(&before_id)
            .ok_or_else(|| VerifyError::VerificationFailed("Before snapshot not found".to_string()))?;
        let after = self.snapshots.get(&after_id)
            .ok_or_else(|| VerifyError::VerificationFailed("After snapshot not found".to_string()))?;
        
        let diff = self.compare_states(&before.state, &after.state)?;
        
        // Check if expected changes match actual changes
        let mut all_expected_found = true;
        let mut unexpected_changes = Vec::new();
        
        for (key, expected_value) in expected_changes {
            if let Some((_, actual_value)) = diff.modified.get(key) {
                if actual_value != expected_value {
                    all_expected_found = false;
                }
            } else if let Some(actual_value) = diff.added.get(key) {
                if actual_value != expected_value {
                    all_expected_found = false;
                }
            } else {
                all_expected_found = false;
            }
        }
        
        // Check for unexpected changes
        for key in diff.modified.keys() {
            if !expected_changes.contains_key(key) {
                unexpected_changes.push(key.clone());
            }
        }
        for key in diff.added.keys() {
            if !expected_changes.contains_key(key) {
                unexpected_changes.push(key.clone());
            }
        }
        
        let success = all_expected_found && unexpected_changes.is_empty();
        let duration_ms = (Utc::now() - start).num_milliseconds() as u64;
        
        let result = if success {
            VerificationResult::success(
                VerificationType::State,
                serde_json::json!({
                    "before_id": before_id,
                    "after_id": after_id,
                    "expected_changes": expected_changes,
                }),
                serde_json::json!({
                    "changes": {
                        "added": diff.added.len(),
                        "modified": diff.modified.len(),
                        "removed": diff.removed.len(),
                    },
                    "similarity": diff.similarity,
                }),
            )
        } else {
            VerificationResult::failure(
                VerificationType::State,
                serde_json::json!({
                    "before_id": before_id,
                    "after_id": after_id,
                    "expected_changes": expected_changes,
                }),
                serde_json::json!({
                    "unexpected_changes": unexpected_changes,
                    "similarity": diff.similarity,
                }),
                "State transition verification failed".to_string(),
            )
        };
        
        let mut final_result = result;
        final_result.duration_ms = duration_ms;
        
        Ok(final_result)
    }
    
    /// Rolls back to a checkpoint
    pub async fn rollback_to_checkpoint(&mut self, checkpoint_id: Uuid) -> Result<()> {
        let checkpoint = self.snapshots.get(&checkpoint_id)
            .ok_or_else(|| VerifyError::VerificationFailed("Checkpoint not found".to_string()))?;
        
        if !checkpoint.metadata.is_checkpoint {
            return Err(VerifyError::VerificationFailed(
                "Snapshot is not a checkpoint".to_string()
            ));
        }
        
        self.current_state = checkpoint.state.clone();
        
        Ok(())
    }
    
    /// Creates a checkpoint
    pub async fn create_checkpoint(&mut self, context: Option<String>) -> Result<StateSnapshot> {
        self.take_snapshot_with_metadata(StateMetadata {
            agent_id: None,
            context,
            tags: vec!["checkpoint".to_string()],
            parent_snapshot: self.checkpoints.last().copied(),
            is_checkpoint: true,
        }).await
    }
    
    /// Gets all checkpoints
    pub fn get_checkpoints(&self) -> Vec<&StateSnapshot> {
        self.checkpoints
            .iter()
            .filter_map(|id| self.snapshots.get(id))
            .collect()
    }
    
    /// Verifies state invariants
    pub async fn verify_invariants(
        &self,
        invariants: &[(String, Value)],
    ) -> Result<VerificationResult> {
        let start = Utc::now();
        let mut failed_invariants = Vec::new();
        
        for (key, expected_value) in invariants {
            if let Some(actual_value) = self.current_state.get(key) {
                if actual_value != expected_value {
                    failed_invariants.push(format!(
                        "{}: expected {:?}, got {:?}",
                        key, expected_value, actual_value
                    ));
                }
            } else {
                failed_invariants.push(format!("{}: not found", key));
            }
        }
        
        let success = failed_invariants.is_empty();
        let duration_ms = (Utc::now() - start).num_milliseconds() as u64;
        
        let result = if success {
            VerificationResult::success(
                VerificationType::State,
                serde_json::json!({ "invariants": invariants }),
                serde_json::json!({ "all_satisfied": true }),
            )
        } else {
            VerificationResult::failure(
                VerificationType::State,
                serde_json::json!({ "invariants": invariants }),
                serde_json::json!({ "failed": failed_invariants }),
                "Invariant verification failed".to_string(),
            )
        };
        
        let mut final_result = result;
        final_result.duration_ms = duration_ms;
        
        Ok(final_result)
    }
    
    // Helper methods
    
    fn calculate_state_hash(state: &HashMap<String, Value>) -> String {
        let mut hasher = Sha256::new();
        let json = serde_json::to_string(state).unwrap_or_default();
        hasher.update(json.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    fn cleanup_old_snapshots(&mut self) {
        if self.snapshots.len() <= self.max_snapshots {
            return;
        }
        
        // Keep checkpoints and recent snapshots
        let mut snapshots_to_keep: Vec<Uuid> = self.checkpoints.clone();
        
        // Sort non-checkpoint snapshots by timestamp
        let mut other_snapshots: Vec<(Uuid, DateTime<Utc>)> = self.snapshots
            .iter()
            .filter(|(id, _)| !self.checkpoints.contains(id))
            .map(|(id, s)| (*id, s.timestamp))
            .collect();
        
        other_snapshots.sort_by_key(|(_, timestamp)| *timestamp);
        other_snapshots.reverse();
        
        // Keep the most recent ones
        let remaining_slots = self.max_snapshots.saturating_sub(snapshots_to_keep.len());
        snapshots_to_keep.extend(
            other_snapshots.iter()
                .take(remaining_slots)
                .map(|(id, _)| *id)
        );
        
        // Remove old snapshots
        self.snapshots.retain(|id, _| snapshots_to_keep.contains(id));
    }
}

impl Default for StateVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_state_snapshot() {
        let mut verifier = StateVerifier::new();
        
        // Set initial state
        verifier.update_state("key1".to_string(), serde_json::json!("value1"));
        verifier.update_state("key2".to_string(), serde_json::json!(42));
        
        // Take snapshot
        let snapshot = verifier.take_snapshot().await.unwrap();
        assert_eq!(snapshot.state.len(), 2);
        assert_eq!(snapshot.state.get("key1"), Some(&serde_json::json!("value1")));
    }
    
    #[tokio::test]
    async fn test_state_comparison() {
        let verifier = StateVerifier::new();
        
        let state1 = HashMap::from([
            ("key1".to_string(), serde_json::json!("value1")),
            ("key2".to_string(), serde_json::json!(42)),
        ]);
        
        let state2 = HashMap::from([
            ("key1".to_string(), serde_json::json!("modified")),
            ("key3".to_string(), serde_json::json!(true)),
        ]);
        
        let diff = verifier.compare_states(&state1, &state2).unwrap();
        
        assert!(!diff.identical);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.removed.len(), 1);
        assert!(diff.similarity > 0.0 && diff.similarity < 1.0);
    }
    
    #[tokio::test]
    async fn test_checkpoint_rollback() {
        let mut verifier = StateVerifier::new();
        
        // Set initial state
        verifier.update_state("key1".to_string(), serde_json::json!("initial"));
        
        // Create checkpoint
        let checkpoint = verifier.create_checkpoint(Some("test".to_string())).await.unwrap();
        
        // Modify state
        verifier.update_state("key1".to_string(), serde_json::json!("modified"));
        assert_eq!(
            verifier.get_state().get("key1"),
            Some(&serde_json::json!("modified"))
        );
        
        // Rollback
        verifier.rollback_to_checkpoint(checkpoint.id).await.unwrap();
        assert_eq!(
            verifier.get_state().get("key1"),
            Some(&serde_json::json!("initial"))
        );
    }
}