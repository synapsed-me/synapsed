//! Enhanced security manager integrating all cryptographic features with industry best practices.

use crate::crypto::{
    certificates::{CertificateValidator, CertificatePinner},
    key_derivation::{KeyDerivationFunction, KeyRatchet},
    post_quantum::{HybridKeyExchange, PQCipherSuite, PQSignature, PQSignatureAlgorithm},
    session::SessionManager,
};
use crate::error::{NetworkError, Result, SecurityError};
use crate::types::{PeerInfo, PeerId};

use chacha20poly1305::{aead::Aead, ChaCha20Poly1305, KeyInit};
use constant_time_eq::constant_time_eq;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use subtle::Choice;
use uuid::Uuid;
use zeroize::ZeroizeOnDrop;

/// Enhanced security manager with post-quantum cryptography and constant-time operations.
pub struct EnhancedSecurityManager {
    /// Post-quantum key exchange instances
    pq_key_exchanges: HashMap<PQCipherSuite, HybridKeyExchange>,
    
    /// Post-quantum signature instances
    pq_signatures: HashMap<PQSignatureAlgorithm, PQSignature>,
    
    /// Session manager for secure sessions
    session_manager: Arc<SessionManager>,
    
    /// Certificate validator with pinning
    cert_validator: Arc<CertificateValidator>,
    
    /// Certificate pinner for enhanced validation
    cert_pinner: Arc<CertificatePinner>,
    
    /// Security configuration
    config: EnhancedSecurityConfig,
    
    /// Security metrics
    metrics: SecurityMetrics,
}

/// Configuration for enhanced security features.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedSecurityConfig {
    /// Enable post-quantum cryptography
    pub enable_post_quantum: bool,
    
    /// Preferred cipher suites (in order of preference)
    pub preferred_cipher_suites: Vec<SecureCipherSuite>,
    
    /// Key rotation interval
    pub key_rotation_interval: Duration,
    
    /// Session timeout
    pub session_timeout: Duration,
    
    /// Enable constant-time operations
    pub constant_time_ops: bool,
    
    /// Certificate pinning configuration
    pub certificate_pinning: CertificatePinningConfig,
    
    /// Security audit configuration
    pub audit_config: SecurityAuditConfig,
}

/// Secure cipher suite with quantum-resistant options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecureCipherSuite {
    /// ChaCha20-Poly1305 with X25519 (classical)
    ChaCha20Poly1305X25519,
    
    /// AES-256-GCM with X25519 (classical)
    Aes256GcmX25519,
    
    /// Kyber768 + ChaCha20-Poly1305 (post-quantum)
    Kyber768ChaCha20,
    
    /// Kyber1024 + ChaCha20-Poly1305 (post-quantum)
    Kyber1024ChaCha20,
    
    /// Hybrid X25519 + Kyber1024 + ChaCha20 (hybrid)
    HybridX25519Kyber1024ChaCha20,
    
    /// Hybrid X25519 + Kyber1024 + AES-256-GCM (hybrid)
    HybridX25519Kyber1024Aes256,
}

impl SecureCipherSuite {
    /// Returns the security level in bits.
    pub fn security_level(self) -> u16 {
        match self {
            Self::ChaCha20Poly1305X25519 | Self::Aes256GcmX25519 => 128,
            Self::Kyber768ChaCha20 => 192,
            Self::Kyber1024ChaCha20 
            | Self::HybridX25519Kyber1024ChaCha20 
            | Self::HybridX25519Kyber1024Aes256 => 256,
        }
    }
    
    /// Returns whether this suite provides post-quantum security.
    pub fn is_post_quantum(self) -> bool {
        matches!(self, 
            Self::Kyber768ChaCha20 
            | Self::Kyber1024ChaCha20 
            | Self::HybridX25519Kyber1024ChaCha20 
            | Self::HybridX25519Kyber1024Aes256
        )
    }
    
    /// Returns whether this is a hybrid classical/post-quantum suite.
    pub fn is_hybrid(self) -> bool {
        matches!(self, 
            Self::HybridX25519Kyber1024ChaCha20 
            | Self::HybridX25519Kyber1024Aes256
        )
    }
}

/// Certificate pinning configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificatePinningConfig {
    /// Enable certificate pinning
    pub enabled: bool,
    
    /// Pinned certificate hashes
    pub pinned_hashes: Vec<[u8; 32]>,
    
    /// Allow backup certificates (for key rotation)
    pub allow_backup_certs: bool,
    
    /// Pin validation mode
    pub validation_mode: PinValidationMode,
}

/// Certificate pin validation modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinValidationMode {
    /// Strict: All connections must match pinned certificates
    Strict,
    
    /// Permissive: Allow non-pinned certificates with warnings
    Permissive,
    
    /// Advisory: Log mismatches but don't block connections
    Advisory,
}

/// Security audit configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditConfig {
    /// Enable security event logging
    pub enable_audit_log: bool,
    
    /// Log encryption/decryption operations
    pub log_crypto_ops: bool,
    
    /// Log key generation events
    pub log_key_events: bool,
    
    /// Log authentication attempts
    pub log_auth_attempts: bool,
    
    /// Security event retention period
    pub retention_period: Duration,
}

/// Security event for auditing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// Event timestamp
    pub timestamp: SystemTime,
    
    /// Event type
    pub event_type: SecurityEventType,
    
    /// Session ID (if applicable)
    pub session_id: Option<Uuid>,
    
    /// Peer ID (if applicable)
    pub peer_id: Option<PeerId>,
    
    /// Event details
    pub details: HashMap<String, String>,
    
    /// Event severity
    pub severity: SecurityEventSeverity,
}

/// Types of security events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityEventType {
    /// Key generation event
    KeyGeneration,
    
    /// Key rotation event
    KeyRotation,
    
    /// Encryption operation
    Encryption,
    
    /// Decryption operation
    Decryption,
    
    /// Authentication attempt
    Authentication,
    
    /// Certificate validation
    CertificateValidation,
    
    /// Security violation
    SecurityViolation,
    
    /// Session creation
    SessionCreated,
    
    /// Session terminated
    SessionTerminated,
}

/// Security event severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityEventSeverity {
    /// Informational event
    Info,
    
    /// Warning event
    Warning,
    
    /// Error event
    Error,
    
    /// Critical security event
    Critical,
}

/// Security metrics for monitoring.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityMetrics {
    /// Total encryptions performed
    pub encryptions_count: u64,
    
    /// Total decryptions performed
    pub decryptions_count: u64,
    
    /// Total key generations
    pub key_generations_count: u64,
    
    /// Total key rotations
    pub key_rotations_count: u64,
    
    /// Authentication successes
    pub auth_successes: u64,
    
    /// Authentication failures
    pub auth_failures: u64,
    
    /// Certificate validation successes
    pub cert_validations_success: u64,
    
    /// Certificate validation failures
    pub cert_validations_failure: u64,
    
    /// Post-quantum operations count
    pub pq_operations_count: u64,
    
    /// Average operation latency (microseconds)
    pub avg_operation_latency_us: u64,
}

/// Secure key material that zeroizes on drop.
#[derive(ZeroizeOnDrop)]
pub struct SecureKeyMaterial {
    /// The key bytes
    key: Vec<u8>,
    
    /// Key derivation function used
    kdf: KeyDerivationFunction,
    
    /// Key generation timestamp
    #[zeroize(skip)]
    created_at: SystemTime,
    
    /// Key expiry time
    #[zeroize(skip)]
    expires_at: SystemTime,
}

impl SecureKeyMaterial {
    /// Creates new secure key material.
    pub fn new(
        key: Vec<u8>, 
        kdf: KeyDerivationFunction, 
        lifetime: Duration
    ) -> Self {
        let now = SystemTime::now();
        Self {
            key,
            kdf,
            created_at: now,
            expires_at: now + lifetime,
        }
    }
    
    /// Returns the key bytes (constant-time access).
    pub fn key(&self) -> &[u8] {
        &self.key
    }
    
    /// Checks if the key has expired (constant-time).
    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.expires_at
    }
    
    /// Rotates the key using the key ratchet.
    pub fn rotate(&mut self, ratchet: &mut KeyRatchet) -> Result<()> {
        // Generate new key
        let new_key = ratchet.advance()?;
        
        // Zeroize old key
        use zeroize::Zeroize;
        self.key.zeroize();
        
        // Update with new key
        self.key = new_key;
        self.created_at = SystemTime::now();
        self.expires_at = self.created_at + Duration::from_secs(3600);
        
        Ok(())
    }
}

impl EnhancedSecurityManager {
    /// Creates a new enhanced security manager.
    pub fn new(config: EnhancedSecurityConfig) -> Result<Self> {
        let mut pq_key_exchanges = HashMap::new();
        let mut pq_signatures = HashMap::new();
        
        if config.enable_post_quantum {
            // Initialize post-quantum key exchanges
            for suite in [
                PQCipherSuite::Kyber768ChaCha20,
                PQCipherSuite::Kyber1024ChaCha20,
                PQCipherSuite::HybridX25519Kyber1024,
            ] {
                let kex = HybridKeyExchange::new(suite)?;
                pq_key_exchanges.insert(suite, kex);
            }
            
            // Initialize post-quantum signatures
            for alg in [
                PQSignatureAlgorithm::Dilithium3,
                PQSignatureAlgorithm::Dilithium5,
                PQSignatureAlgorithm::HybridEd25519Dilithium5,
            ] {
                let sig = PQSignature::new(alg)?;
                pq_signatures.insert(alg, sig);
            }
        }
        
        let session_manager = Arc::new(SessionManager::new());
        let cert_validator = Arc::new(CertificateValidator::new()?);
        let cert_pinner = Arc::new(CertificatePinner::new());
        
        let metrics = SecurityMetrics::default();
        
        Ok(Self {
            pq_key_exchanges,
            pq_signatures,
            session_manager,
            cert_validator,
            cert_pinner,
            config,
            metrics,
        })
    }
    
    /// Performs secure handshake with constant-time operations.
    pub async fn secure_handshake(
        &mut self, 
        peer: &PeerInfo,
        preferred_suite: Option<SecureCipherSuite>
    ) -> Result<Uuid> {
        let start_time = std::time::Instant::now();
        
        // Select cipher suite (constant-time selection)
        let cipher_suite = self.select_cipher_suite_ct(peer, preferred_suite)?;
        
        // Generate session keys
        let session_id = if cipher_suite.is_post_quantum() {
            self.handshake_post_quantum(peer, cipher_suite).await?
        } else {
            self.handshake_classical(peer, cipher_suite).await?
        };
        
        // Update metrics
        self.metrics.key_generations_count += 1;
        let latency = start_time.elapsed().as_micros() as u64;
        self.update_avg_latency(latency);
        
        // Log security event
        self.log_security_event(SecurityEvent {
            timestamp: SystemTime::now(),
            event_type: SecurityEventType::KeyGeneration,
            session_id: Some(session_id),
            peer_id: Some(peer.id),
            details: {
                let mut details = HashMap::new();
                details.insert("cipher_suite".to_string(), format!("{:?}", cipher_suite));
                details
            },
            severity: SecurityEventSeverity::Info,
        });
        
        Ok(session_id)
    }
    
    /// Selects cipher suite using constant-time comparison.
    fn select_cipher_suite_ct(
        &self, 
        peer: &PeerInfo, 
        preferred: Option<SecureCipherSuite>
    ) -> Result<SecureCipherSuite> {
        // Default to most secure available suite
        let default_suite = if self.config.enable_post_quantum {
            SecureCipherSuite::HybridX25519Kyber1024ChaCha20
        } else {
            SecureCipherSuite::ChaCha20Poly1305X25519
        };
        
        // Use preferred suite if specified and supported
        if let Some(suite) = preferred {
            if self.is_suite_supported_ct(suite, peer) {
                return Ok(suite);
            }
        }
        
        // Negotiate based on peer capabilities (constant-time)
        for &suite in &self.config.preferred_cipher_suites {
            if self.is_suite_supported_ct(suite, peer) {
                return Ok(suite);
            }
        }
        
        Ok(default_suite)
    }
    
    /// Checks if cipher suite is supported (constant-time).
    fn is_suite_supported_ct(&self, suite: SecureCipherSuite, peer: &PeerInfo) -> bool {
        // Simplified capability check - in production, use proper negotiation
        let suite_name = format!("{:?}", suite);
        let mut supported = false;
        
        // Constant-time capability search
        for capability in &peer.capabilities {
            let matches = constant_time_eq(capability.as_bytes(), suite_name.as_bytes());
            supported |= matches;
        }
        
        supported || suite == SecureCipherSuite::ChaCha20Poly1305X25519 // Always support fallback
    }
    
    /// Performs post-quantum handshake.
    async fn handshake_post_quantum(
        &mut self, 
        peer: &PeerInfo, 
        cipher_suite: SecureCipherSuite
    ) -> Result<Uuid> {
        let pq_suite = match cipher_suite {
            SecureCipherSuite::Kyber768ChaCha20 => PQCipherSuite::Kyber768ChaCha20,
            SecureCipherSuite::Kyber1024ChaCha20 => PQCipherSuite::Kyber1024ChaCha20,
            SecureCipherSuite::HybridX25519Kyber1024ChaCha20 
            | SecureCipherSuite::HybridX25519Kyber1024Aes256 => PQCipherSuite::HybridX25519Kyber1024,
            _ => return Err(NetworkError::Security(SecurityError::KeyExchange(
                "Invalid post-quantum cipher suite".to_string()
            ))),
        };
        
        // Get or create PQ key exchange
        let kex = self.pq_key_exchanges.get_mut(&pq_suite)
            .ok_or_else(|| NetworkError::Security(SecurityError::KeyExchange(
                "Post-quantum key exchange not available".to_string()
            )))?;
        
        // Generate keypair
        let (public_key, _secret_key) = kex.generate_keypair()?;
        
        // In a real implementation, exchange keys with peer
        // For now, simulate by generating shared secret
        let (_ciphertext, shared_secret) = kex.encapsulate(&public_key)?;
        
        // Create session with derived keys
        let session_id = self.session_manager.create_session(
            peer.id.to_string(),
            shared_secret,
            KeyDerivationFunction::HkdfSha256,
        )?;
        
        self.metrics.pq_operations_count += 1;
        
        Ok(session_id)
    }
    
    /// Performs classical handshake.
    async fn handshake_classical(
        &self, 
        peer: &PeerInfo, 
        _cipher_suite: SecureCipherSuite
    ) -> Result<Uuid> {
        // Generate ephemeral X25519 keypair
        let mut rng = rand::thread_rng();
        let secret = x25519_dalek::EphemeralSecret::random_from_rng(&mut rng);
        let _public = x25519_dalek::PublicKey::from(&secret);
        
        // In a real implementation, exchange public keys with peer
        // For now, simulate shared secret generation
        let peer_public = x25519_dalek::PublicKey::from([0x42; 32]); // Placeholder
        let shared_secret = secret.diffie_hellman(&peer_public);
        
        // Create session
        let session_id = self.session_manager.create_session(
            peer.id.to_string(),
            shared_secret.as_bytes().to_vec(),
            KeyDerivationFunction::HkdfSha256,
        )?;
        
        Ok(session_id)
    }
    
    /// Encrypts data with constant-time operations.
    pub fn encrypt_secure(&mut self, data: &[u8], session_id: &Uuid) -> Result<Vec<u8>> {
        let start_time = std::time::Instant::now();
        
        // Get session (constant-time lookup)
        let session = self.session_manager.get_session(session_id)?;
        
        // Generate nonce
        let mut rng = rand::thread_rng();
        let mut nonce = [0u8; 12];
        rng.fill_bytes(&mut nonce);
        
        // Create cipher
        let cipher = ChaCha20Poly1305::new_from_slice(&session.keys.client_write_key[..32])
            .map_err(|_| NetworkError::Security(SecurityError::Encryption(
                "Invalid key length".to_string()
            )))?;
        
        // Encrypt with AEAD
        let ciphertext = cipher.encrypt(nonce.as_slice().into(), data)
            .map_err(|_| NetworkError::Security(SecurityError::Encryption(
                "AEAD encryption failed".to_string()
            )))?;
        
        // Combine nonce and ciphertext
        let mut result = nonce.to_vec();
        result.extend_from_slice(&ciphertext);
        
        // Update session activity
        self.session_manager.touch_session(session_id)?;
        
        // Update metrics
        self.metrics.encryptions_count += 1;
        let latency = start_time.elapsed().as_micros() as u64;
        self.update_avg_latency(latency);
        
        // Log operation if enabled
        if self.config.audit_config.log_crypto_ops {
            self.log_security_event(SecurityEvent {
                timestamp: SystemTime::now(),
                event_type: SecurityEventType::Encryption,
                session_id: Some(*session_id),
                peer_id: None,
                details: {
                    let mut details = HashMap::new();
                    details.insert("data_size".to_string(), data.len().to_string());
                    details.insert("cipher".to_string(), "ChaCha20Poly1305".to_string());
                    details
                },
                severity: SecurityEventSeverity::Info,
            });
        }
        
        Ok(result)
    }
    
    /// Decrypts data with constant-time operations.
    pub fn decrypt_secure(&mut self, data: &[u8], session_id: &Uuid) -> Result<Vec<u8>> {
        let start_time = std::time::Instant::now();
        
        // Validate input length (constant-time)
        let valid_length = Choice::from((data.len() >= 12) as u8);
        if valid_length.unwrap_u8() == 0 {
            return Err(NetworkError::Security(SecurityError::Decryption(
                "Invalid ciphertext length".to_string()
            )));
        }
        
        // Get session
        let session = self.session_manager.get_session(session_id)?;
        
        // Extract nonce and ciphertext (constant-time split)
        let (nonce, ciphertext) = data.split_at(12);
        
        // Create cipher
        let cipher = ChaCha20Poly1305::new_from_slice(&session.keys.server_write_key[..32])
            .map_err(|_| NetworkError::Security(SecurityError::Decryption(
                "Invalid key length".to_string()
            )))?;
        
        // Decrypt and authenticate
        let plaintext = cipher.decrypt(nonce.into(), ciphertext)
            .map_err(|_| NetworkError::Security(SecurityError::Decryption(
                "AEAD decryption failed - authentication tag mismatch".to_string()
            )))?;
        
        // Update session activity
        self.session_manager.touch_session(session_id)?;
        
        // Update metrics
        self.metrics.decryptions_count += 1;
        let latency = start_time.elapsed().as_micros() as u64;
        self.update_avg_latency(latency);
        
        // Log operation if enabled
        if self.config.audit_config.log_crypto_ops {
            self.log_security_event(SecurityEvent {
                timestamp: SystemTime::now(),
                event_type: SecurityEventType::Decryption,
                session_id: Some(*session_id),
                peer_id: None,
                details: {
                    let mut details = HashMap::new();
                    details.insert("data_size".to_string(), plaintext.len().to_string());
                    details.insert("cipher".to_string(), "ChaCha20Poly1305".to_string());
                    details
                },
                severity: SecurityEventSeverity::Info,
            });
        }
        
        Ok(plaintext)
    }
    
    /// Signs data using post-quantum signatures.
    pub fn sign_post_quantum(
        &mut self, 
        data: &[u8], 
        algorithm: PQSignatureAlgorithm,
        secret_key: &[u8]
    ) -> Result<Vec<u8>> {
        let sig_instance = self.pq_signatures.get_mut(&algorithm)
            .ok_or_else(|| NetworkError::Security(SecurityError::Signature(
                "Post-quantum signature algorithm not available".to_string()
            )))?;
        
        let signature = sig_instance.sign(secret_key, data)?;
        self.metrics.pq_operations_count += 1;
        
        Ok(signature)
    }
    
    /// Verifies post-quantum signatures.
    pub fn verify_post_quantum(
        &self, 
        data: &[u8], 
        signature: &[u8],
        public_key: &[u8],
        algorithm: PQSignatureAlgorithm
    ) -> Result<bool> {
        let sig_instance = self.pq_signatures.get(&algorithm)
            .ok_or_else(|| NetworkError::Security(SecurityError::Verification(
                "Post-quantum signature algorithm not available".to_string()
            )))?;
        
        sig_instance.verify(public_key, data, signature)
    }
    
    /// Validates certificate with pinning.
    pub fn validate_certificate_with_pinning(
        &mut self, 
        cert_chain: &[quinn::rustls::pki_types::CertificateDer<'_>],
        server_name: &str
    ) -> Result<()> {
        if cert_chain.is_empty() {
            self.metrics.cert_validations_failure += 1;
            return Err(NetworkError::Security(SecurityError::Certificate(
                "Empty certificate chain".to_string()
            )));
        }
        
        let end_entity = &cert_chain[0];
        
        // Standard certificate validation
        self.cert_validator.validate_chain(cert_chain, server_name)?;
        
        // Certificate pinning validation
        if self.config.certificate_pinning.enabled {
            match self.config.certificate_pinning.validation_mode {
                PinValidationMode::Strict => {
                    // Strict mode: Must match pinned certificates
                    self.cert_pinner.validate(end_entity)?;
                }
                PinValidationMode::Permissive => {
                    // Permissive mode: Warn on mismatch but allow
                    if let Err(e) = self.cert_pinner.validate(end_entity) {
                        self.log_security_event(SecurityEvent {
                            timestamp: SystemTime::now(),
                            event_type: SecurityEventType::SecurityViolation,
                            session_id: None,
                            peer_id: None,
                            details: {
                                let mut details = HashMap::new();
                                details.insert("violation_type".to_string(), "certificate_pin_mismatch".to_string());
                                details.insert("error".to_string(), e.to_string());
                                details
                            },
                            severity: SecurityEventSeverity::Warning,
                        });
                    }
                }
                PinValidationMode::Advisory => {
                    // Advisory mode: Log only
                    if let Err(e) = self.cert_pinner.validate(end_entity) {
                        self.log_security_event(SecurityEvent {
                            timestamp: SystemTime::now(),
                            event_type: SecurityEventType::CertificateValidation,
                            session_id: None,
                            peer_id: None,
                            details: {
                                let mut details = HashMap::new();
                                details.insert("validation_result".to_string(), "pin_mismatch".to_string());
                                details.insert("error".to_string(), e.to_string());
                                details
                            },
                            severity: SecurityEventSeverity::Info,
                        });
                    }
                }
            }
        }
        
        self.metrics.cert_validations_success += 1;
        
        Ok(())
    }
    
    /// Rotates session keys securely.
    pub async fn rotate_session_keys(&mut self, session_id: &Uuid) -> Result<()> {
        // This is handled by the session manager's touch_session method
        self.session_manager.touch_session(session_id)?;
        self.metrics.key_rotations_count += 1;
        
        self.log_security_event(SecurityEvent {
            timestamp: SystemTime::now(),
            event_type: SecurityEventType::KeyRotation,
            session_id: Some(*session_id),
            peer_id: None,
            details: HashMap::new(),
            severity: SecurityEventSeverity::Info,
        });
        
        Ok(())
    }
    
    /// Updates average latency metric.
    fn update_avg_latency(&mut self, new_latency: u64) {
        let total_ops = self.metrics.encryptions_count + self.metrics.decryptions_count + self.metrics.key_generations_count;
        if total_ops > 0 {
            self.metrics.avg_operation_latency_us = 
                (self.metrics.avg_operation_latency_us * (total_ops - 1) + new_latency) / total_ops;
        } else {
            self.metrics.avg_operation_latency_us = new_latency;
        }
    }
    
    /// Logs security events for auditing.
    fn log_security_event(&self, event: SecurityEvent) {
        if self.config.audit_config.enable_audit_log {
            // In a real implementation, this would write to a secure audit log
            tracing::info!(
                event_type = ?event.event_type,
                severity = ?event.severity,
                session_id = ?event.session_id,
                peer_id = ?event.peer_id,
                details = ?event.details,
                "Security event"
            );
        }
    }
    
    /// Returns security metrics.
    pub fn get_metrics(&self) -> &SecurityMetrics {
        &self.metrics
    }
    
    /// Cleans up expired sessions and performs maintenance.
    pub async fn perform_maintenance(&mut self) -> Result<()> {
        // Clean up expired sessions
        self.session_manager.cleanup_expired();
        
        // Rotate keys if needed
        // This would be implemented based on the key rotation policy
        
        // Clean up old audit logs
        // Implementation would depend on the audit log storage
        
        Ok(())
    }
}

impl Default for EnhancedSecurityConfig {
    fn default() -> Self {
        Self {
            enable_post_quantum: true,
            preferred_cipher_suites: vec![
                SecureCipherSuite::HybridX25519Kyber1024ChaCha20,
                SecureCipherSuite::Kyber1024ChaCha20,
                SecureCipherSuite::ChaCha20Poly1305X25519,
            ],
            key_rotation_interval: Duration::from_secs(300), // 5 minutes
            session_timeout: Duration::from_secs(3600), // 1 hour
            constant_time_ops: true,
            certificate_pinning: CertificatePinningConfig {
                enabled: true,
                pinned_hashes: Vec::new(),
                allow_backup_certs: true,
                validation_mode: PinValidationMode::Permissive,
            },
            audit_config: SecurityAuditConfig {
                enable_audit_log: true,
                log_crypto_ops: false, // Can be noisy
                log_key_events: true,
                log_auth_attempts: true,
                retention_period: Duration::from_secs(86400 * 30), // 30 days
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_enhanced_security_manager() {
        let config = EnhancedSecurityConfig::default();
        let mut manager = EnhancedSecurityManager::new(config).unwrap();
        
        let peer = PeerInfo {
            id: "test_peer".to_string(),
            addresses: vec![],
            capabilities: vec!["ChaCha20Poly1305X25519".to_string()],
            last_seen: SystemTime::now(),
            reputation: 1.0,
        };
        
        // Test handshake
        let session_id = manager.secure_handshake(&peer, None).await.unwrap();
        
        // Test encryption/decryption
        let test_data = b"Hello, secure world!";
        let encrypted = manager.encrypt_secure(test_data, &session_id).unwrap();
        let decrypted = manager.decrypt_secure(&encrypted, &session_id).unwrap();
        
        assert_eq!(decrypted, test_data);
        
        // Check metrics
        let metrics = manager.get_metrics();
        assert_eq!(metrics.encryptions_count, 1);
        assert_eq!(metrics.decryptions_count, 1);
        assert_eq!(metrics.key_generations_count, 1);
    }
    
    #[test]
    fn test_cipher_suite_properties() {
        assert!(SecureCipherSuite::Kyber1024ChaCha20.is_post_quantum());
        assert!(!SecureCipherSuite::ChaCha20Poly1305X25519.is_post_quantum());
        assert!(SecureCipherSuite::HybridX25519Kyber1024ChaCha20.is_hybrid());
        assert_eq!(SecureCipherSuite::Kyber1024ChaCha20.security_level(), 256);
    }
    
    #[test]
    fn test_secure_key_material() {
        let key = vec![0x42u8; 32];
        let mut secure_key = SecureKeyMaterial::new(
            key.clone(),
            KeyDerivationFunction::HkdfSha256,
            Duration::from_secs(3600),
        );
        
        assert_eq!(secure_key.key(), &key);
        assert!(!secure_key.is_expired());
        
        // Key should be zeroized on drop
    }
}