//! DID Document implementation according to W3C DID Core v1.0

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::{Result, Error};
use super::Did;

/// W3C DID Document according to DID Core v1.0
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidDocument {
    /// The DID that this document describes
    #[serde(rename = "id")]
    pub id: Did,
    
    /// Context for JSON-LD processing
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    
    /// Also known as (alternative identifiers)
    #[serde(rename = "alsoKnownAs", skip_serializing_if = "Vec::is_empty")]
    pub also_known_as: Vec<String>,
    
    /// Controllers of this DID
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub controller: Vec<Did>,
    
    /// Verification methods
    #[serde(rename = "verificationMethod", skip_serializing_if = "Vec::is_empty")]
    pub verification_method: Vec<VerificationMethod>,
    
    /// Authentication methods (references to verification methods)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub authentication: Vec<VerificationRelationship>,
    
    /// Assertion methods for creating verifiable credentials
    #[serde(rename = "assertionMethod", skip_serializing_if = "Vec::is_empty")]
    pub assertion_method: Vec<VerificationRelationship>,
    
    /// Key agreement methods for key exchange
    #[serde(rename = "keyAgreement", skip_serializing_if = "Vec::is_empty")]
    pub key_agreement: Vec<VerificationRelationship>,
    
    /// Capability invocation methods
    #[serde(rename = "capabilityInvocation", skip_serializing_if = "Vec::is_empty")]
    pub capability_invocation: Vec<VerificationRelationship>,
    
    /// Capability delegation methods
    #[serde(rename = "capabilityDelegation", skip_serializing_if = "Vec::is_empty")]
    pub capability_delegation: Vec<VerificationRelationship>,
    
    /// Service endpoints
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub service: Vec<Service>,
    
    /// Additional properties
    #[serde(flatten)]
    pub additional_properties: HashMap<String, serde_json::Value>,
}

impl DidDocument {
    /// Create a new DID document
    pub fn new(id: Did) -> Self {
        Self {
            id,
            context: vec![
                "https://www.w3.org/ns/did/v1".to_string(),
            ],
            also_known_as: Vec::new(),
            controller: Vec::new(),
            verification_method: Vec::new(),
            authentication: Vec::new(),
            assertion_method: Vec::new(),
            key_agreement: Vec::new(),
            capability_invocation: Vec::new(),
            capability_delegation: Vec::new(),
            service: Vec::new(),
            additional_properties: HashMap::new(),
        }
    }

    /// Add a verification method
    pub fn add_verification_method(&mut self, method: VerificationMethod) {
        self.verification_method.push(method);
    }

    /// Add authentication method by reference
    pub fn add_authentication_reference(&mut self, reference: String) {
        self.authentication.push(VerificationRelationship::Reference(reference));
    }

    /// Add authentication method by embedding
    pub fn add_authentication_embedded(&mut self, method: VerificationMethod) {
        self.authentication.push(VerificationRelationship::Embedded(method));
    }

    /// Add key agreement method
    pub fn add_key_agreement(&mut self, relationship: VerificationRelationship) {
        self.key_agreement.push(relationship);
    }

    /// Add service endpoint
    pub fn add_service(&mut self, service: Service) {
        self.service.push(service);
    }

    /// Find verification method by ID
    pub fn find_verification_method(&self, id: &str) -> Option<&VerificationMethod> {
        self.verification_method.iter().find(|vm| vm.id == id)
    }

    /// Get all verification methods including embedded ones
    pub fn all_verification_methods(&self) -> Vec<&VerificationMethod> {
        let mut methods = Vec::new();
        
        // Add explicit verification methods
        for method in &self.verification_method {
            methods.push(method);
        }
        
        // Add embedded methods from relationships
        for auth in &self.authentication {
            if let VerificationRelationship::Embedded(method) = auth {
                methods.push(method);
            }
        }
        
        for assertion in &self.assertion_method {
            if let VerificationRelationship::Embedded(method) = assertion {
                methods.push(method);
            }
        }
        
        for key_agreement in &self.key_agreement {
            if let VerificationRelationship::Embedded(method) = key_agreement {
                methods.push(method);
            }
        }
        
        methods
    }

    /// Validate the document structure
    pub fn validate(&self) -> Result<()> {
        // Check that all references point to valid verification methods
        let method_ids: std::collections::HashSet<String> = 
            self.verification_method.iter().map(|vm| vm.id.clone()).collect();

        for auth in &self.authentication {
            if let VerificationRelationship::Reference(ref id) = auth {
                if !method_ids.contains(id) && !id.starts_with(&self.id.to_string()) {
                    return Err(Error::DidDocumentError(
                        format!("Authentication reference '{}' not found in verification methods", id)
                    ));
                }
            }
        }

        // Validate service endpoints
        for service in &self.service {
            service.validate()?;
        }

        Ok(())
    }
}

/// Verification method for cryptographic operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMethod {
    /// Verification method ID
    pub id: String,
    
    /// Type of verification method
    #[serde(rename = "type")]
    pub verification_type: String,
    
    /// Controller of this verification method
    pub controller: Did,
    
    /// Public key material (depends on type)
    #[serde(flatten)]
    pub public_key: PublicKeyMaterial,
    
    /// Additional properties
    #[serde(flatten)]
    pub additional_properties: HashMap<String, serde_json::Value>,
}

impl VerificationMethod {
    /// Create a new verification method
    pub fn new(
        id: String,
        verification_type: String,
        controller: Did,
        public_key: PublicKeyMaterial,
    ) -> Self {
        Self {
            id,
            verification_type,
            controller,
            public_key,
            additional_properties: HashMap::new(),
        }
    }
}

/// Public key material for verification methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PublicKeyMaterial {
    /// Base58-encoded public key
    #[serde(rename_all = "camelCase")]
    PublicKeyBase58 {
        public_key_base58: String,
    },
    
    /// Multibase-encoded public key
    #[serde(rename_all = "camelCase")]
    PublicKeyMultibase {
        public_key_multibase: String,
    },
    
    /// JWK format public key
    #[serde(rename_all = "camelCase")]
    PublicKeyJwk {
        public_key_jwk: serde_json::Value,
    },
    
    /// PEM format public key
    #[serde(rename_all = "camelCase")]
    PublicKeyPem {
        public_key_pem: String,
    },
    
    /// Hex-encoded public key
    #[serde(rename_all = "camelCase")]
    PublicKeyHex {
        public_key_hex: String,
    },
}

/// Verification relationship (reference or embedded)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VerificationRelationship {
    /// Reference to a verification method by ID
    Reference(String),
    /// Embedded verification method
    Embedded(VerificationMethod),
}

/// Service endpoint in DID document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    /// Service ID
    pub id: String,
    
    /// Service type
    #[serde(rename = "type")]
    pub service_type: String,
    
    /// Service endpoint(s)
    #[serde(rename = "serviceEndpoint")]
    pub service_endpoint: ServiceEndpoint,
    
    /// Additional properties
    #[serde(flatten)]
    pub additional_properties: HashMap<String, serde_json::Value>,
}

impl Service {
    /// Create a new service
    pub fn new(id: String, service_type: String, service_endpoint: ServiceEndpoint) -> Self {
        Self {
            id,
            service_type,
            service_endpoint,
            additional_properties: HashMap::new(),
        }
    }

    /// Validate service endpoint
    pub fn validate(&self) -> Result<()> {
        match &self.service_endpoint {
            ServiceEndpoint::String(url) => {
                if url.is_empty() {
                    return Err(Error::DidDocumentError("Service endpoint URL cannot be empty".into()));
                }
            }
            ServiceEndpoint::Array(urls) => {
                if urls.is_empty() {
                    return Err(Error::DidDocumentError("Service endpoint array cannot be empty".into()));
                }
                for url in urls {
                    if url.is_empty() {
                        return Err(Error::DidDocumentError("Service endpoint URL cannot be empty".into()));
                    }
                }
            }
            ServiceEndpoint::Object(_) => {
                // Complex validation would depend on service type specification
            }
        }
        Ok(())
    }
}

/// Service endpoint can be a string, array of strings, or object
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ServiceEndpoint {
    String(String),
    Array(Vec<String>),
    Object(HashMap<String, serde_json::Value>),
}

/// DID document metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidMetadata {
    /// Created timestamp
    pub created: Option<DateTime<Utc>>,
    
    /// Updated timestamp
    pub updated: Option<DateTime<Utc>>,
    
    /// Deactivated status
    pub deactivated: Option<bool>,
    
    /// Version ID
    #[serde(rename = "versionId")]
    pub version_id: Option<String>,
    
    /// Next update
    #[serde(rename = "nextUpdate")]
    pub next_update: Option<DateTime<Utc>>,
    
    /// Next version ID
    #[serde(rename = "nextVersionId")]
    pub next_version_id: Option<String>,
    
    /// Equivalent IDs
    #[serde(rename = "equivalentId")]
    pub equivalent_id: Vec<String>,
    
    /// Canonical ID
    #[serde(rename = "canonicalId")]
    pub canonical_id: Option<String>,
    
    /// Additional metadata
    #[serde(flatten)]
    pub additional_properties: HashMap<String, serde_json::Value>,
}

impl Default for DidMetadata {
    fn default() -> Self {
        Self {
            created: Some(Utc::now()),
            updated: Some(Utc::now()),
            deactivated: Some(false),
            version_id: None,
            next_update: None,
            next_version_id: None,
            equivalent_id: Vec::new(),
            canonical_id: None,
            additional_properties: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_did_document_creation() {
        let did = Did::new("key", "z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK");
        let doc = DidDocument::new(did.clone());
        
        assert_eq!(doc.id, did);
        assert_eq!(doc.context[0], "https://www.w3.org/ns/did/v1");
        assert!(doc.verification_method.is_empty());
    }

    #[test]
    fn test_verification_method() {
        let did = Did::new("key", "test");
        let public_key = PublicKeyMaterial::PublicKeyMultibase {
            public_key_multibase: "z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string(),
        };
        
        let vm = VerificationMethod::new(
            format!("{}#key-1", did.to_string()),
            "Ed25519VerificationKey2020".to_string(),
            did,
            public_key,
        );
        
        assert_eq!(vm.verification_type, "Ed25519VerificationKey2020");
    }

    #[test]
    fn test_service_validation() {
        let service = Service::new(
            "did:example:123#messaging".to_string(),
            "MessagingService".to_string(),
            ServiceEndpoint::String("https://example.com/messaging".to_string()),
        );
        
        assert!(service.validate().is_ok());
        
        let invalid_service = Service::new(
            "did:example:123#messaging".to_string(),
            "MessagingService".to_string(),
            ServiceEndpoint::String("".to_string()),
        );
        
        assert!(invalid_service.validate().is_err());
    }
}