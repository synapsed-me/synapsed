//! DID method implementations
//! 
//! This module implements various DID methods including:
//! - did:key (RFC draft)
//! - did:web (W3C DID Method)

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::{Result, Error};
use super::{Did, DidDocument, VerificationMethod, PublicKeyMaterial, VerificationRelationship};

/// Trait for DID method implementations
pub trait DidMethod {
    /// Method name (e.g., "key", "web")
    fn method_name(&self) -> &str;
    
    /// Generate a new DID for this method
    fn generate(&mut self) -> Result<Did>;
    
    /// Create a DID document from a DID
    fn create_document(&self, did: &Did) -> Result<DidDocument>;
    
    /// Validate a DID for this method
    fn validate(&self, did: &Did) -> Result<()>;
    
    /// Resolve a DID to its document (if supported)
    fn resolve(&self, did: &Did) -> Result<Option<DidDocument>> {
        if self.validate(did).is_ok() {
            Ok(Some(self.create_document(did)?))
        } else {
            Ok(None)
        }
    }
}

/// did:key method implementation
/// 
/// The did:key method is a DID method that encodes a cryptographic public key
/// directly in the identifier. It's particularly useful for offline scenarios
/// and doesn't require a registry or ledger.
pub struct DidKey {
    /// Supported key types
    supported_key_types: Vec<KeyType>,
}

impl DidKey {
    /// Create a new did:key method instance
    pub fn new() -> Self {
        Self {
            supported_key_types: vec![
                KeyType::Ed25519,
                KeyType::X25519,
                KeyType::Secp256k1,
                KeyType::P256,
                KeyType::P384,
                KeyType::P521,
                KeyType::Rsa,
                KeyType::PostQuantumSign,
                KeyType::PostQuantumKem,
            ],
        }
    }

    /// Generate a did:key from raw public key bytes
    pub fn from_public_key(&self, key_type: KeyType, public_key: &[u8]) -> Result<Did> {
        let multicodec_key = self.encode_multicodec_key(key_type, public_key)?;
        let multibase_key = multibase::encode(multibase::Base::Base58Btc, &multicodec_key);
        
        Ok(Did::new("key", &multibase_key))
    }

    /// Extract public key from a did:key
    pub fn extract_public_key(&self, did: &Did) -> Result<(KeyType, Vec<u8>)> {
        if did.method != "key" {
            return Err(Error::DidMethodError("Not a did:key".into()));
        }

        let (_base, decoded) = multibase::decode(&did.method_specific_id)
            .map_err(|e| Error::DidMethodError(format!("Invalid multibase encoding: {}", e)))?;

        self.decode_multicodec_key(&decoded)
    }

    /// Encode public key with multicodec prefix
    fn encode_multicodec_key(&self, key_type: KeyType, public_key: &[u8]) -> Result<Vec<u8>> {
        let mut result = Vec::new();
        
        let multicodec_prefix = match key_type {
            KeyType::Ed25519 => 0xed, // ed25519-pub
            KeyType::X25519 => 0xec,  // x25519-pub
            KeyType::Secp256k1 => 0xe7, // secp256k1-pub
            KeyType::P256 => 0x1200,  // p256-pub
            KeyType::P384 => 0x1201,  // p384-pub
            KeyType::P521 => 0x1202,  // p521-pub
            KeyType::Rsa => 0x1205,   // rsa-pub
            KeyType::PostQuantumSign => 0x1300, // Custom for post-quantum signatures
            KeyType::PostQuantumKem => 0x1301,  // Custom for post-quantum KEM
        };

        // Encode varint
        if multicodec_prefix < 0x80 {
            result.push(multicodec_prefix as u8);
        } else if multicodec_prefix < 0x4000 {
            result.push(((multicodec_prefix >> 8) | 0x80) as u8);
            result.push((multicodec_prefix & 0xFF) as u8);
        } else {
            result.push(((multicodec_prefix >> 8) | 0x80) as u8);
            result.push(((multicodec_prefix >> 8) | 0x80) as u8);
            result.push((multicodec_prefix & 0xFF) as u8);
        }

        result.extend_from_slice(public_key);
        Ok(result)
    }

    /// Decode multicodec key to get key type and public key
    fn decode_multicodec_key(&self, data: &[u8]) -> Result<(KeyType, Vec<u8>)> {
        if data.is_empty() {
            return Err(Error::DidMethodError("Empty key data".into()));
        }

        let (multicodec_prefix, key_data) = if data[0] < 0x80 {
            (data[0] as u16, &data[1..])
        } else if data.len() >= 2 {
            let prefix = ((data[0] & 0x7F) as u16) << 8 | data[1] as u16;
            (prefix, &data[2..])
        } else {
            return Err(Error::DidMethodError("Invalid multicodec encoding".into()));
        };

        let key_type = match multicodec_prefix {
            0xed => KeyType::Ed25519,
            0xec => KeyType::X25519,
            0xe7 => KeyType::Secp256k1,
            0x1200 => KeyType::P256,
            0x1201 => KeyType::P384,
            0x1202 => KeyType::P521,
            0x1205 => KeyType::Rsa,
            0x1300 => KeyType::PostQuantumSign,
            0x1301 => KeyType::PostQuantumKem,
            _ => return Err(Error::DidMethodError(format!("Unsupported key type: 0x{:x}", multicodec_prefix))),
        };

        Ok((key_type, key_data.to_vec()))
    }
}

impl Default for DidKey {
    fn default() -> Self {
        Self::new()
    }
}

impl DidMethod for DidKey {
    fn method_name(&self) -> &str {
        "key"
    }

    fn generate(&mut self) -> Result<Did> {
        // Generate Ed25519 key by default
        use crate::crypto::{IdentityKeyManager, KeyType as CryptoKeyType};
        let keypair = IdentityKeyManager::generate_keypair(CryptoKeyType::Ed25519)?;
        self.from_public_key(KeyType::Ed25519, &keypair.public_key)
    }

    fn create_document(&self, did: &Did) -> Result<DidDocument> {
        let (key_type, public_key) = self.extract_public_key(did)?;
        
        let mut doc = DidDocument::new(did.clone());
        
        // Create verification method
        let verification_method_id = format!("{}#{}", did.to_string(), did.method_specific_id);
        let verification_type = match key_type {
            KeyType::Ed25519 => "Ed25519VerificationKey2020",
            KeyType::X25519 => "X25519KeyAgreementKey2020",
            KeyType::Secp256k1 => "EcdsaSecp256k1VerificationKey2019",
            KeyType::P256 => "EcdsaSecp256r1VerificationKey2019",
            KeyType::P384 => "EcdsaSecp384r1VerificationKey2019",
            KeyType::P521 => "EcdsaSecp521r1VerificationKey2019",
            KeyType::Rsa => "RsaVerificationKey2018",
            KeyType::PostQuantumSign => "PostQuantumSignature2024",
            KeyType::PostQuantumKem => "PostQuantumKeyAgreement2024",
        };

        let verification_method = VerificationMethod::new(
            verification_method_id.clone(),
            verification_type.to_string(),
            did.clone(),
            PublicKeyMaterial::PublicKeyMultibase {
                public_key_multibase: did.method_specific_id.clone(),
            },
        );

        doc.add_verification_method(verification_method);

        // Add appropriate verification relationships based on key type
        match key_type {
            KeyType::Ed25519 | KeyType::Secp256k1 | KeyType::P256 | KeyType::P384 | KeyType::P521 | KeyType::Rsa | KeyType::PostQuantumSign => {
                doc.add_authentication_reference(verification_method_id.clone());
                doc.capability_invocation.push(VerificationRelationship::Reference(verification_method_id.clone()));
                doc.assertion_method.push(VerificationRelationship::Reference(verification_method_id));
            }
            KeyType::X25519 | KeyType::PostQuantumKem => {
                doc.key_agreement.push(VerificationRelationship::Reference(verification_method_id));
            }
        }

        Ok(doc)
    }

    fn validate(&self, did: &Did) -> Result<()> {
        if did.method != "key" {
            return Err(Error::DidMethodError("Not a did:key".into()));
        }

        // Validate that we can decode the key
        self.extract_public_key(did)?;
        Ok(())
    }
}

/// did:web method implementation
/// 
/// The did:web method uses web domains and DNS to host DID documents.
/// It provides a bridge between existing web infrastructure and DIDs.
pub struct DidWeb {
    /// HTTP client for resolution
    #[cfg(feature = "http-client")]
    client: reqwest::Client,
}

impl DidWeb {
    /// Create a new did:web method instance
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "http-client")]
            client: reqwest::Client::new(),
        }
    }

    /// Create a did:web from domain and optional path
    pub fn from_domain(&self, domain: &str, path: Option<&str>) -> Result<Did> {
        let method_specific_id = if let Some(path) = path {
            format!("{}:{}", domain, path.replace('/', ":"))
        } else {
            domain.to_string()
        };

        Ok(Did::new("web", &method_specific_id))
    }

    /// Convert did:web to HTTPS URL for document resolution
    pub fn to_https_url(&self, did: &Did) -> Result<String> {
        if did.method != "web" {
            return Err(Error::DidMethodError("Not a did:web".into()));
        }

        let parts: Vec<&str> = did.method_specific_id.split(':').collect();
        if parts.is_empty() {
            return Err(Error::DidMethodError("Invalid did:web format".into()));
        }

        let domain = parts[0];
        let path = if parts.len() > 1 {
            format!("/{}/did.json", parts[1..].join("/"))
        } else {
            "/.well-known/did.json".to_string()
        };

        Ok(format!("https://{}{}", domain, path))
    }

    /// Resolve did:web over HTTPS
    #[cfg(feature = "http-client")]
    pub async fn resolve_https(&self, did: &Did) -> Result<DidDocument> {
        let url = self.to_https_url(did)?;
        
        let response = self.client
            .get(&url)
            .header("Accept", "application/did+json, application/json")
            .send()
            .await
            .map_err(|e| Error::DidResolutionError(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::DidResolutionError(
                format!("HTTP {} from {}", response.status(), url)
            ));
        }

        let document: DidDocument = response
            .json()
            .await
            .map_err(|e| Error::DidResolutionError(format!("Failed to parse JSON: {}", e)))?;

        // Validate that the document ID matches the DID
        if document.id != *did {
            return Err(Error::DidResolutionError(
                "Document ID does not match requested DID".into()
            ));
        }

        Ok(document)
    }
}

impl Default for DidWeb {
    fn default() -> Self {
        Self::new()
    }
}

impl DidMethod for DidWeb {
    fn method_name(&self) -> &str {
        "web"
    }

    fn generate(&mut self) -> Result<Did> {
        Err(Error::DidMethodError(
            "did:web cannot be generated, it must be created from a domain".into()
        ))
    }

    fn create_document(&self, did: &Did) -> Result<DidDocument> {
        // did:web documents are hosted externally, so we return a minimal document
        let doc = DidDocument::new(did.clone());
        Ok(doc)
    }

    fn validate(&self, did: &Did) -> Result<()> {
        if did.method != "web" {
            return Err(Error::DidMethodError("Not a did:web".into()));
        }

        // Validate domain format
        let parts: Vec<&str> = did.method_specific_id.split(':').collect();
        if parts.is_empty() {
            return Err(Error::DidMethodError("Invalid did:web format".into()));
        }

        let domain = parts[0];
        if domain.is_empty() || domain.contains("..") || domain.starts_with('.') || domain.ends_with('.') {
            return Err(Error::DidMethodError("Invalid domain in did:web".into()));
        }

        Ok(())
    }

    #[cfg(feature = "http-client")]
    fn resolve(&self, did: &Did) -> Result<Option<DidDocument>> {
        // For did:web, we need async resolution, so this returns None
        // Use resolve_https for actual resolution
        Ok(None)
    }
}

/// Supported key types for DID methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyType {
    Ed25519,
    X25519,
    Secp256k1,
    P256,
    P384,
    P521,
    Rsa,
    PostQuantumSign,
    PostQuantumKem,
}

impl KeyType {
    /// Get the verification method type string for this key type
    pub fn verification_method_type(&self) -> &'static str {
        match self {
            KeyType::Ed25519 => "Ed25519VerificationKey2020",
            KeyType::X25519 => "X25519KeyAgreementKey2020",
            KeyType::Secp256k1 => "EcdsaSecp256k1VerificationKey2019",
            KeyType::P256 => "EcdsaSecp256r1VerificationKey2019",
            KeyType::P384 => "EcdsaSecp384r1VerificationKey2019",
            KeyType::P521 => "EcdsaSecp521r1VerificationKey2019",
            KeyType::Rsa => "RsaVerificationKey2018",
            KeyType::PostQuantumSign => "PostQuantumSignature2024",
            KeyType::PostQuantumKem => "PostQuantumKeyAgreement2024",
        }
    }

    /// Check if this key type is suitable for authentication
    pub fn supports_authentication(&self) -> bool {
        matches!(self, KeyType::Ed25519 | KeyType::Secp256k1 | KeyType::P256 | KeyType::P384 | KeyType::P521 | KeyType::Rsa | KeyType::PostQuantumSign)
    }

    /// Check if this key type is suitable for key agreement
    pub fn supports_key_agreement(&self) -> bool {
        matches!(self, KeyType::X25519 | KeyType::PostQuantumKem)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_did_key_generation() {
        let mut did_key = DidKey::new();
        let did = did_key.generate().unwrap();
        
        assert_eq!(did.method, "key");
        assert!(did.method_specific_id.starts_with('z'));
        
        // Should be able to create document
        let doc = did_key.create_document(&did).unwrap();
        assert_eq!(doc.id, did);
        assert!(!doc.verification_method.is_empty());
    }

    #[test]
    fn test_did_key_public_key_extraction() {
        let did_key = DidKey::new();
        let public_key = b"test_public_key_32_bytes_long___";
        let did = did_key.from_public_key(KeyType::Ed25519, public_key).unwrap();
        
        let (extracted_type, extracted_key) = did_key.extract_public_key(&did).unwrap();
        assert_eq!(extracted_type, KeyType::Ed25519);
        assert_eq!(extracted_key, public_key);
    }

    #[test]
    fn test_did_web_validation() {
        let did_web = DidWeb::new();
        
        let valid_did = Did::new("web", "example.com");
        assert!(did_web.validate(&valid_did).is_ok());
        
        let valid_did_with_path = Did::new("web", "example.com:user:alice");
        assert!(did_web.validate(&valid_did_with_path).is_ok());
        
        let invalid_did = Did::new("web", "");
        assert!(did_web.validate(&invalid_did).is_err());
    }

    #[test]
    fn test_did_web_url_conversion() {
        let did_web = DidWeb::new();
        
        let did = Did::new("web", "example.com");
        let url = did_web.to_https_url(&did).unwrap();
        assert_eq!(url, "https://example.com/.well-known/did.json");
        
        let did_with_path = Did::new("web", "example.com:user:alice");
        let url_with_path = did_web.to_https_url(&did_with_path).unwrap();
        assert_eq!(url_with_path, "https://example.com/user/alice/did.json");
    }

    #[test]
    fn test_key_type_properties() {
        assert!(KeyType::Ed25519.supports_authentication());
        assert!(!KeyType::Ed25519.supports_key_agreement());
        
        assert!(!KeyType::X25519.supports_authentication());
        assert!(KeyType::X25519.supports_key_agreement());
        
        assert!(KeyType::PostQuantumSign.supports_authentication());
        assert!(KeyType::PostQuantumKem.supports_key_agreement());
    }
}