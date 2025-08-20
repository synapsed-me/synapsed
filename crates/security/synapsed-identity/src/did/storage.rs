//! Local-first encrypted storage for DID data
//! 
//! This module implements:
//! - Encrypted local storage for identity data
//! - Multi-device synchronization
//! - Portable contact vault
//! - Offline-first operation

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use zeroize::{Zeroize, ZeroizeOnDrop};
use crate::{Result, Error};
use super::{Did, DidDocument, KeyHierarchy, EncryptedKeyMaterial};

/// Local-first storage manager for DID data
pub struct LocalFirstStorage {
    /// Storage directory
    storage_dir: PathBuf,
    /// Encryption manager
    encryption: EncryptionManager,
    /// Synchronization manager
    sync_manager: SyncManager,
    /// Contact vault
    contact_vault: ContactVault,
    /// Storage configuration
    config: StorageConfig,
}

impl LocalFirstStorage {
    /// Create a new local-first storage instance
    pub fn new<P: AsRef<Path>>(
        storage_dir: P,
        master_password: &str,
        config: StorageConfig,
    ) -> Result<Self> {
        let storage_dir = storage_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&storage_dir)
            .map_err(|e| Error::StorageError(format!("Failed to create storage directory: {}", e)))?;

        let encryption = EncryptionManager::new(master_password)?;
        let sync_manager = SyncManager::new(&storage_dir)?;
        let contact_vault = ContactVault::new(&storage_dir, &encryption)?;

        Ok(Self {
            storage_dir,
            encryption,
            sync_manager,
            contact_vault,
            config,
        })
    }

    /// Store DID document
    pub async fn store_did_document(&mut self, document: &DidDocument) -> Result<()> {
        let serialized = serde_json::to_vec(document)
            .map_err(|e| Error::StorageError(format!("Serialization failed: {}", e)))?;

        let encrypted = self.encryption.encrypt(&serialized)?;
        
        let file_path = self.storage_dir.join("documents").join(format!("{}.did", self.sanitize_did(&document.id)));
        std::fs::create_dir_all(file_path.parent().unwrap())
            .map_err(|e| Error::StorageError(format!("Failed to create directory: {}", e)))?;

        tokio::fs::write(&file_path, &encrypted).await
            .map_err(|e| Error::StorageError(format!("Failed to write file: {}", e)))?;

        // Update index
        self.update_document_index(&document.id, &file_path).await?;

        // Trigger sync if enabled
        if self.config.auto_sync {
            self.sync_manager.queue_sync(SyncItem::DidDocument(document.id.clone())).await?;
        }

        Ok(())
    }

    /// Load DID document
    pub async fn load_did_document(&self, did: &Did) -> Result<Option<DidDocument>> {
        let file_path = self.storage_dir.join("documents").join(format!("{}.did", self.sanitize_did(did)));
        
        if !file_path.exists() {
            return Ok(None);
        }

        let encrypted = tokio::fs::read(&file_path).await
            .map_err(|e| Error::StorageError(format!("Failed to read file: {}", e)))?;

        let decrypted = self.encryption.decrypt(&encrypted)?;
        
        let document: DidDocument = serde_json::from_slice(&decrypted)
            .map_err(|e| Error::StorageError(format!("Deserialization failed: {}", e)))?;

        Ok(Some(document))
    }

    /// Store key hierarchy
    pub async fn store_key_hierarchy(&mut self, did: &Did, hierarchy: &KeyHierarchy) -> Result<()> {
        // Serialize hierarchy (this would need to be implemented on KeyHierarchy)
        let serialized = self.serialize_key_hierarchy(hierarchy)?;
        let encrypted = self.encryption.encrypt(&serialized)?;
        
        let file_path = self.storage_dir.join("keys").join(format!("{}.keys", self.sanitize_did(did)));
        std::fs::create_dir_all(file_path.parent().unwrap())
            .map_err(|e| Error::StorageError(format!("Failed to create directory: {}", e)))?;

        tokio::fs::write(&file_path, &encrypted).await
            .map_err(|e| Error::StorageError(format!("Failed to write file: {}", e)))?;

        // Trigger sync
        if self.config.auto_sync {
            self.sync_manager.queue_sync(SyncItem::KeyHierarchy(did.clone())).await?;
        }

        Ok(())
    }

    /// Load key hierarchy
    pub async fn load_key_hierarchy(&self, did: &Did) -> Result<Option<KeyHierarchy>> {
        let file_path = self.storage_dir.join("keys").join(format!("{}.keys", self.sanitize_did(did)));
        
        if !file_path.exists() {
            return Ok(None);
        }

        let encrypted = tokio::fs::read(&file_path).await
            .map_err(|e| Error::StorageError(format!("Failed to read file: {}", e)))?;

        let decrypted = self.encryption.decrypt(&encrypted)?;
        let hierarchy = self.deserialize_key_hierarchy(&decrypted)?;

        Ok(Some(hierarchy))
    }

    /// Store contact in vault
    pub async fn store_contact(&mut self, contact: &Contact) -> Result<()> {
        self.contact_vault.store_contact(contact).await
    }

    /// Load contact from vault
    pub async fn load_contact(&self, did: &Did) -> Result<Option<Contact>> {
        self.contact_vault.load_contact(did).await
    }

    /// List all contacts
    pub async fn list_contacts(&self) -> Result<Vec<Contact>> {
        self.contact_vault.list_contacts().await
    }

    /// Export all data for backup/migration
    pub async fn export_all(&self, password: &str) -> Result<EncryptedBackup> {
        let mut backup_data = BackupData {
            version: "1.0".to_string(),
            created_at: Utc::now(),
            documents: HashMap::new(),
            key_hierarchies: HashMap::new(),
            contacts: self.contact_vault.export_all().await?,
            metadata: HashMap::new(),
        };

        // Export all DID documents
        let docs_dir = self.storage_dir.join("documents");
        if docs_dir.exists() {
            let mut entries = tokio::fs::read_dir(&docs_dir).await
                .map_err(|e| Error::StorageError(format!("Failed to read directory: {}", e)))?;

            while let Some(entry) = entries.next_entry().await
                .map_err(|e| Error::StorageError(format!("Failed to read entry: {}", e)))? {
                
                if let Some(ext) = entry.path().extension() {
                    if ext == "did" {
                        let encrypted = tokio::fs::read(entry.path()).await
                            .map_err(|e| Error::StorageError(format!("Failed to read file: {}", e)))?;
                        
                        let decrypted = self.encryption.decrypt(&encrypted)?;
                        let document: DidDocument = serde_json::from_slice(&decrypted)
                            .map_err(|e| Error::StorageError(format!("Deserialization failed: {}", e)))?;
                        
                        backup_data.documents.insert(document.id.to_string(), document);
                    }
                }
            }
        }

        // Export all key hierarchies
        let keys_dir = self.storage_dir.join("keys");
        if keys_dir.exists() {
            let mut entries = tokio::fs::read_dir(&keys_dir).await
                .map_err(|e| Error::StorageError(format!("Failed to read directory: {}", e)))?;

            while let Some(entry) = entries.next_entry().await
                .map_err(|e| Error::StorageError(format!("Failed to read entry: {}", e)))? {
                
                if let Some(ext) = entry.path().extension() {
                    if ext == "keys" {
                        let encrypted = tokio::fs::read(entry.path()).await
                            .map_err(|e| Error::StorageError(format!("Failed to read file: {}", e)))?;
                        
                        let decrypted = self.encryption.decrypt(&encrypted)?;
                        let hierarchy = self.deserialize_key_hierarchy(&decrypted)?;
                        
                        let did_str = entry.path().file_stem().unwrap().to_string_lossy().to_string();
                        backup_data.key_hierarchies.insert(did_str, hierarchy);
                    }
                }
            }
        }

        // Encrypt entire backup
        let serialized = serde_json::to_vec(&backup_data)
            .map_err(|e| Error::StorageError(format!("Backup serialization failed: {}", e)))?;

        let backup_encryption = EncryptionManager::new(password)?;
        let encrypted_backup = backup_encryption.encrypt(&serialized)?;

        Ok(EncryptedBackup {
            data: encrypted_backup,
            created_at: Utc::now(),
            version: "1.0".to_string(),
        })
    }

    /// Import data from backup
    pub async fn import_backup(&mut self, backup: &EncryptedBackup, password: &str) -> Result<()> {
        let backup_encryption = EncryptionManager::new(password)?;
        let decrypted = backup_encryption.decrypt(&backup.data)?;
        
        let backup_data: BackupData = serde_json::from_slice(&decrypted)
            .map_err(|e| Error::StorageError(format!("Backup deserialization failed: {}", e)))?;

        // Import documents
        for (_, document) in backup_data.documents {
            self.store_did_document(&document).await?;
        }

        // Import key hierarchies
        for (did_str, hierarchy) in backup_data.key_hierarchies {
            let did = Did::parse(&did_str)?;
            self.store_key_hierarchy(&did, &hierarchy).await?;
        }

        // Import contacts
        self.contact_vault.import_contacts(backup_data.contacts).await?;

        Ok(())
    }

    /// Start synchronization with remote devices
    pub async fn start_sync(&mut self, sync_endpoints: Vec<SyncEndpoint>) -> Result<()> {
        self.sync_manager.configure_endpoints(sync_endpoints).await?;
        self.sync_manager.start_sync().await
    }

    /// Stop synchronization
    pub async fn stop_sync(&mut self) -> Result<()> {
        self.sync_manager.stop_sync().await
    }

    /// Get sync status
    pub fn get_sync_status(&self) -> SyncStatus {
        self.sync_manager.get_status()
    }

    /// Helper methods
    fn sanitize_did(&self, did: &Did) -> String {
        did.to_string().replace(':', "_").replace('/', "_")
    }

    async fn update_document_index(&self, did: &Did, file_path: &Path) -> Result<()> {
        // Update index for faster lookups
        let index_path = self.storage_dir.join("index.json");
        let mut index: HashMap<String, String> = if index_path.exists() {
            let data = tokio::fs::read(&index_path).await
                .map_err(|e| Error::StorageError(format!("Failed to read index: {}", e)))?;
            serde_json::from_slice(&data).unwrap_or_default()
        } else {
            HashMap::new()
        };

        index.insert(did.to_string(), file_path.to_string_lossy().to_string());

        let serialized = serde_json::to_vec(&index)
            .map_err(|e| Error::StorageError(format!("Index serialization failed: {}", e)))?;

        tokio::fs::write(&index_path, &serialized).await
            .map_err(|e| Error::StorageError(format!("Failed to write index: {}", e)))?;

        Ok(())
    }

    fn serialize_key_hierarchy(&self, hierarchy: &KeyHierarchy) -> Result<Vec<u8>> {
        // This is a placeholder - would need actual serialization implementation
        // for KeyHierarchy
        Ok(Vec::new())
    }

    fn deserialize_key_hierarchy(&self, data: &[u8]) -> Result<KeyHierarchy> {
        // This is a placeholder - would need actual deserialization implementation
        // for KeyHierarchy
        Err(Error::StorageError("Not implemented".into()))
    }
}

/// Encryption manager for local storage
pub struct EncryptionManager {
    /// Master key for encryption
    master_key: Vec<u8>,
    /// Salt for key derivation
    salt: Vec<u8>,
}

impl EncryptionManager {
    /// Create new encryption manager
    pub fn new(password: &str) -> Result<Self> {
        use scrypt::{Scrypt, scrypt};
        use rand::RngCore;

        let mut salt = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut salt);

        let mut master_key = vec![0u8; 32];
        scrypt(
            password.as_bytes(),
            &salt,
            &scrypt::Params::new(15, 8, 1, 32).unwrap(),
            &mut master_key
        ).map_err(|e| Error::CryptographicError(format!("Key derivation failed: {}", e)))?;

        Ok(Self { master_key, salt })
    }

    /// Encrypt data
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
        use chacha20poly1305::aead::Aead;
        use rand::RngCore;

        let key = Key::from_slice(&self.master_key);
        let cipher = ChaCha20Poly1305::new(&key);

        let mut nonce = vec![0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce);

        let ciphertext = cipher.encrypt(Nonce::from_slice(&nonce), data)
            .map_err(|e| Error::CryptographicError(format!("Encryption failed: {}", e)))?;

        // Prepend nonce to ciphertext
        let mut result = nonce;
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    /// Decrypt data
    pub fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
        use chacha20poly1305::aead::Aead;

        if encrypted_data.len() < 12 {
            return Err(Error::CryptographicError("Invalid encrypted data".into()));
        }

        let (nonce, ciphertext) = encrypted_data.split_at(12);
        let key = Key::from_slice(&self.master_key);
        let cipher = ChaCha20Poly1305::new(&key);

        cipher.decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|e| Error::CryptographicError(format!("Decryption failed: {}", e)))
    }
}

/// Contact vault for managing DID contacts
pub struct ContactVault {
    storage_path: PathBuf,
    encryption: EncryptionManager,
}

impl ContactVault {
    pub fn new(storage_dir: &Path, encryption: &EncryptionManager) -> Result<Self> {
        let storage_path = storage_dir.join("contacts.vault");
        
        // Clone encryption manager (would need Clone trait implementation)
        let encryption = EncryptionManager::new("placeholder")?; // This would need proper key sharing

        Ok(Self {
            storage_path,
            encryption,
        })
    }

    pub async fn store_contact(&mut self, contact: &Contact) -> Result<()> {
        let mut contacts = self.load_all_contacts().await?;
        contacts.insert(contact.did.to_string(), contact.clone());
        self.save_all_contacts(&contacts).await
    }

    pub async fn load_contact(&self, did: &Did) -> Result<Option<Contact>> {
        let contacts = self.load_all_contacts().await?;
        Ok(contacts.get(&did.to_string()).cloned())
    }

    pub async fn list_contacts(&self) -> Result<Vec<Contact>> {
        let contacts = self.load_all_contacts().await?;
        Ok(contacts.into_values().collect())
    }

    pub async fn export_all(&self) -> Result<Vec<Contact>> {
        self.list_contacts().await
    }

    pub async fn import_contacts(&mut self, contacts: Vec<Contact>) -> Result<()> {
        for contact in contacts {
            self.store_contact(&contact).await?;
        }
        Ok(())
    }

    async fn load_all_contacts(&self) -> Result<HashMap<String, Contact>> {
        if !self.storage_path.exists() {
            return Ok(HashMap::new());
        }

        let encrypted = tokio::fs::read(&self.storage_path).await
            .map_err(|e| Error::StorageError(format!("Failed to read contacts: {}", e)))?;

        let decrypted = self.encryption.decrypt(&encrypted)?;
        
        serde_json::from_slice(&decrypted)
            .map_err(|e| Error::StorageError(format!("Contact deserialization failed: {}", e)))
    }

    async fn save_all_contacts(&self, contacts: &HashMap<String, Contact>) -> Result<()> {
        let serialized = serde_json::to_vec(contacts)
            .map_err(|e| Error::StorageError(format!("Contact serialization failed: {}", e)))?;

        let encrypted = self.encryption.encrypt(&serialized)?;

        tokio::fs::write(&self.storage_path, &encrypted).await
            .map_err(|e| Error::StorageError(format!("Failed to write contacts: {}", e)))?;

        Ok(())
    }
}

/// Synchronization manager for multi-device sync
pub struct SyncManager {
    storage_dir: PathBuf,
    endpoints: Vec<SyncEndpoint>,
    status: SyncStatus,
    sync_queue: Vec<SyncItem>,
}

impl SyncManager {
    pub fn new(storage_dir: &Path) -> Result<Self> {
        Ok(Self {
            storage_dir: storage_dir.to_path_buf(),
            endpoints: Vec::new(),
            status: SyncStatus::Stopped,
            sync_queue: Vec::new(),
        })
    }

    pub async fn configure_endpoints(&mut self, endpoints: Vec<SyncEndpoint>) -> Result<()> {
        self.endpoints = endpoints;
        Ok(())
    }

    pub async fn start_sync(&mut self) -> Result<()> {
        self.status = SyncStatus::Active;
        // Implementation would start background sync tasks
        Ok(())
    }

    pub async fn stop_sync(&mut self) -> Result<()> {
        self.status = SyncStatus::Stopped;
        Ok(())
    }

    pub async fn queue_sync(&mut self, item: SyncItem) -> Result<()> {
        self.sync_queue.push(item);
        Ok(())
    }

    pub fn get_status(&self) -> SyncStatus {
        self.status.clone()
    }
}

/// Contact information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub did: Did,
    pub display_name: String,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub public_keys: Vec<ContactPublicKey>,
    pub service_endpoints: Vec<ContactService>,
    pub tags: Vec<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactPublicKey {
    pub key_id: String,
    pub key_type: String,
    pub public_key: Vec<u8>,
    pub purposes: Vec<String>, // ["authentication", "keyAgreement", etc.]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactService {
    pub service_type: String,
    pub endpoint: String,
    pub priority: u32,
}

/// Storage configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub auto_sync: bool,
    pub compression_enabled: bool,
    pub backup_retention_days: u32,
    pub max_storage_size_mb: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            auto_sync: true,
            compression_enabled: true,
            backup_retention_days: 30,
            max_storage_size_mb: 1024, // 1GB
        }
    }
}

/// Backup data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupData {
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub documents: HashMap<String, DidDocument>,
    pub key_hierarchies: HashMap<String, KeyHierarchy>,
    pub contacts: Vec<Contact>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Encrypted backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedBackup {
    pub data: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub version: String,
}

/// Sync endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEndpoint {
    pub url: String,
    pub auth_token: Option<String>,
    pub enabled: bool,
    pub priority: u32,
}

/// Sync status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStatus {
    Stopped,
    Active,
    Error(String),
}

/// Items that can be synchronized
#[derive(Debug, Clone)]
pub enum SyncItem {
    DidDocument(Did),
    KeyHierarchy(Did),
    Contact(Did),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_local_storage_creation() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalFirstStorage::new(
            temp_dir.path(),
            "test_password",
            StorageConfig::default(),
        ).unwrap();

        assert!(temp_dir.path().exists());
    }

    #[tokio::test]
    async fn test_did_document_storage() {
        let temp_dir = TempDir::new().unwrap();
        let mut storage = LocalFirstStorage::new(
            temp_dir.path(),
            "test_password",
            StorageConfig::default(),
        ).unwrap();

        let did = Did::new("test", "example");
        let document = super::super::DidDocument::new(did.clone());

        storage.store_did_document(&document).await.unwrap();
        let loaded = storage.load_did_document(&did).await.unwrap();

        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().id, did);
    }

    #[test]
    fn test_encryption_manager() {
        let encryption = EncryptionManager::new("test_password").unwrap();
        let data = b"Hello, World!";
        
        let encrypted = encryption.encrypt(data).unwrap();
        let decrypted = encryption.decrypt(&encrypted).unwrap();
        
        assert_eq!(data, decrypted.as_slice());
    }

    #[tokio::test]
    async fn test_contact_vault() {
        let temp_dir = TempDir::new().unwrap();
        let encryption = EncryptionManager::new("test_password").unwrap();
        let mut vault = ContactVault::new(temp_dir.path(), &encryption).unwrap();

        let contact = Contact {
            did: Did::new("test", "alice"),
            display_name: "Alice".to_string(),
            nickname: Some("Al".to_string()),
            avatar_url: None,
            public_keys: Vec::new(),
            service_endpoints: Vec::new(),
            tags: vec!["friend".to_string()],
            notes: Some("Test contact".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            verified: true,
        };

        vault.store_contact(&contact).await.unwrap();
        let loaded = vault.load_contact(&contact.did).await.unwrap();

        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().display_name, "Alice");
    }
}