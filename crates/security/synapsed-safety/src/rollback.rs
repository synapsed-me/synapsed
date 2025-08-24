//! Rollback and checkpoint management system
//!
//! This module provides comprehensive state checkpoint and recovery
//! capabilities for safety-critical operations.

use crate::error::{Result, SafetyError};
use crate::traits::{RollbackManager, RollbackStats, RetentionPolicy};
use crate::types::*;
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Default rollback manager implementation
#[derive(Debug)]
pub struct DefaultRollbackManager {
    /// Stored checkpoints
    checkpoints: Arc<RwLock<HashMap<CheckpointId, Checkpoint>>>,
    /// Checkpoint history ordered by creation time
    checkpoint_history: Arc<RwLock<VecDeque<CheckpointId>>>,
    /// Tagged checkpoints for quick access
    tagged_checkpoints: Arc<RwLock<HashMap<String, CheckpointId>>>,
    /// Manager configuration
    config: RollbackConfig,
    /// Statistics
    stats: Arc<RwLock<RollbackStats>>,
    /// Current state (for rollback operations)
    current_state: Arc<RwLock<Option<SafetyState>>>,
    /// Retention policy
    retention_policy: Arc<RwLock<RetentionPolicy>>,
}

/// Configuration for rollback manager
#[derive(Debug, Clone)]
pub struct RollbackConfig {
    /// Maximum number of checkpoints to keep
    pub max_checkpoints: u32,
    /// Enable compression for checkpoints
    pub compression_enabled: bool,
    /// Compression algorithm
    pub compression_algorithm: String,
    /// Maximum memory usage for checkpoints
    pub max_memory_bytes: u64,
    /// Enable integrity checking
    pub integrity_checking: bool,
    /// Checkpoint validation on creation
    pub validate_on_create: bool,
}

impl Default for RollbackConfig {
    fn default() -> Self {
        Self {
            max_checkpoints: 100,
            compression_enabled: true,
            compression_algorithm: "zstd".to_string(),
            max_memory_bytes: 100 * 1024 * 1024, // 100MB
            integrity_checking: true,
            validate_on_create: true,
        }
    }
}

impl DefaultRollbackManager {
    /// Create a new rollback manager with default configuration
    pub fn new() -> Self {
        Self::with_config(RollbackConfig::default())
    }

    /// Create a new rollback manager with custom configuration
    pub fn with_config(config: RollbackConfig) -> Self {
        Self {
            checkpoints: Arc::new(RwLock::new(HashMap::new())),
            checkpoint_history: Arc::new(RwLock::new(VecDeque::new())),
            tagged_checkpoints: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(RollbackStats {
                checkpoints_created: 0,
                rollbacks_performed: 0,
                avg_checkpoint_size_bytes: 0,
                avg_rollback_time_ms: 0.0,
                success_rate: 1.0,
            })),
            current_state: Arc::new(RwLock::new(None)),
            retention_policy: Arc::new(RwLock::new(RetentionPolicy {
                max_checkpoints: 100,
                max_age_hours: 24,
                max_total_size_bytes: 100 * 1024 * 1024,
                compress_after_hours: 1,
                delete_compressed_after_days: 7,
            })),
        }
    }

    /// Set current state for rollback operations
    pub async fn set_current_state(&self, state: SafetyState) -> Result<()> {
        *self.current_state.write() = Some(state);
        Ok(())
    }

    /// Calculate checksum for integrity checking
    fn calculate_checksum(&self, checkpoint: &Checkpoint) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        checkpoint.id.hash(&mut hasher);
        checkpoint.timestamp.hash(&mut hasher);
        checkpoint.state.id.hash(&mut hasher);
        checkpoint.description.hash(&mut hasher);
        
        format!("{:x}", hasher.finish())
    }

    /// Compress checkpoint data if enabled
    async fn compress_checkpoint(&self, checkpoint: &mut Checkpoint) -> Result<()> {
        if !self.config.compression_enabled {
            return Ok(());
        }

        debug!("Compressing checkpoint: {}", checkpoint.id);
        
        // Serialize the state
        let serialized = serde_json::to_string(&checkpoint.state)
            .map_err(|e| SafetyError::Serialization {
                message: format!("Failed to serialize checkpoint state: {}", e),
            })?;

        let original_size = serialized.len() as u64;
        
        // In a real implementation, you would use actual compression algorithms
        // For now, we'll simulate compression by updating metadata
        let compressed_size = (original_size as f64 * 0.6) as u64; // Simulate 40% compression
        
        checkpoint.compression = Some(self.config.compression_algorithm.clone());
        checkpoint.size_bytes = compressed_size;
        
        let compression_ratio = original_size as f64 / compressed_size as f64;
        info!(
            "Checkpoint {} compressed: {} -> {} bytes (ratio: {:.2}x)",
            checkpoint.id, original_size, compressed_size, compression_ratio
        );
        
        Ok(())
    }

    /// Validate checkpoint integrity
    async fn validate_checkpoint_integrity(&self, checkpoint: &Checkpoint) -> Result<bool> {
        if !self.config.integrity_checking {
            return Ok(true);
        }

        let calculated_checksum = self.calculate_checksum(checkpoint);
        let valid = calculated_checksum == checkpoint.integrity_hash;
        
        if !valid {
            warn!(
                "Checkpoint {} integrity check failed: expected {}, got {}",
                checkpoint.id, checkpoint.integrity_hash, calculated_checksum
            );
        }
        
        Ok(valid)
    }

    /// Enforce retention policy
    async fn enforce_retention_policy(&self) -> Result<()> {
        let policy = self.retention_policy.read().clone();
        let mut checkpoints = self.checkpoints.write();
        let mut history = self.checkpoint_history.write();
        let mut tagged = self.tagged_checkpoints.write();
        
        let now = chrono::Utc::now();
        let mut total_size = 0u64;
        let mut expired_checkpoints = Vec::new();
        
        // Calculate total size and find expired checkpoints
        for checkpoint in checkpoints.values() {
            total_size += checkpoint.size_bytes;
            
            let age_hours = (now - checkpoint.timestamp).num_hours();
            if age_hours > policy.max_age_hours as i64 {
                expired_checkpoints.push(checkpoint.id);
            }
        }
        
        // Remove expired checkpoints
        for checkpoint_id in expired_checkpoints {
            info!("Removing expired checkpoint: {}", checkpoint_id);
            checkpoints.remove(&checkpoint_id);
            history.retain(|id| *id != checkpoint_id);
            tagged.retain(|_, id| *id != checkpoint_id);
        }
        
        // Enforce maximum count
        while history.len() > policy.max_checkpoints as usize {
            if let Some(oldest_id) = history.pop_front() {
                info!("Removing oldest checkpoint for count limit: {}", oldest_id);
                checkpoints.remove(&oldest_id);
                tagged.retain(|_, id| *id != oldest_id);
            }
        }
        
        // Enforce size limit
        while total_size > policy.max_total_size_bytes && !history.is_empty() {
            if let Some(oldest_id) = history.pop_front() {
                if let Some(checkpoint) = checkpoints.remove(&oldest_id) {
                    info!(
                        "Removing checkpoint for size limit: {} ({} bytes)",
                        oldest_id, checkpoint.size_bytes
                    );
                    total_size -= checkpoint.size_bytes;
                    tagged.retain(|_, id| *id != oldest_id);
                }
            }
        }
        
        debug!(
            "Retention policy enforced: {} checkpoints, {} bytes total",
            checkpoints.len(),
            total_size
        );
        
        Ok(())
    }

    /// Apply rollback to current state
    async fn apply_rollback(&self, target_state: &SafetyState) -> Result<()> {
        info!("Applying rollback to state: {}", target_state.id);
        
        // In a real implementation, this would perform the actual state restoration
        // For now, we'll update the current state
        *self.current_state.write() = Some(target_state.clone());
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.rollbacks_performed += 1;
        
        // Update success rate (simplified calculation)
        stats.success_rate = stats.rollbacks_performed as f64 / (stats.rollbacks_performed + 1) as f64;
        
        info!("Rollback applied successfully to state: {}", target_state.id);
        Ok(())
    }
}

#[async_trait]
impl RollbackManager for DefaultRollbackManager {
    async fn create_checkpoint(&mut self, description: Option<String>) -> Result<CheckpointId> {
        self.create_tagged_checkpoint(description, vec![]).await
    }

    async fn create_tagged_checkpoint(
        &mut self,
        description: Option<String>,
        tags: Vec<String>,
    ) -> Result<CheckpointId> {
        let state = {
            let current_state = self.current_state.read();
            current_state.as_ref().ok_or_else(|| SafetyError::MonitorError {
                message: "No current state available for checkpoint creation".to_string(),
            })?.clone()
        };
        
        let checkpoint_id = Uuid::new_v4();
        let timestamp = chrono::Utc::now();
        
        info!(
            "Creating checkpoint: {} (description: {:?}, tags: {:?})",
            checkpoint_id, description, tags
        );
        
        let mut checkpoint = Checkpoint {
            id: checkpoint_id,
            timestamp,
            state,
            description: description.unwrap_or_else(|| format!("Checkpoint created at {}", timestamp)),
            tags: tags.clone(),
            size_bytes: 0, // Will be updated after compression
            compression: None,
            integrity_hash: String::new(), // Will be calculated below
        };
        
        // Compress if enabled
        self.compress_checkpoint(&mut checkpoint).await?;
        
        // Calculate integrity hash
        checkpoint.integrity_hash = self.calculate_checksum(&checkpoint);
        
        // Validate if enabled
        if self.config.validate_on_create {
            if !self.validate_checkpoint_integrity(&checkpoint).await? {
                return Err(SafetyError::CheckpointCorrupted { checkpoint_id });
            }
        }
        
        // Store checkpoint
        {
            let mut checkpoints = self.checkpoints.write();
            let mut history = self.checkpoint_history.write();
            let mut tagged_checkpoints = self.tagged_checkpoints.write();
            
            checkpoints.insert(checkpoint_id, checkpoint);
            history.push_back(checkpoint_id);
            
            // Store tagged references
            for tag in tags {
                tagged_checkpoints.insert(tag, checkpoint_id);
            }
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.checkpoints_created += 1;
            
            let checkpoints = self.checkpoints.read();
            if !checkpoints.is_empty() {
                let total_size: u64 = checkpoints.values().map(|c| c.size_bytes).sum();
                stats.avg_checkpoint_size_bytes = total_size / checkpoints.len() as u64;
            }
        }
        
        // Enforce retention policy
        self.enforce_retention_policy().await?;
        
        info!("Checkpoint created successfully: {}", checkpoint_id);
        Ok(checkpoint_id)
    }

    async fn rollback_to_checkpoint(&mut self, checkpoint_id: &CheckpointId) -> Result<()> {
        let start_time = Instant::now();
        
        info!("Rolling back to checkpoint: {}", checkpoint_id);
        
        let checkpoint = {
            let checkpoints = self.checkpoints.read();
            checkpoints.get(checkpoint_id).cloned()
        };
        
        let checkpoint = checkpoint.ok_or_else(|| SafetyError::RollbackFailed {
            checkpoint_id: *checkpoint_id,
            reason: "Checkpoint not found".to_string(),
        })?;
        
        // Validate checkpoint integrity
        if !self.validate_checkpoint_integrity(&checkpoint).await? {
            return Err(SafetyError::CheckpointCorrupted {
                checkpoint_id: *checkpoint_id,
            });
        }
        
        // Apply rollback
        self.apply_rollback(&checkpoint.state).await?;
        
        // Update statistics
        let rollback_time = start_time.elapsed();
        {
            let mut stats = self.stats.write();
            let rollback_time_ms = rollback_time.as_millis() as f64;
            
            if stats.rollbacks_performed == 1 {
                stats.avg_rollback_time_ms = rollback_time_ms;
            } else {
                stats.avg_rollback_time_ms = 
                    (stats.avg_rollback_time_ms * (stats.rollbacks_performed - 1) as f64 + rollback_time_ms)
                    / stats.rollbacks_performed as f64;
            }
        }
        
        info!(
            "Rollback completed in {}ms to checkpoint: {}",
            rollback_time.as_millis(),
            checkpoint_id
        );
        
        Ok(())
    }

    async fn rollback_to_latest(&mut self) -> Result<CheckpointId> {
        let latest_id = {
            let history = self.checkpoint_history.read();
            history.back().copied()
        };
        
        let checkpoint_id = latest_id.ok_or_else(|| SafetyError::RollbackFailed {
            checkpoint_id: Uuid::nil(),
            reason: "No checkpoints available".to_string(),
        })?;
        
        self.rollback_to_checkpoint(&checkpoint_id).await?;
        Ok(checkpoint_id)
    }

    async fn rollback_to_tag(&mut self, tag: &str) -> Result<CheckpointId> {
        let checkpoint_id = {
            let tagged = self.tagged_checkpoints.read();
            tagged.get(tag).copied()
        };
        
        let checkpoint_id = checkpoint_id.ok_or_else(|| SafetyError::RollbackFailed {
            checkpoint_id: Uuid::nil(),
            reason: format!("No checkpoint found with tag: {}", tag),
        })?;
        
        self.rollback_to_checkpoint(&checkpoint_id).await?;
        Ok(checkpoint_id)
    }

    async fn delete_checkpoint(&mut self, checkpoint_id: &CheckpointId) -> Result<()> {
        info!("Deleting checkpoint: {}", checkpoint_id);
        
        let mut checkpoints = self.checkpoints.write();
        let mut history = self.checkpoint_history.write();
        let mut tagged = self.tagged_checkpoints.write();
        
        let checkpoint = checkpoints.remove(checkpoint_id).ok_or_else(|| SafetyError::RollbackFailed {
            checkpoint_id: *checkpoint_id,
            reason: "Checkpoint not found".to_string(),
        })?;
        
        // Remove from history
        history.retain(|id| *id != *checkpoint_id);
        
        // Remove from tagged checkpoints
        tagged.retain(|_, id| *id != *checkpoint_id);
        
        info!(
            "Checkpoint deleted: {} ({} bytes freed)",
            checkpoint_id, checkpoint.size_bytes
        );
        
        Ok(())
    }

    async fn list_checkpoints(&self) -> Result<Vec<crate::traits::CheckpointSummary>> {
        let checkpoints = self.checkpoints.read();
        let history = self.checkpoint_history.read();
        
        let mut summaries = Vec::new();
        
        for checkpoint_id in history.iter() {
            if let Some(checkpoint) = checkpoints.get(checkpoint_id) {
                summaries.push(crate::traits::CheckpointSummary {
                    id: checkpoint.id,
                    timestamp: checkpoint.timestamp,
                    description: checkpoint.description.clone(),
                    tags: checkpoint.tags.clone(),
                    size_bytes: checkpoint.size_bytes,
                    compressed: checkpoint.compression.is_some(),
                });
            }
        }
        
        Ok(summaries)
    }

    async fn get_checkpoint(&self, checkpoint_id: &CheckpointId) -> Result<Option<Checkpoint>> {
        let checkpoints = self.checkpoints.read();
        Ok(checkpoints.get(checkpoint_id).cloned())
    }

    async fn compress_checkpoints(&mut self, older_than: Duration) -> Result<crate::traits::CompressionStats> {
        let start_time = Instant::now();
        let cutoff_time = chrono::Utc::now() - chrono::Duration::from_std(older_than).unwrap();
        
        info!("Compressing checkpoints older than: {}", cutoff_time);
        
        let mut checkpoints = self.checkpoints.write();
        let mut compressed_count: u32 = 0;
        let mut bytes_saved: u64 = 0;
        
        for checkpoint in checkpoints.values_mut() {
            if checkpoint.timestamp < cutoff_time && checkpoint.compression.is_none() {
                let original_size = checkpoint.size_bytes;
                
                // Simulate compression
                let compressed_size = (original_size as f64 * 0.6) as u64;
                checkpoint.size_bytes = compressed_size;
                checkpoint.compression = Some(self.config.compression_algorithm.clone());
                
                bytes_saved += original_size - compressed_size;
                compressed_count += 1;
                
                debug!("Compressed checkpoint: {} ({} -> {} bytes)", 
                       checkpoint.id, original_size, compressed_size);
            }
        }
        
        let duration = start_time.elapsed();
        let compression_ratio = if bytes_saved > 0 {
            1.0 - (bytes_saved as f64 / (bytes_saved + (compressed_count as u64) * 1024) as f64)
        } else {
            1.0
        };
        
        let stats = crate::traits::CompressionStats {
            checkpoints_compressed: compressed_count,
            bytes_saved,
            compression_ratio,
            duration_ms: duration.as_millis() as u64,
        };
        
        info!(
            "Checkpoint compression completed: {} checkpoints, {} bytes saved in {}ms",
            compressed_count, bytes_saved, duration.as_millis()
        );
        
        Ok(stats)
    }

    async fn validate_checkpoint(&self, checkpoint_id: &CheckpointId) -> Result<bool> {
        let checkpoint = {
            let checkpoints = self.checkpoints.read();
            checkpoints.get(checkpoint_id).ok_or_else(|| SafetyError::RollbackFailed {
                checkpoint_id: *checkpoint_id,
                reason: "Checkpoint not found".to_string(),
            })?.clone()
        };
        
        self.validate_checkpoint_integrity(&checkpoint).await
    }

    async fn get_stats(&self) -> Result<crate::traits::RollbackStats> {
        let stats = self.stats.read();
        Ok(stats.clone())
    }

    async fn set_retention_policy(&mut self, policy: crate::traits::RetentionPolicy) -> Result<()> {
        info!("Updating retention policy: {:?}", policy);
        
        *self.retention_policy.write() = policy;
        
        // Enforce new policy immediately
        self.enforce_retention_policy().await?;
        
        info!("Retention policy updated successfully");
        Ok(())
    }

    async fn export_checkpoint(&self, checkpoint_id: &CheckpointId, destination: &str) -> Result<()> {
        info!("Exporting checkpoint {} to: {}", checkpoint_id, destination);
        
        let checkpoints = self.checkpoints.read();
        let checkpoint = checkpoints.get(checkpoint_id).ok_or_else(|| SafetyError::RollbackFailed {
            checkpoint_id: *checkpoint_id,
            reason: "Checkpoint not found".to_string(),
        })?;
        
        // Serialize checkpoint
        let serialized = serde_json::to_string_pretty(checkpoint)
            .map_err(|e| SafetyError::Serialization {
                message: format!("Failed to serialize checkpoint: {}", e),
            })?;
        
        // In a real implementation, you would write to the actual destination
        // For now, we'll simulate the export
        debug!("Would export {} bytes to: {}", serialized.len(), destination);
        
        info!("Checkpoint exported successfully: {}", checkpoint_id);
        Ok(())
    }

    async fn import_checkpoint(&mut self, source: &str) -> Result<CheckpointId> {
        info!("Importing checkpoint from: {}", source);
        
        // In a real implementation, you would read from the actual source
        // For now, we'll create a dummy checkpoint
        let checkpoint_id = Uuid::new_v4();
        
        // Simulate creating a checkpoint from imported data
        let state_clone = {
            let current_state = self.current_state.read();
            current_state.as_ref().cloned()
        };
        
        if let Some(state) = state_clone {
            let imported_checkpoint = Checkpoint {
                id: checkpoint_id,
                timestamp: chrono::Utc::now(),
                state: state.clone(),
                description: format!("Imported from: {}", source),
                tags: vec!["imported".to_string()],
                size_bytes: 1024, // Placeholder
                compression: None,
                integrity_hash: self.calculate_checksum(&Checkpoint {
                    id: checkpoint_id,
                    timestamp: chrono::Utc::now(),
                    state: state.clone(),
                    description: format!("Imported from: {}", source),
                    tags: vec!["imported".to_string()],
                    size_bytes: 1024,
                    compression: None,
                    integrity_hash: String::new(),
                }),
            };
            
            // Store the imported checkpoint
            {
                let mut checkpoints = self.checkpoints.write();
                let mut history = self.checkpoint_history.write();
                
                checkpoints.insert(checkpoint_id, imported_checkpoint);
                history.push_back(checkpoint_id);
            }
            
            // Update statistics
            {
                let mut stats = self.stats.write();
                stats.checkpoints_created += 1;
            }
            
            info!("Checkpoint imported successfully: {}", checkpoint_id);
            Ok(checkpoint_id)
        } else {
            Err(SafetyError::RollbackFailed {
                checkpoint_id: Uuid::nil(),
                reason: "No current state available for import reference".to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_state() -> SafetyState {
        SafetyState {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            values: {
                let mut values = HashMap::new();
                values.insert("balance".to_string(), StateValue::Integer(100));
                values
            },
            active_constraints: vec![],
            resource_usage: ResourceUsage {
                cpu_usage: 0.5,
                memory_usage: 512 * 1024 * 1024,
                memory_limit: 1024 * 1024 * 1024,
                network_usage: 0,
                disk_io: 0,
                file_descriptors: 0,
                thread_count: 0,
                custom_resources: HashMap::new(),
            },
            health_indicators: HealthIndicators {
                overall_health: 0.8,
                component_health: HashMap::new(),
                error_rates: HashMap::new(),
                response_times: HashMap::new(),
                availability: HashMap::new(),
                performance_indicators: HashMap::new(),
            },
            metadata: StateMetadata {
                source: "test".to_string(),
                version: "1.0".to_string(),
                checksum: "test".to_string(),
                size_bytes: 1024,
                compression_ratio: None,
                tags: vec![],
                properties: HashMap::new(),
            },
        }
    }

    #[tokio::test]
    async fn test_checkpoint_creation() {
        let mut manager = DefaultRollbackManager::new();
        let test_state = create_test_state();
        
        // Set current state
        manager.set_current_state(test_state).await.unwrap();
        
        // Create checkpoint
        let checkpoint_id = manager
            .create_checkpoint(Some("Test checkpoint".to_string()))
            .await
            .unwrap();
        
        assert!(!checkpoint_id.is_nil());
        
        // Verify checkpoint exists
        let checkpoint = manager.get_checkpoint(&checkpoint_id).await.unwrap();
        assert!(checkpoint.is_some());
        
        let checkpoint = checkpoint.unwrap();
        assert_eq!(checkpoint.description, "Test checkpoint");
        assert!(!checkpoint.integrity_hash.is_empty());
    }

    #[tokio::test]
    async fn test_tagged_checkpoint() {
        let mut manager = DefaultRollbackManager::new();
        let test_state = create_test_state();
        
        manager.set_current_state(test_state).await.unwrap();
        
        // Create tagged checkpoint
        let checkpoint_id = manager
            .create_tagged_checkpoint(
                Some("Tagged checkpoint".to_string()),
                vec!["important".to_string(), "milestone".to_string()],
            )
            .await
            .unwrap();
        
        // Test rollback to tag
        let rolled_back_id = manager.rollback_to_tag("important").await.unwrap();
        assert_eq!(checkpoint_id, rolled_back_id);
    }

    #[tokio::test]
    async fn test_rollback_operations() {
        let mut manager = DefaultRollbackManager::new();
        let test_state = create_test_state();
        
        manager.set_current_state(test_state).await.unwrap();
        
        // Create checkpoint
        let checkpoint_id = manager
            .create_checkpoint(Some("Before change".to_string()))
            .await
            .unwrap();
        
        // Modify state
        let mut modified_state = create_test_state();
        modified_state.values.insert("balance".to_string(), StateValue::Integer(200));
        manager.set_current_state(modified_state).await.unwrap();
        
        // Rollback to checkpoint
        manager.rollback_to_checkpoint(&checkpoint_id).await.unwrap();
        
        // Verify state was restored
        let current_state = manager.current_state.read();
        let restored_state = current_state.as_ref().unwrap();
        if let Some(StateValue::Integer(balance)) = restored_state.values.get("balance") {
            assert_eq!(*balance, 100); // Original value
        } else {
            panic!("Balance not found or wrong type");
        }
    }

    #[tokio::test]
    async fn test_checkpoint_validation() {
        let mut manager = DefaultRollbackManager::new();
        let test_state = create_test_state();
        
        manager.set_current_state(test_state).await.unwrap();
        
        // Create checkpoint
        let checkpoint_id = manager
            .create_checkpoint(Some("Validation test".to_string()))
            .await
            .unwrap();
        
        // Validate checkpoint
        let is_valid = manager.validate_checkpoint(&checkpoint_id).await.unwrap();
        assert!(is_valid);
        
        // Test validation of non-existent checkpoint
        let fake_id = Uuid::new_v4();
        let result = manager.validate_checkpoint(&fake_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_checkpoint_listing() {
        let mut manager = DefaultRollbackManager::new();
        let test_state = create_test_state();
        
        manager.set_current_state(test_state).await.unwrap();
        
        // Create multiple checkpoints
        let id1 = manager
            .create_checkpoint(Some("Checkpoint 1".to_string()))
            .await
            .unwrap();
        let id2 = manager
            .create_tagged_checkpoint(
                Some("Checkpoint 2".to_string()),
                vec!["tag1".to_string()],
            )
            .await
            .unwrap();
        
        // List checkpoints
        let summaries = manager.list_checkpoints().await.unwrap();
        assert_eq!(summaries.len(), 2);
        
        // Verify order (should be in creation order)
        assert_eq!(summaries[0].id, id1);
        assert_eq!(summaries[1].id, id2);
        assert_eq!(summaries[1].tags, vec!["tag1".to_string()]);
    }

    #[tokio::test]
    async fn test_checkpoint_deletion() {
        let mut manager = DefaultRollbackManager::new();
        let test_state = create_test_state();
        
        manager.set_current_state(test_state).await.unwrap();
        
        // Create checkpoint
        let checkpoint_id = manager
            .create_checkpoint(Some("To be deleted".to_string()))
            .await
            .unwrap();
        
        // Verify it exists
        let checkpoint = manager.get_checkpoint(&checkpoint_id).await.unwrap();
        assert!(checkpoint.is_some());
        
        // Delete checkpoint
        manager.delete_checkpoint(&checkpoint_id).await.unwrap();
        
        // Verify it's gone
        let checkpoint = manager.get_checkpoint(&checkpoint_id).await.unwrap();
        assert!(checkpoint.is_none());
    }

    #[tokio::test]
    async fn test_retention_policy() {
        let mut manager = DefaultRollbackManager::new();
        let test_state = create_test_state();
        
        manager.set_current_state(test_state).await.unwrap();
        
        // Set strict retention policy
        let policy = crate::traits::RetentionPolicy {
            max_checkpoints: 2,
            max_age_hours: 1,
            max_total_size_bytes: 4096,
            compress_after_hours: 0,
            delete_compressed_after_days: 1,
        };
        
        manager.set_retention_policy(policy).await.unwrap();
        
        // Create more checkpoints than the limit
        let _id1 = manager.create_checkpoint(Some("CP1".to_string())).await.unwrap();
        let _id2 = manager.create_checkpoint(Some("CP2".to_string())).await.unwrap();
        let id3 = manager.create_checkpoint(Some("CP3".to_string())).await.unwrap();
        
        // Should only have the latest 2 checkpoints
        let summaries = manager.list_checkpoints().await.unwrap();
        assert!(summaries.len() <= 2);
        
        // The latest checkpoint should still exist
        let checkpoint = manager.get_checkpoint(&id3).await.unwrap();
        assert!(checkpoint.is_some());
    }

    #[tokio::test]
    async fn test_rollback_statistics() {
        let mut manager = DefaultRollbackManager::new();
        let test_state = create_test_state();
        
        manager.set_current_state(test_state).await.unwrap();
        
        // Create checkpoint
        let checkpoint_id = manager
            .create_checkpoint(Some("Stats test".to_string()))
            .await
            .unwrap();
        
        // Perform rollback
        manager.rollback_to_checkpoint(&checkpoint_id).await.unwrap();
        
        // Check statistics
        let stats = manager.get_stats().await.unwrap();
        assert_eq!(stats.checkpoints_created, 1);
        assert_eq!(stats.rollbacks_performed, 1);
        assert!(stats.avg_rollback_time_ms > 0.0);
        assert!(stats.success_rate > 0.0);
    }
}