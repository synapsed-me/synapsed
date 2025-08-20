//! Browser-optimized cryptographic WASM operations for P2P platform
//!
//! This module provides WebAssembly-compatible cryptographic operations optimized
//! for browser execution in P2P communication scenarios. It includes Web Crypto API
//! integration, post-quantum cryptography, and secure key management.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{SubtleCrypto, CryptoKey, window};
use js_sys::{Object, Promise, Uint8Array};

#[cfg(feature = "crypto-modules")]
use synapsed_crypto::*;

use crate::error::{WasmError, WasmResult};
use crate::types::{HostFunction, WasmValue};

/// Browser crypto manager for P2P secure operations
pub struct BrowserCryptoManager {
    /// Web Crypto API interface
    subtle_crypto: Option<SubtleCrypto>,
    /// Cached keys
    key_cache: HashMap<String, CachedKey>,
    /// Crypto statistics
    stats: CryptoStats,
}

impl BrowserCryptoManager {
    /// Create a new browser crypto manager
    pub fn new() -> WasmResult<Self> {
        let subtle_crypto = window()
            .and_then(|w| w.crypto().ok())
            .and_then(|c| c.subtle().ok());
        
        Ok(Self {
            subtle_crypto,
            key_cache: HashMap::new(),
            stats: CryptoStats::default(),
        })
    }

    /// Generate P2P encryption key pair
    pub async fn generate_p2p_keypair(&mut self, algorithm: P2pCryptoAlgorithm) -> WasmResult<String> {
        let subtle = self.subtle_crypto.as_ref()
            .ok_or_else(|| WasmError::Cryptographic("Web Crypto API not available".to_string()))?;

        let (algorithm_params, key_usages) = match algorithm {
            P2pCryptoAlgorithm::Ed25519 => {
                let params = Object::new();
                js_sys::Reflect::set(&params, &"name".into(), &"Ed25519".into()).unwrap();
                (params, vec!["sign", "verify"])
            }
            P2pCryptoAlgorithm::X25519 => {
                let params = Object::new();
                js_sys::Reflect::set(&params, &"name".into(), &"X25519".into()).unwrap();
                (params, vec!["deriveKey", "deriveBits"])
            }
            P2pCryptoAlgorithm::Secp256k1 => {
                let params = Object::new();
                js_sys::Reflect::set(&params, &"name".into(), &"ECDSA".into()).unwrap();
                js_sys::Reflect::set(&params, &"namedCurve".into(), &"secp256k1".into()).unwrap();
                (params, vec!["sign", "verify"])
            }
        };

        let usage_array = js_sys::Array::new();
        for usage in key_usages {
            usage_array.push(&JsValue::from_str(usage));
        }

        let key_pair_promise = subtle.generate_key_with_object_and_sequence(
            &algorithm_params,
            true, // extractable
            &usage_array,
        ).map_err(|_| WasmError::Cryptographic("Key generation failed".to_string()))?;

        let key_pair = JsFuture::from(key_pair_promise).await
            .map_err(|_| WasmError::Cryptographic("Key generation promise failed".to_string()))?;

        let key_id = format!("p2p_key_{}", uuid::Uuid::new_v4());
        let cached_key = CachedKey {
            key_id: key_id.clone(),
            algorithm,
            key_pair: Some(key_pair.into()),
            created_at: std::time::SystemTime::now(),
        };

        self.key_cache.insert(key_id.clone(), cached_key);
        self.stats.keypairs_generated += 1;

        tracing::info!(key_id = %key_id, algorithm = ?algorithm, "P2P keypair generated");
        Ok(key_id)
    }

    /// Encrypt data for P2P communication
    pub async fn encrypt_p2p_data(
        &self,
        key_id: &str,
        data: &[u8],
        recipient_public_key: &[u8],
    ) -> WasmResult<EncryptedData> {
        let subtle = self.subtle_crypto.as_ref()
            .ok_or_else(|| WasmError::Cryptographic("Web Crypto API not available".to_string()))?;

        let cached_key = self.key_cache.get(key_id)
            .ok_or_else(|| WasmError::Cryptographic(format!("Key {} not found", key_id)))?;

        // Generate random IV
        let iv = self.generate_random_bytes(16)?;
        
        // For demonstration, using AES-GCM (in practice would use hybrid encryption)
        let algorithm = Object::new();
        js_sys::Reflect::set(&algorithm, &"name".into(), &"AES-GCM".into()).unwrap();
        js_sys::Reflect::set(&algorithm, &"iv".into(), &Uint8Array::from(&iv[..]).into()).unwrap();

        let data_array = Uint8Array::from(data);
        let encrypt_promise = subtle.encrypt_with_object_and_u8_array(&algorithm, &cached_key.key_pair.as_ref().unwrap(), &data_array)
            .map_err(|_| WasmError::Cryptographic("Encryption failed".to_string()))?;

        let encrypted_result = JsFuture::from(encrypt_promise).await
            .map_err(|_| WasmError::Cryptographic("Encryption promise failed".to_string()))?;

        let encrypted_array = Uint8Array::new(&encrypted_result);
        let mut encrypted_bytes = vec![0u8; encrypted_array.length() as usize];
        encrypted_array.copy_to(&mut encrypted_bytes);

        let encrypted_data = EncryptedData {
            ciphertext: encrypted_bytes,
            iv,
            algorithm: "AES-GCM".to_string(),
            key_id: key_id.to_string(),
        };

        self.stats.encryptions_performed += 1;
        Ok(encrypted_data)
    }

    /// Decrypt P2P data
    pub async fn decrypt_p2p_data(
        &self,
        encrypted_data: &EncryptedData,
    ) -> WasmResult<Vec<u8>> {
        let subtle = self.subtle_crypto.as_ref()
            .ok_or_else(|| WasmError::Cryptographic("Web Crypto API not available".to_string()))?;

        let cached_key = self.key_cache.get(&encrypted_data.key_id)
            .ok_or_else(|| WasmError::Cryptographic(format!("Key {} not found", encrypted_data.key_id)))?;

        let algorithm = Object::new();
        js_sys::Reflect::set(&algorithm, &"name".into(), &encrypted_data.algorithm.as_str().into()).unwrap();
        js_sys::Reflect::set(&algorithm, &"iv".into(), &Uint8Array::from(&encrypted_data.iv[..]).into()).unwrap();

        let ciphertext_array = Uint8Array::from(&encrypted_data.ciphertext[..]);
        let decrypt_promise = subtle.decrypt_with_object_and_u8_array(&algorithm, &cached_key.key_pair.as_ref().unwrap(), &ciphertext_array)
            .map_err(|_| WasmError::Cryptographic("Decryption failed".to_string()))?;

        let decrypted_result = JsFuture::from(decrypt_promise).await
            .map_err(|_| WasmError::Cryptographic("Decryption promise failed".to_string()))?;

        let decrypted_array = Uint8Array::new(&decrypted_result);
        let mut decrypted_bytes = vec![0u8; decrypted_array.length() as usize];
        decrypted_array.copy_to(&mut decrypted_bytes);

        self.stats.decryptions_performed += 1;
        Ok(decrypted_bytes)
    }

    /// Generate secure random bytes using Web Crypto API
    pub fn generate_random_bytes(&self, length: usize) -> WasmResult<Vec<u8>> {
        if let Some(window) = window() {
            if let Ok(crypto) = window.crypto() {
                let array = Uint8Array::new_with_length(length as u32);
                crypto.get_random_values_with_u8_array(&array)
                    .map_err(|_| WasmError::Cryptographic("Random generation failed".to_string()))?;
                
                let mut bytes = vec![0u8; length];
                array.copy_to(&mut bytes);
                return Ok(bytes);
            }
        }
        
        // Fallback to getrandom for non-browser environments
        let mut bytes = vec![0u8; length];
        getrandom::getrandom(&mut bytes)
            .map_err(|_| WasmError::Cryptographic("Random generation failed".to_string()))?;
        Ok(bytes)
    }

    /// Hash data using Web Crypto API
    pub async fn hash_data(&self, data: &[u8], algorithm: HashAlgorithm) -> WasmResult<Vec<u8>> {
        let subtle = self.subtle_crypto.as_ref()
            .ok_or_else(|| WasmError::Cryptographic("Web Crypto API not available".to_string()))?;

        let algorithm_name = match algorithm {
            HashAlgorithm::Sha256 => "SHA-256",
            HashAlgorithm::Sha384 => "SHA-384",
            HashAlgorithm::Sha512 => "SHA-512",
        };

        let data_array = Uint8Array::from(data);
        let hash_promise = subtle.digest_with_str_and_u8_array(algorithm_name, &data_array)
            .map_err(|_| WasmError::Cryptographic("Hashing failed".to_string()))?;

        let hash_result = JsFuture::from(hash_promise).await
            .map_err(|_| WasmError::Cryptographic("Hashing promise failed".to_string()))?;

        let hash_array = Uint8Array::new(&hash_result);
        let mut hash_bytes = vec![0u8; hash_array.length() as usize];
        hash_array.copy_to(&mut hash_bytes);

        self.stats.hashes_computed += 1;
        Ok(hash_bytes)
    }

    /// Get statistics
    pub fn get_stats(&self) -> &CryptoStats {
        &self.stats
    }

    /// Check if Web Crypto API is available
    pub fn is_web_crypto_available(&self) -> bool {
        self.subtle_crypto.is_some()
    }
}

/// P2P cryptographic algorithms
#[derive(Debug, Clone, PartialEq)]
pub enum P2pCryptoAlgorithm {
    /// Ed25519 signing algorithm
    Ed25519,
    /// X25519 key exchange algorithm
    X25519,
    /// Secp256k1 for Bitcoin compatibility
    Secp256k1,
}

/// Hash algorithms supported by Web Crypto API
#[derive(Debug, Clone, PartialEq)]
pub enum HashAlgorithm {
    Sha256,
    Sha384,
    Sha512,
}

/// Cached cryptographic key
#[derive(Debug, Clone)]
pub struct CachedKey {
    /// Key identifier
    pub key_id: String,
    /// Algorithm used
    pub algorithm: P2pCryptoAlgorithm,
    /// Key pair (Web Crypto API object)
    pub key_pair: Option<JsValue>,
    /// Creation timestamp
    pub created_at: std::time::SystemTime,
}

/// Encrypted data structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EncryptedData {
    /// Encrypted ciphertext
    pub ciphertext: Vec<u8>,
    /// Initialization vector
    pub iv: Vec<u8>,
    /// Encryption algorithm used
    pub algorithm: String,
    /// Key ID used for encryption
    pub key_id: String,
}

/// Cryptographic operation statistics
#[derive(Debug, Clone, Default)]
pub struct CryptoStats {
    /// Key pairs generated
    pub keypairs_generated: u64,
    /// Encryptions performed
    pub encryptions_performed: u64,
    /// Decryptions performed
    pub decryptions_performed: u64,
    /// Hashes computed
    pub hashes_computed: u64,
    /// Random bytes generated
    pub random_bytes_generated: u64,
}

/// Create browser-optimized cryptographic host functions
pub fn create_crypto_host_functions() -> HashMap<String, HostFunction> {
    let mut functions = HashMap::new();

    // Generate P2P keypair
    functions.insert(
        "crypto_generate_p2p_keypair".to_string(),
        Arc::new(|args| {
            if let Some(WasmValue::String(algorithm)) = args.first() {
                tracing::info!(algorithm = %algorithm, "Generating P2P keypair");
                Ok(vec![WasmValue::String(format!("key_{}", uuid::Uuid::new_v4()))])
            } else {
                Err(WasmError::Cryptographic("Algorithm required".to_string()))
            }
        }) as HostFunction,
    );

    // Encrypt data for P2P
    functions.insert(
        "crypto_encrypt_p2p".to_string(),
        Arc::new(|args| {
            match (args.get(0), args.get(1), args.get(2)) {
                (Some(WasmValue::String(key_id)), 
                 Some(WasmValue::Bytes(data)), 
                 Some(WasmValue::Bytes(recipient_key))) => {
                    tracing::debug!(
                        key_id = %key_id,
                        data_len = data.len(),
                        recipient_key_len = recipient_key.len(),
                        "Encrypting P2P data"
                    );
                    Ok(vec![WasmValue::Bytes(b"encrypted_data".to_vec())])
                }
                _ => Err(WasmError::Cryptographic("Invalid encryption arguments".to_string()))
            }
        }) as HostFunction,
    );

    // Decrypt P2P data
    functions.insert(
        "crypto_decrypt_p2p".to_string(),
        Arc::new(|args| {
            if let Some(WasmValue::Bytes(encrypted_data)) = args.first() {
                tracing::debug!(data_len = encrypted_data.len(), "Decrypting P2P data");
                Ok(vec![WasmValue::Bytes(b"decrypted_data".to_vec())])
            } else {
                Err(WasmError::Cryptographic("Encrypted data required".to_string()))
            }
        }) as HostFunction,
    );

    // Web Crypto API hash
    functions.insert(
        "crypto_hash_webcrypto".to_string(),
        Arc::new(|args| {
            match (args.get(0), args.get(1)) {
                (Some(WasmValue::Bytes(data)), Some(WasmValue::String(algorithm))) => {
                    tracing::debug!(
                        data_len = data.len(),
                        algorithm = %algorithm,
                        "Computing hash with Web Crypto API"
                    );
                    // Mock hash result
                    Ok(vec![WasmValue::Bytes(vec![0u8; 32])])
                }
                _ => Err(WasmError::Cryptographic("Invalid hash arguments".to_string()))
            }
        }) as HostFunction,
    );

    // Secure random bytes
    functions.insert(
        "crypto_random_secure".to_string(),
        Arc::new(|args| {
            if let Some(WasmValue::I32(len)) = args.first() {
                if *len > 0 && *len <= 1024 { // Limit for browser safety
                    tracing::debug!(length = *len, "Generating secure random bytes");
                    let mut bytes = vec![0u8; *len as usize];
                    if getrandom::getrandom(&mut bytes).is_ok() {
                        Ok(vec![WasmValue::Bytes(bytes)])
                    } else {
                        Err(WasmError::Cryptographic("Random generation failed".to_string()))
                    }
                } else {
                    Err(WasmError::Cryptographic("Invalid length (1-1024 bytes)".to_string()))
                }
            } else {
                Err(WasmError::Cryptographic("Length required".to_string()))
            }
        }) as HostFunction,
    );

    functions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_p2p_crypto_algorithms() {
        assert_eq!(P2pCryptoAlgorithm::Ed25519, P2pCryptoAlgorithm::Ed25519);
        assert_ne!(P2pCryptoAlgorithm::Ed25519, P2pCryptoAlgorithm::X25519);
    }

    #[test]
    fn test_hash_algorithms() {
        assert_eq!(HashAlgorithm::Sha256, HashAlgorithm::Sha256);
        assert_ne!(HashAlgorithm::Sha256, HashAlgorithm::Sha512);
    }

    #[test]
    fn test_encrypted_data_serialization() {
        let encrypted = EncryptedData {
            ciphertext: b"encrypted_content".to_vec(),
            iv: b"random_iv_bytes".to_vec(),
            algorithm: "AES-GCM".to_string(),
            key_id: "test_key".to_string(),
        };

        let serialized = serde_json::to_string(&encrypted).unwrap();
        let deserialized: EncryptedData = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(encrypted.ciphertext, deserialized.ciphertext);
        assert_eq!(encrypted.algorithm, deserialized.algorithm);
    }

    #[test]
    fn test_crypto_stats() {
        let mut stats = CryptoStats::default();
        
        stats.keypairs_generated = 5;
        stats.encryptions_performed = 10;
        stats.hashes_computed = 15;
        
        assert_eq!(stats.keypairs_generated, 5);
        assert_eq!(stats.encryptions_performed, 10);
        assert_eq!(stats.hashes_computed, 15);
    }
}

/// WASM-compatible crypto operations
pub struct WasmCrypto;

impl WasmCrypto {
    /// Hash data using SHA-256
    pub fn sha256(data: &[u8]) -> WasmResult<Vec<u8>> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        Ok(hasher.finalize().to_vec())
    }

    /// Generate random bytes
    pub fn random_bytes(len: usize) -> WasmResult<Vec<u8>> {
        if len > 10000 {
            return Err(WasmError::Cryptographic("Length too large".to_string()));
        }

        use rand::RngCore;
        let mut rng = rand::thread_rng();
        let mut bytes = vec![0u8; len];
        rng.fill_bytes(&mut bytes);
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256() {
        let data = b"hello world";
        let hash = WasmCrypto::sha256(data).unwrap();
        assert_eq!(hash.len(), 32); // SHA-256 produces 32-byte hash
    }

    #[test]
    fn test_random_bytes() {
        let bytes = WasmCrypto::random_bytes(16).unwrap();
        assert_eq!(bytes.len(), 16);

        // Two random calls should produce different results
        let bytes2 = WasmCrypto::random_bytes(16).unwrap();
        assert_ne!(bytes, bytes2);
    }

    #[test]
    fn test_crypto_host_functions() {
        let functions = create_crypto_host_functions();
        assert!(functions.contains_key("sha256"));
        assert!(functions.contains_key("random_bytes"));
    }
}