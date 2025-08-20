//! Common test utilities and fixtures for synapsed-storage

use synapsed_storage::{Result, Storage, StorageConfig};
use std::sync::Arc;
use tempfile::TempDir;

/// Test fixture for creating temporary storage instances
pub struct StorageTestFixture {
    _temp_dir: Option<TempDir>,
    pub storage: Arc<dyn Storage<Error = synapsed_storage::StorageError>>,
}

impl StorageTestFixture {
    /// Create a new in-memory storage for testing
    pub async fn new_memory() -> Result<Self> {
        let config = StorageConfig::Memory(synapsed_storage::config::MemoryConfig {
            initial_capacity: 1024 * 1024, // 1MB
            max_memory_bytes: 0, // unlimited
        });
        
        let storage = synapsed_storage::StorageBuilder::new(config)
            .build()
            .await?;
            
        Ok(Self {
            _temp_dir: None,
            storage,
        })
    }
    
    /// Create a new file-based storage for testing
    #[cfg(feature = "sqlite")]
    pub async fn new_sqlite() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("test.db");
        
        let config = StorageConfig::Sqlite(synapsed_storage::config::SqliteConfig {
            path,
            options: Default::default(),
        });
        
        let storage = synapsed_storage::StorageBuilder::new(config)
            .build()
            .await?;
            
        Ok(Self {
            _temp_dir: Some(temp_dir),
            storage,
        })
    }
}

/// Generate test data of specified size
pub fn generate_test_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
}

/// Generate a unique test key
pub fn generate_test_key(prefix: &str) -> String {
    format!("{}-{}", prefix, uuid::Uuid::new_v4())
}

/// Macro for testing multiple storage backends
#[macro_export]
macro_rules! test_all_backends {
    ($test_name:ident, $test_fn:expr) => {
        mod $test_name {
            use super::*;
            
            #[tokio::test]
            async fn memory() {
                let fixture = $crate::common::StorageTestFixture::new_memory()
                    .await
                    .expect("Failed to create memory storage");
                $test_fn(fixture).await;
            }
            
            #[cfg(feature = "sqlite")]
            #[tokio::test]
            async fn sqlite() {
                let fixture = $crate::common::StorageTestFixture::new_sqlite()
                    .await
                    .expect("Failed to create sqlite storage");
                $test_fn(fixture).await;
            }
            
            // Add more backends as they are implemented
        }
    };
}