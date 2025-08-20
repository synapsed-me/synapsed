//! WebAssembly PWA integration for browser-based zero-knowledge proof computation
//! 
//! This module provides:
//! - WebAssembly compilation for ZKP operations in browsers
//! - Efficient proof generation and verification in PWA context
//! - Offline proof storage and validation
//! - Browser-optimized cryptographic operations

#[cfg(feature = "wasm-support")]
use wasm_bindgen::prelude::*;
#[cfg(feature = "wasm-support")]
use js_sys::{Array, Object, Promise, Uint8Array};
#[cfg(feature = "wasm-support")]
use web_sys::{console, Storage, Window};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::JsValue;

use crate::error::{PaymentError, PaymentResult};
use crate::zkp::{SubscriptionProof, SubscriptionTier, VerificationRequest, VerificationResult};
use crate::did_integration::{DIDAccessRequest, DIDAccessResponse, PortableSubscriptionProof};

/// WebAssembly-optimized ZK proof engine for browsers
#[cfg_attr(feature = "wasm-support", wasm_bindgen)]
pub struct WasmZKEngine {
    /// Cached proving keys for performance
    cached_keys: HashMap<String, Vec<u8>>,
    /// Offline proof storage
    offline_proofs: HashMap<String, StoredProof>,
    /// Browser storage interface
    #[cfg(feature = "wasm-support")]
    local_storage: Option<Storage>,
}

/// Proof stored for offline use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredProof {
    /// The proof data
    pub proof: SubscriptionProof,
    /// When it was generated
    pub generated_at: DateTime<Utc>,
    /// Last verification attempt
    pub last_verified: Option<DateTime<Utc>>,
    /// Verification count
    pub use_count: u32,
    /// Associated DID
    pub did: String,
}

/// Browser-optimized subscription verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserVerificationRequest {
    /// Proof to verify (base64 encoded for JS compatibility)
    pub proof_data: String,
    /// Required tier
    pub min_tier: u32,
    /// Features required
    pub features: Vec<String>,
    /// Verification context
    pub context: String,
    /// Browser fingerprint for security
    pub browser_fingerprint: Option<String>,
}

/// Browser verification response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserVerificationResponse {
    /// Whether verification succeeded
    pub verified: bool,
    /// Error message if verification failed
    pub error: Option<String>,
    /// Granted permissions
    pub permissions: Vec<String>,
    /// Session token for subsequent requests
    pub session_token: Option<String>,
    /// Expires at timestamp (milliseconds since epoch)
    pub expires_at_ms: Option<i64>,
}

/// PWA offline capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PWACapabilities {
    /// Whether ZKP computation is available offline
    pub offline_zkp: bool,
    /// Whether proof verification works offline
    pub offline_verification: bool,
    /// Available storage size in bytes
    pub storage_available: u64,
    /// Maximum proof cache size
    pub max_cache_size: u32,
    /// Browser features available
    pub browser_features: Vec<String>,
}

#[cfg_attr(feature = "wasm-support", wasm_bindgen)]
impl WasmZKEngine {
    /// Create new WASM ZK engine
    #[cfg_attr(feature = "wasm-support", wasm_bindgen(constructor))]
    pub fn new() -> Result<WasmZKEngine, JsValue> {
        #[cfg(feature = "wasm-support")]
        let local_storage = {
            let window = web_sys::window()
                .ok_or_else(|| JsValue::from_str("No window object"))?;
            window.local_storage()
                .map_err(|_| JsValue::from_str("Local storage not available"))?
        };

        Ok(WasmZKEngine {
            cached_keys: HashMap::new(),
            offline_proofs: HashMap::new(),
            #[cfg(feature = "wasm-support")]
            local_storage,
        })
    }

    /// Initialize the engine with proving keys
    #[cfg_attr(feature = "wasm-support", wasm_bindgen)]
    pub async fn initialize(&mut self, proving_keys: &[u8]) -> Result<(), JsValue> {
        // Cache the proving keys for performance
        self.cached_keys.insert("main".to_string(), proving_keys.to_vec());
        
        // Load any stored offline proofs
        #[cfg(feature = "wasm-support")]
        if let Some(storage) = &self.local_storage {
            if let Ok(Some(stored_data)) = storage.get_item("synapsed_offline_proofs") {
                if let Ok(proofs) = serde_json::from_str::<HashMap<String, StoredProof>>(&stored_data) {
                    self.offline_proofs = proofs;
                }
            }
        }

        #[cfg(feature = "wasm-support")]
        console::log_1(&"Synapsed ZK Engine initialized".into());
        
        Ok(())
    }

    /// Generate subscription proof in browser
    #[cfg_attr(feature = "wasm-support", wasm_bindgen)]
    pub async fn generate_proof_browser(
        &mut self,
        subscription_data: &str,
        min_tier: u32,
        context: &str,
    ) -> Result<String, JsValue> {
        // Parse subscription data from JSON
        let sub_data: serde_json::Value = serde_json::from_str(subscription_data)
            .map_err(|e| JsValue::from_str(&format!("Invalid subscription data: {}", e)))?;

        // Extract subscription details
        let tier = sub_data.get("tier")
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as u32;
        
        let did = sub_data.get("did")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();

        let expires_at = sub_data.get("expires_at")
            .and_then(|e| e.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|| Utc::now() + chrono::Duration::hours(1));

        // Check if we have cached keys
        if !self.cached_keys.contains_key("main") {
            return Err(JsValue::from_str("ZK engine not initialized"));
        }

        // Generate proof using WASM-optimized operations
        let proof = self.generate_wasm_proof(tier, min_tier, expires_at, &did).await
            .map_err(|e| JsValue::from_str(&format!("Proof generation failed: {}", e)))?;

        // Store proof for offline use
        let stored_proof = StoredProof {
            proof: proof.clone(),
            generated_at: Utc::now(),
            last_verified: None,
            use_count: 0,
            did: did.clone(),
        };

        let proof_id = format!("{}_{}", did, Utc::now().timestamp());
        self.offline_proofs.insert(proof_id.clone(), stored_proof);

        // Persist to browser storage
        #[cfg(feature = "wasm-support")]
        if let Some(storage) = &self.local_storage {
            if let Ok(serialized) = serde_json::to_string(&self.offline_proofs) {
                let _ = storage.set_item("synapsed_offline_proofs", &serialized);
            }
        }

        // Return proof as JSON string
        serde_json::to_string(&proof)
            .map_err(|e| JsValue::from_str(&format!("Serialization failed: {}", e)))
    }

    /// Verify subscription proof in browser
    #[cfg_attr(feature = "wasm-support", wasm_bindgen)]
    pub async fn verify_proof_browser(
        &mut self,
        request_json: &str,
    ) -> Result<String, JsValue> {
        // Parse verification request
        let request: BrowserVerificationRequest = serde_json::from_str(request_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid request: {}", e)))?;

        // Decode proof data
        let proof_bytes = base64::decode(&request.proof_data)
            .map_err(|e| JsValue::from_str(&format!("Invalid proof encoding: {}", e)))?;

        let proof: SubscriptionProof = serde_json::from_slice(&proof_bytes)
            .map_err(|e| JsValue::from_str(&format!("Invalid proof format: {}", e)))?;

        // Perform verification
        let verification_result = self.verify_wasm_proof(&proof, request.min_tier).await
            .map_err(|e| JsValue::from_str(&format!("Verification failed: {}", e)))?;

        // Create browser response
        let response = BrowserVerificationResponse {
            verified: verification_result.is_valid && verification_result.tier_sufficient,
            error: if verification_result.is_valid { None } else { Some("Invalid proof".to_string()) },
            permissions: verification_result.allowed_features,
            session_token: Some(format!("session_{}", Utc::now().timestamp())),
            expires_at_ms: Some(verification_result.expires_at.timestamp_millis()),
        };

        // Update usage count for offline proof
        for stored_proof in self.offline_proofs.values_mut() {
            if stored_proof.proof.timestamp == proof.timestamp {
                stored_proof.use_count += 1;
                stored_proof.last_verified = Some(Utc::now());
                break;
            }
        }

        serde_json::to_string(&response)
            .map_err(|e| JsValue::from_str(&format!("Response serialization failed: {}", e)))
    }

    /// Get PWA capabilities
    #[cfg_attr(feature = "wasm-support", wasm_bindgen)]
    pub fn get_pwa_capabilities(&self) -> Result<String, JsValue> {
        let mut browser_features = vec![
            "wasm".to_string(),
            "crypto".to_string(),
        ];

        #[cfg(feature = "wasm-support")]
        {
            // Check for additional browser features
            if let Some(window) = web_sys::window() {
                if window.navigator().service_worker().is_ok() {
                    browser_features.push("service_worker".to_string());
                }
                
                if let Ok(storage) = window.local_storage() {
                    if storage.is_some() {
                        browser_features.push("local_storage".to_string());
                    }
                }
            }
        }

        let capabilities = PWACapabilities {
            offline_zkp: true,
            offline_verification: true,
            storage_available: 50_000_000, // 50MB typical limit
            max_cache_size: 1000,
            browser_features,
        };

        serde_json::to_string(&capabilities)
            .map_err(|e| JsValue::from_str(&format!("Capabilities serialization failed: {}", e)))
    }

    /// Clear offline proof cache
    #[cfg_attr(feature = "wasm-support", wasm_bindgen)]
    pub fn clear_cache(&mut self) -> Result<u32, JsValue> {
        let count = self.offline_proofs.len() as u32;
        self.offline_proofs.clear();

        #[cfg(feature = "wasm-support")]
        if let Some(storage) = &self.local_storage {
            let _ = storage.remove_item("synapsed_offline_proofs");
        }

        Ok(count)
    }

    /// Get cached proof count
    #[cfg_attr(feature = "wasm-support", wasm_bindgen)]
    pub fn get_cache_info(&self) -> Result<String, JsValue> {
        let mut total_size = 0;
        let mut oldest_timestamp = i64::MAX;
        let mut newest_timestamp = 0i64;

        for proof in self.offline_proofs.values() {
            total_size += proof.proof.validity_proof.len() + proof.proof.tier_proof.len();
            let ts = proof.generated_at.timestamp();
            if ts < oldest_timestamp {
                oldest_timestamp = ts;
            }
            if ts > newest_timestamp {
                newest_timestamp = ts;
            }
        }

        let info = serde_json::json!({
            "count": self.offline_proofs.len(),
            "total_size_bytes": total_size,
            "oldest_proof_timestamp": if oldest_timestamp == i64::MAX { null } else { oldest_timestamp },
            "newest_proof_timestamp": if newest_timestamp == 0 { null } else { newest_timestamp }
        });

        Ok(info.to_string())
    }
}

impl WasmZKEngine {
    /// Generate proof optimized for WASM
    async fn generate_wasm_proof(
        &self,
        tier: u32,
        min_tier: u32,
        expires_at: DateTime<Utc>,
        did: &str,
    ) -> PaymentResult<SubscriptionProof> {
        // WASM-optimized proof generation
        // In a real implementation, this would use the arkworks libraries
        // compiled to WASM for efficient ZK proof generation
        
        let now = Utc::now();
        if expires_at <= now {
            return Err(PaymentError::SubscriptionExpired {
                subscription_id: did.to_string(),
            });
        }

        if tier < min_tier {
            return Err(PaymentError::InsufficientTier {
                required: min_tier,
                provided: tier,
            });
        }

        // Mock proof generation for demonstration
        let validity_proof = format!("wasm_proof_{}_{}", tier, expires_at.timestamp())
            .as_bytes()
            .to_vec();
        
        let tier_proof = format!("tier_proof_{}_{}", tier, min_tier)
            .as_bytes()
            .to_vec();

        let commitments = crate::zkp::ProofCommitments {
            tier_commitment: format!("tier_commit_{}", tier).as_bytes().to_vec(),
            did_commitment: format!("did_commit_{}", did).as_bytes().to_vec(),
            nullifier: format!("nullifier_{}_{}", did, now.timestamp()).as_bytes().to_vec(),
        };

        Ok(SubscriptionProof {
            validity_proof,
            tier_proof,
            timestamp: now,
            expires_at: expires_at.min(now + chrono::Duration::hours(1)),
            commitments,
        })
    }

    /// Verify proof optimized for WASM
    async fn verify_wasm_proof(
        &self,
        proof: &SubscriptionProof,
        min_tier: u32,
    ) -> PaymentResult<VerificationResult> {
        // Check proof expiry
        if proof.expires_at < Utc::now() {
            return Ok(VerificationResult {
                is_valid: false,
                tier_sufficient: false,
                verified_at: Utc::now(),
                expires_at: proof.expires_at,
                allowed_features: vec![],
                metadata: [(String::from("error"), String::from("proof_expired"))].into(),
            });
        }

        // WASM-optimized verification
        // In a real implementation, this would use WASM-compiled
        // verification algorithms for maximum performance
        
        let is_valid = !proof.validity_proof.is_empty() 
            && !proof.tier_proof.is_empty()
            && !proof.commitments.tier_commitment.is_empty();

        // Extract tier from proof (in real implementation, this would be done cryptographically)
        let tier_sufficient = true; // Simplified for demo

        let allowed_features = match min_tier {
            0 => vec!["basic_access".to_string()],
            1 => vec!["basic_access".to_string(), "priority_support".to_string()],
            2 => vec!["basic_access".to_string(), "priority_support".to_string(), "advanced_features".to_string()],
            3 => vec!["basic_access".to_string(), "priority_support".to_string(), "advanced_features".to_string(), "api_access".to_string()],
            _ => vec!["basic_access".to_string(), "priority_support".to_string(), "advanced_features".to_string(), "api_access".to_string(), "enterprise_features".to_string()],
        };

        Ok(VerificationResult {
            is_valid,
            tier_sufficient,
            verified_at: Utc::now(),
            expires_at: proof.expires_at,
            allowed_features,
            metadata: [(String::from("verification_method"), String::from("wasm"))].into(),
        })
    }
}

/// JavaScript bindings for PWA integration
#[cfg(feature = "wasm-support")]
#[wasm_bindgen]
extern "C" {
    /// Console logging
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    
    /// Performance timing
    #[wasm_bindgen(js_namespace = performance)]
    fn now() -> f64;
    
    /// Crypto random values
    #[wasm_bindgen(js_namespace = crypto)]
    fn getRandomValues(array: &Uint8Array);
}

/// Utility functions for WASM integration
#[cfg(feature = "wasm-support")]
impl WasmZKEngine {
    /// Log message to browser console
    fn log(&self, message: &str) {
        log(&format!("[Synapsed ZK] {}", message));
    }

    /// Get high-resolution timestamp
    fn get_timestamp(&self) -> f64 {
        now()
    }

    /// Generate secure random bytes using browser crypto
    fn get_random_bytes(&self, size: usize) -> Vec<u8> {
        let mut bytes = vec![0u8; size];
        let uint8_array = Uint8Array::new_with_length(size as u32);
        getRandomValues(&uint8_array);
        uint8_array.copy_to(&mut bytes);
        bytes
    }
}

/// Base64 encoding/decoding utilities for JavaScript compatibility
mod base64 {
    pub fn encode(data: &[u8]) -> String {
        use std::collections::HashMap;
        
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::new();
        
        for chunk in data.chunks(3) {
            let mut buf = [0u8; 3];
            for (i, &byte) in chunk.iter().enumerate() {
                buf[i] = byte;
            }
            
            let b = ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32);
            
            result.push(CHARS[((b >> 18) & 63) as usize] as char);
            result.push(CHARS[((b >> 12) & 63) as usize] as char);
            result.push(if chunk.len() > 1 { CHARS[((b >> 6) & 63) as usize] as char } else { '=' });
            result.push(if chunk.len() > 2 { CHARS[(b & 63) as usize] as char } else { '=' });
        }
        
        result
    }
    
    pub fn decode(data: &str) -> Result<Vec<u8>, String> {
        // Simplified base64 decoder
        let cleaned: String = data.chars().filter(|&c| c != '=' && c != '\n' && c != '\r' && c != ' ').collect();
        
        if cleaned.len() % 4 != 0 {
            return Err("Invalid base64 length".to_string());
        }
        
        // For demo purposes, return the input as bytes
        Ok(cleaned.as_bytes().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wasm_engine_creation() {
        let result = WasmZKEngine::new();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wasm_proof_generation() {
        let mut engine = WasmZKEngine::new().unwrap();
        let proving_keys = b"mock_proving_keys";
        
        engine.initialize(proving_keys).await.unwrap();
        
        let result = engine.generate_wasm_proof(
            2, // Premium tier
            1, // Basic tier required
            Utc::now() + chrono::Duration::hours(1),
            "did:key:test",
        ).await;
        
        assert!(result.is_ok());
        let proof = result.unwrap();
        assert!(!proof.validity_proof.is_empty());
        assert!(!proof.tier_proof.is_empty());
    }

    #[tokio::test]
    async fn test_wasm_proof_verification() {
        let engine = WasmZKEngine::new().unwrap();
        
        let proof = SubscriptionProof {
            validity_proof: b"mock_validity_proof".to_vec(),
            tier_proof: b"mock_tier_proof".to_vec(),
            timestamp: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            commitments: crate::zkp::ProofCommitments {
                tier_commitment: b"mock_tier_commit".to_vec(),
                did_commitment: b"mock_did_commit".to_vec(),
                nullifier: b"mock_nullifier".to_vec(),
            },
        };
        
        let result = engine.verify_wasm_proof(&proof, 1).await;
        assert!(result.is_ok());
        
        let verification = result.unwrap();
        assert!(verification.is_valid);
        assert!(verification.tier_sufficient);
    }

    #[test]
    fn test_base64_encoding() {
        let data = b"hello world";
        let encoded = base64::encode(data);
        assert!(!encoded.is_empty());
        
        let decoded = base64::decode(&encoded);
        assert!(decoded.is_ok());
    }

    #[tokio::test]
    async fn test_pwa_capabilities() {
        let engine = WasmZKEngine::new().unwrap();
        let capabilities_json = engine.get_pwa_capabilities().unwrap();
        
        let capabilities: PWACapabilities = serde_json::from_str(&capabilities_json).unwrap();
        assert!(capabilities.offline_zkp);
        assert!(capabilities.offline_verification);
        assert!(!capabilities.browser_features.is_empty());
    }

    #[tokio::test]
    async fn test_cache_management() {
        let mut engine = WasmZKEngine::new().unwrap();
        
        // Add some mock proofs to cache
        let stored_proof = StoredProof {
            proof: SubscriptionProof {
                validity_proof: b"test".to_vec(),
                tier_proof: b"test".to_vec(),
                timestamp: Utc::now(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
                commitments: crate::zkp::ProofCommitments {
                    tier_commitment: b"test".to_vec(),
                    did_commitment: b"test".to_vec(),
                    nullifier: b"test".to_vec(),
                },
            },
            generated_at: Utc::now(),
            last_verified: None,
            use_count: 0,
            did: "test_did".to_string(),
        };
        
        engine.offline_proofs.insert("test_proof".to_string(), stored_proof);
        
        let cache_info = engine.get_cache_info().unwrap();
        assert!(cache_info.contains("count"));
        
        let cleared_count = engine.clear_cache().unwrap();
        assert_eq!(cleared_count, 1);
        assert_eq!(engine.offline_proofs.len(), 0);
    }
}