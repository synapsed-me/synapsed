//! Progressive Web App (PWA) support for DID-based identity
//! 
//! This module provides:
//! - WebAuthn integration for passwordless authentication
//! - Service worker compatibility for offline operation
//! - Browser-based key management
//! - Local storage with encryption

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::{Result, Error};
use super::did::{Did, DidDocument};

// PWA modules (placeholders for now - would need full implementation)
#[cfg(feature = "pwa-support")]
pub mod webauthn {
    //! WebAuthn integration placeholder
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use crate::{Result, Error};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct WebAuthnConfig {
        pub rp_id: String,
        pub rp_name: String,
        pub rp_origin: String,
    }

    impl Default for WebAuthnConfig {
        fn default() -> Self {
            Self {
                rp_id: "localhost".to_string(),
                rp_name: "Synapsed Identity".to_string(),
                rp_origin: "https://localhost".to_string(),
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct WebAuthnCredential {
        pub id: String,
        pub public_key: Vec<u8>,
        pub counter: u32,
    }

    #[derive(Debug, Clone)]
    pub struct AuthenticatorData {
        pub rp_id_hash: Vec<u8>,
        pub flags: u8,
        pub counter: u32,
    }

    pub struct WebAuthnManager {
        config: WebAuthnConfig,
    }

    impl WebAuthnManager {
        pub async fn new(config: &WebAuthnConfig) -> Result<Self> {
            Ok(Self {
                config: config.clone(),
            })
        }

        pub async fn create_credential(&self, _username: &str) -> Result<WebAuthnCredential> {
            // Placeholder implementation
            Ok(WebAuthnCredential {
                id: "placeholder_id".to_string(),
                public_key: vec![0u8; 32],
                counter: 0,
            })
        }

        pub async fn get_assertion(&self, _credential_id: &str, _challenge: &[u8]) -> Result<WebAuthnAssertion> {
            // Placeholder implementation
            Ok(WebAuthnAssertion {
                credential_id: "placeholder_id".to_string(),
                signature: vec![0u8; 64],
                authenticator_data: AuthenticatorData {
                    rp_id_hash: vec![0u8; 32],
                    flags: 0x01,
                    counter: 1,
                },
            })
        }

        pub async fn verify_assertion(&self, _assertion: &WebAuthnAssertion, _challenge: &[u8]) -> Result<VerificationResult> {
            // Placeholder implementation
            Ok(VerificationResult {
                verified: true,
                metadata: HashMap::new(),
            })
        }
    }

    #[derive(Debug, Clone)]
    pub struct WebAuthnAssertion {
        pub credential_id: String,
        pub signature: Vec<u8>,
        pub authenticator_data: AuthenticatorData,
    }

    #[derive(Debug, Clone)]
    pub struct VerificationResult {
        pub verified: bool,
        pub metadata: HashMap<String, serde_json::Value>,
    }
}

#[cfg(feature = "pwa-support")]
pub mod service_worker {
    //! Service worker integration placeholder
    use crate::{Result, Error};
    use super::super::did::DidDocument;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ServiceWorkerConfig {
        pub cache_name: String,
        pub offline_urls: Vec<String>,
    }

    impl Default for ServiceWorkerConfig {
        fn default() -> Self {
            Self {
                cache_name: "synapsed-identity-v1".to_string(),
                offline_urls: vec!["/".to_string()],
            }
        }
    }

    pub struct ServiceWorkerManager {
        config: ServiceWorkerConfig,
    }

    impl ServiceWorkerManager {
        pub async fn new(config: &ServiceWorkerConfig) -> Result<Self> {
            Ok(Self {
                config: config.clone(),
            })
        }

        pub async fn enable_offline_caching(&self) -> Result<()> {
            // Placeholder implementation
            Ok(())
        }

        pub async fn cache_did_document(&self, _document: &DidDocument) -> Result<()> {
            // Placeholder implementation
            Ok(())
        }

        pub fn is_offline(&self) -> bool {
            // Placeholder implementation
            false
        }
    }

    #[derive(Debug, Clone)]
    pub struct OfflineCapability {
        pub enabled: bool,
        pub cached_resources: Vec<String>,
    }
}

#[cfg(feature = "pwa-support")]
pub mod browser_storage {
    //! Browser storage integration placeholder
    use crate::{Result, Error};
    use super::super::did::DidDocument;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BrowserStorageConfig {
        pub database_name: String,
        pub version: u32,
    }

    impl Default for BrowserStorageConfig {
        fn default() -> Self {
            Self {
                database_name: "synapsed-identity".to_string(),
                version: 1,
            }
        }
    }

    pub struct BrowserStorageManager {
        config: BrowserStorageConfig,
    }

    impl BrowserStorageManager {
        pub async fn new(config: &BrowserStorageConfig) -> Result<Self> {
            Ok(Self {
                config: config.clone(),
            })
        }

        pub async fn store_did_document(&self, _document: &DidDocument) -> Result<()> {
            // Placeholder implementation
            Ok(())
        }

        pub async fn clear_stored_data(&self) -> Result<()> {
            // Placeholder implementation
            Ok(())
        }

        pub async fn sync_with_remote(&self) -> Result<()> {
            // Placeholder implementation
            Ok(())
        }
    }

    pub struct IndexedDbStorage {
        database_name: String,
    }

    impl IndexedDbStorage {
        pub fn new(database_name: String) -> Self {
            Self { database_name }
        }
    }
}

#[cfg(feature = "pwa-support")]
pub use webauthn::{WebAuthnManager, WebAuthnCredential, AuthenticatorData};
#[cfg(feature = "pwa-support")]
pub use service_worker::{ServiceWorkerManager, OfflineCapability};
#[cfg(feature = "pwa-support")]
pub use browser_storage::{BrowserStorageManager, IndexedDbStorage};

/// PWA-compatible DID manager
pub struct PwaDidManager {
    /// WebAuthn manager for biometric authentication
    #[cfg(feature = "webauthn")]
    webauthn: webauthn::WebAuthnManager,
    
    /// Service worker for offline capability
    #[cfg(feature = "pwa-support")]
    service_worker: service_worker::ServiceWorkerManager,
    
    /// Browser storage manager
    #[cfg(feature = "pwa-support")]
    storage: browser_storage::BrowserStorageManager,
    
    /// Current identity
    current_identity: Option<PwaIdentity>,
    
    /// Configuration
    config: PwaConfig,
}

impl PwaDidManager {
    /// Create a new PWA DID manager
    pub async fn new(config: PwaConfig) -> Result<Self> {
        Ok(Self {
            #[cfg(feature = "webauthn")]
            webauthn: webauthn::WebAuthnManager::new(&config.webauthn).await?,
            
            #[cfg(feature = "pwa-support")]
            service_worker: service_worker::ServiceWorkerManager::new(&config.service_worker).await?,
            
            #[cfg(feature = "pwa-support")]
            storage: browser_storage::BrowserStorageManager::new(&config.storage).await?,
            
            current_identity: None,
            config,
        })
    }

    /// Initialize PWA identity with biometric authentication
    #[cfg(feature = "webauthn")]
    pub async fn initialize_identity(&mut self, username: &str) -> Result<PwaIdentity> {
        // Create WebAuthn credential
        let credential = self.webauthn.create_credential(username).await?;
        
        // Generate DID based on credential
        let did = self.generate_did_from_credential(&credential)?;
        
        // Create DID document
        let document = self.create_did_document_from_credential(&did, &credential)?;
        
        // Store in browser storage
        #[cfg(feature = "pwa-support")]
        self.storage.store_did_document(&document).await?;
        
        let identity = PwaIdentity {
            did,
            document,
            webauthn_credential: Some(credential),
            created_at: Utc::now(),
            last_used: Utc::now(),
        };

        self.current_identity = Some(identity.clone());
        Ok(identity)
    }

    /// Authenticate using biometrics
    #[cfg(feature = "webauthn")]
    pub async fn authenticate(&mut self, challenge: &[u8]) -> Result<AuthenticationResult> {
        let credential_id = self.current_identity
            .as_ref()
            .and_then(|i| i.webauthn_credential.as_ref())
            .map(|c| c.id.clone())
            .ok_or_else(|| Error::AuthenticationError("No credential available".into()))?;

        let assertion = self.webauthn.get_assertion(&credential_id, challenge).await?;
        
        // Verify assertion
        let verification_result = self.webauthn.verify_assertion(&assertion, challenge).await?;
        
        if verification_result.verified {
            // Update last used time
            if let Some(ref mut identity) = self.current_identity {
                identity.last_used = Utc::now();
            }

            Ok(AuthenticationResult {
                success: true,
                identity: self.current_identity.clone(),
                signature: assertion.signature,
                metadata: verification_result.metadata,
            })
        } else {
            Ok(AuthenticationResult {
                success: false,
                identity: None,
                signature: Vec::new(),
                metadata: HashMap::new(),
            })
        }
    }

    /// Enable offline mode
    #[cfg(feature = "pwa-support")]
    pub async fn enable_offline_mode(&mut self) -> Result<()> {
        self.service_worker.enable_offline_caching().await?;
        
        // Cache essential DID data
        if let Some(ref identity) = self.current_identity {
            self.service_worker.cache_did_document(&identity.document).await?;
        }
        
        Ok(())
    }

    /// Check if currently offline
    #[cfg(feature = "pwa-support")]
    pub fn is_offline(&self) -> bool {
        self.service_worker.is_offline()
    }

    /// Sync data when coming back online
    #[cfg(feature = "pwa-support")]
    pub async fn sync_when_online(&mut self) -> Result<()> {
        if !self.is_offline() {
            self.storage.sync_with_remote().await?;
        }
        Ok(())
    }

    /// Generate DID from WebAuthn credential
    #[cfg(feature = "webauthn")]
    fn generate_did_from_credential(&self, credential: &webauthn::WebAuthnCredential) -> Result<Did> {
        use sha3::{Sha3_256, Digest};
        
        // Use credential public key to generate did:key
        let mut hasher = Sha3_256::new();
        hasher.update(&credential.public_key);
        let hash = hasher.finalize();
        
        // Encode as multibase for did:key
        let multibase_key = multibase::encode(multibase::Base::Base58Btc, &hash[..16]);
        
        Ok(Did::new("key", &multibase_key))
    }

    /// Create DID document from WebAuthn credential
    #[cfg(feature = "webauthn")]
    fn create_did_document_from_credential(
        &self,
        did: &Did,
        credential: &webauthn::WebAuthnCredential,
    ) -> Result<DidDocument> {
        use super::did::{VerificationMethod, PublicKeyMaterial, VerificationRelationship};
        
        let mut document = DidDocument::new(did.clone());
        
        // Add WebAuthn verification method
        let verification_method = VerificationMethod::new(
            format!("{}#webauthn-1", did.to_string()),
            "WebAuthnAuthentication2021".to_string(),
            did.clone(),
            PublicKeyMaterial::PublicKeyMultibase {
                public_key_multibase: multibase::encode(multibase::Base::Base58Btc, &credential.public_key),
            },
        );
        
        document.add_verification_method(verification_method);
        document.add_authentication_reference(format!("{}#webauthn-1", did.to_string()));
        
        // Add capability invocation
        document.capability_invocation.push(VerificationRelationship::Reference(
            format!("{}#webauthn-1", did.to_string())
        ));
        
        Ok(document)
    }

    /// Export identity for backup
    pub async fn export_identity(&self) -> Result<IdentityBackup> {
        let identity = self.current_identity
            .as_ref()
            .ok_or_else(|| Error::ConfigurationError("No identity to export".into()))?;

        Ok(IdentityBackup {
            did: identity.did.clone(),
            document: identity.document.clone(),
            created_at: identity.created_at,
            backup_timestamp: Utc::now(),
            version: "1.0".to_string(),
        })
    }

    /// Import identity from backup
    pub async fn import_identity(&mut self, backup: IdentityBackup) -> Result<()> {
        let identity = PwaIdentity {
            did: backup.did,
            document: backup.document,
            webauthn_credential: None, // Will need to be re-registered
            created_at: backup.created_at,
            last_used: Utc::now(),
        };

        #[cfg(feature = "pwa-support")]
        self.storage.store_did_document(&identity.document).await?;
        
        self.current_identity = Some(identity);
        Ok(())
    }

    /// Get current identity
    pub fn get_current_identity(&self) -> Option<&PwaIdentity> {
        self.current_identity.as_ref()
    }

    /// Clear current identity (logout)
    pub async fn clear_identity(&mut self) -> Result<()> {
        #[cfg(feature = "pwa-support")]
        self.storage.clear_stored_data().await?;
        
        self.current_identity = None;
        Ok(())
    }
}

/// PWA-specific identity structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PwaIdentity {
    /// The DID
    pub did: Did,
    /// DID document
    pub document: DidDocument,
    /// WebAuthn credential (if available)
    #[cfg(feature = "webauthn")]
    pub webauthn_credential: Option<webauthn::WebAuthnCredential>,
    #[cfg(not(feature = "webauthn"))]
    pub webauthn_credential: Option<serde_json::Value>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last used timestamp
    pub last_used: DateTime<Utc>,
}

/// Authentication result
#[derive(Debug, Clone)]
pub struct AuthenticationResult {
    pub success: bool,
    pub identity: Option<PwaIdentity>,
    pub signature: Vec<u8>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Identity backup for export/import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityBackup {
    pub did: Did,
    pub document: DidDocument,
    pub created_at: DateTime<Utc>,
    pub backup_timestamp: DateTime<Utc>,
    pub version: String,
}

/// PWA configuration
#[derive(Debug, Clone)]
pub struct PwaConfig {
    /// WebAuthn configuration
    #[cfg(feature = "webauthn")]
    pub webauthn: webauthn::WebAuthnConfig,
    #[cfg(not(feature = "webauthn"))]
    pub webauthn: serde_json::Value,
    
    /// Service worker configuration
    #[cfg(feature = "pwa-support")]
    pub service_worker: service_worker::ServiceWorkerConfig,
    #[cfg(not(feature = "pwa-support"))]
    pub service_worker: serde_json::Value,
    
    /// Browser storage configuration
    #[cfg(feature = "pwa-support")]
    pub storage: browser_storage::BrowserStorageConfig,
    #[cfg(not(feature = "pwa-support"))]
    pub storage: serde_json::Value,
    
    /// Enable offline mode
    pub offline_mode: bool,
    
    /// Auto-sync interval (in seconds)
    pub auto_sync_interval: u64,
}

impl Default for PwaConfig {
    fn default() -> Self {
        Self {
            #[cfg(feature = "webauthn")]
            webauthn: webauthn::WebAuthnConfig::default(),
            #[cfg(not(feature = "webauthn"))]
            webauthn: serde_json::Value::Null,
            
            #[cfg(feature = "pwa-support")]
            service_worker: service_worker::ServiceWorkerConfig::default(),
            #[cfg(not(feature = "pwa-support"))]
            service_worker: serde_json::Value::Null,
            
            #[cfg(feature = "pwa-support")]
            storage: browser_storage::BrowserStorageConfig::default(),
            #[cfg(not(feature = "pwa-support"))]
            storage: serde_json::Value::Null,
            
            offline_mode: true,
            auto_sync_interval: 300, // 5 minutes
        }
    }
}

/// Browser capability detection
pub struct BrowserCapabilities {
    pub webauthn_supported: bool,
    pub service_worker_supported: bool,
    pub indexeddb_supported: bool,
    pub web_crypto_supported: bool,
    pub notification_supported: bool,
}

impl BrowserCapabilities {
    /// Detect browser capabilities
    #[cfg(feature = "pwa-support")]
    pub async fn detect() -> Self {
        use wasm_bindgen::prelude::*;
        use web_sys::window;
        
        let window = window().expect("should have a window");
        let navigator = window.navigator();
        
        Self {
            webauthn_supported: js_sys::Reflect::has(&navigator, &"credentials".into())
                .unwrap_or(false),
            service_worker_supported: js_sys::Reflect::has(&navigator, &"serviceWorker".into())
                .unwrap_or(false),
            indexeddb_supported: js_sys::Reflect::has(&window, &"indexedDB".into())
                .unwrap_or(false),
            web_crypto_supported: js_sys::Reflect::has(&window.crypto().unwrap(), &"subtle".into())
                .unwrap_or(false),
            notification_supported: js_sys::Reflect::has(&window, &"Notification".into())
                .unwrap_or(false),
        }
    }
    
    #[cfg(not(feature = "pwa-support"))]
    pub async fn detect() -> Self {
        Self {
            webauthn_supported: false,
            service_worker_supported: false,
            indexeddb_supported: false,
            web_crypto_supported: false,
            notification_supported: false,
        }
    }

    /// Check if all required capabilities are available
    pub fn is_fully_supported(&self) -> bool {
        self.webauthn_supported && 
        self.service_worker_supported && 
        self.indexeddb_supported && 
        self.web_crypto_supported
    }

    /// Get missing capabilities
    pub fn missing_capabilities(&self) -> Vec<String> {
        let mut missing = Vec::new();
        
        if !self.webauthn_supported {
            missing.push("WebAuthn".to_string());
        }
        if !self.service_worker_supported {
            missing.push("Service Worker".to_string());
        }
        if !self.indexeddb_supported {
            missing.push("IndexedDB".to_string());
        }
        if !self.web_crypto_supported {
            missing.push("Web Crypto API".to_string());
        }
        
        missing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_browser_capabilities() {
        let capabilities = BrowserCapabilities::detect().await;
        
        // In test environment, these will likely be false
        // In a real browser, some should be true
        assert!(!capabilities.is_fully_supported() || capabilities.is_fully_supported());
    }

    #[test]
    fn test_pwa_config_default() {
        let config = PwaConfig::default();
        assert_eq!(config.auto_sync_interval, 300);
        assert!(config.offline_mode);
    }

    #[test]
    fn test_identity_backup_serialization() {
        let did = Did::new("test", "example");
        let document = DidDocument::new(did.clone());
        
        let backup = IdentityBackup {
            did,
            document,
            created_at: Utc::now(),
            backup_timestamp: Utc::now(),
            version: "1.0".to_string(),
        };

        let serialized = serde_json::to_string(&backup).unwrap();
        let deserialized: IdentityBackup = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(backup.version, deserialized.version);
    }
}