# Trust Persistence System

The Synapsed Swarm trust persistence system provides a robust, production-ready storage layer for agent trust scores with multiple storage backends, transaction support, schema migrations, and comprehensive error handling.

## Overview

The system is designed around the `TrustStore` trait, which provides a consistent interface for storing and retrieving trust data across different storage implementations. This enables flexible deployment options while maintaining data integrity and performance.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     TrustManager                        │
├─────────────────────────────────────────────────────────┤
│  • In-memory cache for performance                      │
│  • Periodic backup management                           │
│  • Transaction coordination                             │
│  • Time decay and maintenance                           │
└─────────────────┬───────────────────────────────────────┘
                  │ TrustStore trait
┌─────────────────┼───────────────────────────────────────┐
│                 ▼                                       │
├─────────────────────────────────────────────────────────┤
│  SqliteTrustStore  │  FileTrustStore  │  InMemoryTrustStore│
├─────────────────────────────────────────────────────────┤
│  • ACID transactions │  • JSON files    │  • Testing only   │
│  • Schema migrations │  • Atomic writes │  • Fast operations │
│  • Backup/restore    │  • File locking  │  • No persistence  │
│  • Concurrent access │  • Backup support│  • Full features   │
└─────────────────────────────────────────────────────────┘
```

## Storage Implementations

### 1. SqliteTrustStore

Production-ready SQLite-based storage with full ACID compliance.

**Features:**
- ACID transactions for data integrity
- Schema versioning and migrations
- Automatic periodic backups
- Concurrent access support
- Query optimization with indexes
- Comprehensive error handling

**Usage:**
```rust
use synapsed_swarm::persistence::SqliteTrustStore;
use std::path::Path;

let store = SqliteTrustStore::new(
    Path::new("trust.db"),
    Some(Path::new("./backups"))
).await?;

store.initialize().await?;
```

**Schema:**
- `trust_scores`: Current trust scores for agents
- `trust_updates`: Historical trust update events  
- `schema_info`: Version tracking for migrations
- `agent_metadata`: Optional agent information (v3+)

### 2. FileTrustStore

JSON file-based storage for simple deployments.

**Features:**
- Human-readable JSON format
- Atomic file operations
- Read/write locking for concurrency
- Backup and restore support
- No external dependencies

**Usage:**
```rust
use synapsed_swarm::persistence::FileTrustStore;

let store = FileTrustStore::new(
    Path::new("./trust_data"),
    Some(Path::new("./backups"))
)?;

store.initialize().await?;
```

**Files:**
- `trust_scores.json`: Current trust scores
- `trust_updates.json`: Update history

### 3. InMemoryTrustStore

In-memory storage for testing and development.

**Features:**
- Lightning-fast operations
- Full feature compatibility
- No persistence (intentional)
- Perfect for unit tests

**Usage:**
```rust
use synapsed_swarm::persistence::InMemoryTrustStore;

let store = InMemoryTrustStore::new();
store.initialize().await?;
```

## Transaction Support

All storage implementations support atomic transactions for data consistency:

```rust
// Begin transaction
let mut tx = store.begin_transaction().await?;

// Perform multiple operations atomically
tx.store_trust_score(agent_id, new_score).await?;
tx.store_trust_update(&update_event).await?;

// Commit all changes
tx.commit().await?;

// Or rollback on error
// tx.rollback().await?;
```

**Transaction guarantees:**
- Atomicity: All operations succeed or fail together
- Consistency: Data integrity constraints maintained
- Isolation: Concurrent transactions don't interfere
- Durability: Committed changes persist across restarts

## Schema Migrations

The SQLite implementation supports versioned schema migrations:

```rust
// Check current version
let current_version = store.get_schema_version().await?;

// Migrate to latest version
store.migrate_schema(LATEST_VERSION).await?;
```

**Migration versions:**
- v1: Initial schema with trust scores and updates
- v2: Added performance indexes
- v3: Added agent metadata table

**Migration safety:**
- Migrations run in transactions
- Automatic rollback on failure
- Version tracking prevents re-application
- Backwards compatibility maintained

## Backup and Restore

Comprehensive backup system for data protection:

### Automatic Backups
```rust
// Periodic backups (every 24 hours by default)
let store = SqliteTrustStore::new(db_path, backup_dir).await?;
store.initialize().await?; // Starts backup task
```

### Manual Backups
```rust
// Create backup manually
let backup_path = Path::new("trust_backup_20241224.db");
store.create_backup(&backup_path).await?;

// Restore from backup
store.restore_backup(&backup_path).await?;
```

### Trust Manager Integration
```rust
let backup_config = BackupConfig {
    enabled: true,
    interval_secs: 3600, // 1 hour
    on_significant_change: true,
    significant_change_threshold: 0.1,
};

let manager = TrustManager::with_storage(store)
    .with_backup_config(backup_config);
```

## Error Handling

Comprehensive error types for robust error handling:

```rust
use synapsed_swarm::error::SwarmError;

match store.store_trust_score(agent_id, score).await {
    Ok(()) => println!("Success"),
    Err(SwarmError::StorageError(msg)) => eprintln!("Storage error: {}", msg),
    Err(SwarmError::TransactionFailed(msg)) => eprintln!("Transaction failed: {}", msg),
    Err(SwarmError::ConcurrencyError(msg)) => eprintln!("Concurrency error: {}", msg),
    Err(e) => eprintln!("Other error: {}", e),
}
```

**Error categories:**
- `StorageError`: Storage backend issues
- `TransactionFailed`: Transaction rollback/commit failures  
- `ConcurrencyError`: Concurrent access conflicts
- `MigrationFailed`: Schema migration problems
- `BackupFailed`: Backup/restore operations

## Concurrent Access

All implementations handle concurrent access safely:

### SQLite Implementation
- WAL mode for concurrent readers/writers
- Connection pooling for scalability
- Row-level locking where applicable
- Timeout handling for lock contention

### File Implementation  
- Read/write locks using `RwLock`
- Atomic file operations (write-then-rename)
- Temporary files for safe updates
- Concurrent read access supported

### In-Memory Implementation
- `DashMap` for lock-free concurrent access
- No blocking operations
- Thread-safe by design

## Performance Considerations

### Caching Strategy
The `TrustManager` implements intelligent caching:
- In-memory cache for frequently accessed scores
- Write-through to persistent storage
- Cache invalidation on updates
- Configurable cache size limits

### Indexing
SQLite implementation includes optimized indexes:
- Primary key indexes on agent IDs
- Timestamp indexes for history queries
- Compound indexes for complex queries

### Batch Operations
Support for batch operations to reduce overhead:
```rust
// Transaction batching
let mut tx = store.begin_transaction().await?;
for (agent_id, score) in batch_updates {
    tx.store_trust_score(agent_id, score).await?;
}
tx.commit().await?;
```

## Health Monitoring

Built-in health checking capabilities:

```rust
let health = store.health_check().await?;
println!("Storage health: {}", health.is_healthy);
println!("Total agents: {}", health.total_agents);
println!("Total updates: {}", health.total_updates);
```

**Health metrics:**
- Storage accessibility
- Data integrity status
- Record counts
- Last backup timestamp
- Storage size information

## Maintenance Operations

### Data Cleanup
Remove old trust update records:
```rust
let cutoff = Utc::now() - chrono::Duration::days(90);
let removed = store.cleanup_old_data(cutoff).await?;
println!("Removed {} old records", removed);
```

### Backup Management
Automatic cleanup of old backup files:
- Configurable retention period
- Size-based cleanup policies
- Automatic rotation

## Integration with Trust Manager

The `TrustManager` provides high-level operations built on the storage layer:

```rust
use synapsed_swarm::{
    TrustManager, 
    persistence::SqliteTrustStore,
    trust::BackupConfig,
};

// Create storage backend
let store = Arc::new(SqliteTrustStore::new("trust.db", None).await?);

// Create trust manager with storage
let manager = TrustManager::with_storage(store)
    .with_backup_config(BackupConfig::default());

// Initialize and use
manager.initialize().await?;
manager.initialize_agent(agent_id, 0.7).await?;
manager.update_trust(agent_id, true, true).await?;
```

## Configuration Options

### BackupConfig
```rust
BackupConfig {
    enabled: true,                           // Enable periodic backups
    interval_secs: 86400,                   // 24 hours
    on_significant_change: true,            // Backup on major trust changes
    significant_change_threshold: 0.1,      // 10% change threshold
}
```

### TrustThresholds
```rust
TrustThresholds {
    basic_task: 0.3,        // Minimum trust for basic operations
    critical_task: 0.7,     // Minimum trust for critical operations  
    delegation: 0.5,        // Minimum trust for task delegation
    verification: 0.6,      // Minimum trust for verification roles
    consensus: 0.5,         // Minimum trust for consensus participation
}
```

## Example Usage

See `examples/trust_persistence_demo.rs` for a comprehensive example demonstrating:
- Different storage backends
- Transaction usage
- Backup and restore
- Error handling
- Performance monitoring

## Testing

The system includes comprehensive tests for:
- All storage implementations
- Transaction behavior
- Concurrent access patterns
- Migration procedures
- Error conditions
- Performance characteristics

Run tests with:
```bash
cargo test --package synapsed-swarm --tests persistence
```

## Production Deployment

### Recommended Configuration
For production deployments, use `SqliteTrustStore` with:
- Separate backup directory
- Regular automated backups
- Health monitoring integration
- Proper error logging
- Connection pooling for high load

### Scaling Considerations
- SQLite handles moderate concurrent loads well
- For high-scale deployments, consider:
  - Database connection pooling
  - Read replicas for query scaling
  - Sharding by agent ID ranges
  - External database backends (future)

### Monitoring
Monitor these metrics in production:
- Storage health status
- Backup success/failure rates
- Transaction failure rates
- Query performance metrics
- Storage size growth
- Concurrent connection counts