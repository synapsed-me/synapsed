//! Core identity implementation

use crate::{Result, crypto::{IdentityKeyPair, IdentityKeyManager, KeyType}};
use base64::{engine::general_purpose::STANDARD, Engine as _};

// String, Vec, and Box are available in std prelude

/// Trait for identity implementations
pub trait Identity: Send + Sync {
    /// Get the identity ID
    fn id(&self) -> &str;
    
    /// Get the public key
    fn public_key(&self) -> &[u8];
    
    /// Sign data
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>>;
    
    /// Verify a signature
    fn verify(&self, data: &[u8], signature: &[u8]) -> Result<bool>;
}

/// Basic identity implementation
#[derive(Clone)]
pub struct BasicIdentity {
    /// Identity ID
    id: String,
    /// Key pair
    keypair: IdentityKeyPair,
}

impl BasicIdentity {
    /// Create a new identity
    pub fn new(id: String, key_type: KeyType) -> Result<Self> {
        let keypair = IdentityKeyManager::generate_keypair(key_type)?;
        Ok(Self { id, keypair })
    }
    
    /// Create identity with existing keypair
    pub fn with_keypair(id: String, keypair: IdentityKeyPair) -> Self {
        Self { id, keypair }
    }
}

impl Identity for BasicIdentity {
    fn id(&self) -> &str {
        &self.id
    }
    
    fn public_key(&self) -> &[u8] {
        &self.keypair.public_key
    }
    
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        IdentityKeyManager::sign(&self.keypair.private_key, data, self.keypair.key_type)
    }
    
    fn verify(&self, data: &[u8], signature: &[u8]) -> Result<bool> {
        IdentityKeyManager::verify(&self.keypair.public_key, data, signature, self.keypair.key_type)
    }
}

/// Decentralized Identifier (DID) implementation
#[derive(Clone)]
pub struct DecentralizedIdentity {
    /// DID (e.g., "did:synapsed:12345")
    did: String,
    /// Identity key pair
    keypair: IdentityKeyPair,
    /// DID document
    document: DidDocument,
}

/// DID Document
#[derive(Clone, Debug)]
pub struct DidDocument {
    /// DID
    pub id: String,
    /// Public keys
    pub public_keys: Vec<DidPublicKey>,
    /// Authentication methods
    pub authentication: Vec<String>,
    /// Service endpoints
    pub services: Vec<DidService>,
}

/// DID public key entry
#[derive(Clone, Debug)]
pub struct DidPublicKey {
    /// Key ID
    pub id: String,
    /// Key type
    pub key_type: String,
    /// Controller DID
    pub controller: String,
    /// Public key bytes
    pub public_key: Vec<u8>,
}

/// DID service endpoint
#[derive(Clone, Debug)]
pub struct DidService {
    /// Service ID
    pub id: String,
    /// Service type
    pub service_type: String,
    /// Service endpoint URL
    pub endpoint: String,
}

impl DecentralizedIdentity {
    /// Create a new DID
    pub fn new(namespace: &str) -> Result<Self> {
        let keypair = IdentityKeyManager::generate_keypair(KeyType::PostQuantumSign)?;
        
        // Generate DID from public key
        use sha3::{Sha3_256, Digest};
        let mut hasher = Sha3_256::new();
        hasher.update(&keypair.public_key);
        let hash = hasher.finalize();
        let did_suffix = STANDARD.encode(&hash[..16]);
        let did = format!("did:{}:{}", namespace, did_suffix);
        
        // Create DID document
        let document = DidDocument {
            id: did.clone(),
            public_keys: vec![DidPublicKey {
                id: format!("{}#key-1", did),
                key_type: "PostQuantumSign".to_string(),
                controller: did.clone(),
                public_key: keypair.public_key.clone(),
            }],
            authentication: vec![format!("{}#key-1", did)],
            services: vec![],
        };
        
        Ok(Self {
            did,
            keypair,
            document,
        })
    }
    
    /// Get DID
    pub fn did(&self) -> &str {
        &self.did
    }
    
    /// Get DID document
    pub fn document(&self) -> &DidDocument {
        &self.document
    }
    
    /// Add a service endpoint
    pub fn add_service(&mut self, service: DidService) {
        self.document.services.push(service);
    }
}

impl Identity for DecentralizedIdentity {
    fn id(&self) -> &str {
        &self.did
    }
    
    fn public_key(&self) -> &[u8] {
        &self.keypair.public_key
    }
    
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        IdentityKeyManager::sign(&self.keypair.private_key, data, self.keypair.key_type)
    }
    
    fn verify(&self, data: &[u8], signature: &[u8]) -> Result<bool> {
        IdentityKeyManager::verify(&self.keypair.public_key, data, signature, self.keypair.key_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_identity() {
        let identity = BasicIdentity::new("user123".to_string(), KeyType::Ed25519).unwrap();
        assert_eq!(identity.id(), "user123");
        assert!(!identity.public_key().is_empty());
        
        // Test signing and verification
        let data = b"test message";
        let signature = identity.sign(data).unwrap();
        assert!(identity.verify(data, &signature).unwrap());
    }
    
    #[test]
    fn test_decentralized_identity() {
        let mut did_identity = DecentralizedIdentity::new("synapsed").unwrap();
        assert!(did_identity.did().starts_with("did:synapsed:"));
        
        // Add service
        did_identity.add_service(DidService {
            id: format!("{}#inbox", did_identity.did()),
            service_type: "MessagingService".to_string(),
            endpoint: "https://example.com/inbox".to_string(),
        });
        
        assert_eq!(did_identity.document().services.len(), 1);
    }
}