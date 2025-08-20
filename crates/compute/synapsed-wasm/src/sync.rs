//! Rsync-like synchronization algorithms for P2P data sync
//!
//! This module provides WebAssembly-compatible rsync-like chunking and synchronization
//! algorithms optimized for P2P networks. It includes efficient delta calculation,
//! chunk-based transfers, and bandwidth optimization.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use sha2::{Digest, Sha256};

use crate::error::{WasmError, WasmResult};
use crate::types::{HostFunction, WasmValue};
use crate::{MAX_SYNC_CHUNK_SIZE};

/// Sync engine for efficient P2P data synchronization
pub struct SyncEngine {
    /// Active sync operations
    sync_ops: HashMap<String, SyncOperation>,
    /// Chunk cache
    chunk_cache: HashMap<String, ChunkInfo>,
    /// Sync configuration
    config: SyncConfig,
    /// Sync statistics
    stats: SyncStats,
}

impl SyncEngine {
    /// Create a new sync engine
    pub fn new() -> WasmResult<Self> {
        Ok(Self {
            sync_ops: HashMap::new(),
            chunk_cache: HashMap::new(),
            config: SyncConfig::default(),
            stats: SyncStats::default(),
        })
    }

    /// Start sync operation between local and remote data
    pub async fn start_sync(
        &mut self,
        sync_id: String,
        local_data: &[u8],
        remote_checksums: Vec<ChunkChecksum>,
    ) -> WasmResult<SyncPlan> {
        let chunks = self.chunk_data(local_data)?;
        let local_checksums = self.calculate_checksums(&chunks)?;
        
        let sync_plan = self.create_sync_plan(&local_checksums, &remote_checksums)?;
        
        let sync_op = SyncOperation::new(sync_id.clone(), chunks, sync_plan.clone());
        self.sync_ops.insert(sync_id.clone(), sync_op);
        self.stats.sync_operations_started += 1;

        tracing::info!(sync_id = %sync_id, chunks = chunks.len(), "Sync operation started");
        Ok(sync_plan)
    }

    /// Get next chunk to send based on sync plan
    pub async fn get_next_chunk(&mut self, sync_id: &str) -> WasmResult<Option<ChunkData>> {
        let sync_op = self.sync_ops.get_mut(sync_id)
            .ok_or_else(|| WasmError::Configuration(format!("Sync operation {} not found", sync_id)))?;

        if let Some(chunk_index) = sync_op.get_next_chunk_to_send() {
            if let Some(chunk) = sync_op.chunks.get(chunk_index) {
                let chunk_data = ChunkData {
                    index: chunk_index,
                    data: chunk.data.clone(),
                    checksum: chunk.checksum.clone(),
                };
                self.stats.chunks_sent += 1;
                Ok(Some(chunk_data))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Process received chunk
    pub async fn process_chunk(
        &mut self,
        sync_id: &str,
        chunk_data: ChunkData,
    ) -> WasmResult<()> {
        let sync_op = self.sync_ops.get_mut(sync_id)
            .ok_or_else(|| WasmError::Configuration(format!("Sync operation {} not found", sync_id)))?;

        // Verify chunk integrity
        let calculated_checksum = self.calculate_chunk_checksum(&chunk_data.data)?;
        if calculated_checksum != chunk_data.checksum {
            return Err(WasmError::Configuration("Chunk checksum mismatch".to_string()));
        }

        sync_op.add_received_chunk(chunk_data)?;
        self.stats.chunks_received += 1;

        tracing::debug!(sync_id = %sync_id, chunk_index = chunk_data.index, "Chunk processed");
        Ok(())
    }

    /// Finalize sync operation and reconstruct data
    pub async fn finalize_sync(&mut self, sync_id: &str) -> WasmResult<Vec<u8>> {
        let sync_op = self.sync_ops.remove(sync_id)
            .ok_or_else(|| WasmError::Configuration(format!("Sync operation {} not found", sync_id)))?;

        let reconstructed_data = sync_op.reconstruct_data()?;
        self.stats.sync_operations_completed += 1;
        self.stats.bytes_synced += reconstructed_data.len() as u64;

        tracing::info!(sync_id = %sync_id, data_size = reconstructed_data.len(), "Sync operation finalized");
        Ok(reconstructed_data)
    }

    /// Calculate bandwidth savings
    pub fn calculate_savings(&self, sync_id: &str) -> WasmResult<SyncSavings> {
        let sync_op = self.sync_ops.get(sync_id)
            .ok_or_else(|| WasmError::Configuration(format!("Sync operation {} not found", sync_id)))?;

        Ok(sync_op.calculate_savings())
    }

    /// Get sync statistics
    pub fn get_stats(&self) -> &SyncStats {
        &self.stats
    }

    /// Chunk data into fixed-size blocks
    fn chunk_data(&self, data: &[u8]) -> WasmResult<Vec<Chunk>> {
        let chunk_size = self.config.chunk_size;
        let mut chunks = Vec::new();

        for (index, chunk_data) in data.chunks(chunk_size).enumerate() {
            let checksum = self.calculate_chunk_checksum(chunk_data)?;
            let chunk = Chunk {
                index,
                data: chunk_data.to_vec(),
                checksum,
                size: chunk_data.len(),
            };
            chunks.push(chunk);
        }

        Ok(chunks)
    }

    /// Calculate checksums for chunks
    fn calculate_checksums(&self, chunks: &[Chunk]) -> WasmResult<Vec<ChunkChecksum>> {
        Ok(chunks.iter().map(|chunk| ChunkChecksum {
            index: chunk.index,
            weak_hash: self.calculate_weak_hash(&chunk.data),
            strong_hash: chunk.checksum.clone(),
        }).collect())
    }

    /// Create sync plan by comparing local and remote checksums
    fn create_sync_plan(
        &self,
        local_checksums: &[ChunkChecksum],
        remote_checksums: &[ChunkChecksum],
    ) -> WasmResult<SyncPlan> {
        let mut chunks_to_send = Vec::new();
        let mut chunks_to_request = Vec::new();
        let mut matching_chunks = Vec::new();

        // Create lookup maps for efficiency
        let remote_map: HashMap<String, &ChunkChecksum> = remote_checksums.iter()
            .map(|cs| (cs.strong_hash.clone(), cs))
            .collect();

        let local_map: HashMap<String, &ChunkChecksum> = local_checksums.iter()
            .map(|cs| (cs.strong_hash.clone(), cs))
            .collect();

        // Find chunks to send (local has, remote doesn't)
        for local_checksum in local_checksums {
            if !remote_map.contains_key(&local_checksum.strong_hash) {
                chunks_to_send.push(local_checksum.index);
            } else {
                matching_chunks.push(local_checksum.index);
            }
        }

        // Find chunks to request (remote has, local doesn't)
        for remote_checksum in remote_checksums {
            if !local_map.contains_key(&remote_checksum.strong_hash) {
                chunks_to_request.push(remote_checksum.index);
            }
        }

        Ok(SyncPlan {
            chunks_to_send,
            chunks_to_request,
            matching_chunks,
            total_local_chunks: local_checksums.len(),
            total_remote_chunks: remote_checksums.len(),
        })
    }

    /// Calculate chunk checksum using SHA-256
    fn calculate_chunk_checksum(&self, data: &[u8]) -> WasmResult<String> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    /// Calculate weak hash for rolling hash algorithm
    fn calculate_weak_hash(&self, data: &[u8]) -> u32 {
        // Simple Adler-32-like weak hash
        let mut a: u32 = 1;
        let mut b: u32 = 0;
        
        for &byte in data {
            a = (a + byte as u32) % 65521;
            b = (b + a) % 65521;
        }
        
        (b << 16) | a
    }
}

/// Individual sync operation
pub struct SyncOperation {
    /// Sync operation ID
    pub id: String,
    /// Local chunks
    pub chunks: Vec<Chunk>,
    /// Sync plan
    pub plan: SyncPlan,
    /// Received chunks from remote
    received_chunks: HashMap<usize, ChunkData>,
    /// Chunks sent to remote
    chunks_sent: Vec<usize>,
    /// Operation start time
    started_at: std::time::SystemTime,
}

impl SyncOperation {
    /// Create a new sync operation
    pub fn new(id: String, chunks: Vec<Chunk>, plan: SyncPlan) -> Self {
        Self {
            id,
            chunks,
            plan,
            received_chunks: HashMap::new(),
            chunks_sent: Vec::new(),
            started_at: std::time::SystemTime::now(),
        }
    }

    /// Get next chunk index to send
    pub fn get_next_chunk_to_send(&mut self) -> Option<usize> {
        for &chunk_index in &self.plan.chunks_to_send {
            if !self.chunks_sent.contains(&chunk_index) {
                self.chunks_sent.push(chunk_index);
                return Some(chunk_index);
            }
        }
        None
    }

    /// Add received chunk
    pub fn add_received_chunk(&mut self, chunk_data: ChunkData) -> WasmResult<()> {
        self.received_chunks.insert(chunk_data.index, chunk_data);
        Ok(())
    }

    /// Reconstruct data from local and received chunks
    pub fn reconstruct_data(self) -> WasmResult<Vec<u8>> {
        let mut reconstructed = Vec::new();
        let total_chunks = std::cmp::max(
            self.plan.total_local_chunks,
            self.plan.total_remote_chunks
        );

        for i in 0..total_chunks {
            if let Some(received_chunk) = self.received_chunks.get(&i) {
                // Use received chunk
                reconstructed.extend_from_slice(&received_chunk.data);
            } else if i < self.chunks.len() {
                // Use local chunk
                reconstructed.extend_from_slice(&self.chunks[i].data);
            }
        }

        Ok(reconstructed)
    }

    /// Calculate bandwidth savings
    pub fn calculate_savings(&self) -> SyncSavings {
        let total_data_size = self.chunks.iter().map(|c| c.size).sum::<usize>();
        let chunks_to_send_size: usize = self.plan.chunks_to_send.iter()
            .filter_map(|&i| self.chunks.get(i).map(|c| c.size))
            .sum();

        let bytes_saved = total_data_size.saturating_sub(chunks_to_send_size);
        let savings_percentage = if total_data_size > 0 {
            (bytes_saved as f64 / total_data_size as f64) * 100.0
        } else {
            0.0
        };

        SyncSavings {
            total_size: total_data_size,
            bytes_transferred: chunks_to_send_size,
            bytes_saved,
            savings_percentage,
        }
    }
}

/// Data chunk
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Chunk index
    pub index: usize,
    /// Chunk data
    pub data: Vec<u8>,
    /// Strong checksum (SHA-256)
    pub checksum: String,
    /// Chunk size
    pub size: usize,
}

/// Chunk checksum for comparison
#[derive(Debug, Clone)]
pub struct ChunkChecksum {
    /// Chunk index
    pub index: usize,
    /// Weak rolling hash
    pub weak_hash: u32,
    /// Strong cryptographic hash
    pub strong_hash: String,
}

/// Chunk data for transfer
#[derive(Debug, Clone)]
pub struct ChunkData {
    /// Chunk index
    pub index: usize,
    /// Chunk data
    pub data: Vec<u8>,
    /// Checksum for verification
    pub checksum: String,
}

/// Chunk information for caching
#[derive(Debug, Clone)]
pub struct ChunkInfo {
    /// Chunk checksum
    pub checksum: String,
    /// Last access time
    pub last_accessed: std::time::SystemTime,
    /// Access count
    pub access_count: u64,
}

/// Sync plan generated by comparing checksums
#[derive(Debug, Clone)]
pub struct SyncPlan {
    /// Chunks that need to be sent to remote
    pub chunks_to_send: Vec<usize>,
    /// Chunks that need to be requested from remote
    pub chunks_to_request: Vec<usize>,
    /// Chunks that match between local and remote
    pub matching_chunks: Vec<usize>,
    /// Total number of local chunks
    pub total_local_chunks: usize,
    /// Total number of remote chunks
    pub total_remote_chunks: usize,
}

/// Sync configuration
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Chunk size in bytes
    pub chunk_size: usize,
    /// Enable compression
    pub enable_compression: bool,
    /// Maximum concurrent transfers
    pub max_concurrent_transfers: usize,
    /// Cache size limit
    pub cache_size_limit: usize,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            chunk_size: std::cmp::min(MAX_SYNC_CHUNK_SIZE, 64 * 1024), // 64KB default
            enable_compression: true,
            max_concurrent_transfers: 4,
            cache_size_limit: 100 * 1024 * 1024, // 100MB
        }
    }
}

/// Bandwidth savings calculation
#[derive(Debug, Clone)]
pub struct SyncSavings {
    /// Total data size
    pub total_size: usize,
    /// Bytes actually transferred
    pub bytes_transferred: usize,
    /// Bytes saved
    pub bytes_saved: usize,
    /// Savings percentage
    pub savings_percentage: f64,
}

/// Sync statistics
#[derive(Debug, Clone, Default)]
pub struct SyncStats {
    /// Number of sync operations started
    pub sync_operations_started: u64,
    /// Number of sync operations completed
    pub sync_operations_completed: u64,
    /// Number of chunks sent
    pub chunks_sent: u64,
    /// Number of chunks received
    pub chunks_received: u64,
    /// Total bytes synced
    pub bytes_synced: u64,
    /// Total bandwidth saved
    pub bandwidth_saved: u64,
}

/// Create sync host functions for WASM modules
pub fn create_sync_host_functions() -> HashMap<String, HostFunction> {
    let mut functions = HashMap::new();

    // Start sync operation
    functions.insert(
        "sync_start".to_string(),
        Arc::new(|args| {
            match (args.get(0), args.get(1)) {
                (Some(WasmValue::String(sync_id)), Some(WasmValue::Bytes(data))) => {
                    tracing::info!(
                        sync_id = %sync_id,
                        data_size = data.len(),
                        "Starting sync operation"
                    );
                    Ok(vec![WasmValue::String("sync_plan".to_string())])
                }
                _ => Err(WasmError::Configuration("Invalid arguments for sync start".to_string()))
            }
        }) as HostFunction,
    );

    // Get chunk checksums
    functions.insert(
        "sync_get_checksums".to_string(),
        Arc::new(|args| {
            if let Some(WasmValue::Bytes(data)) = args.get(0) {
                tracing::debug!(data_size = data.len(), "Calculating chunk checksums");
                // Return mock checksums
                Ok(vec![WasmValue::Bytes(b"chunk_checksums".to_vec())])
            } else {
                Err(WasmError::Configuration("Data required for checksum calculation".to_string()))
            }
        }) as HostFunction,
    );

    // Send chunk
    functions.insert(
        "sync_send_chunk".to_string(),
        Arc::new(|args| {
            match (args.get(0), args.get(1), args.get(2)) {
                (Some(WasmValue::String(sync_id)),
                 Some(WasmValue::I32(chunk_index)), 
                 Some(WasmValue::Bytes(chunk_data))) => {
                    tracing::debug!(
                        sync_id = %sync_id,
                        chunk_index = *chunk_index,
                        chunk_size = chunk_data.len(),
                        "Sending chunk"
                    );
                    Ok(vec![WasmValue::I32(1)]) // Success
                }
                _ => Err(WasmError::Configuration("Invalid arguments for chunk send".to_string()))
            }
        }) as HostFunction,
    );

    // Receive chunk
    functions.insert(
        "sync_receive_chunk".to_string(),
        Arc::new(|args| {
            if let Some(WasmValue::String(sync_id)) = args.get(0) {
                tracing::debug!(sync_id = %sync_id, "Receiving chunk");
                // Return mock chunk data
                Ok(vec![
                    WasmValue::I32(0), // chunk index
                    WasmValue::Bytes(b"chunk_data".to_vec())
                ])
            } else {
                Err(WasmError::Configuration("Sync ID required".to_string()))
            }
        }) as HostFunction,
    );

    functions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_engine() {
        let mut engine = SyncEngine::new().unwrap();
        
        let data = b"Hello, World! This is test data for chunking.";
        let chunks = engine.chunk_data(data).unwrap();
        
        assert!(!chunks.is_empty());
        assert!(chunks.iter().all(|c| c.size <= engine.config.chunk_size));
    }

    #[test]
    fn test_checksum_calculation() {
        let engine = SyncEngine::new().unwrap();
        let data = b"test data";
        
        let checksum1 = engine.calculate_chunk_checksum(data).unwrap();
        let checksum2 = engine.calculate_chunk_checksum(data).unwrap();
        
        assert_eq!(checksum1, checksum2);
        assert!(!checksum1.is_empty());
    }

    #[test]
    fn test_weak_hash() {
        let engine = SyncEngine::new().unwrap();
        let data1 = b"test data";
        let data2 = b"test data";
        let data3 = b"different";
        
        let hash1 = engine.calculate_weak_hash(data1);
        let hash2 = engine.calculate_weak_hash(data2);
        let hash3 = engine.calculate_weak_hash(data3);
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_sync_config() {
        let config = SyncConfig::default();
        assert!(config.chunk_size <= MAX_SYNC_CHUNK_SIZE);
        assert!(config.enable_compression);
        assert_eq!(config.max_concurrent_transfers, 4);
    }

    #[test]
    fn test_sync_savings() {
        let savings = SyncSavings {
            total_size: 1000,
            bytes_transferred: 300,
            bytes_saved: 700,
            savings_percentage: 70.0,
        };
        
        assert_eq!(savings.total_size, 1000);
        assert_eq!(savings.bytes_saved, 700);
        assert_eq!(savings.savings_percentage, 70.0);
    }
}