//! Key rotation and lifecycle management for DIDs
//! 
//! This module implements hierarchical key management with:
//! - Data Encryption Keys (DEKs) and Key Encryption Keys (KEKs)
//! - Key rotation with historical preservation
//! - Recovery mechanisms for lost devices
//! - Forward secrecy and post-compromise security

use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};
use crate::{Result, Error};
use super::{Did, DidDocument, VerificationMethod, PublicKeyMaterial, RecoveryMethod, SecretShare};
use synapsed_crypto::prelude::KeyPair;

/// Key rotation manager for DID documents
pub struct KeyRotationManager {
    /// Key hierarchies by DID
    hierarchies: HashMap<Did, KeyHierarchy>,
    /// Rotation policies
    policies: RotationPolicy,
    /// Recovery mechanisms
    recovery: RecoveryMechanism,
}

impl KeyRotationManager {
    /// Create a new key rotation manager
    pub fn new(policies: RotationPolicy, recovery: RecoveryMechanism) -> Self {
        Self {
            hierarchies: HashMap::new(),
            policies,
            recovery,
        }
    }

    /// Generate DID with hierarchical keys per Algorithm 1 from specification
    pub async fn generate_did_with_keys(&mut self, method: &str, master_password: &str) -> Result<(Did, KeyHierarchy)> {
        use crate::crypto::{IdentityKeyManager, KeyType};
        use multibase::Base;
        use sha3::{Sha3_256, Digest};

        // Step 1: Generate master key from password
        let master_key = MasterKey::new(master_password, None)?;
        
        // Step 2: Generate initial key pairs
        let signing_keypair = IdentityKeyManager::generate_keypair(KeyType::Ed25519)?;
        let encryption_keypair = IdentityKeyManager::generate_keypair(KeyType::X25519)?;
        
        // Step 3: Create DID based on method
        let method_specific_id = match method {
            "key" => {
                // For did:key, use multibase encoding of public key
                multibase::encode(Base::Base58Btc, &signing_keypair.public_key)
            },
            "web" => {
                return Err(Error::Configuration("did:web requires domain specification".into()));
            },
            "synapsed" => {
                // For did:synapsed, use SHA3-256 hash of public key
                let mut hasher = Sha3_256::new();
                hasher.update(&signing_keypair.public_key);
                let hash = hasher.finalize();
                multibase::encode(Base::Base58Btc, &hash)
            },
            _ => {
                return Err(Error::Configuration(format!("Unsupported DID method: {}", method)));
            }
        };
        
        let did = Did::new(method, &method_specific_id);
        
        // Step 4: Create key hierarchy
        let hierarchy = KeyHierarchy::new_with_keys(
            did.clone(), 
            master_key, 
            signing_keypair, 
            encryption_keypair
        )?;
        
        Ok((did, hierarchy))
    }

    /// Perform key recovery per Algorithm 7 from specification
    pub async fn recover_keys(&mut self, did: &Did, recovery_data: RecoveryData) -> Result<RotationResult> {
        // Step 1: Recover master key based on method
        let recovered_master_key = match recovery_data.recovery_method {
            RecoveryMethod::RecoveryPhrase => {
                if let Some(phrase) = &recovery_data.recovery_phrase {
                    self.recover_from_mnemonic(phrase)?
                } else {
                    return Err(Error::KeyManagementError("Recovery phrase required".into()));
                }
            },
            RecoveryMethod::SocialRecovery => {
                if let Some(shares) = &recovery_data.social_shares {
                    self.recover_from_social_shares(shares)?
                } else {
                    return Err(Error::KeyManagementError("Social shares required".into()));
                }
            },
            RecoveryMethod::HardwareRecovery => {
                if let Some(data) = &recovery_data.hardware_data {
                    self.recover_from_hardware(data)?
                } else {
                    return Err(Error::KeyManagementError("Hardware data required".into()));
                }
            },
            RecoveryMethod::CombinedRecovery => {
                // Use multiple recovery sources
                self.recover_from_multiple_sources(&recovery_data)?
            }
        };

        // Step 2: Verify and restore key hierarchy
        if !recovery_data.encrypted_hierarchy.is_empty() {
            let decrypted_data = self.decrypt_hierarchy(&recovery_data.encrypted_hierarchy, &recovered_master_key)?;
            let mut hierarchy: KeyHierarchy = serde_json::from_slice(&decrypted_data)
                .map_err(|e| Error::KeyManagementError(format!("Failed to deserialize hierarchy: {}", e)))?;
            
            // Update master key
            hierarchy.master_key = recovered_master_key;
            
            // Verify integrity
            if !self.verify_hierarchy_integrity(&hierarchy)? {
                return Err(Error::KeyManagementError("Key hierarchy integrity check failed".into()));
            }
            
            // Store recovered hierarchy
            self.hierarchies.insert(did.clone(), hierarchy);
            
            let hierarchy = self.hierarchies.get(did).unwrap();
            let updated_document = self.update_did_document(did, hierarchy)?;

            Ok(RotationResult {
                rotated: true,
                new_keys: hierarchy.get_active_key_ids(),
                deprecated_keys: Vec::new(),
                updated_document: Some(updated_document),
            })
        } else {
            Err(Error::KeyManagementError("No encrypted hierarchy data provided".into()))
        }
    }

    /// Helper: Recover master key from BIP39 mnemonic
    fn recover_from_mnemonic(&self, phrase: &str) -> Result<MasterKey> {
        // In a real implementation, this would use proper BIP39 libraries
        // For now, we'll use the phrase as a password
        MasterKey::new(phrase, None)
    }

    /// Helper: Recover master key from Shamir secret shares
    fn recover_from_social_shares(&self, shares: &[SecretShare]) -> Result<MasterKey> {
        if shares.len() < 3 {
            return Err(Error::KeyManagementError("Insufficient shares for recovery".into()));
        }

        // Simplified Lagrange interpolation for demo
        // In production, use proper Shamir secret sharing library
        let recovered_bytes = self.lagrange_interpolation(shares)?;
        
        Ok(MasterKey {
            key_bytes: recovered_bytes,
            salt: vec![0u8; 32], // Would be recovered from shares in real implementation
            kdf_params: KdfParams {
                algorithm: "scrypt".to_string(),
                n: 32768,
                r: 8,
                p: 1,
            },
        })
    }

    /// Helper: Recover from hardware
    fn recover_from_hardware(&self, _data: &[u8]) -> Result<MasterKey> {
        // Placeholder for hardware recovery
        Err(Error::KeyManagementError("Hardware recovery not yet implemented".into()))
    }

    /// Helper: Recover from multiple sources
    fn recover_from_multiple_sources(&self, recovery_data: &RecoveryData) -> Result<MasterKey> {
        // Combine recovery methods
        if let Some(phrase) = &recovery_data.recovery_phrase {
            // Try mnemonic first
            if let Ok(key) = self.recover_from_mnemonic(phrase) {
                return Ok(key);
            }
        }

        if let Some(data) = &recovery_data.hardware_data {
            // Try hardware as fallback
            if let Ok(key) = self.recover_from_hardware(data) {
                return Ok(key);
            }
        }

        Err(Error::KeyManagementError("All recovery methods failed".into()))
    }

    /// Helper: Decrypt hierarchy data
    fn decrypt_hierarchy(&self, encrypted_data: &[u8], master_key: &MasterKey) -> Result<Vec<u8>> {
        // Placeholder - would use proper encryption
        Ok(encrypted_data.to_vec())
    }

    /// Helper: Verify hierarchy integrity
    fn verify_hierarchy_integrity(&self, _hierarchy: &KeyHierarchy) -> Result<bool> {
        // Placeholder for integrity verification
        Ok(true)
    }

    /// Helper: Simplified Lagrange interpolation
    fn lagrange_interpolation(&self, shares: &[SecretShare]) -> Result<Vec<u8>> {
        // Simplified implementation for demo
        if shares.is_empty() {
            return Err(Error::KeyManagementError("No shares provided".into()));
        }
        
        // For demo, just use the first share's y values
        Ok(shares[0].y.clone())
    }

    /// Initialize key hierarchy for a DID
    pub fn initialize_hierarchy(&mut self, did: &Did, master_key: MasterKey) -> Result<()> {
        let hierarchy = KeyHierarchy::new(did.clone(), master_key)?;
        self.hierarchies.insert(did.clone(), hierarchy);
        Ok(())
    }

    /// Rotate keys for a DID based on policy
    pub fn rotate_keys(&mut self, did: &Did, reason: RotationReason) -> Result<RotationResult> {
        let hierarchy = self.hierarchies.get_mut(did)
            .ok_or_else(|| Error::KeyManagementError("Key hierarchy not found".into()))?;

        let should_rotate = match reason {
            RotationReason::Scheduled => self.policies.should_rotate_scheduled(hierarchy),
            RotationReason::Compromise => true,
            RotationReason::Manual => true,
            RotationReason::Device => self.policies.should_rotate_device_change(hierarchy),
        };

        if !should_rotate {
            return Ok(RotationResult {
                rotated: false,
                new_keys: Vec::new(),
                deprecated_keys: Vec::new(),
                updated_document: None,
            });
        }

        hierarchy.rotate_keys(reason)?;
        
        // Clone hierarchy reference to avoid borrowing issues
        let hierarchy_clone = hierarchy.clone();
        let updated_document = self.update_did_document(did, &hierarchy_clone)?;
        
        Ok(RotationResult {
            rotated: true,
            new_keys: hierarchy.get_active_key_ids(),
            deprecated_keys: hierarchy.get_deprecated_key_ids(),
            updated_document: Some(updated_document),
        })
    }

    /// Get recovery information for a DID
    pub fn get_recovery_info(&self, did: &Did) -> Option<&RecoveryInfo> {
        self.hierarchies.get(did)?.recovery_info.as_ref()
    }

    /// Perform key recovery
    pub fn recover_keys(&mut self, did: &Did, recovery_data: RecoveryData) -> Result<RotationResult> {
        let hierarchy = self.hierarchies.get_mut(did)
            .ok_or_else(|| Error::KeyManagementError("Key hierarchy not found".into()))?;

        hierarchy.recover_from_data(recovery_data)?;
        let hierarchy_clone = hierarchy.clone();
        let updated_document = self.update_did_document(did, &hierarchy_clone)?;

        Ok(RotationResult {
            rotated: true,
            new_keys: hierarchy.get_active_key_ids(),
            deprecated_keys: Vec::new(),
            updated_document: Some(updated_document),
        })
    }

    /// Update DID document with new keys
    fn update_did_document(&self, did: &Did, hierarchy: &KeyHierarchy) -> Result<DidDocument> {
        let mut document = DidDocument::new(did.clone());
        
        // Add active keys as verification methods
        for (key_id, key_material) in hierarchy.get_active_keys() {
            let verification_method = VerificationMethod::new(
                format!("{}#{}", did.to_string(), key_id),
                key_material.key_type.verification_method_type().to_string(),
                did.clone(),
                PublicKeyMaterial::PublicKeyMultibase {
                    public_key_multibase: key_material.public_key_multibase.clone(),
                },
            );
            document.add_verification_method(verification_method);
        }

        // Set up verification relationships
        let active_key_ids: Vec<String> = hierarchy.get_active_key_ids();
        for key_id in &active_key_ids {
            let full_key_id = format!("{}#{}", did.to_string(), key_id);
            document.add_authentication_reference(full_key_id.clone());
            
            // Add to capability invocation and assertion method for signing keys
            if key_id.contains("sign") {
                document.capability_invocation.push(
                    super::VerificationRelationship::Reference(full_key_id.clone())
                );
                document.assertion_method.push(
                    super::VerificationRelationship::Reference(full_key_id)
                );
            }
            
            // Add to key agreement for encryption keys
            if key_id.contains("encrypt") || key_id.contains("kem") {
                document.key_agreement.push(
                    super::VerificationRelationship::Reference(full_key_id.clone())
                );
            }
        }

        Ok(document)
    }

    /// Get key material for signing
    pub fn get_signing_key(&self, did: &Did, key_id: &str) -> Result<&PrivateKeyMaterial> {
        let hierarchy = self.hierarchies.get(did)
            .ok_or_else(|| Error::KeyManagementError("Key hierarchy not found".into()))?;
        
        hierarchy.get_private_key(key_id)
    }

    /// Check if a key is valid for the given time
    pub fn is_key_valid(&self, did: &Did, key_id: &str, at_time: DateTime<Utc>) -> bool {
        if let Some(hierarchy) = self.hierarchies.get(did) {
            hierarchy.is_key_valid(key_id, at_time)
        } else {
            false
        }
    }
}

/// Hierarchical key structure for a DID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyHierarchy {
    /// The DID this hierarchy belongs to
    did: Did,
    /// Master key (KEK)
    master_key: MasterKey,
    /// Current generation of keys
    current_generation: u32,
    /// Active keys by purpose
    active_keys: HashMap<String, KeyMaterial>,
    /// Historical keys (for backward compatibility)
    historical_keys: HashMap<String, KeyMaterial>,
    /// Recovery information
    recovery_info: Option<RecoveryInfo>,
    /// Rotation history
    rotation_history: Vec<RotationEvent>,
}

impl KeyHierarchy {
    /// Create a new key hierarchy
    pub fn new(did: Did, master_key: MasterKey) -> Result<Self> {
        let mut hierarchy = Self {
            did,
            master_key,
            current_generation: 1,
            active_keys: HashMap::new(),
            historical_keys: HashMap::new(),
            recovery_info: None,
            rotation_history: Vec::new(),
        };

        // Generate initial keys
        hierarchy.generate_initial_keys()?;
        
        Ok(hierarchy)
    }

    /// Create a new key hierarchy with provided keys
    pub fn new_with_keys(
        did: Did, 
        master_key: MasterKey, 
        signing_keypair: KeyPair,
        encryption_keypair: KeyPair
    ) -> Result<Self> {
        let mut hierarchy = Self {
            did,
            master_key,
            current_generation: 1,
            active_keys: HashMap::new(),
            historical_keys: HashMap::new(),
            recovery_info: None,
            rotation_history: Vec::new(),
        };

        // Create key materials from provided keypairs
        let signing_key = KeyMaterial {
            key_id: "signing-1".to_string(),
            key_type: super::methods::KeyType::Ed25519,
            public_key_multibase: hierarchy.encode_multibase(&signing_keypair.public_key)?,
            private_key: Some(PrivateKeyMaterial {
                private_key_bytes: signing_keypair.private_key,
                encrypted: false,
            }),
            created_at: chrono::Utc::now(),
            expires_at: None,
            revoked_at: None,
            generation: 1,
        };

        let encryption_key = KeyMaterial {
            key_id: "encryption-1".to_string(),
            key_type: super::methods::KeyType::X25519,
            public_key_multibase: hierarchy.encode_multibase(&encryption_keypair.public_key)?,
            private_key: Some(PrivateKeyMaterial {
                private_key_bytes: encryption_keypair.private_key,
                encrypted: false,
            }),
            created_at: chrono::Utc::now(),
            expires_at: None,
            revoked_at: None,
            generation: 1,
        };

        hierarchy.active_keys.insert("signing-1".to_string(), signing_key);
        hierarchy.active_keys.insert("encryption-1".to_string(), encryption_key);

        Ok(hierarchy)
    }

    /// Generate initial set of keys
    fn generate_initial_keys(&mut self) -> Result<()> {
        use crate::crypto::{IdentityKeyManager, KeyType};
        
        // Generate signing key (Ed25519)
        let signing_keypair = IdentityKeyManager::generate_keypair(KeyType::Ed25519)?;
        let signing_key = KeyMaterial {
            key_id: "signing-1".to_string(),
            key_type: super::methods::KeyType::Ed25519,
            public_key_multibase: self.encode_multibase(&signing_keypair.public_key)?,
            private_key: Some(PrivateKeyMaterial {
                private_key_bytes: signing_keypair.private_key,
                encrypted: false,
            }),
            created_at: Utc::now(),
            expires_at: None,
            revoked_at: None,
            generation: 1,
        };

        // Generate encryption key (X25519)
        let encryption_keypair = IdentityKeyManager::generate_keypair(KeyType::X25519)?;
        let encryption_key = KeyMaterial {
            key_id: "encryption-1".to_string(),
            key_type: super::methods::KeyType::X25519,
            public_key_multibase: self.encode_multibase(&encryption_keypair.public_key)?,
            private_key: Some(PrivateKeyMaterial {
                private_key_bytes: encryption_keypair.private_key,
                encrypted: false,
            }),
            created_at: Utc::now(),
            expires_at: None,
            revoked_at: None,
            generation: 1,
        };

        // Generate post-quantum keys if enabled
        #[cfg(feature = "post-quantum")]
        {
            let pq_sign_keypair = IdentityKeyManager::generate_keypair(KeyType::PostQuantumSign)?;
            let pq_sign_key = KeyMaterial {
                key_id: "pq-signing-1".to_string(),
                key_type: super::methods::KeyType::PostQuantumSign,
                public_key_multibase: self.encode_multibase(&pq_sign_keypair.public_key)?,
                private_key: Some(PrivateKeyMaterial {
                    private_key_bytes: pq_sign_keypair.private_key,
                    encrypted: false,
                }),
                created_at: Utc::now(),
                expires_at: None,
                revoked_at: None,
                generation: 1,
            };

            let pq_kem_keypair = IdentityKeyManager::generate_keypair(KeyType::PostQuantumKem)?;
            let pq_kem_key = KeyMaterial {
                key_id: "pq-kem-1".to_string(),
                key_type: super::methods::KeyType::PostQuantumKem,
                public_key_multibase: self.encode_multibase(&pq_kem_keypair.public_key)?,
                private_key: Some(PrivateKeyMaterial {
                    private_key_bytes: pq_kem_keypair.private_key,
                    encrypted: false,
                }),
                created_at: Utc::now(),
                expires_at: None,
                revoked_at: None,
                generation: 1,
            };

            self.active_keys.insert("pq-signing-1".to_string(), pq_sign_key);
            self.active_keys.insert("pq-kem-1".to_string(), pq_kem_key);
        }

        self.active_keys.insert("signing-1".to_string(), signing_key);
        self.active_keys.insert("encryption-1".to_string(), encryption_key);

        Ok(())
    }

    /// Rotate keys
    pub fn rotate_keys(&mut self, reason: RotationReason) -> Result<()> {
        // Move current keys to historical
        for (key_id, mut key_material) in self.active_keys.drain() {
            key_material.revoked_at = Some(Utc::now());
            self.historical_keys.insert(key_id, key_material);
        }

        // Increment generation
        self.current_generation += 1;

        // Generate new keys
        self.generate_keys_for_generation(self.current_generation)?;

        // Record rotation event
        self.rotation_history.push(RotationEvent {
            timestamp: Utc::now(),
            reason,
            generation: self.current_generation,
            rotated_keys: self.get_active_key_ids(),
        });

        Ok(())
    }

    /// Generate keys for a specific generation
    fn generate_keys_for_generation(&mut self, generation: u32) -> Result<()> {
        use crate::crypto::{IdentityKeyManager, KeyType};
        
        // Generate new signing key
        let signing_keypair = IdentityKeyManager::generate_keypair(KeyType::Ed25519)?;
        let signing_key = KeyMaterial {
            key_id: format!("signing-{}", generation),
            key_type: super::methods::KeyType::Ed25519,
            public_key_multibase: self.encode_multibase(&signing_keypair.public_key)?,
            private_key: Some(PrivateKeyMaterial {
                private_key_bytes: signing_keypair.private_key,
                encrypted: false,
            }),
            created_at: Utc::now(),
            expires_at: None,
            revoked_at: None,
            generation,
        };

        // Generate new encryption key
        let encryption_keypair = IdentityKeyManager::generate_keypair(KeyType::X25519)?;
        let encryption_key = KeyMaterial {
            key_id: format!("encryption-{}", generation),
            key_type: super::methods::KeyType::X25519,
            public_key_multibase: self.encode_multibase(&encryption_keypair.public_key)?,
            private_key: Some(PrivateKeyMaterial {
                private_key_bytes: encryption_keypair.private_key,
                encrypted: false,
            }),
            created_at: Utc::now(),
            expires_at: None,
            revoked_at: None,
            generation,
        };

        self.active_keys.insert(signing_key.key_id.clone(), signing_key);
        self.active_keys.insert(encryption_key.key_id.clone(), encryption_key);

        Ok(())
    }

    /// Encode public key as multibase
    fn encode_multibase(&self, public_key: &[u8]) -> Result<String> {
        Ok(multibase::encode(multibase::Base::Base58Btc, public_key))
    }

    /// Get active key IDs
    pub fn get_active_key_ids(&self) -> Vec<String> {
        self.active_keys.keys().cloned().collect()
    }

    /// Get deprecated key IDs
    pub fn get_deprecated_key_ids(&self) -> Vec<String> {
        self.historical_keys.keys().cloned().collect()
    }

    /// Get active keys
    pub fn get_active_keys(&self) -> Vec<(&String, &KeyMaterial)> {
        self.active_keys.iter().collect()
    }

    /// Get private key material
    pub fn get_private_key(&self, key_id: &str) -> Result<&PrivateKeyMaterial> {
        // Check active keys first
        if let Some(key_material) = self.active_keys.get(key_id) {
            return key_material.private_key.as_ref()
                .ok_or_else(|| Error::KeyManagementError("Private key not available".into()));
        }

        // Check historical keys
        if let Some(key_material) = self.historical_keys.get(key_id) {
            return key_material.private_key.as_ref()
                .ok_or_else(|| Error::KeyManagementError("Private key not available".into()));
        }

        Err(Error::KeyManagementError("Key not found".into()))
    }

    /// Check if key is valid at given time
    pub fn is_key_valid(&self, key_id: &str, at_time: DateTime<Utc>) -> bool {
        let key_material = self.active_keys.get(key_id)
            .or_else(|| self.historical_keys.get(key_id));

        if let Some(key_material) = key_material {
            // Check if key was created before the time
            if key_material.created_at > at_time {
                return false;
            }

            // Check if key was revoked before the time
            if let Some(revoked_at) = key_material.revoked_at {
                if revoked_at <= at_time {
                    return false;
                }
            }

            // Check expiration
            if let Some(expires_at) = key_material.expires_at {
                if expires_at <= at_time {
                    return false;
                }
            }

            true
        } else {
            false
        }
    }

    /// Recover from recovery data
    pub fn recover_from_data(&mut self, recovery_data: RecoveryData) -> Result<()> {
        // Verify recovery data with master key
        if !self.master_key.verify_recovery_data(&recovery_data)? {
            return Err(Error::KeyManagementError("Invalid recovery data".into()));
        }

        // Decrypt and restore keys
        for encrypted_key in recovery_data.encrypted_keys {
            let key_material = self.master_key.decrypt_key_material(encrypted_key)?;
            self.active_keys.insert(key_material.key_id.clone(), key_material);
        }

        // Update generation
        self.current_generation = recovery_data.generation;

        Ok(())
    }
}

/// Master key for encrypting other keys (KEK)
#[derive(Debug, Clone, Serialize, Deserialize, ZeroizeOnDrop)]
pub struct MasterKey {
    /// Key material
    pub key_bytes: Vec<u8>,
    /// Salt for key derivation
    pub salt: Vec<u8>,
    /// Key derivation parameters
    pub kdf_params: KdfParams,
}

impl MasterKey {
    /// Create a new master key
    pub fn new(password: &str, salt: Option<Vec<u8>>) -> Result<Self> {
        use scrypt::{Scrypt, scrypt};
        use sha3::{Sha3_256, Digest};
        
        let salt = salt.unwrap_or_else(|| {
            use rand::RngCore;
            let mut salt = vec![0u8; 32];
            rand::thread_rng().fill_bytes(&mut salt);
            salt
        });

        let kdf_params = KdfParams {
            algorithm: "scrypt".to_string(),
            n: 32768,  // CPU/memory cost factor
            r: 8,      // Block size
            p: 1,      // Parallelization factor
        };

        let mut key_bytes = vec![0u8; 32];
        scrypt(
            password.as_bytes(),
            &salt,
            &scrypt::Params::new(
                (kdf_params.n as f64).log2() as u8,
                kdf_params.r,
                kdf_params.p,
                32
            ).unwrap(),
            &mut key_bytes
        ).map_err(|e| Error::CryptographicError(format!("Key derivation failed: {}", e)))?;

        Ok(Self {
            key_bytes,
            salt,
            kdf_params,
        })
    }

    /// Encrypt key material
    pub fn encrypt_key_material(&self, key_material: &KeyMaterial) -> Result<EncryptedKeyMaterial> {
        use chacha20poly1305::{aead::{Aead, KeyInit}, ChaCha20Poly1305, Key, Nonce};
        use rand::RngCore;

        let key = Key::from_slice(&self.key_bytes);
        let cipher = ChaCha20Poly1305::new(key);

        let mut nonce_bytes = vec![0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let plaintext = serde_json::to_vec(key_material)
            .map_err(|e| Error::CryptographicError(format!("Serialization failed: {}", e)))?;

        let ciphertext = cipher.encrypt(nonce, plaintext.as_ref())
            .map_err(|e| Error::CryptographicError(format!("Encryption failed: {}", e)))?;

        Ok(EncryptedKeyMaterial {
            key_id: key_material.key_id.clone(),
            ciphertext,
            nonce: nonce_bytes,
            algorithm: "ChaCha20Poly1305".to_string(),
        })
    }

    /// Decrypt key material
    pub fn decrypt_key_material(&self, encrypted: EncryptedKeyMaterial) -> Result<KeyMaterial> {
        use chacha20poly1305::{aead::{Aead, KeyInit}, ChaCha20Poly1305, Key, Nonce};

        let key = Key::from_slice(&self.key_bytes);
        let cipher = ChaCha20Poly1305::new(key);
        let nonce = Nonce::from_slice(&encrypted.nonce);

        let plaintext = cipher.decrypt(nonce, encrypted.ciphertext.as_ref())
            .map_err(|e| Error::CryptographicError(format!("Decryption failed: {}", e)))?;

        serde_json::from_slice(&plaintext)
            .map_err(|e| Error::CryptographicError(format!("Deserialization failed: {}", e)))
    }

    /// Verify recovery data authenticity
    pub fn verify_recovery_data(&self, recovery_data: &RecoveryData) -> Result<bool> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(&self.key_bytes)
            .map_err(|e| Error::CryptographicError(format!("HMAC initialization failed: {}", e)))?;

        // Create message to authenticate
        let message = format!("{}:{}", recovery_data.generation, recovery_data.encrypted_keys.len());
        mac.update(message.as_bytes());

        let expected_tag = mac.finalize().into_bytes();
        Ok(expected_tag.as_slice() == recovery_data.authentication_tag)
    }
}

/// Key material with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMaterial {
    /// Unique key identifier
    pub key_id: String,
    /// Key type
    pub key_type: super::methods::KeyType,
    /// Public key in multibase format
    pub public_key_multibase: String,
    /// Private key material (if available)
    pub private_key: Option<PrivateKeyMaterial>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,
    /// Revocation timestamp
    pub revoked_at: Option<DateTime<Utc>>,
    /// Key generation number
    pub generation: u32,
}

/// Private key material
#[derive(Debug, Clone, Serialize, Deserialize, ZeroizeOnDrop)]
pub struct PrivateKeyMaterial {
    /// Private key bytes
    #[zeroize(skip)]
    pub private_key_bytes: Vec<u8>,
    /// Whether the private key is encrypted
    pub encrypted: bool,
}

/// Encrypted key material for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedKeyMaterial {
    /// Key identifier
    pub key_id: String,
    /// Encrypted key data
    pub ciphertext: Vec<u8>,
    /// Nonce for encryption
    pub nonce: Vec<u8>,
    /// Encryption algorithm
    pub algorithm: String,
}

/// Key derivation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdfParams {
    /// Algorithm name
    pub algorithm: String,
    /// Cost parameter (N for scrypt)
    pub n: u32,
    /// Block size (r for scrypt)
    pub r: u32,
    /// Parallelization (p for scrypt)
    pub p: u32,
}

/// Rotation policy configuration
#[derive(Debug, Clone)]
pub struct RotationPolicy {
    /// Maximum age before automatic rotation
    pub max_key_age: Duration,
    /// Rotate on device change
    pub rotate_on_device_change: bool,
    /// Rotate on suspected compromise
    pub rotate_on_compromise: bool,
    /// Rotation schedule (cron-like)
    pub rotation_schedule: Option<String>,
}

impl RotationPolicy {
    /// Check if scheduled rotation is needed
    pub fn should_rotate_scheduled(&self, hierarchy: &KeyHierarchy) -> bool {
        if let Some((_, newest_key)) = hierarchy.active_keys.iter()
            .max_by_key(|(_, key)| key.created_at) {
            let age = Utc::now().signed_duration_since(newest_key.created_at);
            age > self.max_key_age
        } else {
            true // No keys, should rotate
        }
    }

    /// Check if rotation is needed for device change
    pub fn should_rotate_device_change(&self, _hierarchy: &KeyHierarchy) -> bool {
        self.rotate_on_device_change
    }
}

impl Default for RotationPolicy {
    fn default() -> Self {
        Self {
            max_key_age: Duration::days(90), // 90 days default
            rotate_on_device_change: true,
            rotate_on_compromise: true,
            rotation_schedule: None,
        }
    }
}

/// Recovery mechanism configuration
#[derive(Debug, Clone)]
pub struct RecoveryMechanism {
    /// Recovery phrase length
    pub recovery_phrase_length: usize,
    /// Social recovery threshold
    pub social_recovery_threshold: Option<usize>,
    /// Hardware recovery support
    pub hardware_recovery: bool,
}

impl Default for RecoveryMechanism {
    fn default() -> Self {
        Self {
            recovery_phrase_length: 24, // BIP39 standard
            social_recovery_threshold: None,
            hardware_recovery: false,
        }
    }
}

/// Recovery information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryInfo {
    /// Recovery phrase
    pub recovery_phrase: Option<String>,
    /// Social recovery contacts
    pub social_recovery_contacts: Vec<String>,
    /// Hardware recovery data
    pub hardware_recovery_data: Option<Vec<u8>>,
}

/// Recovery data for key restoration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryData {
    /// Key generation being recovered
    pub generation: u32,
    /// Encrypted key materials
    pub encrypted_keys: Vec<EncryptedKeyMaterial>,
    /// Authentication tag
    pub authentication_tag: Vec<u8>,
}

/// Reason for key rotation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RotationReason {
    /// Scheduled rotation based on policy
    Scheduled,
    /// Security compromise detected
    Compromise,
    /// Manual rotation requested
    Manual,
    /// Device change or addition
    Device,
}

/// Rotation event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationEvent {
    /// When the rotation occurred
    pub timestamp: DateTime<Utc>,
    /// Reason for rotation
    pub reason: RotationReason,
    /// Key generation after rotation
    pub generation: u32,
    /// Keys that were rotated
    pub rotated_keys: Vec<String>,
}

/// Result of key rotation
#[derive(Debug, Clone)]
pub struct RotationResult {
    /// Whether rotation was performed
    pub rotated: bool,
    /// New key IDs created
    pub new_keys: Vec<String>,
    /// Deprecated key IDs
    pub deprecated_keys: Vec<String>,
    /// Updated DID document
    pub updated_document: Option<DidDocument>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_key_creation() {
        let master_key = MasterKey::new("test_password", None).unwrap();
        assert_eq!(master_key.key_bytes.len(), 32);
        assert_eq!(master_key.salt.len(), 32);
    }

    #[test]
    fn test_key_hierarchy_creation() {
        let did = Did::new("test", "example");
        let master_key = MasterKey::new("test_password", None).unwrap();
        let hierarchy = KeyHierarchy::new(did, master_key).unwrap();
        
        assert_eq!(hierarchy.current_generation, 1);
        assert!(!hierarchy.active_keys.is_empty());
        assert!(hierarchy.active_keys.contains_key("signing-1"));
        assert!(hierarchy.active_keys.contains_key("encryption-1"));
    }

    #[test]
    fn test_key_rotation() {
        let did = Did::new("test", "example");
        let master_key = MasterKey::new("test_password", None).unwrap();
        let mut hierarchy = KeyHierarchy::new(did, master_key).unwrap();
        
        let initial_keys = hierarchy.get_active_key_ids();
        hierarchy.rotate_keys(RotationReason::Manual).unwrap();
        
        assert_eq!(hierarchy.current_generation, 2);
        let rotated_keys = hierarchy.get_active_key_ids();
        
        // Keys should be different after rotation
        assert_ne!(initial_keys, rotated_keys);
        
        // Historical keys should contain old keys
        for key_id in &initial_keys {
            assert!(hierarchy.historical_keys.contains_key(key_id));
        }
    }

    #[test]
    fn test_rotation_policy() {
        let policy = RotationPolicy::default();
        let did = Did::new("test", "example");
        let master_key = MasterKey::new("test_password", None).unwrap();
        let hierarchy = KeyHierarchy::new(did, master_key).unwrap();
        
        // Fresh keys shouldn't need rotation immediately
        assert!(!policy.should_rotate_scheduled(&hierarchy));
    }
}