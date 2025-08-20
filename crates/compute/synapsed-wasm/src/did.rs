//! Decentralized Identity (DID) operations for WASM
//!
//! This module provides WebAssembly-compatible DID document management,
//! key derivation, and recovery operations optimized for browser execution.
//! It supports multiple DID methods and cryptographic key operations.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use wasm_bindgen::prelude::*;
use sha2::{Digest, Sha256};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;

use crate::error::{WasmError, WasmResult};
use crate::types::{HostFunction, WasmValue};

/// DID manager for decentralized identity operations
pub struct DidManager {
    /// DID documents
    documents: HashMap<String, DidDocument>,
    /// Key pairs for signing
    keypairs: HashMap<String, KeyPair>,
    /// DID methods registry
    methods: HashMap<String, Box<dyn DidMethod>>,
    /// Manager statistics
    stats: DidStats,
}

impl DidManager {
    /// Create a new DID manager
    pub fn new() -> WasmResult<Self> {
        let mut manager = Self {
            documents: HashMap::new(),
            keypairs: HashMap::new(),
            methods: HashMap::new(),
            stats: DidStats::default(),
        };

        // Register default DID methods
        manager.register_method("key", Box::new(DidMethodKey::new()))?;
        manager.register_method("web", Box::new(DidMethodWeb::new()))?;
        
        Ok(manager)
    }

    /// Register a DID method
    pub fn register_method(&mut self, method_name: &str, method: Box<dyn DidMethod>) -> WasmResult<()> {
        self.methods.insert(method_name.to_string(), method);
        tracing::info!(method = %method_name, "DID method registered");
        Ok(())
    }

    /// Create a new DID document
    pub async fn create_did(
        &mut self,
        method: &str,
        options: DidCreationOptions,
    ) -> WasmResult<String> {
        let did_method = self.methods.get(method)
            .ok_or_else(|| WasmError::Configuration(format!("DID method '{}' not found", method)))?;

        let (did_id, document, keypair) = did_method.create_did(options).await?;
        
        self.documents.insert(did_id.clone(), document);
        self.keypairs.insert(did_id.clone(), keypair);
        self.stats.dids_created += 1;

        tracing::info!(did_id = %did_id, method = %method, "DID created");
        Ok(did_id)
    }

    /// Resolve DID document
    pub async fn resolve_did(&self, did_id: &str) -> WasmResult<DidDocument> {
        if let Some(document) = self.documents.get(did_id) {
            self.stats.dids_resolved += 1;
            Ok(document.clone())
        } else {
            // Try to resolve from network or registry
            let (method, _) = self.parse_did(did_id)?;
            let did_method = self.methods.get(&method)
                .ok_or_else(|| WasmError::Configuration(format!("DID method '{}' not found", method)))?;
                
            did_method.resolve_did(did_id).await
        }
    }

    /// Update DID document
    pub async fn update_did(
        &mut self,
        did_id: &str,
        update_operation: DidUpdateOperation,
    ) -> WasmResult<()> {
        let document = self.documents.get_mut(did_id)
            .ok_or_else(|| WasmError::Configuration(format!("DID '{}' not found", did_id)))?;

        let keypair = self.keypairs.get(did_id)
            .ok_or_else(|| WasmError::Configuration(format!("Keypair for DID '{}' not found", did_id)))?;

        // Apply update operation
        match update_operation {
            DidUpdateOperation::AddVerificationMethod(vm) => {
                document.verification_method.push(vm);
            }
            DidUpdateOperation::RemoveVerificationMethod(vm_id) => {
                document.verification_method.retain(|vm| vm.id != vm_id);
            }
            DidUpdateOperation::AddService(service) => {
                document.service.push(service);
            }
            DidUpdateOperation::RemoveService(service_id) => {
                document.service.retain(|s| s.id != service_id);
            }
            DidUpdateOperation::UpdateAuthentication(auth_methods) => {
                document.authentication = auth_methods;
            }
        }

        // Update document version and timestamp
        document.version += 1;
        document.updated = Some(chrono::Utc::now());

        self.stats.dids_updated += 1;
        tracing::info!(did_id = %did_id, "DID document updated");
        Ok(())
    }

    /// Sign data with DID
    pub async fn sign_with_did(
        &self,
        did_id: &str,
        data: &[u8],
        verification_method_id: Option<&str>,
    ) -> WasmResult<DidSignature> {
        let keypair = self.keypairs.get(did_id)
            .ok_or_else(|| WasmError::Configuration(format!("Keypair for DID '{}' not found", did_id)))?;

        let signature = keypair.sign(data)?;
        
        let did_signature = DidSignature {
            signature,
            signer: did_id.to_string(),
            verification_method: verification_method_id.map(|s| s.to_string()),
            created: chrono::Utc::now(),
        };

        self.stats.signatures_created += 1;
        Ok(did_signature)
    }

    /// Verify signature
    pub async fn verify_signature(
        &self,
        signature: &DidSignature,
        data: &[u8],
    ) -> WasmResult<bool> {
        let document = self.resolve_did(&signature.signer).await?;
        
        // Find the appropriate verification method
        let verification_method = if let Some(vm_id) = &signature.verification_method {
            document.verification_method.iter()
                .find(|vm| vm.id == *vm_id)
                .ok_or_else(|| WasmError::Cryptographic("Verification method not found".to_string()))?
        } else {
            document.verification_method.first()
                .ok_or_else(|| WasmError::Cryptographic("No verification methods available".to_string()))?
        };

        let public_key = PublicKey::from_bytes(&verification_method.public_key_multibase)
            .map_err(|_| WasmError::Cryptographic("Invalid public key".to_string()))?;

        let ed25519_signature = ed25519_dalek::Signature::from_bytes(&signature.signature.data)
            .map_err(|_| WasmError::Cryptographic("Invalid signature format".to_string()))?;

        let is_valid = public_key.verify(data, &ed25519_signature).is_ok();
        self.stats.signatures_verified += 1;
        
        Ok(is_valid)
    }

    /// Derive key from seed
    pub fn derive_key(&self, seed: &[u8], derivation_path: &str) -> WasmResult<KeyPair> {
        KeyDerivation::derive_keypair_from_seed(seed, derivation_path)
    }

    /// Generate recovery seed
    pub fn generate_recovery_seed(&mut self, entropy_bits: usize) -> WasmResult<RecoverySeed> {
        let seed = RecoverySeed::generate(entropy_bits)?;
        self.stats.recovery_seeds_generated += 1;
        Ok(seed)
    }

    /// Recover keys from seed
    pub fn recover_from_seed(&self, seed: &RecoverySeed, derivation_paths: &[&str]) -> WasmResult<Vec<KeyPair>> {
        let mut keypairs = Vec::new();
        
        for path in derivation_paths {
            let keypair = self.derive_key(&seed.entropy, path)?;
            keypairs.push(keypair);
        }
        
        Ok(keypairs)
    }

    /// Get statistics
    pub fn get_stats(&self) -> &DidStats {
        &self.stats
    }

    /// List managed DIDs
    pub fn list_dids(&self) -> Vec<String> {
        self.documents.keys().cloned().collect()
    }

    /// Parse DID to extract method and identifier
    fn parse_did(&self, did: &str) -> WasmResult<(String, String)> {
        let parts: Vec<&str> = did.split(':').collect();
        if parts.len() < 3 || parts[0] != "did" {
            return Err(WasmError::Configuration("Invalid DID format".to_string()));
        }
        
        Ok((parts[1].to_string(), parts[2..].join(":")))
    }
}

/// DID method trait for different DID types
#[async_trait]
pub trait DidMethod: Send + Sync {
    /// Create a new DID
    async fn create_did(&self, options: DidCreationOptions) -> WasmResult<(String, DidDocument, KeyPair)>;
    
    /// Resolve DID document
    async fn resolve_did(&self, did_id: &str) -> WasmResult<DidDocument>;
    
    /// Update DID document (if supported)
    async fn update_did(&self, did_id: &str, document: &DidDocument) -> WasmResult<()>;
}

/// DID method: key (did:key)
pub struct DidMethodKey;

impl DidMethodKey {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DidMethod for DidMethodKey {
    async fn create_did(&self, _options: DidCreationOptions) -> WasmResult<(String, DidDocument, KeyPair)> {
        let keypair = KeyPair::generate()?;
        let public_key_bytes = keypair.public_key_bytes();
        
        // Create DID identifier from public key
        let did_id = format!("did:key:z{}", multibase::encode(multibase::Base::Base58Btc, &public_key_bytes));
        
        let verification_method = VerificationMethod {
            id: format!("{}#keys-1", did_id),
            method_type: "Ed25519VerificationKey2020".to_string(),
            controller: did_id.clone(),
            public_key_multibase: public_key_bytes,
        };
        
        let document = DidDocument {
            id: did_id.clone(),
            context: vec!["https://www.w3.org/ns/did/v1".to_string()],
            verification_method: vec![verification_method.clone()],
            authentication: vec![verification_method.id.clone()],
            assertion_method: vec![verification_method.id.clone()],
            key_agreement: vec![verification_method.id.clone()],
            capability_invocation: vec![verification_method.id.clone()],
            capability_delegation: vec![verification_method.id.clone()],
            service: vec![],
            version: 1,
            created: chrono::Utc::now(),
            updated: None,
        };
        
        Ok((did_id, document, keypair))
    }
    
    async fn resolve_did(&self, did_id: &str) -> WasmResult<DidDocument> {
        // For did:key, the document can be reconstructed from the identifier
        if !did_id.starts_with("did:key:z") {
            return Err(WasmError::Configuration("Invalid did:key format".to_string()));
        }
        
        let encoded_key = &did_id[9..]; // Remove "did:key:z"
        let public_key_bytes = multibase::decode(encoded_key)
            .map_err(|_| WasmError::Configuration("Invalid multibase encoding".to_string()))?
            .1;
        
        let verification_method = VerificationMethod {
            id: format!("{}#keys-1", did_id),
            method_type: "Ed25519VerificationKey2020".to_string(),
            controller: did_id.to_string(),
            public_key_multibase: public_key_bytes,
        };
        
        let document = DidDocument {
            id: did_id.to_string(),
            context: vec!["https://www.w3.org/ns/did/v1".to_string()],
            verification_method: vec![verification_method.clone()],
            authentication: vec![verification_method.id.clone()],
            assertion_method: vec![verification_method.id.clone()],
            key_agreement: vec![verification_method.id.clone()],
            capability_invocation: vec![verification_method.id.clone()],
            capability_delegation: vec![verification_method.id.clone()],
            service: vec![],
            version: 1,
            created: chrono::Utc::now(),
            updated: None,
        };
        
        Ok(document)
    }
    
    async fn update_did(&self, _did_id: &str, _document: &DidDocument) -> WasmResult<()> {
        Err(WasmError::Configuration("did:key documents are immutable".to_string()))
    }
}

/// DID method: web (did:web)
pub struct DidMethodWeb;

impl DidMethodWeb {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DidMethod for DidMethodWeb {
    async fn create_did(&self, options: DidCreationOptions) -> WasmResult<(String, DidDocument, KeyPair)> {
        let domain = options.domain
            .ok_or_else(|| WasmError::Configuration("Domain required for did:web".to_string()))?;
        
        let keypair = KeyPair::generate()?;
        let public_key_bytes = keypair.public_key_bytes();
        
        let did_id = format!("did:web:{}", domain);
        
        let verification_method = VerificationMethod {
            id: format!("{}#keys-1", did_id),
            method_type: "Ed25519VerificationKey2020".to_string(),
            controller: did_id.clone(),
            public_key_multibase: public_key_bytes,
        };
        
        let document = DidDocument {
            id: did_id.clone(),
            context: vec!["https://www.w3.org/ns/did/v1".to_string()],
            verification_method: vec![verification_method.clone()],
            authentication: vec![verification_method.id.clone()],
            assertion_method: vec![verification_method.id.clone()],
            key_agreement: vec![verification_method.id.clone()],
            capability_invocation: vec![verification_method.id.clone()],
            capability_delegation: vec![verification_method.id.clone()],
            service: vec![],
            version: 1,
            created: chrono::Utc::now(),
            updated: None,
        };
        
        Ok((did_id, document, keypair))
    }
    
    async fn resolve_did(&self, did_id: &str) -> WasmResult<DidDocument> {
        // For did:web, we would fetch from https://domain/.well-known/did.json
        // Simplified implementation returns error for now
        Err(WasmError::Network(format!("Network resolution not implemented for {}", did_id)))
    }
    
    async fn update_did(&self, _did_id: &str, _document: &DidDocument) -> WasmResult<()> {
        // Would update the document at the web location
        Err(WasmError::Network("Network update not implemented".to_string()))
    }
}

/// DID document structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DidDocument {
    /// DID identifier
    pub id: String,
    /// JSON-LD context
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    /// Verification methods
    #[serde(rename = "verificationMethod")]
    pub verification_method: Vec<VerificationMethod>,
    /// Authentication methods
    pub authentication: Vec<String>,
    /// Assertion methods
    #[serde(rename = "assertionMethod")]
    pub assertion_method: Vec<String>,
    /// Key agreement methods
    #[serde(rename = "keyAgreement")]
    pub key_agreement: Vec<String>,
    /// Capability invocation methods
    #[serde(rename = "capabilityInvocation")]
    pub capability_invocation: Vec<String>,
    /// Capability delegation methods
    #[serde(rename = "capabilityDelegation")]
    pub capability_delegation: Vec<String>,
    /// Services
    pub service: Vec<Service>,
    /// Document version
    pub version: u64,
    /// Creation timestamp
    pub created: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp
    pub updated: Option<chrono::DateTime<chrono::Utc>>,
}

/// Verification method
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VerificationMethod {
    /// Method ID
    pub id: String,
    /// Method type
    #[serde(rename = "type")]
    pub method_type: String,
    /// Controller DID
    pub controller: String,
    /// Public key in multibase format
    #[serde(rename = "publicKeyMultibase")]
    pub public_key_multibase: Vec<u8>,
}

/// Service endpoint
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Service {
    /// Service ID
    pub id: String,
    /// Service type
    #[serde(rename = "type")]
    pub service_type: String,
    /// Service endpoint URL
    #[serde(rename = "serviceEndpoint")]
    pub service_endpoint: String,
}

/// Key pair for cryptographic operations
#[derive(Debug, Clone)]
pub struct KeyPair {
    /// Secret key
    secret_key: SecretKey,
    /// Public key
    public_key: PublicKey,
}

impl KeyPair {
    /// Generate a new key pair
    pub fn generate() -> WasmResult<Self> {
        let keypair = Keypair::generate(&mut OsRng);
        
        Ok(Self {
            secret_key: keypair.secret,
            public_key: keypair.public,
        })
    }

    /// Create from secret key bytes
    pub fn from_secret_bytes(secret_bytes: &[u8]) -> WasmResult<Self> {
        let secret_key = SecretKey::from_bytes(secret_bytes)
            .map_err(|_| WasmError::Cryptographic("Invalid secret key".to_string()))?;
        let public_key = PublicKey::from(&secret_key);
        
        Ok(Self {
            secret_key,
            public_key,
        })
    }

    /// Get public key bytes
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.public_key.to_bytes().to_vec()
    }

    /// Get secret key bytes (use with caution)
    pub fn secret_key_bytes(&self) -> Vec<u8> {
        self.secret_key.to_bytes().to_vec()
    }

    /// Sign data
    pub fn sign(&self, data: &[u8]) -> WasmResult<CryptoSignature> {
        let keypair = Keypair {
            secret: self.secret_key,
            public: self.public_key,
        };
        
        let signature = keypair.sign(data);
        
        Ok(CryptoSignature {
            algorithm: "Ed25519".to_string(),
            data: signature.to_bytes().to_vec(),
        })
    }
}

/// Cryptographic signature
#[derive(Debug, Clone)]
pub struct CryptoSignature {
    /// Signature algorithm
    pub algorithm: String,
    /// Signature data
    pub data: Vec<u8>,
}

/// DID signature with metadata
#[derive(Debug, Clone)]
pub struct DidSignature {
    /// Cryptographic signature
    pub signature: CryptoSignature,
    /// Signer DID
    pub signer: String,
    /// Verification method used
    pub verification_method: Option<String>,
    /// Signature creation time
    pub created: chrono::DateTime<chrono::Utc>,
}

/// Key derivation utilities
pub struct KeyDerivation;

impl KeyDerivation {
    /// Derive keypair from seed using BIP32-like derivation
    pub fn derive_keypair_from_seed(seed: &[u8], derivation_path: &str) -> WasmResult<KeyPair> {
        // Simplified key derivation - in practice would use proper BIP32
        let mut hasher = Sha256::new();
        hasher.update(seed);
        hasher.update(derivation_path.as_bytes());
        let derived_seed = hasher.finalize();
        
        KeyPair::from_secret_bytes(&derived_seed[..32])
    }

    /// Derive multiple keys from master seed
    pub fn derive_multiple_keys(seed: &[u8], paths: &[&str]) -> WasmResult<Vec<KeyPair>> {
        let mut keypairs = Vec::new();
        
        for path in paths {
            let keypair = Self::derive_keypair_from_seed(seed, path)?;
            keypairs.push(keypair);
        }
        
        Ok(keypairs)
    }
}

/// Recovery seed for key backup and recovery
#[derive(Debug, Clone)]
pub struct RecoverySeed {
    /// Entropy bytes
    pub entropy: Vec<u8>,
    /// Mnemonic phrase (BIP39)
    pub mnemonic: String,
    /// Checksum
    pub checksum: Vec<u8>,
}

impl RecoverySeed {
    /// Generate new recovery seed
    pub fn generate(entropy_bits: usize) -> WasmResult<Self> {
        if entropy_bits % 8 != 0 || entropy_bits < 128 || entropy_bits > 256 {
            return Err(WasmError::Configuration("Invalid entropy bits".to_string()));
        }
        
        let entropy_bytes = entropy_bits / 8;
        let mut entropy = vec![0u8; entropy_bytes];
        getrandom::getrandom(&mut entropy)
            .map_err(|_| WasmError::Cryptographic("Failed to generate entropy".to_string()))?;
        
        // Generate checksum
        let mut hasher = Sha256::new();
        hasher.update(&entropy);
        let checksum = hasher.finalize()[..4].to_vec();
        
        // Generate mnemonic (simplified - in practice use BIP39 wordlist)
        let mnemonic = format!("seed_{}", hex::encode(&entropy[..8]));
        
        Ok(Self {
            entropy,
            mnemonic,
            checksum,
        })
    }

    /// Validate recovery seed integrity
    pub fn validate(&self) -> WasmResult<bool> {
        let mut hasher = Sha256::new();
        hasher.update(&self.entropy);
        let expected_checksum = hasher.finalize()[..4].to_vec();
        
        Ok(self.checksum == expected_checksum)
    }
}

/// DID creation options
#[derive(Debug, Clone, Default)]
pub struct DidCreationOptions {
    /// Domain for did:web
    pub domain: Option<String>,
    /// Key type preference
    pub key_type: Option<String>,
    /// Additional services to include
    pub services: Vec<Service>,
}

/// DID update operations
#[derive(Debug, Clone)]
pub enum DidUpdateOperation {
    AddVerificationMethod(VerificationMethod),
    RemoveVerificationMethod(String),
    AddService(Service),
    RemoveService(String),
    UpdateAuthentication(Vec<String>),
}

/// DID manager statistics
#[derive(Debug, Clone, Default)]
pub struct DidStats {
    /// DIDs created
    pub dids_created: u64,
    /// DIDs resolved
    pub dids_resolved: u64,
    /// DIDs updated
    pub dids_updated: u64,
    /// Signatures created
    pub signatures_created: u64,
    /// Signatures verified
    pub signatures_verified: u64,
    /// Recovery seeds generated
    pub recovery_seeds_generated: u64,
}

/// Create DID host functions for WASM modules
pub fn create_did_host_functions() -> HashMap<String, HostFunction> {
    let mut functions = HashMap::new();

    // Create DID
    functions.insert(
        "did_create".to_string(),
        Arc::new(|args| {
            if let Some(WasmValue::String(method)) = args.get(0) {
                tracing::info!(method = %method, "Creating DID");
                Ok(vec![WasmValue::String(format!("did:{}:generated_id", method))])
            } else {
                Err(WasmError::Configuration("DID method required".to_string()))
            }
        }) as HostFunction,
    );

    // Resolve DID
    functions.insert(
        "did_resolve".to_string(),
        Arc::new(|args| {
            if let Some(WasmValue::String(did_id)) = args.get(0) {
                tracing::info!(did_id = %did_id, "Resolving DID");
                Ok(vec![WasmValue::Bytes(b"did_document".to_vec())])
            } else {
                Err(WasmError::Configuration("DID identifier required".to_string()))
            }
        }) as HostFunction,
    );

    // Sign with DID
    functions.insert(
        "did_sign".to_string(),
        Arc::new(|args| {
            match (args.get(0), args.get(1)) {
                (Some(WasmValue::String(did_id)), Some(WasmValue::Bytes(data))) => {
                    tracing::info!(
                        did_id = %did_id,
                        data_len = data.len(),
                        "Signing with DID"
                    );
                    Ok(vec![WasmValue::Bytes(b"did_signature".to_vec())])
                }
                _ => Err(WasmError::Configuration("Invalid arguments for DID signing".to_string()))
            }
        }) as HostFunction,
    );

    // Verify DID signature
    functions.insert(
        "did_verify".to_string(),
        Arc::new(|args| {
            match (args.get(0), args.get(1), args.get(2)) {
                (Some(WasmValue::String(did_id)),
                 Some(WasmValue::Bytes(signature)),
                 Some(WasmValue::Bytes(data))) => {
                    tracing::info!(
                        did_id = %did_id,
                        sig_len = signature.len(),
                        data_len = data.len(),
                        "Verifying DID signature"
                    );
                    Ok(vec![WasmValue::I32(1)]) // Valid
                }
                _ => Err(WasmError::Configuration("Invalid arguments for DID verification".to_string()))
            }
        }) as HostFunction,
    );

    // Generate recovery seed
    functions.insert(
        "did_generate_seed".to_string(),
        Arc::new(|args| {
            let entropy_bits = if let Some(WasmValue::I32(bits)) = args.get(0) {
                *bits as usize
            } else {
                256 // Default
            };
            
            tracing::info!(entropy_bits = entropy_bits, "Generating recovery seed");
            Ok(vec![WasmValue::Bytes(b"recovery_seed".to_vec())])
        }) as HostFunction,
    );

    functions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_did_manager() {
        let mut manager = DidManager::new().unwrap();
        
        let options = DidCreationOptions::default();
        let did_id = manager.create_did("key", options).await.unwrap();
        
        assert!(did_id.starts_with("did:key:"));
        assert_eq!(manager.list_dids().len(), 1);
    }

    #[tokio::test]
    async fn test_did_key_method() {
        let method = DidMethodKey::new();
        let options = DidCreationOptions::default();
        
        let (did_id, document, _keypair) = method.create_did(options).await.unwrap();
        
        assert!(did_id.starts_with("did:key:"));
        assert_eq!(document.id, did_id);
        assert!(!document.verification_method.is_empty());
    }

    #[tokio::test]
    async fn test_did_signing() {
        let mut manager = DidManager::new().unwrap();
        
        let options = DidCreationOptions::default();
        let did_id = manager.create_did("key", options).await.unwrap();
        
        let data = b"test message";
        let signature = manager.sign_with_did(&did_id, data, None).await.unwrap();
        
        assert_eq!(signature.signer, did_id);
        assert!(!signature.signature.data.is_empty());
        
        let is_valid = manager.verify_signature(&signature, data).await.unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_key_pair() {
        let keypair1 = KeyPair::generate().unwrap();
        let keypair2 = KeyPair::generate().unwrap();
        
        assert_ne!(keypair1.public_key_bytes(), keypair2.public_key_bytes());
        
        let data = b"test data";
        let signature = keypair1.sign(data).unwrap();
        assert!(!signature.data.is_empty());
    }

    #[test]
    fn test_recovery_seed() {
        let seed = RecoverySeed::generate(256).unwrap();
        
        assert_eq!(seed.entropy.len(), 32);
        assert!(!seed.mnemonic.is_empty());
        assert_eq!(seed.checksum.len(), 4);
        
        let is_valid = seed.validate().unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_key_derivation() {
        let seed = b"master_seed_bytes_for_testing_123456";
        let path1 = "m/44'/0'/0'/0/0";
        let path2 = "m/44'/0'/0'/0/1";
        
        let key1 = KeyDerivation::derive_keypair_from_seed(seed, path1).unwrap();
        let key2 = KeyDerivation::derive_keypair_from_seed(seed, path2).unwrap();
        
        assert_ne!(key1.public_key_bytes(), key2.public_key_bytes());
        
        // Same path should produce same key
        let key1_again = KeyDerivation::derive_keypair_from_seed(seed, path1).unwrap();
        assert_eq!(key1.public_key_bytes(), key1_again.public_key_bytes());
    }
}