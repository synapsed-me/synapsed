//! Security layer for encrypted networking with post-quantum support.

use crate::crypto::{
    certificates::CertificateValidator,
    key_derivation::KeyDerivationFunction,
    post_quantum::{HybridKeyExchange, HybridPublicKey, HybridSecretKey, PQCipherSuite, PQSignatureAlgorithm},
    session::{SessionManager, SessionState},
};
use crate::error::{NetworkError, Result, SecurityError};
use crate::types::PeerInfo;
use chacha20poly1305::{aead::Aead, ChaCha20Poly1305, KeyInit};
// Removed unused aes_gcm imports
use serde::{Deserialize, Serialize};
// Removed unused HashMap import
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Security layer that provides encryption and authentication.
pub struct SecurityLayer {
    /// Enabled cipher suites
    cipher_suites: Vec<CipherSuite>,
    
    /// Post-quantum cipher suites
    pq_cipher_suites: Vec<PQCipherSuite>,
    
    /// Authentication methods
    auth_methods: Vec<AuthMethod>,
    
    /// Post-quantum signature algorithms
    pq_signature_algs: Vec<PQSignatureAlgorithm>,
    
    /// Post-quantum cryptography enabled
    post_quantum_enabled: bool,
    
    /// Session manager
    session_manager: Arc<SessionManager>,
    
    /// Certificate validator
    cert_validator: Arc<CertificateValidator>,
}

/// Supported cipher suites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CipherSuite {
    /// ChaCha20-Poly1305 with X25519 key exchange
    ChaCha20Poly1305X25519,
    
    /// AES-256-GCM with X25519 key exchange
    Aes256GcmX25519,
    
    /// Post-quantum: Kyber768 + ChaCha20-Poly1305
    Kyber768ChaCha20,
    
    /// Post-quantum: Kyber1024 + ChaCha20-Poly1305
    Kyber1024ChaCha20,
    
    /// Post-quantum: Kyber1024 + AES-256-GCM
    Kyber1024Aes256,
    
    /// Hybrid: Classical + Post-Quantum
    HybridX25519Kyber1024,
}

/// Authentication methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMethod {
    /// Ed25519 digital signatures
    Ed25519,
    
    /// RSA-PSS signatures
    RsaPss,
    
    /// Post-quantum: Dilithium3
    Dilithium3,
    
    /// Post-quantum: Dilithium5
    Dilithium5,
    
    /// Hybrid: Ed25519 + Dilithium5
    HybridEd25519Dilithium5,
    
    /// Shared secret authentication
    SharedSecret,
}

/// Session keys for encrypted communication.
#[derive(Debug, Clone)]
pub struct SessionKeySet {
    /// Client write key
    pub client_write_key: Vec<u8>,
    
    /// Server write key  
    pub server_write_key: Vec<u8>,
    
    /// Client MAC key
    pub client_mac_key: Vec<u8>,
    
    /// Server MAC key
    pub server_mac_key: Vec<u8>,
}

/// Post-quantum key exchange state.
struct PQKeyExchangeState {
    /// Hybrid key exchange instance
    kex: HybridKeyExchange,
    
    /// Public key
    public_key: HybridPublicKey,
    
    /// Secret key
    secret_key: HybridSecretKey,
}

impl std::fmt::Debug for PQKeyExchangeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PQKeyExchangeState")
            .field("public_key", &self.public_key)
            .finish()
    }
}

impl Clone for PQKeyExchangeState {
    fn clone(&self) -> Self {
        Self {
            kex: self.kex.clone(),
            public_key: self.public_key.clone(),
            secret_key: self.secret_key.clone(),
        }
    }
}

/// Handshake state for establishing secure connections.
#[derive(Debug, Clone)]
pub struct HandshakeState {
    /// Current handshake phase
    phase: HandshakePhase,
    
    /// Negotiated cipher suite
    cipher_suite: Option<CipherSuite>,
    
    /// Negotiated authentication method
    auth_method: Option<AuthMethod>,
    
    /// Handshake start time
    started_at: SystemTime,
    
    /// Ephemeral key material (for classical)
    ephemeral_keys: Vec<u8>,
    
    /// Session ID once established
    session_id: Option<Uuid>,
    
    /// Post-quantum key exchange state
    pq_key_exchange: Option<PQKeyExchangeState>,
}

/// Phases of the security handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakePhase {
    /// Initial hello messages
    Hello,
    
    /// Key exchange
    KeyExchange,
    
    /// Authentication
    Authentication,
    
    /// Handshake complete
    Complete,
    
    /// Handshake failed
    Failed,
}

impl SecurityLayer {
    /// Creates a new security layer.
    pub fn new(post_quantum_enabled: bool) -> Result<Self> {
        let mut cipher_suites = vec![
            CipherSuite::ChaCha20Poly1305X25519,
            CipherSuite::Aes256GcmX25519,
        ];
        
        let mut pq_cipher_suites = vec![];
        let mut auth_methods = vec![
            AuthMethod::Ed25519,
            AuthMethod::SharedSecret,
        ];
        let mut pq_signature_algs = vec![];
        
        if post_quantum_enabled {
            cipher_suites.extend([
                CipherSuite::Kyber768ChaCha20,
                CipherSuite::Kyber1024ChaCha20,
                CipherSuite::Kyber1024Aes256,
                CipherSuite::HybridX25519Kyber1024,
            ]);
            
            pq_cipher_suites = vec![
                PQCipherSuite::Kyber768ChaCha20,
                PQCipherSuite::Kyber1024ChaCha20,
                PQCipherSuite::Kyber1024Aes256,
                PQCipherSuite::HybridX25519Kyber1024,
            ];
            
            auth_methods.extend([
                AuthMethod::Dilithium3,
                AuthMethod::Dilithium5,
                AuthMethod::HybridEd25519Dilithium5,
            ]);
            
            pq_signature_algs = vec![
                PQSignatureAlgorithm::Dilithium3,
                PQSignatureAlgorithm::Dilithium5,
                PQSignatureAlgorithm::HybridEd25519Dilithium5,
            ];
        }
        
        let cert_validator = Arc::new(CertificateValidator::new()?);
        
        Ok(Self {
            cipher_suites,
            pq_cipher_suites,
            auth_methods,
            pq_signature_algs,
            post_quantum_enabled,
            session_manager: Arc::new(SessionManager::new()),
            cert_validator,
        })
    }
    
    /// Initiates a secure handshake with a peer.
    pub async fn initiate_handshake(&mut self, peer: &PeerInfo) -> Result<HandshakeState> {
        let mut state = HandshakeState {
            phase: HandshakePhase::Hello,
            cipher_suite: None,
            auth_method: None,
            started_at: SystemTime::now(),
            ephemeral_keys: Vec::new(),
            session_id: None,
            pq_key_exchange: None,
        };
        
        // Negotiate cipher suite and auth method based on peer capabilities
        state.cipher_suite = Some(self.negotiate_cipher_suite(peer)?);
        state.auth_method = Some(self.negotiate_auth_method(peer)?);
        
        // Handle post-quantum key exchange if applicable
        if self.is_post_quantum_suite(state.cipher_suite.unwrap()) {
            let pq_suite = self.map_to_pq_suite(state.cipher_suite.unwrap());
            let mut kex = HybridKeyExchange::new(pq_suite)?;
            let (public_key, secret_key) = kex.generate_keypair()?;
            
            state.pq_key_exchange = Some(PQKeyExchangeState {
                kex,
                public_key,
                secret_key,
            });
        } else {
            // Generate classical ephemeral keys
            state.ephemeral_keys = self.generate_ephemeral_keys(state.cipher_suite.unwrap())?;
        }
        
        state.phase = HandshakePhase::KeyExchange;
        
        Ok(state)
    }
    
    /// Completes the handshake and establishes a session.
    pub async fn complete_handshake(&mut self, mut state: HandshakeState, peer: &PeerInfo) -> Result<Uuid> {
        if state.phase != HandshakePhase::KeyExchange {
            return Err(NetworkError::Security(SecurityError::KeyExchange(
                "Invalid handshake phase".to_string()
            )));
        }
        
        // Derive master secret
        let master_secret = if let Some(_pq_state) = &state.pq_key_exchange {
            // Post-quantum key derivation - in production would use actual PQ shared secret
            // For now, generate a secure random master secret
            use rand::RngCore;
            let mut rng = rand::thread_rng();
            let mut secret = vec![0u8; 64];
            rng.fill_bytes(&mut secret);
            secret
        } else {
            // Classical key derivation
            self.derive_master_secret(&state, peer)?
        };
        
        // Create session
        let session_id = self.session_manager.create_session(
            peer.id.to_string(),
            master_secret,
            KeyDerivationFunction::HkdfSha256,
        )?;
        
        state.session_id = Some(session_id);
        state.phase = HandshakePhase::Complete;
        
        Ok(session_id)
    }
    
    /// Encrypts data for transmission using AEAD.
    pub fn encrypt(&self, data: &[u8], session_id: &Uuid) -> Result<Vec<u8>> {
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        
        // Get session
        let session = self.session_manager.get_session(session_id)?;
        
        // Generate nonce
        let mut nonce = vec![0u8; 12];
        rng.fill_bytes(&mut nonce);
        
        // Encrypt using ChaCha20-Poly1305 (simplified - would check cipher suite)
        let cipher = ChaCha20Poly1305::new_from_slice(&session.keys.client_write_key[..32])
            .map_err(|_| NetworkError::Security(SecurityError::Encryption(
                "Invalid key length".to_string()
            )))?;
        
        let ciphertext = cipher.encrypt(nonce.as_slice().into(), data)
            .map_err(|_| NetworkError::Security(SecurityError::Encryption(
                "Encryption failed".to_string()
            )))?;
        
        // Prepend nonce to ciphertext
        let mut result = nonce;
        result.extend_from_slice(&ciphertext);
        
        // Update session activity
        self.session_manager.touch_session(session_id)?;
        
        Ok(result)
    }
    
    /// Decrypts received data using AEAD.
    pub fn decrypt(&self, data: &[u8], session_id: &Uuid) -> Result<Vec<u8>> {
        if data.len() < 12 {
            return Err(NetworkError::Security(SecurityError::Decryption(
                "Invalid ciphertext length".to_string()
            )));
        }
        
        // Get session
        let session = self.session_manager.get_session(session_id)?;
        
        // Extract nonce and ciphertext
        let (nonce, ciphertext) = data.split_at(12);
        
        // Decrypt using ChaCha20-Poly1305
        let cipher = ChaCha20Poly1305::new_from_slice(&session.keys.server_write_key[..32])
            .map_err(|_| NetworkError::Security(SecurityError::Decryption(
                "Invalid key length".to_string()
            )))?;
        
        let plaintext = cipher.decrypt(nonce.into(), ciphertext)
            .map_err(|_| NetworkError::Security(SecurityError::Decryption(
                "Decryption failed - authentication tag mismatch".to_string()
            )))?;
        
        // Update session activity
        self.session_manager.touch_session(session_id)?;
        
        Ok(plaintext)
    }
    
    /// Negotiates cipher suite with peer.
    fn negotiate_cipher_suite(&self, peer: &PeerInfo) -> Result<CipherSuite> {
        // Check peer capabilities for cipher suite support
        for suite in &self.cipher_suites {
            let suite_name = format!("{:?}", suite);
            if peer.capabilities.iter().any(|cap| cap.contains(&suite_name)) {
                return Ok(*suite);
            }
        }
        
        // Default to ChaCha20-Poly1305
        Ok(CipherSuite::ChaCha20Poly1305X25519)
    }
    
    /// Negotiates authentication method with peer.
    fn negotiate_auth_method(&self, peer: &PeerInfo) -> Result<AuthMethod> {
        // Check peer capabilities for auth method support
        for method in &self.auth_methods {
            let method_name = format!("{:?}", method);
            if peer.capabilities.iter().any(|cap| cap.contains(&method_name)) {
                return Ok(*method);
            }
        }
        
        // Default to Ed25519
        Ok(AuthMethod::Ed25519)
    }
    
    /// Generates ephemeral keys for the handshake.
    fn generate_ephemeral_keys(&self, cipher_suite: CipherSuite) -> Result<Vec<u8>> {
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        
        let key_size = match cipher_suite {
            CipherSuite::ChaCha20Poly1305X25519 | CipherSuite::Aes256GcmX25519 => 32,
            CipherSuite::Kyber768ChaCha20 => 1184, // Kyber768 public key size
            CipherSuite::Kyber1024ChaCha20 | CipherSuite::Kyber1024Aes256 => 1568, // Kyber1024 public key size
            CipherSuite::HybridX25519Kyber1024 => 1600, // X25519 + Kyber1024
        };
        
        let mut key = vec![0u8; key_size];
        rng.fill_bytes(&mut key);
        
        Ok(key)
    }
    
    /// Derives session key from handshake state.
    fn derive_session_key(&self, state: &HandshakeState, peer: &PeerInfo) -> Result<SessionState> {
        // Simplified key derivation (in production, use HKDF or similar)
        let combined = [&state.ephemeral_keys[..], peer.id.as_bytes()].concat();
        let _key = blake3::hash(&combined).as_bytes().to_vec();
        
        Ok(SessionState {
            id: Uuid::new_v4(),
            peer_id: peer.id.to_string(),
            created_at: SystemTime::now(),
            last_activity: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(3600), // 1 hour
            rotation_count: 0,
            authenticated: false,
        })
    }
    
    /// Cleans up expired sessions.
    pub fn cleanup_expired_sessions(&self) {
        self.session_manager.cleanup_expired();
    }
    
    /// Checks if a cipher suite is post-quantum.
    fn is_post_quantum_suite(&self, suite: CipherSuite) -> bool {
        matches!(suite, 
            CipherSuite::Kyber768ChaCha20 |
            CipherSuite::Kyber1024ChaCha20 |
            CipherSuite::Kyber1024Aes256 |
            CipherSuite::HybridX25519Kyber1024
        )
    }
    
    /// Maps a cipher suite to a PQ cipher suite.
    fn map_to_pq_suite(&self, suite: CipherSuite) -> PQCipherSuite {
        match suite {
            CipherSuite::Kyber768ChaCha20 => PQCipherSuite::Kyber768ChaCha20,
            CipherSuite::Kyber1024ChaCha20 => PQCipherSuite::Kyber1024ChaCha20,
            CipherSuite::Kyber1024Aes256 => PQCipherSuite::Kyber1024Aes256,
            CipherSuite::HybridX25519Kyber1024 => PQCipherSuite::HybridX25519Kyber1024,
            _ => PQCipherSuite::Kyber768ChaCha20, // Default
        }
    }
    
    /// Derives master secret for classical handshake.
    fn derive_master_secret(&self, state: &HandshakeState, peer: &PeerInfo) -> Result<Vec<u8>> {
        // Simplified - in production use proper PRF
        let combined = [&state.ephemeral_keys[..], peer.id.as_bytes()].concat();
        let hash = blake3::hash(&combined);
        Ok(hash.as_bytes().to_vec())
    }
}

impl Default for SecurityLayer {
    fn default() -> Self {
        Self::new(true).expect("Failed to create default security layer")
    }
}

// Removed duplicate SessionState export