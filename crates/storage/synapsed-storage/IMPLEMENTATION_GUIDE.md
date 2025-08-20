# Synapsed Storage Implementation Guide

## Overview

Based on the research findings, this guide provides a concrete implementation plan for the synapsed-storage module.

## 1. Module Structure

```
synapsed-storage/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs              # Public API and re-exports
│   ├── traits.rs           # Core storage traits
│   ├── error.rs            # Error types
│   ├── config.rs           # Configuration structures
│   ├── backends/           # Storage backend implementations
│   │   ├── mod.rs
│   │   ├── memory.rs       # In-memory storage for testing
│   │   ├── rocksdb.rs      # RocksDB backend
│   │   ├── sled.rs         # Sled backend
│   │   └── sqlite.rs       # SQLite backend
│   ├── cache/              # Caching layer
│   │   ├── mod.rs
│   │   ├── lru.rs          # LRU cache implementation
│   │   ├── arc.rs          # ARC cache implementation
│   │   └── distributed.rs  # Distributed cache
│   ├── compression/        # Compression layer
│   │   ├── mod.rs
│   │   ├── lz4.rs          # LZ4 compression
│   │   ├── zstd.rs         # Zstandard compression
│   │   └── adaptive.rs     # Adaptive compression strategy
│   ├── distributed/        # Distributed storage features
│   │   ├── mod.rs
│   │   ├── partitioner.rs  # Partitioning strategies
│   │   ├── replication.rs  # Replication logic
│   │   └── consensus.rs    # Raft-based consensus
│   ├── metrics/            # Monitoring and metrics
│   │   ├── mod.rs
│   │   └── collector.rs    # Metrics collection
│   └── utils/              # Utilities
│       ├── mod.rs
│       └── buffer_pool.rs  # Buffer management
├── benches/                # Performance benchmarks
│   ├── storage_bench.rs
│   └── compression_bench.rs
├── examples/               # Usage examples
│   ├── basic_usage.rs
│   ├── distributed.rs
│   └── custom_backend.rs
└── tests/                  # Integration tests
    ├── common/
    │   └── mod.rs
    ├── backend_tests.rs
    ├── cache_tests.rs
    └── distributed_tests.rs
```

## 2. Core Trait Definitions

### `src/traits.rs`
```rust
use async_trait::async_trait;
use std::error::Error;
use bytes::Bytes;

/// Core storage trait that all backends must implement
#[async_trait]
pub trait Storage: Send + Sync {
    type Error: Error + Send + Sync + 'static;
    
    /// Get a value by key
    async fn get(&self, key: &[u8]) -> Result<Option<Bytes>, Self::Error>;
    
    /// Store a key-value pair
    async fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Self::Error>;
    
    /// Delete a key
    async fn delete(&self, key: &[u8]) -> Result<(), Self::Error>;
    
    /// Check if a key exists
    async fn exists(&self, key: &[u8]) -> Result<bool, Self::Error> {
        Ok(self.get(key).await?.is_some())
    }
    
    /// Flush any pending writes
    async fn flush(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Batched operations for improved throughput
#[async_trait]
pub trait BatchedStorage: Storage {
    /// Get multiple values
    async fn batch_get(&self, keys: &[&[u8]]) -> Result<Vec<Option<Bytes>>, Self::Error>;
    
    /// Store multiple key-value pairs
    async fn batch_put(&self, items: &[(&[u8], &[u8])]) -> Result<(), Self::Error>;
    
    /// Delete multiple keys
    async fn batch_delete(&self, keys: &[&[u8]]) -> Result<(), Self::Error>;
}

/// Iteration support
#[async_trait]
pub trait IterableStorage: Storage {
    type Iterator: StorageIterator;
    
    /// Iterate over a key range
    async fn iter(&self, start: Option<&[u8]>, end: Option<&[u8]>) 
        -> Result<Self::Iterator, Self::Error>;
    
    /// Iterate with a key prefix
    async fn prefix_iter(&self, prefix: &[u8]) 
        -> Result<Self::Iterator, Self::Error>;
}

/// Iterator trait for storage traversal
#[async_trait]
pub trait StorageIterator: Send {
    type Error: Error + Send + Sync + 'static;
    
    /// Get next key-value pair
    async fn next(&mut self) -> Result<Option<(Bytes, Bytes)>, Self::Error>;
    
    /// Seek to a specific key
    async fn seek(&mut self, key: &[u8]) -> Result<(), Self::Error>;
}

/// Transaction support
#[async_trait]
pub trait TransactionalStorage: Storage {
    type Transaction: StorageTransaction;
    
    /// Begin a new transaction
    async fn begin_transaction(&self) -> Result<Self::Transaction, Self::Error>;
}

/// Transaction operations
#[async_trait]
pub trait StorageTransaction: Send {
    type Error: Error + Send + Sync + 'static;
    
    async fn get(&self, key: &[u8]) -> Result<Option<Bytes>, Self::Error>;
    async fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), Self::Error>;
    async fn delete(&mut self, key: &[u8]) -> Result<(), Self::Error>;
    async fn commit(self) -> Result<(), Self::Error>;
    async fn rollback(self) -> Result<(), Self::Error>;
}
```

## 3. Configuration System

### `src/config.rs`
```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StorageConfig {
    Memory(MemoryConfig),
    RocksDb(RocksDbConfig),
    Sled(SledConfig),
    Sqlite(SqliteConfig),
    Distributed(DistributedConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    #[serde(default = "default_capacity")]
    pub initial_capacity: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocksDbConfig {
    pub path: PathBuf,
    #[serde(default)]
    pub options: RocksDbOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocksDbOptions {
    #[serde(default = "default_cache_size")]
    pub block_cache_size: usize,
    #[serde(default = "default_write_buffer_size")]
    pub write_buffer_size: usize,
    #[serde(default = "default_max_open_files")]
    pub max_open_files: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    #[serde(default = "default_cache_type")]
    pub cache_type: CacheType,
    #[serde(default = "default_max_entries")]
    pub max_entries: usize,
    #[serde(default = "default_ttl")]
    pub ttl_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheType {
    Lru,
    Lfu,
    Arc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_algorithm")]
    pub algorithm: CompressionAlgorithm,
    #[serde(default = "default_min_size")]
    pub min_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionAlgorithm {
    Lz4,
    Zstd,
    Snappy,
    None,
}

fn default_capacity() -> usize { 1024 * 1024 }
fn default_cache_size() -> usize { 64 * 1024 * 1024 }
fn default_write_buffer_size() -> usize { 16 * 1024 * 1024 }
fn default_max_open_files() -> i32 { 1000 }
fn default_cache_type() -> CacheType { CacheType::Lru }
fn default_max_entries() -> usize { 10000 }
fn default_ttl() -> Option<u64> { None }
fn default_algorithm() -> CompressionAlgorithm { CompressionAlgorithm::Lz4 }
fn default_min_size() -> usize { 1024 }
```

## 4. Storage Builder

### `src/lib.rs`
```rust
use crate::config::StorageConfig;
use crate::traits::Storage;
use std::sync::Arc;

/// Main storage builder
pub struct StorageBuilder {
    config: StorageConfig,
    cache_config: Option<CacheConfig>,
    compression_config: Option<CompressionConfig>,
}

impl StorageBuilder {
    pub fn new(config: StorageConfig) -> Self {
        Self {
            config,
            cache_config: None,
            compression_config: None,
        }
    }
    
    pub fn with_cache(mut self, config: CacheConfig) -> Self {
        self.cache_config = Some(config);
        self
    }
    
    pub fn with_compression(mut self, config: CompressionConfig) -> Self {
        self.compression_config = Some(config);
        self
    }
    
    pub async fn build(self) -> Result<Arc<dyn Storage<Error = StorageError>>, StorageError> {
        // Build base storage
        let storage = match self.config {
            StorageConfig::Memory(cfg) => {
                Arc::new(backends::memory::MemoryStorage::new(cfg)) as Arc<dyn Storage<Error = StorageError>>
            }
            StorageConfig::RocksDb(cfg) => {
                Arc::new(backends::rocksdb::RocksDbStorage::new(cfg)?) as Arc<dyn Storage<Error = StorageError>>
            }
            // ... other backends
        };
        
        // Apply compression layer if configured
        let storage = if let Some(compression_cfg) = self.compression_config {
            Arc::new(compression::CompressionLayer::new(storage, compression_cfg))
        } else {
            storage
        };
        
        // Apply cache layer if configured
        let storage = if let Some(cache_cfg) = self.cache_config {
            Arc::new(cache::CacheLayer::new(storage, cache_cfg))
        } else {
            storage
        };
        
        Ok(storage)
    }
}
```

## 5. Example Usage

### `examples/basic_usage.rs`
```rust
use synapsed_storage::{StorageBuilder, StorageConfig, MemoryConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an in-memory storage
    let storage = StorageBuilder::new(StorageConfig::Memory(MemoryConfig {
        initial_capacity: 1024 * 1024,
    }))
    .build()
    .await?;
    
    // Basic operations
    storage.put(b"key1", b"value1").await?;
    
    if let Some(value) = storage.get(b"key1").await? {
        println!("Retrieved: {}", String::from_utf8_lossy(&value));
    }
    
    storage.delete(b"key1").await?;
    
    Ok(())
}
```

## 6. Testing Strategy

### Unit Tests
- Test each storage backend independently
- Test cache implementations
- Test compression algorithms
- Test error handling

### Integration Tests
- Test layered storage stack
- Test distributed scenarios
- Test concurrent access
- Test transaction semantics

### Performance Tests
- Benchmark different backends
- Measure cache hit rates
- Profile compression overhead
- Test scalability limits

## 7. Integration Points

### With Synapsed Substrates
```rust
impl<S: Storage> Source for StorageSource<S> {
    type Item = StorageEvent;
    type Error = StorageError;
    
    fn subscribe(&self) -> impl Stream<Item = Result<Self::Item, Self::Error>> {
        // Implementation
    }
}
```

### With Synapsed Net
- Use for distributed state synchronization
- Integrate with transport layer for replication
- Leverage observability for monitoring

### With Synapsed Identity
- Store identity data securely
- Support for encrypted storage
- Access control integration

## 8. Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Get Latency (p50) | < 100μs | In-memory backend |
| Get Latency (p99) | < 1ms | Persistent backend |
| Put Latency (p50) | < 500μs | With compression |
| Throughput | > 100K ops/s | Single node |
| Cache Hit Rate | > 80% | Typical workload |

## 9. Security Considerations

1. **Encryption at Rest**: Optional encryption layer using synapsed-crypto
2. **Access Control**: Integration with synapsed-identity for permissions
3. **Audit Logging**: All operations can be logged for compliance
4. **Data Integrity**: Checksums for corruption detection

## 10. Cross-Module Integration

### 10.1 Integration with Synapsed Substrates

The storage module provides event sourcing capabilities for the substrates system:

```rust
use synapsed_substrates::{Source, Event, EventStream};
use synapsed_storage::{Storage, StorageBuilder};

/// Storage-backed event source
pub struct StorageEventSource<S: Storage> {
    storage: Arc<S>,
    event_log_prefix: String,
}

impl<S: Storage> StorageEventSource<S> {
    pub fn new(storage: Arc<S>, prefix: &str) -> Self {
        Self {
            storage,
            event_log_prefix: prefix.to_string(),
        }
    }
}

#[async_trait]
impl<S: Storage> Source for StorageEventSource<S> {
    type Item = StorageEvent;
    type Error = S::Error;
    
    fn subscribe(&self) -> impl Stream<Item = Result<Self::Item, Self::Error>> {
        // Create event stream from storage changes
        let prefix = self.event_log_prefix.clone();
        let storage = Arc::clone(&self.storage);
        
        async_stream::stream! {
            // Monitor storage for changes with prefix
            let mut last_sequence = 0u64;
            
            loop {
                // Poll for new events
                let events = storage
                    .prefix_iter(prefix.as_bytes())
                    .await
                    .map_err(|e| e)?;
                
                // Yield new events
                while let Some((key, data)) = events.next().await? {
                    let event = StorageEvent::deserialize(&data)?;
                    if event.sequence > last_sequence {
                        last_sequence = event.sequence;
                        yield Ok(event);
                    }
                }
                
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
}
```

### 10.2 Integration with Synapsed Net

Network synchronization and distributed storage coordination:

```rust
use synapsed_net::{Transport, Node, Message};
use synapsed_storage::{Storage, ReplicationConfig};

/// Network-aware distributed storage
pub struct NetworkStorage<T: Transport, S: Storage> {
    local_storage: Arc<S>,
    transport: Arc<T>,
    replication_config: ReplicationConfig,
    peers: Vec<Node>,
}

impl<T: Transport, S: Storage> NetworkStorage<T, S> {
    pub async fn sync_with_peers(&self) -> Result<(), NetworkError> {
        let local_version = self.local_storage.version().await?;
        
        for peer in &self.peers {
            let peer_version = self.transport
                .request(peer, Message::VersionRequest)
                .await?;
                
            if peer_version > local_version {
                // Pull updates from peer
                self.pull_updates_from_peer(peer).await?;
            } else if local_version > peer_version {
                // Push updates to peer
                self.push_updates_to_peer(peer).await?;
            }
        }
        
        Ok(())
    }
    
    async fn replicate_write(&self, key: &[u8], value: &[u8]) -> Result<(), NetworkError> {
        // Write to local storage first
        self.local_storage.put(key, value).await?;
        
        // Replicate to required number of peers
        let required_replicas = self.replication_config.min_replicas;
        let mut successful_replicas = 0;
        
        for peer in &self.peers {
            if successful_replicas >= required_replicas {
                break;
            }
            
            let replication_msg = Message::ReplicationWrite {
                key: key.to_vec(),
                value: value.to_vec(),
                timestamp: SystemTime::now(),
            };
            
            match self.transport.send(peer, replication_msg).await {
                Ok(_) => successful_replicas += 1,
                Err(e) => tracing::warn!("Failed to replicate to peer {}: {}", peer.id(), e),
            }
        }
        
        if successful_replicas < required_replicas {
            return Err(NetworkError::InsufficientReplicas {
                required: required_replicas,
                achieved: successful_replicas,
            });
        }
        
        Ok(())
    }
}
```

### 10.3 Integration with Synapsed Identity

Access control and encryption key management:

```rust
use synapsed_identity::{Identity, Permission, KeyManager};
use synapsed_storage::{Storage, EncryptionLayer};

/// Identity-aware encrypted storage
pub struct SecureStorage<S: Storage> {
    inner: Arc<S>,
    key_manager: Arc<dyn KeyManager>,
    access_control: Arc<dyn AccessControl>,
}

impl<S: Storage> SecureStorage<S> {
    pub async fn authorized_get(
        &self, 
        identity: &Identity, 
        key: &[u8]
    ) -> Result<Option<Vec<u8>>, SecurityError> {
        // Check read permission
        self.access_control
            .check_permission(identity, key, Permission::Read)
            .await?;
        
        // Get encrypted data
        let encrypted_data = self.inner.get(key).await?;
        
        if let Some(data) = encrypted_data {
            // Decrypt using identity's key
            let decryption_key = self.key_manager
                .get_decryption_key(identity, key)
                .await?;
                
            let decrypted = decrypt(&data, &decryption_key)?;
            Ok(Some(decrypted))
        } else {
            Ok(None)
        }
    }
    
    pub async fn authorized_put(
        &self,
        identity: &Identity,
        key: &[u8],
        value: &[u8]
    ) -> Result<(), SecurityError> {
        // Check write permission
        self.access_control
            .check_permission(identity, key, Permission::Write)
            .await?;
        
        // Encrypt using identity's key
        let encryption_key = self.key_manager
            .get_encryption_key(identity, key)
            .await?;
            
        let encrypted_data = encrypt(value, &encryption_key)?;
        
        // Store encrypted data
        self.inner.put(key, &encrypted_data).await?;
        
        // Log access for audit
        tracing::info!(
            identity = %identity.id(),
            key = %hex::encode(key),
            operation = "put",
            "Authorized storage operation"
        );
        
        Ok(())
    }
}
```

### 10.4 Integration with Synapsed Crypto

Cryptographic operations and secure data handling:

```rust
use synapsed_crypto::{
    Hash, Cipher, KeyDerivation, 
    SymmetricKey, AsymmetricKeyPair
};
use synapsed_storage::Storage;

/// Crypto-enhanced storage with integrity verification
pub struct CryptoStorage<S: Storage> {
    inner: Arc<S>,
    cipher: Arc<dyn Cipher>,
    hash_function: Arc<dyn Hash>,
    master_key: SymmetricKey,
}

impl<S: Storage> CryptoStorage<S> {
    pub async fn put_with_integrity(
        &self,
        key: &[u8],
        value: &[u8]
    ) -> Result<(), CryptoError> {
        // Generate content hash for integrity
        let content_hash = self.hash_function.hash(value);
        
        // Create envelope with metadata
        let envelope = StorageEnvelope {
            content_hash,
            timestamp: SystemTime::now(),
            encryption_algorithm: self.cipher.algorithm(),
            data: value.to_vec(),
        };
        
        // Serialize and encrypt envelope
        let serialized = bincode::serialize(&envelope)?;
        let encrypted = self.cipher.encrypt(&serialized, &self.master_key)?;
        
        // Store encrypted envelope
        self.inner.put(key, &encrypted).await?;
        
        // Store hash separately for quick integrity checks
        let hash_key = format!("hash:{}", hex::encode(key));
        self.inner.put(hash_key.as_bytes(), &content_hash).await?;
        
        Ok(())
    }
    
    pub async fn get_with_verification(
        &self,
        key: &[u8]
    ) -> Result<Option<Vec<u8>>, CryptoError> {
        // Get encrypted envelope
        let encrypted_data = match self.inner.get(key).await? {
            Some(data) => data,
            None => return Ok(None),
        };
        
        // Decrypt envelope
        let decrypted = self.cipher.decrypt(&encrypted_data, &self.master_key)?;
        let envelope: StorageEnvelope = bincode::deserialize(&decrypted)?;
        
        // Verify integrity
        let computed_hash = self.hash_function.hash(&envelope.data);
        if computed_hash != envelope.content_hash {
            return Err(CryptoError::IntegrityCheckFailed {
                expected: envelope.content_hash,
                computed: computed_hash,
            });
        }
        
        Ok(Some(envelope.data))
    }
    
    pub async fn rotate_encryption_key(&self) -> Result<(), CryptoError> {
        // Generate new key
        let new_key = SymmetricKey::generate()?;
        
        // Re-encrypt all data with new key
        let all_keys = self.inner.list_keys(None).await?;
        
        for stored_key in all_keys {
            if stored_key.starts_with(b"hash:") {
                continue; // Skip hash entries
            }
            
            // Decrypt with old key
            let data = self.get_with_verification(&stored_key).await?;
            if let Some(plaintext) = data {
                // Re-encrypt with new key
                let old_master_key = self.master_key.clone();
                let mut temp_storage = self.clone();
                temp_storage.master_key = new_key.clone();
                
                temp_storage.put_with_integrity(&stored_key, &plaintext).await?;
            }
        }
        
        // Update master key
        // This would typically be done atomically
        // with proper key management procedures
        
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct StorageEnvelope {
    content_hash: Vec<u8>,
    timestamp: SystemTime,
    encryption_algorithm: String,
    data: Vec<u8>,
}
```

### 10.5 Integration Testing Framework

Comprehensive testing framework for cross-module integration:

```rust
use synapsed_storage::*;
use synapsed_net::*;
use synapsed_identity::*;
use synapsed_crypto::*;

#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_complete_integration_flow() -> Result<(), Box<dyn std::error::Error>> {
        // Set up all components
        let storage = create_test_storage().await?;
        let identity_manager = create_test_identity_manager().await?;
        let crypto_provider = create_test_crypto_provider().await?;
        let network_transport = create_test_network().await?;
        
        // Create integrated storage system
        let secure_storage = SecureStorage::new(
            storage,
            identity_manager.key_manager(),
            identity_manager.access_control(),
        );
        
        let crypto_storage = CryptoStorage::new(
            secure_storage,
            crypto_provider.cipher(),
            crypto_provider.hash_function(),
            crypto_provider.generate_master_key()?,
        );
        
        let network_storage = NetworkStorage::new(
            crypto_storage,
            network_transport,
            ReplicationConfig::default(),
        );
        
        // Test complete workflow
        let test_identity = identity_manager.create_test_identity("test_user").await?;
        let test_key = b"integration_test_key";
        let test_value = b"sensitive_test_data";
        
        // Write with full security and replication
        network_storage
            .authorized_put(&test_identity, test_key, test_value)
            .await?;
        
        // Read with verification
        let retrieved = network_storage
            .authorized_get(&test_identity, test_key)
            .await?
            .expect("Data should exist");
        
        assert_eq!(retrieved, test_value);
        
        // Test network synchronization
        network_storage.sync_with_peers().await?;
        
        // Verify data integrity after sync
        let post_sync_data = network_storage
            .authorized_get(&test_identity, test_key)
            .await?
            .expect("Data should still exist after sync");
        
        assert_eq!(post_sync_data, test_value);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_storage_substrate_integration() -> Result<(), Box<dyn std::error::Error>> {
        let storage = create_test_storage().await?;
        let event_source = StorageEventSource::new(storage.clone(), "events:");
        
        // Write some events
        for i in 0..10 {
            let event = StorageEvent {
                sequence: i,
                event_type: "test_event".to_string(),
                data: format!("Event {}", i).into_bytes(),
                timestamp: SystemTime::now(),
            };
            
            let key = format!("events:{:08}", i);
            let serialized = bincode::serialize(&event)?;
            storage.put(key.as_bytes(), &serialized).await?;
        }
        
        // Subscribe to events
        let mut event_stream = event_source.subscribe();
        let mut received_events = Vec::new();
        
        // Collect events with timeout
        tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(event_result) = event_stream.next().await {
                match event_result {
                    Ok(event) => {
                        received_events.push(event);
                        if received_events.len() >= 10 {
                            break;
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
            Ok::<_, Box<dyn std::error::Error>>(())
        }).await??;
        
        assert_eq!(received_events.len(), 10);
        assert!(received_events.iter().all(|e| e.event_type == "test_event"));
        
        Ok(())
    }
}
```

## 11. Next Steps

1. Implement core traits and error types
2. Build in-memory backend for testing
3. Add RocksDB backend for production
4. Implement LRU cache
5. Add compression support
6. Create comprehensive test suite
7. Performance benchmarking
8. Documentation and examples
9. **Complete cross-module integration implementations**
10. **Integration testing with other Synapsed modules**
11. Production hardening and optimization

This implementation guide provides a solid foundation for building a robust, performant, and flexible storage system for the Synapsed ecosystem with comprehensive integration capabilities.