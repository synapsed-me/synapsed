//! Decentralized Identifier (DID) implementation
//! 
//! This module provides W3C DID Core v1.0 compliant implementation with support for:
//! - did:key method (RFC draft)
//! - did:web method 
//! - Key rotation and lifecycle management
//! - Local-first storage with encryption
//! - Zero-knowledge proof integration

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::{Result, Error};

pub mod document;
pub mod methods;
pub mod resolver;
pub mod key_management;
pub mod zkp;
pub mod storage;
pub mod zkp_subscription;
pub mod recovery_system;

pub use document::{DidDocument, VerificationMethod, Service, DidMetadata, PublicKeyMaterial, VerificationRelationship};
pub use methods::{DidKey, DidWeb, DidMethod};
pub use resolver::{DidResolver, ResolutionResult};
pub use key_management::{KeyRotationManager, KeyHierarchy, RecoveryMechanism, EncryptedKeyMaterial};
pub use zkp::{ZkpVerifier, AnonymousCredential, ProofRequest};
pub use storage::{LocalFirstStorage, SyncManager, ContactVault};
pub use zkp_subscription::{
    AnonymousSubscription, SubscriptionTier, SubscriptionProof, VerificationResult,
    PaymentStatus, Amount, SubscriptionPrivateData, ProofCommitments,
    generate_subscription_proof, verify_subscription_proof
};
pub use recovery_system::{RecoveryMethod, RecoveryData, SecretShare, generate_recovery_info};

/// DID URI structure according to W3C DID Core v1.0
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Did {
    /// DID scheme (always "did")
    pub scheme: String,
    /// DID method (e.g., "key", "web", "synapsed")
    pub method: String,
    /// Method-specific identifier
    pub method_specific_id: String,
    /// Optional path component
    pub path: Option<String>,
    /// Optional query component
    pub query: Option<String>,
    /// Optional fragment component
    pub fragment: Option<String>,
}

impl Did {
    /// Create a new DID
    pub fn new(method: &str, method_specific_id: &str) -> Self {
        Self {
            scheme: "did".to_string(),
            method: method.to_string(),
            method_specific_id: method_specific_id.to_string(),
            path: None,
            query: None,
            fragment: None,
        }
    }

    /// Parse a DID string into components
    pub fn parse(did_string: &str) -> Result<Self> {
        if !did_string.starts_with("did:") {
            return Err(Error::DidParsingError("DID must start with 'did:'".into()));
        }

        let parts: Vec<&str> = did_string.splitn(3, ':').collect();
        if parts.len() < 3 {
            return Err(Error::DidParsingError("Invalid DID format".into()));
        }

        let method = parts[1].to_string();
        let remaining = parts[2];

        // Parse path, query, and fragment
        let (method_specific_id, path, query, fragment) = Self::parse_components(remaining)?;

        Ok(Self {
            scheme: "did".to_string(),
            method,
            method_specific_id,
            path,
            query,
            fragment,
        })
    }

    /// Parse components after method:
    fn parse_components(remaining: &str) -> Result<(String, Option<String>, Option<String>, Option<String>)> {
        let mut parts = remaining.split('#');
        let before_fragment = parts.next().unwrap_or("");
        let fragment = parts.next().map(|s| s.to_string());

        let mut parts = before_fragment.split('?');
        let before_query = parts.next().unwrap_or("");
        let query = parts.next().map(|s| s.to_string());

        let mut parts = before_query.split('/');
        let method_specific_id = parts.next().unwrap_or("").to_string();
        let path = if let Some(path_part) = parts.next() {
            let mut path_parts = vec![path_part];
            path_parts.extend(parts);
            Some(format!("/{}", path_parts.join("/")))
        } else {
            None
        };

        Ok((method_specific_id, path, query, fragment))
    }

    /// Convert DID to string representation
    pub fn to_string(&self) -> String {
        let mut result = format!("{}:{}:{}", self.scheme, self.method, self.method_specific_id);
        
        if let Some(ref path) = self.path {
            result.push_str(path);
        }
        
        if let Some(ref query) = self.query {
            result.push('?');
            result.push_str(query);
        }
        
        if let Some(ref fragment) = self.fragment {
            result.push('#');
            result.push_str(fragment);
        }
        
        result
    }

    /// Get DID without fragment (for document ID)
    pub fn without_fragment(&self) -> Self {
        Self {
            scheme: self.scheme.clone(),
            method: self.method.clone(),
            method_specific_id: self.method_specific_id.clone(),
            path: self.path.clone(),
            query: self.query.clone(),
            fragment: None,
        }
    }
}

impl std::fmt::Display for Did {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl std::str::FromStr for Did {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

/// DID URL parameters for resolution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DidResolutionOptions {
    /// Accept header for content negotiation
    pub accept: Option<String>,
    /// Service ID for service endpoint resolution
    pub service: Option<String>,
    /// Relative reference for service endpoint resolution
    pub relative_ref: Option<String>,
    /// Transform keys for key agreement
    pub transform_keys: Option<String>,
    /// Version ID for versioned documents
    pub version_id: Option<String>,
    /// Version time for time-based resolution
    pub version_time: Option<DateTime<Utc>>,
    /// Additional parameters
    pub additional: HashMap<String, String>,
}

/// DID resolution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidResolutionMetadata {
    /// Content type of the resolved document
    pub content_type: Option<String>,
    /// Error code if resolution failed
    pub error: Option<String>,
    /// Additional metadata
    pub additional: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_did_parsing() {
        let did_str = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
        let did = Did::parse(did_str).unwrap();
        
        assert_eq!(did.scheme, "did");
        assert_eq!(did.method, "key");
        assert_eq!(did.method_specific_id, "z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK");
        assert_eq!(did.to_string(), did_str);
    }

    #[test]
    fn test_did_with_components() {
        let did_str = "did:web:example.com:user:123/path?query=value#fragment";
        let did = Did::parse(did_str).unwrap();
        
        assert_eq!(did.method, "web");
        assert_eq!(did.method_specific_id, "example.com:user:123");
        assert_eq!(did.path, Some("/path".to_string()));
        assert_eq!(did.query, Some("query=value".to_string()));
        assert_eq!(did.fragment, Some("fragment".to_string()));
    }

    #[test]
    fn test_did_without_fragment() {
        let did_str = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK#key-1";
        let did = Did::parse(did_str).unwrap();
        let without_fragment = did.without_fragment();
        
        assert_eq!(without_fragment.fragment, None);
        assert_eq!(without_fragment.to_string(), "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK");
    }
}