//! Storage module for persisting identity data
//! 
//! Provides:
//! - Identity storage traits
//! - User storage
//! - Credential storage
//! - Session storage backends

// Re-exports from traits module

#[cfg(not(feature = "std"))]
// String, Vec, and Box are available in std prelude

pub mod memory;
pub mod traits;

pub use traits::{
    IdentityStore, UserStore, CredentialStore, SessionStore,
    User, StoredCredential, StoredSession
};

/// Storage backend for all identity-related data
pub trait IdentityStorageBackend: Send + Sync {
    /// Get user store
    fn user_store(&self) -> &dyn UserStore;
    
    /// Get credential store
    fn credential_store(&self) -> &dyn CredentialStore;
    
    /// Get session store
    fn session_store(&self) -> &dyn SessionStore;
    
    /// Get identity store
    fn identity_store(&self) -> &dyn IdentityStore;
}

/// Configuration for storage backends
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Enable encryption at rest
    pub encryption_enabled: bool,
    /// Encryption key (if enabled)
    pub encryption_key: Option<Vec<u8>>,
    /// Enable compression
    pub compression_enabled: bool,
    /// Cache size for frequently accessed data
    pub cache_size: usize,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            encryption_enabled: true,
            encryption_key: None,
            compression_enabled: true,
            cache_size: 1000,
        }
    }
}

/// Create a storage backend based on configuration
pub fn create_storage_backend(config: StorageConfig) -> Box<dyn IdentityStorageBackend> {
    // For now, only in-memory backend is implemented
    Box::new(memory::InMemoryStorageBackend::new(config))
}