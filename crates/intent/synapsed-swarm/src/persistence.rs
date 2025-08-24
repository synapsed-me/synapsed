//! Persistent storage system for trust scores
//!
//! This module provides a flexible storage layer for trust scores with multiple
//! implementations including SQLite, file-based JSON storage, and in-memory storage
//! for testing. The system supports concurrent access, atomic transactions, and
//! schema migrations for production deployment.

use crate::{
    error::{SwarmError, SwarmResult}, 
    trust::{TrustScore, TrustUpdate, TrustUpdateReason},
    types::AgentId
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{sync::RwLock, time::sleep};
use tokio_rusqlite::Connection as AsyncConnection;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Current schema version for migration support
pub const SCHEMA_VERSION: i32 = 1;

/// Backup interval in seconds (24 hours)
pub const BACKUP_INTERVAL_SECS: u64 = 86400;

/// Maximum number of backup files to keep
pub const MAX_BACKUP_FILES: usize = 30;

/// Abstract trait for trust score storage
#[async_trait]
pub trait TrustStore: Send + Sync {
    /// Initialize the storage system
    async fn initialize(&self) -> SwarmResult<()>;

    /// Store or update a trust score for an agent
    async fn store_trust_score(&self, agent_id: AgentId, score: TrustScore) -> SwarmResult<()>;

    /// Retrieve a trust score for an agent
    async fn get_trust_score(&self, agent_id: AgentId) -> SwarmResult<Option<TrustScore>>;

    /// Get all trust scores
    async fn get_all_trust_scores(&self) -> SwarmResult<HashMap<AgentId, TrustScore>>;

    /// Store a trust update event
    async fn store_trust_update(&self, update: &TrustUpdate) -> SwarmResult<()>;

    /// Get trust update history for an agent
    async fn get_trust_history(&self, agent_id: AgentId, limit: Option<usize>) -> SwarmResult<Vec<TrustUpdate>>;

    /// Get trust updates since a specific timestamp
    async fn get_trust_updates_since(&self, timestamp: DateTime<Utc>) -> SwarmResult<Vec<TrustUpdate>>;

    /// Remove all data for an agent
    async fn remove_agent(&self, agent_id: AgentId) -> SwarmResult<()>;

    /// Begin a transaction (for atomic operations)
    async fn begin_transaction(&self) -> SwarmResult<Box<dyn TrustTransaction>>;

    /// Create a backup of the storage
    async fn create_backup(&self, backup_path: &Path) -> SwarmResult<()>;

    /// Restore from a backup
    async fn restore_backup(&self, backup_path: &Path) -> SwarmResult<()>;

    /// Get current schema version
    async fn get_schema_version(&self) -> SwarmResult<i32>;

    /// Migrate to a new schema version
    async fn migrate_schema(&self, target_version: i32) -> SwarmResult<()>;

    /// Health check for the storage system
    async fn health_check(&self) -> SwarmResult<StorageHealth>;

    /// Cleanup old data (for maintenance)
    async fn cleanup_old_data(&self, older_than: DateTime<Utc>) -> SwarmResult<usize>;
}

/// Transaction trait for atomic operations
#[async_trait]
pub trait TrustTransaction: Send + Sync {
    /// Store or update a trust score within the transaction
    async fn store_trust_score(&mut self, agent_id: AgentId, score: TrustScore) -> SwarmResult<()>;

    /// Store a trust update within the transaction
    async fn store_trust_update(&mut self, update: &TrustUpdate) -> SwarmResult<()>;

    /// Commit the transaction
    async fn commit(self: Box<Self>) -> SwarmResult<()>;

    /// Rollback the transaction
    async fn rollback(self: Box<Self>) -> SwarmResult<()>;
}

/// Storage health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageHealth {
    pub is_healthy: bool,
    pub error_message: Option<String>,
    pub last_backup: Option<DateTime<Utc>>,
    pub total_agents: usize,
    pub total_updates: usize,
    pub storage_size_bytes: Option<u64>,
}

/// SQLite implementation of TrustStore
pub struct SqliteTrustStore {
    connection: Arc<AsyncConnection>,
    backup_dir: PathBuf,
    backup_enabled: bool,
}

impl SqliteTrustStore {
    /// Create a new SQLite trust store
    pub async fn new<P: AsRef<Path>>(
        db_path: P,
        backup_dir: Option<P>,
    ) -> SwarmResult<Self> {
        let db_path = db_path.as_ref();
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                SwarmError::StorageError(format!("Failed to create database directory: {}", e))
            })?;
        }

        let connection = AsyncConnection::open(db_path).await.map_err(|e| {
            SwarmError::StorageError(format!("Failed to open SQLite database: {}", e))
        })?;

        let backup_dir = backup_dir.map(|p| p.as_ref().to_path_buf()).unwrap_or_else(|| {
            db_path.parent().unwrap_or(Path::new(".")).join("backups")
        });
        
        let backup_enabled = true;
        if backup_enabled {
            fs::create_dir_all(&backup_dir).map_err(|e| {
                SwarmError::StorageError(format!("Failed to create backup directory: {}", e))
            })?;
        }

        Ok(Self {
            connection: Arc::new(connection),
            backup_dir,
            backup_enabled,
        })
    }

    /// Initialize database schema
    async fn init_schema(&self) -> SwarmResult<()> {
        self.connection.call(move |conn| {
            // Create schema_info table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS schema_info (
                    version INTEGER PRIMARY KEY,
                    created_at TEXT NOT NULL,
                    description TEXT
                )",
                [],
            )?;

            // Create trust_scores table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS trust_scores (
                    agent_id TEXT PRIMARY KEY,
                    value REAL NOT NULL,
                    confidence REAL NOT NULL,
                    interactions INTEGER NOT NULL,
                    last_updated TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                )",
                [],
            )?;

            // Create trust_updates table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS trust_updates (
                    id TEXT PRIMARY KEY,
                    agent_id TEXT NOT NULL,
                    previous_value REAL NOT NULL,
                    previous_confidence REAL NOT NULL,
                    previous_interactions INTEGER NOT NULL,
                    previous_last_updated TEXT NOT NULL,
                    current_value REAL NOT NULL,
                    current_confidence REAL NOT NULL,
                    current_interactions INTEGER NOT NULL,
                    current_last_updated TEXT NOT NULL,
                    reason_type TEXT NOT NULL,
                    reason_data TEXT,
                    timestamp TEXT NOT NULL,
                    FOREIGN KEY(agent_id) REFERENCES trust_scores(agent_id) ON DELETE CASCADE
                )",
                [],
            )?;

            // Create indexes for better performance
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_trust_updates_agent_id ON trust_updates(agent_id)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_trust_updates_timestamp ON trust_updates(timestamp)",
                [],
            )?;

            // Insert current schema version if not exists
            conn.execute(
                "INSERT OR IGNORE INTO schema_info (version, created_at, description) 
                 VALUES (?1, ?2, ?3)",
                params![
                    SCHEMA_VERSION,
                    Utc::now().to_rfc3339(),
                    "Initial schema with trust scores and updates"
                ],
            )?;

            Ok::<(), rusqlite::Error>(())
        }).await.map_err(|e| {
            SwarmError::StorageError(format!("Failed to initialize schema: {}", e))
        })
    }

    /// Serialize reason to JSON
    fn serialize_reason(reason: &TrustUpdateReason) -> SwarmResult<String> {
        serde_json::to_string(reason).map_err(|e| {
            SwarmError::StorageError(format!("Failed to serialize reason: {}", e))
        })
    }

    /// Deserialize reason from JSON
    fn deserialize_reason(reason_json: &str) -> SwarmResult<TrustUpdateReason> {
        serde_json::from_str(reason_json).map_err(|e| {
            SwarmError::StorageError(format!("Failed to deserialize reason: {}", e))
        })
    }

    /// Start periodic backup task
    pub async fn start_periodic_backup(&self) -> SwarmResult<()> {
        if !self.backup_enabled {
            return Ok(());
        }

        let backup_dir = self.backup_dir.clone();
        let connection = Arc::clone(&self.connection);

        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(BACKUP_INTERVAL_SECS)).await;
                
                let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
                let backup_path = backup_dir.join(format!("trust_backup_{}.db", timestamp));
                
                if let Err(e) = Self::create_backup_internal(&connection, &backup_path).await {
                    error!("Failed to create periodic backup: {}", e);
                } else {
                    info!("Created periodic backup: {:?}", backup_path);
                    
                    // Cleanup old backups
                    if let Err(e) = Self::cleanup_old_backups(&backup_dir).await {
                        warn!("Failed to cleanup old backups: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Create a backup (internal implementation)
    async fn create_backup_internal(
        connection: &AsyncConnection,
        backup_path: &Path,
    ) -> SwarmResult<()> {
        let backup_path_str = backup_path.to_string_lossy().to_string();
        
        connection.call(move |conn| {
            let backup = rusqlite::backup::Backup::new(conn, &backup_path_str)?;
            backup.run_to_completion(5, Duration::from_millis(250), None)?;
            Ok::<(), rusqlite::Error>(())
        }).await.map_err(|e| {
            SwarmError::StorageError(format!("Failed to create backup: {}", e))
        })
    }

    /// Cleanup old backup files
    async fn cleanup_old_backups(backup_dir: &Path) -> SwarmResult<()> {
        let mut entries: Vec<_> = fs::read_dir(backup_dir)
            .map_err(|e| SwarmError::StorageError(format!("Failed to read backup directory: {}", e)))?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension()? == "db" && 
                   path.file_stem()?.to_str()?.starts_with("trust_backup_") {
                    let metadata = entry.metadata().ok()?;
                    Some((path, metadata.modified().ok()?))
                } else {
                    None
                }
            })
            .collect();

        if entries.len() <= MAX_BACKUP_FILES {
            return Ok(());
        }

        // Sort by modification time (oldest first)
        entries.sort_by_key(|&(_, modified)| modified);

        // Remove oldest files
        let to_remove = entries.len() - MAX_BACKUP_FILES;
        for (path, _) in entries.into_iter().take(to_remove) {
            if let Err(e) = fs::remove_file(&path) {
                warn!("Failed to remove old backup file {:?}: {}", path, e);
            } else {
                debug!("Removed old backup file: {:?}", path);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl TrustStore for SqliteTrustStore {
    async fn initialize(&self) -> SwarmResult<()> {
        self.init_schema().await?;
        self.start_periodic_backup().await?;
        info!("Initialized SQLite trust store");
        Ok(())
    }

    async fn store_trust_score(&self, agent_id: AgentId, score: TrustScore) -> SwarmResult<()> {
        let agent_id_str = agent_id.to_string();
        let now = Utc::now().to_rfc3339();
        
        self.connection.call(move |conn| {
            conn.execute(
                "INSERT OR REPLACE INTO trust_scores 
                 (agent_id, value, confidence, interactions, last_updated, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 
                         COALESCE((SELECT created_at FROM trust_scores WHERE agent_id = ?1), ?6),
                         ?7)",
                params![
                    agent_id_str,
                    score.value,
                    score.confidence,
                    score.interactions,
                    score.last_updated.to_rfc3339(),
                    now,
                    now
                ],
            )?;
            Ok::<(), rusqlite::Error>(())
        }).await.map_err(|e| {
            SwarmError::StorageError(format!("Failed to store trust score: {}", e))
        })
    }

    async fn get_trust_score(&self, agent_id: AgentId) -> SwarmResult<Option<TrustScore>> {
        let agent_id_str = agent_id.to_string();
        
        self.connection.call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT value, confidence, interactions, last_updated 
                 FROM trust_scores WHERE agent_id = ?1"
            )?;
            
            let score: Option<TrustScore> = stmt.query_row(params![agent_id_str], |row| {
                let last_updated_str: String = row.get(3)?;
                let last_updated = DateTime::parse_from_rfc3339(&last_updated_str)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(
                        3, "last_updated".to_string(), rusqlite::types::Type::Text
                    ))?
                    .with_timezone(&Utc);
                
                Ok(TrustScore {
                    value: row.get(0)?,
                    confidence: row.get(1)?,
                    interactions: row.get(2)?,
                    last_updated,
                })
            }).optional()?;
            
            Ok(score)
        }).await.map_err(|e| {
            SwarmError::StorageError(format!("Failed to get trust score: {}", e))
        })
    }

    async fn get_all_trust_scores(&self) -> SwarmResult<HashMap<AgentId, TrustScore>> {
        self.connection.call(|conn| {
            let mut stmt = conn.prepare(
                "SELECT agent_id, value, confidence, interactions, last_updated 
                 FROM trust_scores"
            )?;
            
            let rows = stmt.query_map([], |row| {
                let agent_id_str: String = row.get(0)?;
                let agent_id = AgentId::parse_str(&agent_id_str)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(
                        0, "agent_id".to_string(), rusqlite::types::Type::Text
                    ))?;
                
                let last_updated_str: String = row.get(4)?;
                let last_updated = DateTime::parse_from_rfc3339(&last_updated_str)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(
                        4, "last_updated".to_string(), rusqlite::types::Type::Text
                    ))?
                    .with_timezone(&Utc);
                
                let score = TrustScore {
                    value: row.get(1)?,
                    confidence: row.get(2)?,
                    interactions: row.get(3)?,
                    last_updated,
                };
                
                Ok((agent_id, score))
            })?;
            
            let mut scores = HashMap::new();
            for row in rows {
                let (agent_id, score) = row?;
                scores.insert(agent_id, score);
            }
            
            Ok(scores)
        }).await.map_err(|e| {
            SwarmError::StorageError(format!("Failed to get all trust scores: {}", e))
        })
    }

    async fn store_trust_update(&self, update: &TrustUpdate) -> SwarmResult<()> {
        let update_id = Uuid::new_v4().to_string();
        let agent_id_str = update.agent_id.to_string();
        let reason_json = Self::serialize_reason(&update.reason)?;
        
        self.connection.call(move |conn| {
            conn.execute(
                "INSERT INTO trust_updates 
                 (id, agent_id, previous_value, previous_confidence, previous_interactions, 
                  previous_last_updated, current_value, current_confidence, current_interactions,
                  current_last_updated, reason_type, reason_data, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    update_id,
                    agent_id_str,
                    update.previous.value,
                    update.previous.confidence,
                    update.previous.interactions,
                    update.previous.last_updated.to_rfc3339(),
                    update.current.value,
                    update.current.confidence,
                    update.current.interactions,
                    update.current.last_updated.to_rfc3339(),
                    serde_json::to_string(&update.reason).unwrap_or_default(),
                    reason_json,
                    update.timestamp.to_rfc3339()
                ],
            )?;
            Ok::<(), rusqlite::Error>(())
        }).await.map_err(|e| {
            SwarmError::StorageError(format!("Failed to store trust update: {}", e))
        })
    }

    async fn get_trust_history(&self, agent_id: AgentId, limit: Option<usize>) -> SwarmResult<Vec<TrustUpdate>> {
        let agent_id_str = agent_id.to_string();
        let limit = limit.unwrap_or(100);
        
        self.connection.call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT agent_id, previous_value, previous_confidence, previous_interactions,
                        previous_last_updated, current_value, current_confidence, 
                        current_interactions, current_last_updated, reason_data, timestamp
                 FROM trust_updates WHERE agent_id = ?1 
                 ORDER BY timestamp DESC LIMIT ?2"
            )?;
            
            let rows = stmt.query_map(params![agent_id_str, limit], |row| {
                let agent_id_str: String = row.get(0)?;
                let agent_id = AgentId::parse_str(&agent_id_str)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(
                        0, "agent_id".to_string(), rusqlite::types::Type::Text
                    ))?;
                
                let prev_last_updated = DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(
                        4, "previous_last_updated".to_string(), rusqlite::types::Type::Text
                    ))?
                    .with_timezone(&Utc);
                
                let curr_last_updated = DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(
                        8, "current_last_updated".to_string(), rusqlite::types::Type::Text
                    ))?
                    .with_timezone(&Utc);
                
                let timestamp = DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(
                        10, "timestamp".to_string(), rusqlite::types::Type::Text
                    ))?
                    .with_timezone(&Utc);
                
                let reason_json: String = row.get(9)?;
                let reason = serde_json::from_str(&reason_json)
                    .unwrap_or(TrustUpdateReason::ManualAdjustment("Unknown".to_string()));
                
                Ok(TrustUpdate {
                    agent_id,
                    previous: TrustScore {
                        value: row.get(1)?,
                        confidence: row.get(2)?,
                        interactions: row.get(3)?,
                        last_updated: prev_last_updated,
                    },
                    current: TrustScore {
                        value: row.get(5)?,
                        confidence: row.get(6)?,
                        interactions: row.get(7)?,
                        last_updated: curr_last_updated,
                    },
                    reason,
                    timestamp,
                })
            })?;
            
            let mut updates = Vec::new();
            for row in rows {
                updates.push(row?);
            }
            
            Ok(updates)
        }).await.map_err(|e| {
            SwarmError::StorageError(format!("Failed to get trust history: {}", e))
        })
    }

    async fn get_trust_updates_since(&self, timestamp: DateTime<Utc>) -> SwarmResult<Vec<TrustUpdate>> {
        let timestamp_str = timestamp.to_rfc3339();
        
        self.connection.call(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT agent_id, previous_value, previous_confidence, previous_interactions,
                        previous_last_updated, current_value, current_confidence, 
                        current_interactions, current_last_updated, reason_data, timestamp
                 FROM trust_updates WHERE timestamp >= ?1 
                 ORDER BY timestamp ASC"
            )?;
            
            let rows = stmt.query_map(params![timestamp_str], |row| {
                let agent_id_str: String = row.get(0)?;
                let agent_id = AgentId::parse_str(&agent_id_str)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(
                        0, "agent_id".to_string(), rusqlite::types::Type::Text
                    ))?;
                
                let prev_last_updated = DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(
                        4, "previous_last_updated".to_string(), rusqlite::types::Type::Text
                    ))?
                    .with_timezone(&Utc);
                
                let curr_last_updated = DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(
                        8, "current_last_updated".to_string(), rusqlite::types::Type::Text
                    ))?
                    .with_timezone(&Utc);
                
                let update_timestamp = DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(
                        10, "timestamp".to_string(), rusqlite::types::Type::Text
                    ))?
                    .with_timezone(&Utc);
                
                let reason_json: String = row.get(9)?;
                let reason = serde_json::from_str(&reason_json)
                    .unwrap_or(TrustUpdateReason::ManualAdjustment("Unknown".to_string()));
                
                Ok(TrustUpdate {
                    agent_id,
                    previous: TrustScore {
                        value: row.get(1)?,
                        confidence: row.get(2)?,
                        interactions: row.get(3)?,
                        last_updated: prev_last_updated,
                    },
                    current: TrustScore {
                        value: row.get(5)?,
                        confidence: row.get(6)?,
                        interactions: row.get(7)?,
                        last_updated: curr_last_updated,
                    },
                    reason,
                    timestamp: update_timestamp,
                })
            })?;
            
            let mut updates = Vec::new();
            for row in rows {
                updates.push(row?);
            }
            
            Ok(updates)
        }).await.map_err(|e| {
            SwarmError::StorageError(format!("Failed to get trust updates since timestamp: {}", e))
        })
    }

    async fn remove_agent(&self, agent_id: AgentId) -> SwarmResult<()> {
        let agent_id_str = agent_id.to_string();
        
        self.connection.call(move |conn| {
            // Delete trust updates (cascade will handle this, but be explicit)
            conn.execute("DELETE FROM trust_updates WHERE agent_id = ?1", params![agent_id_str])?;
            // Delete trust scores
            conn.execute("DELETE FROM trust_scores WHERE agent_id = ?1", params![agent_id_str])?;
            Ok::<(), rusqlite::Error>(())
        }).await.map_err(|e| {
            SwarmError::StorageError(format!("Failed to remove agent: {}", e))
        })
    }

    async fn begin_transaction(&self) -> SwarmResult<Box<dyn TrustTransaction>> {
        // For SQLite, we'll use a simpler approach with a single connection
        // In a more sophisticated implementation, you might want to use a connection pool
        let connection = Arc::clone(&self.connection);
        
        Ok(Box::new(SqliteTransaction {
            connection,
            operations: Vec::new(),
        }))
    }

    async fn create_backup(&self, backup_path: &Path) -> SwarmResult<()> {
        Self::create_backup_internal(&self.connection, backup_path).await
    }

    async fn restore_backup(&self, backup_path: &Path) -> SwarmResult<()> {
        let backup_path_str = backup_path.to_string_lossy().to_string();
        
        self.connection.call(move |conn| {
            // First, clear existing data
            conn.execute("DELETE FROM trust_updates", [])?;
            conn.execute("DELETE FROM trust_scores", [])?;
            conn.execute("DELETE FROM schema_info", [])?;
            
            // Attach backup database and copy data
            conn.execute(&format!("ATTACH DATABASE '{}' AS backup", backup_path_str), [])?;
            
            // Copy schema_info
            conn.execute(
                "INSERT INTO schema_info SELECT * FROM backup.schema_info", 
                []
            )?;
            
            // Copy trust_scores
            conn.execute(
                "INSERT INTO trust_scores SELECT * FROM backup.trust_scores", 
                []
            )?;
            
            // Copy trust_updates
            conn.execute(
                "INSERT INTO trust_updates SELECT * FROM backup.trust_updates", 
                []
            )?;
            
            conn.execute("DETACH DATABASE backup", [])?;
            
            Ok::<(), rusqlite::Error>(())
        }).await.map_err(|e| {
            SwarmError::StorageError(format!("Failed to restore backup: {}", e))
        })
    }

    async fn get_schema_version(&self) -> SwarmResult<i32> {
        self.connection.call(|conn| {
            let version: i32 = conn.query_row(
                "SELECT MAX(version) FROM schema_info",
                [],
                |row| row.get(0)
            ).unwrap_or(0);
            Ok(version)
        }).await.map_err(|e| {
            SwarmError::StorageError(format!("Failed to get schema version: {}", e))
        })
    }

    async fn migrate_schema(&self, target_version: i32) -> SwarmResult<()> {
        let current_version = self.get_schema_version().await?;
        
        if current_version >= target_version {
            return Ok(());
        }
        
        info!("Migrating schema from version {} to {}", current_version, target_version);
        
        // Apply migrations sequentially
        for version in (current_version + 1)..=target_version {
            self.apply_migration(version).await?;
        }
        
        info!("Schema migration completed to version {}", target_version);
        Ok(())
    }

    async fn health_check(&self) -> SwarmResult<StorageHealth> {
        let (total_agents, total_updates, last_backup) = self.connection.call(|conn| {
            let total_agents: usize = conn.query_row(
                "SELECT COUNT(*) FROM trust_scores",
                [],
                |row| row.get::<_, i64>(0).map(|v| v as usize)
            ).unwrap_or(0);
            
            let total_updates: usize = conn.query_row(
                "SELECT COUNT(*) FROM trust_updates",
                [],
                |row| row.get::<_, i64>(0).map(|v| v as usize)
            ).unwrap_or(0);
            
            Ok::<(usize, usize, Option<DateTime<Utc>>), rusqlite::Error>((
                total_agents, 
                total_updates, 
                None // We'd need to track this separately
            ))
        }).await.map_err(|e| {
            SwarmError::StorageError(format!("Health check failed: {}", e))
        })?;
        
        Ok(StorageHealth {
            is_healthy: true,
            error_message: None,
            last_backup,
            total_agents,
            total_updates,
            storage_size_bytes: None, // Could calculate from file size
        })
    }

    async fn cleanup_old_data(&self, older_than: DateTime<Utc>) -> SwarmResult<usize> {
        let cutoff_time = older_than.to_rfc3339();
        
        self.connection.call(move |conn| {
            let deleted = conn.execute(
                "DELETE FROM trust_updates WHERE timestamp < ?1",
                params![cutoff_time]
            )?;
            Ok(deleted)
        }).await.map_err(|e| {
            SwarmError::StorageError(format!("Failed to cleanup old data: {}", e))
        })
    }
}

impl SqliteTrustStore {
    /// Apply a specific migration
    async fn apply_migration(&self, version: i32) -> SwarmResult<()> {
        self.connection.call(move |conn| {
            let tx = conn.transaction()?;
            
            match version {
                1 => {
                    // Version 1 is the initial schema - already applied
                    Ok(())
                }
                2 => {
                    // Example migration for version 2: Add index for better performance
                    tx.execute(
                        "CREATE INDEX IF NOT EXISTS idx_trust_scores_last_updated 
                         ON trust_scores(last_updated)",
                        [],
                    )?;
                    
                    // Update schema version
                    tx.execute(
                        "INSERT INTO schema_info (version, created_at, description) 
                         VALUES (?1, ?2, ?3)",
                        params![
                            2,
                            Utc::now().to_rfc3339(),
                            "Added performance indexes"
                        ],
                    )?;
                    
                    info!("Applied migration version 2: Added performance indexes");
                    Ok(())
                }
                3 => {
                    // Example migration for version 3: Add agent metadata table
                    tx.execute(
                        "CREATE TABLE IF NOT EXISTS agent_metadata (
                            agent_id TEXT PRIMARY KEY,
                            name TEXT,
                            description TEXT,
                            created_at TEXT NOT NULL,
                            last_seen TEXT NOT NULL,
                            FOREIGN KEY(agent_id) REFERENCES trust_scores(agent_id) ON DELETE CASCADE
                        )",
                        [],
                    )?;
                    
                    tx.execute(
                        "CREATE INDEX IF NOT EXISTS idx_agent_metadata_last_seen 
                         ON agent_metadata(last_seen)",
                        [],
                    )?;
                    
                    // Update schema version
                    tx.execute(
                        "INSERT INTO schema_info (version, created_at, description) 
                         VALUES (?1, ?2, ?3)",
                        params![
                            3,
                            Utc::now().to_rfc3339(),
                            "Added agent metadata table"
                        ],
                    )?;
                    
                    info!("Applied migration version 3: Added agent metadata table");
                    Ok(())
                }
                _ => {
                    return Err(rusqlite::Error::InvalidPath(
                        format!("Unknown migration version: {}", version).into()
                    ));
                }
            }?;
            
            tx.commit()?;
            Ok::<(), rusqlite::Error>(())
        }).await.map_err(|e| {
            SwarmError::StorageError(format!("Failed to apply migration version {}: {}", version, e))
        })
    }
}

/// Transaction implementation for SQLite
struct SqliteTransaction {
    connection: Arc<AsyncConnection>,
    operations: Vec<TransactionOperation>,
}

#[derive(Debug, Clone)]
enum TransactionOperation {
    StoreTrustScore { agent_id: AgentId, score: TrustScore },
    StoreTrustUpdate { update: TrustUpdate },
}

#[async_trait]
impl TrustTransaction for SqliteTransaction {
    async fn store_trust_score(&mut self, agent_id: AgentId, score: TrustScore) -> SwarmResult<()> {
        self.operations.push(TransactionOperation::StoreTrustScore { agent_id, score });
        Ok(())
    }

    async fn store_trust_update(&mut self, update: &TrustUpdate) -> SwarmResult<()> {
        self.operations.push(TransactionOperation::StoreTrustUpdate { update: update.clone() });
        Ok(())
    }

    async fn commit(self: Box<Self>) -> SwarmResult<()> {
        if self.operations.is_empty() {
            return Ok(());
        }
        
        let operations = self.operations;
        self.connection.call(move |conn| {
            let tx = conn.transaction()?;
            
            for op in operations {
                match op {
                    TransactionOperation::StoreTrustScore { agent_id, score } => {
                        let agent_id_str = agent_id.to_string();
                        let now = Utc::now().to_rfc3339();
                        
                        tx.execute(
                            "INSERT OR REPLACE INTO trust_scores 
                             (agent_id, value, confidence, interactions, last_updated, created_at, updated_at)
                             VALUES (?1, ?2, ?3, ?4, ?5, 
                                     COALESCE((SELECT created_at FROM trust_scores WHERE agent_id = ?1), ?6),
                                     ?7)",
                            params![
                                agent_id_str,
                                score.value,
                                score.confidence,
                                score.interactions,
                                score.last_updated.to_rfc3339(),
                                now,
                                now
                            ],
                        )?;
                    }
                    TransactionOperation::StoreTrustUpdate { update } => {
                        let update_id = Uuid::new_v4().to_string();
                        let agent_id_str = update.agent_id.to_string();
                        let reason_json = serde_json::to_string(&update.reason)
                            .unwrap_or_default();
                        
                        tx.execute(
                            "INSERT INTO trust_updates 
                             (id, agent_id, previous_value, previous_confidence, previous_interactions, 
                              previous_last_updated, current_value, current_confidence, current_interactions,
                              current_last_updated, reason_type, reason_data, timestamp)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                            params![
                                update_id,
                                agent_id_str,
                                update.previous.value,
                                update.previous.confidence,
                                update.previous.interactions,
                                update.previous.last_updated.to_rfc3339(),
                                update.current.value,
                                update.current.confidence,
                                update.current.interactions,
                                update.current.last_updated.to_rfc3339(),
                                reason_json.clone(),
                                reason_json,
                                update.timestamp.to_rfc3339()
                            ],
                        )?;
                    }
                }
            }
            
            tx.commit()?;
            Ok::<(), rusqlite::Error>(())
        }).await.map_err(|e| {
            SwarmError::StorageError(format!("Failed to commit transaction: {}", e))
        })
    }

    async fn rollback(self: Box<Self>) -> SwarmResult<()> {
        // For our implementation, we simply drop the operations
        debug!("Transaction rolled back with {} operations", self.operations.len());
        Ok(())
    }
}

/// File-based JSON implementation of TrustStore
pub struct FileTrustStore {
    scores_file: PathBuf,
    updates_file: PathBuf,
    backup_dir: PathBuf,
    data_lock: Arc<RwLock<()>>,
}

impl FileTrustStore {
    /// Create a new file-based trust store
    pub fn new<P: AsRef<Path>>(data_dir: P, backup_dir: Option<P>) -> SwarmResult<Self> {
        let data_dir = data_dir.as_ref();
        fs::create_dir_all(data_dir).map_err(|e| {
            SwarmError::StorageError(format!("Failed to create data directory: {}", e))
        })?;
        
        let backup_dir = backup_dir.map(|p| p.as_ref().to_path_buf()).unwrap_or_else(|| {
            data_dir.join("backups")
        });
        fs::create_dir_all(&backup_dir).map_err(|e| {
            SwarmError::StorageError(format!("Failed to create backup directory: {}", e))
        })?;
        
        Ok(Self {
            scores_file: data_dir.join("trust_scores.json"),
            updates_file: data_dir.join("trust_updates.json"),
            backup_dir,
            data_lock: Arc::new(RwLock::new(())),
        })
    }

    /// Load trust scores from file
    async fn load_scores(&self) -> SwarmResult<HashMap<AgentId, TrustScore>> {
        let _lock = self.data_lock.read().await;
        
        if !self.scores_file.exists() {
            return Ok(HashMap::new());
        }
        
        let file = File::open(&self.scores_file).map_err(|e| {
            SwarmError::StorageError(format!("Failed to open scores file: {}", e))
        })?;
        
        let reader = BufReader::new(file);
        let scores: HashMap<String, TrustScore> = serde_json::from_reader(reader).map_err(|e| {
            SwarmError::StorageError(format!("Failed to parse scores file: {}", e))
        })?;
        
        // Convert string keys to AgentId
        let mut result = HashMap::new();
        for (agent_id_str, score) in scores {
            let agent_id = AgentId::parse_str(&agent_id_str).map_err(|e| {
                SwarmError::StorageError(format!("Failed to parse agent ID: {}", e))
            })?;
            result.insert(agent_id, score);
        }
        
        Ok(result)
    }

    /// Save trust scores to file
    async fn save_scores(&self, scores: &HashMap<AgentId, TrustScore>) -> SwarmResult<()> {
        let _lock = self.data_lock.write().await;
        
        // Convert AgentId keys to strings for serialization
        let string_scores: HashMap<String, TrustScore> = scores.iter()
            .map(|(id, score)| (id.to_string(), *score))
            .collect();
        
        // Write to temporary file first, then rename for atomicity
        let temp_file = self.scores_file.with_extension("json.tmp");
        let file = File::create(&temp_file).map_err(|e| {
            SwarmError::StorageError(format!("Failed to create temp scores file: {}", e))
        })?;
        
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &string_scores).map_err(|e| {
            SwarmError::StorageError(format!("Failed to write scores file: {}", e))
        })?;
        
        fs::rename(&temp_file, &self.scores_file).map_err(|e| {
            SwarmError::StorageError(format!("Failed to rename scores file: {}", e))
        })?;
        
        Ok(())
    }

    /// Load trust updates from file
    async fn load_updates(&self) -> SwarmResult<HashMap<AgentId, Vec<TrustUpdate>>> {
        let _lock = self.data_lock.read().await;
        
        if !self.updates_file.exists() {
            return Ok(HashMap::new());
        }
        
        let file = File::open(&self.updates_file).map_err(|e| {
            SwarmError::StorageError(format!("Failed to open updates file: {}", e))
        })?;
        
        let reader = BufReader::new(file);
        let updates: HashMap<String, Vec<TrustUpdate>> = serde_json::from_reader(reader).map_err(|e| {
            SwarmError::StorageError(format!("Failed to parse updates file: {}", e))
        })?;
        
        // Convert string keys to AgentId
        let mut result = HashMap::new();
        for (agent_id_str, agent_updates) in updates {
            let agent_id = AgentId::parse_str(&agent_id_str).map_err(|e| {
                SwarmError::StorageError(format!("Failed to parse agent ID: {}", e))
            })?;
            result.insert(agent_id, agent_updates);
        }
        
        Ok(result)
    }

    /// Save trust updates to file
    async fn save_updates(&self, updates: &HashMap<AgentId, Vec<TrustUpdate>>) -> SwarmResult<()> {
        let _lock = self.data_lock.write().await;
        
        // Convert AgentId keys to strings for serialization
        let string_updates: HashMap<String, Vec<TrustUpdate>> = updates.iter()
            .map(|(id, updates)| (id.to_string(), updates.clone()))
            .collect();
        
        // Write to temporary file first, then rename for atomicity
        let temp_file = self.updates_file.with_extension("json.tmp");
        let file = File::create(&temp_file).map_err(|e| {
            SwarmError::StorageError(format!("Failed to create temp updates file: {}", e))
        })?;
        
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &string_updates).map_err(|e| {
            SwarmError::StorageError(format!("Failed to write updates file: {}", e))
        })?;
        
        fs::rename(&temp_file, &self.updates_file).map_err(|e| {
            SwarmError::StorageError(format!("Failed to rename updates file: {}", e))
        })?;
        
        Ok(())
    }
}

#[async_trait]
impl TrustStore for FileTrustStore {
    async fn initialize(&self) -> SwarmResult<()> {
        // Create empty files if they don't exist
        if !self.scores_file.exists() {
            self.save_scores(&HashMap::new()).await?;
        }
        if !self.updates_file.exists() {
            self.save_updates(&HashMap::new()).await?;
        }
        
        info!("Initialized file-based trust store");
        Ok(())
    }

    async fn store_trust_score(&self, agent_id: AgentId, score: TrustScore) -> SwarmResult<()> {
        let mut scores = self.load_scores().await?;
        scores.insert(agent_id, score);
        self.save_scores(&scores).await
    }

    async fn get_trust_score(&self, agent_id: AgentId) -> SwarmResult<Option<TrustScore>> {
        let scores = self.load_scores().await?;
        Ok(scores.get(&agent_id).copied())
    }

    async fn get_all_trust_scores(&self) -> SwarmResult<HashMap<AgentId, TrustScore>> {
        self.load_scores().await
    }

    async fn store_trust_update(&self, update: &TrustUpdate) -> SwarmResult<()> {
        let mut updates = self.load_updates().await?;
        let agent_updates = updates.entry(update.agent_id).or_insert_with(Vec::new);
        agent_updates.push(update.clone());
        
        // Limit history size
        if agent_updates.len() > 100 {
            agent_updates.drain(0..10);
        }
        
        self.save_updates(&updates).await
    }

    async fn get_trust_history(&self, agent_id: AgentId, limit: Option<usize>) -> SwarmResult<Vec<TrustUpdate>> {
        let updates = self.load_updates().await?;
        let agent_updates = updates.get(&agent_id).cloned().unwrap_or_default();
        
        let mut result = agent_updates;
        result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)); // Newest first
        
        if let Some(limit) = limit {
            result.truncate(limit);
        }
        
        Ok(result)
    }

    async fn get_trust_updates_since(&self, timestamp: DateTime<Utc>) -> SwarmResult<Vec<TrustUpdate>> {
        let updates = self.load_updates().await?;
        let mut result = Vec::new();
        
        for (_, agent_updates) in updates {
            for update in agent_updates {
                if update.timestamp >= timestamp {
                    result.push(update);
                }
            }
        }
        
        result.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(result)
    }

    async fn remove_agent(&self, agent_id: AgentId) -> SwarmResult<()> {
        let mut scores = self.load_scores().await?;
        let mut updates = self.load_updates().await?;
        
        scores.remove(&agent_id);
        updates.remove(&agent_id);
        
        self.save_scores(&scores).await?;
        self.save_updates(&updates).await?;
        
        Ok(())
    }

    async fn begin_transaction(&self) -> SwarmResult<Box<dyn TrustTransaction>> {
        Ok(Box::new(FileTransaction {
            store: self,
            operations: Vec::new(),
        }))
    }

    async fn create_backup(&self, backup_path: &Path) -> SwarmResult<()> {
        let _lock = self.data_lock.read().await;
        
        fs::create_dir_all(backup_path.parent().unwrap_or(Path::new("."))).map_err(|e| {
            SwarmError::StorageError(format!("Failed to create backup directory: {}", e))
        })?;
        
        let scores_backup = backup_path.with_extension("scores.json");
        let updates_backup = backup_path.with_extension("updates.json");
        
        if self.scores_file.exists() {
            fs::copy(&self.scores_file, &scores_backup).map_err(|e| {
                SwarmError::StorageError(format!("Failed to backup scores: {}", e))
            })?;
        }
        
        if self.updates_file.exists() {
            fs::copy(&self.updates_file, &updates_backup).map_err(|e| {
                SwarmError::StorageError(format!("Failed to backup updates: {}", e))
            })?;
        }
        
        Ok(())
    }

    async fn restore_backup(&self, backup_path: &Path) -> SwarmResult<()> {
        let _lock = self.data_lock.write().await;
        
        let scores_backup = backup_path.with_extension("scores.json");
        let updates_backup = backup_path.with_extension("updates.json");
        
        if scores_backup.exists() {
            fs::copy(&scores_backup, &self.scores_file).map_err(|e| {
                SwarmError::StorageError(format!("Failed to restore scores: {}", e))
            })?;
        }
        
        if updates_backup.exists() {
            fs::copy(&updates_backup, &self.updates_file).map_err(|e| {
                SwarmError::StorageError(format!("Failed to restore updates: {}", e))
            })?;
        }
        
        Ok(())
    }

    async fn get_schema_version(&self) -> SwarmResult<i32> {
        Ok(SCHEMA_VERSION) // File store doesn't need migrations currently
    }

    async fn migrate_schema(&self, _target_version: i32) -> SwarmResult<()> {
        Ok(()) // File store doesn't need migrations currently
    }

    async fn health_check(&self) -> SwarmResult<StorageHealth> {
        let scores = self.load_scores().await?;
        let updates = self.load_updates().await?;
        
        let total_updates = updates.values().map(|v| v.len()).sum();
        let storage_size = [&self.scores_file, &self.updates_file].iter()
            .filter_map(|path| fs::metadata(path).ok())
            .map(|metadata| metadata.len())
            .sum();
        
        Ok(StorageHealth {
            is_healthy: true,
            error_message: None,
            last_backup: None, // Would need to track this separately
            total_agents: scores.len(),
            total_updates,
            storage_size_bytes: Some(storage_size),
        })
    }

    async fn cleanup_old_data(&self, older_than: DateTime<Utc>) -> SwarmResult<usize> {
        let mut updates = self.load_updates().await?;
        let mut total_removed = 0;
        
        for (_, agent_updates) in updates.iter_mut() {
            let original_len = agent_updates.len();
            agent_updates.retain(|update| update.timestamp >= older_than);
            total_removed += original_len - agent_updates.len();
        }
        
        // Remove empty entries
        updates.retain(|_, agent_updates| !agent_updates.is_empty());
        
        self.save_updates(&updates).await?;
        Ok(total_removed)
    }
}

/// Transaction implementation for file store
struct FileTransaction<'a> {
    store: &'a FileTrustStore,
    operations: Vec<FileTransactionOperation>,
}

#[derive(Debug, Clone)]
enum FileTransactionOperation {
    StoreTrustScore { agent_id: AgentId, score: TrustScore },
    StoreTrustUpdate { update: TrustUpdate },
}

#[async_trait]
impl TrustTransaction for FileTransaction<'_> {
    async fn store_trust_score(&mut self, agent_id: AgentId, score: TrustScore) -> SwarmResult<()> {
        self.operations.push(FileTransactionOperation::StoreTrustScore { agent_id, score });
        Ok(())
    }

    async fn store_trust_update(&mut self, update: &TrustUpdate) -> SwarmResult<()> {
        self.operations.push(FileTransactionOperation::StoreTrustUpdate { update: update.clone() });
        Ok(())
    }

    async fn commit(self: Box<Self>) -> SwarmResult<()> {
        if self.operations.is_empty() {
            return Ok(());
        }
        
        // For file store, we'll apply all operations atomically
        let mut scores = self.store.load_scores().await?;
        let mut updates = self.store.load_updates().await?;
        
        for op in self.operations {
            match op {
                FileTransactionOperation::StoreTrustScore { agent_id, score } => {
                    scores.insert(agent_id, score);
                }
                FileTransactionOperation::StoreTrustUpdate { update } => {
                    let agent_updates = updates.entry(update.agent_id).or_insert_with(Vec::new);
                    agent_updates.push(update);
                    
                    // Limit history size
                    if agent_updates.len() > 100 {
                        agent_updates.drain(0..10);
                    }
                }
            }
        }
        
        self.store.save_scores(&scores).await?;
        self.store.save_updates(&updates).await?;
        
        Ok(())
    }

    async fn rollback(self: Box<Self>) -> SwarmResult<()> {
        debug!("File transaction rolled back with {} operations", self.operations.len());
        Ok(())
    }
}

/// In-memory implementation for testing
pub struct InMemoryTrustStore {
    scores: Arc<DashMap<AgentId, TrustScore>>,
    updates: Arc<DashMap<AgentId, Vec<TrustUpdate>>>,
    schema_version: Arc<Mutex<i32>>,
}

impl InMemoryTrustStore {
    /// Create a new in-memory trust store
    pub fn new() -> Self {
        Self {
            scores: Arc::new(DashMap::new()),
            updates: Arc::new(DashMap::new()),
            schema_version: Arc::new(Mutex::new(SCHEMA_VERSION)),
        }
    }
}

impl Default for InMemoryTrustStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TrustStore for InMemoryTrustStore {
    async fn initialize(&self) -> SwarmResult<()> {
        debug!("Initialized in-memory trust store");
        Ok(())
    }

    async fn store_trust_score(&self, agent_id: AgentId, score: TrustScore) -> SwarmResult<()> {
        self.scores.insert(agent_id, score);
        Ok(())
    }

    async fn get_trust_score(&self, agent_id: AgentId) -> SwarmResult<Option<TrustScore>> {
        Ok(self.scores.get(&agent_id).map(|entry| *entry.value()))
    }

    async fn get_all_trust_scores(&self) -> SwarmResult<HashMap<AgentId, TrustScore>> {
        Ok(self.scores.iter().map(|entry| (*entry.key(), *entry.value())).collect())
    }

    async fn store_trust_update(&self, update: &TrustUpdate) -> SwarmResult<()> {
        let mut agent_updates = self.updates.entry(update.agent_id).or_insert_with(Vec::new);
        agent_updates.push(update.clone());
        
        // Limit history size
        if agent_updates.len() > 100 {
            agent_updates.drain(0..10);
        }
        
        Ok(())
    }

    async fn get_trust_history(&self, agent_id: AgentId, limit: Option<usize>) -> SwarmResult<Vec<TrustUpdate>> {
        let agent_updates = self.updates.get(&agent_id)
            .map(|entry| entry.value().clone())
            .unwrap_or_default();
        
        let mut result = agent_updates;
        result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)); // Newest first
        
        if let Some(limit) = limit {
            result.truncate(limit);
        }
        
        Ok(result)
    }

    async fn get_trust_updates_since(&self, timestamp: DateTime<Utc>) -> SwarmResult<Vec<TrustUpdate>> {
        let mut result = Vec::new();
        
        for entry in self.updates.iter() {
            for update in entry.value() {
                if update.timestamp >= timestamp {
                    result.push(update.clone());
                }
            }
        }
        
        result.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(result)
    }

    async fn remove_agent(&self, agent_id: AgentId) -> SwarmResult<()> {
        self.scores.remove(&agent_id);
        self.updates.remove(&agent_id);
        Ok(())
    }

    async fn begin_transaction(&self) -> SwarmResult<Box<dyn TrustTransaction>> {
        Ok(Box::new(InMemoryTransaction {
            store: self,
            operations: Vec::new(),
        }))
    }

    async fn create_backup(&self, _backup_path: &Path) -> SwarmResult<()> {
        // In-memory store doesn't support persistent backups
        Ok(())
    }

    async fn restore_backup(&self, _backup_path: &Path) -> SwarmResult<()> {
        // In-memory store doesn't support persistent backups
        Ok(())
    }

    async fn get_schema_version(&self) -> SwarmResult<i32> {
        Ok(*self.schema_version.lock().unwrap())
    }

    async fn migrate_schema(&self, target_version: i32) -> SwarmResult<()> {
        *self.schema_version.lock().unwrap() = target_version;
        Ok(())
    }

    async fn health_check(&self) -> SwarmResult<StorageHealth> {
        let total_updates = self.updates.iter().map(|entry| entry.value().len()).sum();
        
        Ok(StorageHealth {
            is_healthy: true,
            error_message: None,
            last_backup: None,
            total_agents: self.scores.len(),
            total_updates,
            storage_size_bytes: None,
        })
    }

    async fn cleanup_old_data(&self, older_than: DateTime<Utc>) -> SwarmResult<usize> {
        let mut total_removed = 0;
        
        for mut entry in self.updates.iter_mut() {
            let original_len = entry.value().len();
            entry.value_mut().retain(|update| update.timestamp >= older_than);
            total_removed += original_len - entry.value().len();
        }
        
        // Remove empty entries
        self.updates.retain(|_, updates| !updates.is_empty());
        
        Ok(total_removed)
    }
}

/// Transaction implementation for in-memory store
struct InMemoryTransaction<'a> {
    store: &'a InMemoryTrustStore,
    operations: Vec<InMemoryTransactionOperation>,
}

#[derive(Debug, Clone)]
enum InMemoryTransactionOperation {
    StoreTrustScore { agent_id: AgentId, score: TrustScore },
    StoreTrustUpdate { update: TrustUpdate },
}

#[async_trait]
impl TrustTransaction for InMemoryTransaction<'_> {
    async fn store_trust_score(&mut self, agent_id: AgentId, score: TrustScore) -> SwarmResult<()> {
        self.operations.push(InMemoryTransactionOperation::StoreTrustScore { agent_id, score });
        Ok(())
    }

    async fn store_trust_update(&mut self, update: &TrustUpdate) -> SwarmResult<()> {
        self.operations.push(InMemoryTransactionOperation::StoreTrustUpdate { update: update.clone() });
        Ok(())
    }

    async fn commit(self: Box<Self>) -> SwarmResult<()> {
        for op in self.operations {
            match op {
                InMemoryTransactionOperation::StoreTrustScore { agent_id, score } => {
                    self.store.scores.insert(agent_id, score);
                }
                InMemoryTransactionOperation::StoreTrustUpdate { update } => {
                    let mut agent_updates = self.store.updates.entry(update.agent_id).or_insert_with(Vec::new);
                    agent_updates.push(update);
                    
                    // Limit history size
                    if agent_updates.len() > 100 {
                        agent_updates.drain(0..10);
                    }
                }
            }
        }
        
        Ok(())
    }

    async fn rollback(self: Box<Self>) -> SwarmResult<()> {
        debug!("In-memory transaction rolled back with {} operations", self.operations.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use uuid::Uuid;

    async fn create_test_agent_and_score() -> (AgentId, TrustScore) {
        let agent_id = AgentId::new_v4();
        let score = TrustScore::new(0.7);
        (agent_id, score)
    }

    #[tokio::test]
    async fn test_sqlite_store_basic_operations() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let store = SqliteTrustStore::new(&db_path, Some(temp_dir.path().join("backups"))).await.unwrap();
        store.initialize().await.unwrap();
        
        let (agent_id, score) = create_test_agent_and_score().await;
        
        // Store and retrieve
        store.store_trust_score(agent_id, score).await.unwrap();
        let retrieved = store.get_trust_score(agent_id).await.unwrap();
        assert_eq!(retrieved, Some(score));
        
        // Update history
        let update = TrustUpdate {
            agent_id,
            previous: score,
            current: score,
            reason: TrustUpdateReason::TaskSuccess,
            timestamp: Utc::now(),
        };
        store.store_trust_update(&update).await.unwrap();
        
        let history = store.get_trust_history(agent_id, Some(10)).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].agent_id, agent_id);
    }

    #[tokio::test]
    async fn test_file_store_basic_operations() {
        let temp_dir = TempDir::new().unwrap();
        
        let store = FileTrustStore::new(temp_dir.path(), None).unwrap();
        store.initialize().await.unwrap();
        
        let (agent_id, score) = create_test_agent_and_score().await;
        
        // Store and retrieve
        store.store_trust_score(agent_id, score).await.unwrap();
        let retrieved = store.get_trust_score(agent_id).await.unwrap();
        assert_eq!(retrieved, Some(score));
        
        // Update history
        let update = TrustUpdate {
            agent_id,
            previous: score,
            current: score,
            reason: TrustUpdateReason::TaskSuccess,
            timestamp: Utc::now(),
        };
        store.store_trust_update(&update).await.unwrap();
        
        let history = store.get_trust_history(agent_id, Some(10)).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].agent_id, agent_id);
    }

    #[tokio::test]
    async fn test_in_memory_store_basic_operations() {
        let store = InMemoryTrustStore::new();
        store.initialize().await.unwrap();
        
        let (agent_id, score) = create_test_agent_and_score().await;
        
        // Store and retrieve
        store.store_trust_score(agent_id, score).await.unwrap();
        let retrieved = store.get_trust_score(agent_id).await.unwrap();
        assert_eq!(retrieved, Some(score));
        
        // Update history
        let update = TrustUpdate {
            agent_id,
            previous: score,
            current: score,
            reason: TrustUpdateReason::TaskSuccess,
            timestamp: Utc::now(),
        };
        store.store_trust_update(&update).await.unwrap();
        
        let history = store.get_trust_history(agent_id, Some(10)).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].agent_id, agent_id);
    }

    #[tokio::test]
    async fn test_transactions() {
        let store = InMemoryTrustStore::new();
        store.initialize().await.unwrap();
        
        let (agent_id, score) = create_test_agent_and_score().await;
        
        // Test commit
        {
            let mut tx = store.begin_transaction().await.unwrap();
            tx.store_trust_score(agent_id, score).await.unwrap();
            tx.commit().await.unwrap();
        }
        
        let retrieved = store.get_trust_score(agent_id).await.unwrap();
        assert_eq!(retrieved, Some(score));
        
        // Test rollback
        let new_score = TrustScore::new(0.9);
        {
            let mut tx = store.begin_transaction().await.unwrap();
            tx.store_trust_score(agent_id, new_score).await.unwrap();
            tx.rollback().await.unwrap();
        }
        
        let still_old = store.get_trust_score(agent_id).await.unwrap();
        assert_eq!(still_old, Some(score)); // Should still be the old score
    }

    #[tokio::test]
    async fn test_health_check() {
        let store = InMemoryTrustStore::new();
        store.initialize().await.unwrap();
        
        let (agent_id, score) = create_test_agent_and_score().await;
        store.store_trust_score(agent_id, score).await.unwrap();
        
        let health = store.health_check().await.unwrap();
        assert!(health.is_healthy);
        assert_eq!(health.total_agents, 1);
        assert_eq!(health.total_updates, 0);
    }

    #[tokio::test]
    async fn test_cleanup_old_data() {
        let store = InMemoryTrustStore::new();
        store.initialize().await.unwrap();
        
        let (agent_id, score) = create_test_agent_and_score().await;
        
        // Create old update
        let old_update = TrustUpdate {
            agent_id,
            previous: score,
            current: score,
            reason: TrustUpdateReason::TaskSuccess,
            timestamp: Utc::now() - chrono::Duration::days(2),
        };
        store.store_trust_update(&old_update).await.unwrap();
        
        // Create new update
        let new_update = TrustUpdate {
            agent_id,
            previous: score,
            current: score,
            reason: TrustUpdateReason::TaskSuccess,
            timestamp: Utc::now(),
        };
        store.store_trust_update(&new_update).await.unwrap();
        
        // Cleanup old data
        let cutoff = Utc::now() - chrono::Duration::days(1);
        let removed = store.cleanup_old_data(cutoff).await.unwrap();
        assert_eq!(removed, 1);
        
        // Verify only new update remains
        let history = store.get_trust_history(agent_id, None).await.unwrap();
        assert_eq!(history.len(), 1);
        assert!(history[0].timestamp >= cutoff);
    }
}