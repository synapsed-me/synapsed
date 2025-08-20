//! CPU fallback implementation for cryptographic operations.

use std::sync::Arc;
use rayon::prelude::*;
use tracing::{debug, info};

use crate::{Result, GpuError};

/// CPU fallback implementation for cryptographic operations.
#[derive(Debug)]
pub struct CryptoFallback {
    // Configuration could be added here if needed
}

impl CryptoFallback {
    /// Create a new crypto fallback processor.
    pub fn new() -> Self {
        info!("Creating cryptographic CPU fallback processor");
        
        Self {}
    }

    /// Batch hash computation on CPU.
    pub async fn batch_hash(
        &self,
        algorithm: &str,
        data: &[u8],
        batch_size: u32,
    ) -> Result<Vec<u8>> {
        debug!("Starting {} CPU batch hash for {} operations", algorithm, batch_size);

        let chunk_size = data.len() / batch_size as usize;
        if chunk_size == 0 {
            return Err(GpuError::FallbackError {
                message: "Insufficient data for batch size".to_string(),
            });
        }

        let results: Result<Vec<_>> = data
            .par_chunks(chunk_size)
            .take(batch_size as usize)
            .map(|chunk| self.single_hash(algorithm, chunk))
            .collect();

        let hash_results = results?;
        let mut output = Vec::new();
        for hash in hash_results {
            output.extend_from_slice(&hash);
        }

        info!("Completed {} CPU batch hash", algorithm);
        Ok(output)
    }

    /// Batch encryption on CPU.
    pub async fn batch_encrypt(
        &self,
        algorithm: &str,
        data: &[u8],
        keys: &[u8],
        batch_size: u32,
    ) -> Result<Vec<u8>> {
        debug!("Starting {} CPU batch encrypt for {} operations", algorithm, batch_size);

        let data_chunk_size = data.len() / batch_size as usize;
        let key_size = self.get_key_size(algorithm)?;
        
        if keys.len() != batch_size as usize * key_size {
            return Err(GpuError::FallbackError {
                message: "Key data size mismatch".to_string(),
            });
        }

        let results: Result<Vec<_>> = (0..batch_size)
            .into_par_iter()
            .map(|i| {
                let data_start = i as usize * data_chunk_size;
                let data_end = data_start + data_chunk_size;
                let data_chunk = &data[data_start..data_end];
                
                let key_start = i as usize * key_size;
                let key_end = key_start + key_size;
                let key_chunk = &keys[key_start..key_end];
                
                self.single_encrypt(algorithm, data_chunk, key_chunk)
            })
            .collect();

        let encrypt_results = results?;
        let mut output = Vec::new();
        for ciphertext in encrypt_results {
            output.extend_from_slice(&ciphertext);
        }

        info!("Completed {} CPU batch encrypt", algorithm);
        Ok(output)
    }

    /// Batch decryption on CPU.
    pub async fn batch_decrypt(
        &self,
        algorithm: &str,
        data: &[u8],
        keys: &[u8],
        batch_size: u32,
    ) -> Result<Vec<u8>> {
        debug!("Starting {} CPU batch decrypt for {} operations", algorithm, batch_size);

        let data_chunk_size = data.len() / batch_size as usize;
        let key_size = self.get_key_size(algorithm)?;
        
        if keys.len() != batch_size as usize * key_size {
            return Err(GpuError::FallbackError {
                message: "Key data size mismatch".to_string(),
            });
        }

        let results: Result<Vec<_>> = (0..batch_size)
            .into_par_iter()
            .map(|i| {
                let data_start = i as usize * data_chunk_size;
                let data_end = data_start + data_chunk_size;
                let data_chunk = &data[data_start..data_end];
                
                let key_start = i as usize * key_size;
                let key_end = key_start + key_size;
                let key_chunk = &keys[key_start..key_end];
                
                self.single_decrypt(algorithm, data_chunk, key_chunk)
            })
            .collect();

        let decrypt_results = results?;
        let mut output = Vec::new();
        for plaintext in decrypt_results {
            output.extend_from_slice(&plaintext);
        }

        info!("Completed {} CPU batch decrypt", algorithm);
        Ok(output)
    }

    /// Batch signature generation on CPU.
    pub async fn batch_sign(
        &self,
        algorithm: &str,
        messages: &[u8],
        private_keys: &[u8],
        batch_size: u32,
    ) -> Result<Vec<u8>> {
        debug!("Starting {} CPU batch sign for {} operations", algorithm, batch_size);

        let msg_chunk_size = messages.len() / batch_size as usize;
        let key_size = self.get_private_key_size(algorithm)?;
        
        if private_keys.len() != batch_size as usize * key_size {
            return Err(GpuError::FallbackError {
                message: "Private key data size mismatch".to_string(),
            });
        }

        let results: Result<Vec<_>> = (0..batch_size)
            .into_par_iter()
            .map(|i| {
                let msg_start = i as usize * msg_chunk_size;
                let msg_end = msg_start + msg_chunk_size;
                let msg_chunk = &messages[msg_start..msg_end];
                
                let key_start = i as usize * key_size;
                let key_end = key_start + key_size;
                let key_chunk = &private_keys[key_start..key_end];
                
                self.single_sign(algorithm, msg_chunk, key_chunk)
            })
            .collect();

        let sign_results = results?;
        let mut output = Vec::new();
        for signature in sign_results {
            output.extend_from_slice(&signature);
        }

        info!("Completed {} CPU batch sign", algorithm);
        Ok(output)
    }

    /// Batch signature verification on CPU.
    pub async fn batch_verify(
        &self,
        algorithm: &str,
        messages: &[u8],
        signatures: &[u8],
        public_keys: &[u8],
        batch_size: u32,
    ) -> Result<Vec<bool>> {
        debug!("Starting {} CPU batch verify for {} operations", algorithm, batch_size);

        let msg_chunk_size = messages.len() / batch_size as usize;
        let sig_size = self.get_signature_size(algorithm)?;
        let key_size = self.get_public_key_size(algorithm)?;
        
        if signatures.len() != batch_size as usize * sig_size {
            return Err(GpuError::FallbackError {
                message: "Signature data size mismatch".to_string(),
            });
        }
        
        if public_keys.len() != batch_size as usize * key_size {
            return Err(GpuError::FallbackError {
                message: "Public key data size mismatch".to_string(),
            });
        }

        let results: Vec<bool> = (0..batch_size)
            .into_par_iter()
            .map(|i| {
                let msg_start = i as usize * msg_chunk_size;
                let msg_end = msg_start + msg_chunk_size;
                let msg_chunk = &messages[msg_start..msg_end];
                
                let sig_start = i as usize * sig_size;
                let sig_end = sig_start + sig_size;
                let sig_chunk = &signatures[sig_start..sig_end];
                
                let key_start = i as usize * key_size;
                let key_end = key_start + key_size;
                let key_chunk = &public_keys[key_start..key_end];
                
                self.single_verify(algorithm, msg_chunk, sig_chunk, key_chunk)
                    .unwrap_or(false)
            })
            .collect();

        info!("Completed {} CPU batch verify", algorithm);
        Ok(results)
    }

    // Single operation implementations

    fn single_hash(&self, algorithm: &str, data: &[u8]) -> Result<Vec<u8>> {
        match algorithm {
            "sha256" => Ok(self.sha256(data)),
            "sha3-256" => Ok(self.sha3_256(data)),
            "blake2b" => Ok(self.blake2b(data)),
            _ => Err(GpuError::FallbackError {
                message: format!("Unsupported hash algorithm: {}", algorithm),
            }),
        }
    }

    fn single_encrypt(&self, algorithm: &str, data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        match algorithm {
            "aes-256-gcm" => self.aes_256_gcm_encrypt(data, key),
            "chacha20-poly1305" => self.chacha20_poly1305_encrypt(data, key),
            _ => Err(GpuError::FallbackError {
                message: format!("Unsupported encryption algorithm: {}", algorithm),
            }),
        }
    }

    fn single_decrypt(&self, algorithm: &str, data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        match algorithm {
            "aes-256-gcm" => self.aes_256_gcm_decrypt(data, key),
            "chacha20-poly1305" => self.chacha20_poly1305_decrypt(data, key),
            _ => Err(GpuError::FallbackError {
                message: format!("Unsupported decryption algorithm: {}", algorithm),
            }),
        }
    }

    fn single_sign(&self, algorithm: &str, message: &[u8], private_key: &[u8]) -> Result<Vec<u8>> {
        match algorithm {
            "ed25519" => self.ed25519_sign(message, private_key),
            _ => Err(GpuError::FallbackError {
                message: format!("Unsupported signature algorithm: {}", algorithm),
            }),
        }
    }

    fn single_verify(
        &self,
        algorithm: &str,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool> {
        match algorithm {
            "ed25519" => self.ed25519_verify(message, signature, public_key),
            _ => Err(GpuError::FallbackError {
                message: format!("Unsupported verification algorithm: {}", algorithm),
            }),
        }
    }

    // Cryptographic implementations (simplified for testing)

    fn sha256(&self, data: &[u8]) -> Vec<u8> {
        // Simplified SHA-256 (in practice, use a proper crypto library)
        let mut hash = vec![0u8; 32];
        let mut state = 0x6a09e667u32;
        
        for &byte in data {
            state ^= byte as u32;
            state = state.wrapping_mul(1103515245).wrapping_add(12345);
        }
        
        for i in 0..8 {
            let word = state.wrapping_add(i * 0x9e3779b9);
            hash[i * 4] = (word >> 24) as u8;
            hash[i * 4 + 1] = (word >> 16) as u8;
            hash[i * 4 + 2] = (word >> 8) as u8;
            hash[i * 4 + 3] = word as u8;
        }
        
        hash
    }

    fn sha3_256(&self, data: &[u8]) -> Vec<u8> {
        // Simplified SHA3-256
        let mut hash = vec![0u8; 32];
        let mut state = 0xcc9e2d51u32;
        
        for &byte in data {
            state ^= byte as u32;
            state = state.wrapping_mul(0x85ebca6b).wrapping_add(0xc2b2ae35);
        }
        
        for i in 0..8 {
            let word = state.wrapping_add(i * 0x27d4eb2d);
            hash[i * 4] = (word >> 24) as u8;
            hash[i * 4 + 1] = (word >> 16) as u8;
            hash[i * 4 + 2] = (word >> 8) as u8;
            hash[i * 4 + 3] = word as u8;
        }
        
        hash
    }

    fn blake2b(&self, data: &[u8]) -> Vec<u8> {
        // Simplified BLAKE2b
        let mut hash = vec![0u8; 64];
        let mut state = 0x6a09e667f3bcc908u64;
        
        for &byte in data {
            state ^= byte as u64;
            state = state.wrapping_mul(0x9e3779b97f4a7c15);
        }
        
        for i in 0..8 {
            let word = state.wrapping_add(i * 0x85ebca6b);
            for j in 0..8 {
                hash[i * 8 + j] = (word >> (j * 8)) as u8;
            }
        }
        
        hash
    }

    fn aes_256_gcm_encrypt(&self, data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        if key.len() != 32 {
            return Err(GpuError::FallbackError {
                message: "AES-256 requires 32-byte key".to_string(),
            });
        }

        // Simplified AES-256-GCM (XOR with key-derived stream)
        let mut ciphertext = Vec::with_capacity(data.len() + 16); // +16 for GCM tag
        let mut keystream_state = self.derive_keystream_state(key);
        
        for &byte in data {
            keystream_state = keystream_state.wrapping_mul(1103515245).wrapping_add(12345);
            ciphertext.push(byte ^ (keystream_state as u8));
        }
        
        // Add mock GCM tag
        for _ in 0..16 {
            keystream_state = keystream_state.wrapping_mul(1103515245).wrapping_add(12345);
            ciphertext.push((keystream_state as u8));
        }
        
        Ok(ciphertext)
    }

    fn aes_256_gcm_decrypt(&self, data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        if key.len() != 32 {
            return Err(GpuError::FallbackError {
                message: "AES-256 requires 32-byte key".to_string(),
            });
        }
        
        if data.len() < 16 {
            return Err(GpuError::FallbackError {
                message: "Invalid ciphertext length".to_string(),
            });
        }

        // Remove GCM tag and decrypt
        let ciphertext = &data[..data.len() - 16];
        let mut plaintext = Vec::with_capacity(ciphertext.len());
        let mut keystream_state = self.derive_keystream_state(key);
        
        for &byte in ciphertext {
            keystream_state = keystream_state.wrapping_mul(1103515245).wrapping_add(12345);
            plaintext.push(byte ^ (keystream_state as u8));
        }
        
        Ok(plaintext)
    }

    fn chacha20_poly1305_encrypt(&self, data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        if key.len() != 32 {
            return Err(GpuError::FallbackError {
                message: "ChaCha20 requires 32-byte key".to_string(),
            });
        }

        // Simplified ChaCha20-Poly1305
        let mut ciphertext = Vec::with_capacity(data.len() + 16);
        let mut state = self.derive_keystream_state(key);
        
        for &byte in data {
            state = self.chacha20_round(state);
            ciphertext.push(byte ^ (state as u8));
        }
        
        // Add mock Poly1305 tag
        for _ in 0..16 {
            state = self.chacha20_round(state);
            ciphertext.push(state as u8);
        }
        
        Ok(ciphertext)
    }

    fn chacha20_poly1305_decrypt(&self, data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
        if key.len() != 32 {
            return Err(GpuError::FallbackError {
                message: "ChaCha20 requires 32-byte key".to_string(),
            });
        }
        
        if data.len() < 16 {
            return Err(GpuError::FallbackError {
                message: "Invalid ciphertext length".to_string(),
            });
        }

        let ciphertext = &data[..data.len() - 16];
        let mut plaintext = Vec::with_capacity(ciphertext.len());
        let mut state = self.derive_keystream_state(key);
        
        for &byte in ciphertext {
            state = self.chacha20_round(state);
            plaintext.push(byte ^ (state as u8));
        }
        
        Ok(plaintext)
    }

    fn ed25519_sign(&self, message: &[u8], private_key: &[u8]) -> Result<Vec<u8>> {
        if private_key.len() != 32 {
            return Err(GpuError::FallbackError {
                message: "Ed25519 requires 32-byte private key".to_string(),
            });
        }

        // Simplified Ed25519 signature
        let mut signature = vec![0u8; 64];
        let mut state = self.derive_keystream_state(private_key);
        
        // Hash message with private key
        for &byte in message {
            state ^= byte as u32;
            state = state.wrapping_mul(0x85ebca6b).wrapping_add(0xc2b2ae35);
        }
        
        // Generate signature bytes
        for i in 0..64 {
            state = state.wrapping_mul(1103515245).wrapping_add(12345);
            signature[i] = (state >> (i % 32)) as u8;
        }
        
        Ok(signature)
    }

    fn ed25519_verify(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool> {
        if signature.len() != 64 {
            return Ok(false);
        }
        
        if public_key.len() != 32 {
            return Ok(false);
        }

        // Simplified verification - check if signature could have been generated
        let mut expected_signature = vec![0u8; 64];
        let mut state = self.derive_keystream_state(public_key);
        
        for &byte in message {
            state ^= byte as u32;
            state = state.wrapping_mul(0x85ebca6b).wrapping_add(0xc2b2ae35);
        }
        
        for i in 0..64 {
            state = state.wrapping_mul(1103515245).wrapping_add(12345);
            expected_signature[i] = (state >> (i % 32)) as u8;
        }
        
        // Simple comparison (in practice, would use constant-time comparison)
        Ok(signature == expected_signature)
    }

    // Helper methods

    fn derive_keystream_state(&self, key: &[u8]) -> u32 {
        let mut state = 0x61707865u32; // "expa" in little endian
        for &byte in key {
            state ^= byte as u32;
            state = state.wrapping_mul(1103515245).wrapping_add(12345);
        }
        state
    }

    fn chacha20_round(&self, state: u32) -> u32 {
        let mut x = state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        x
    }

    fn get_key_size(&self, algorithm: &str) -> Result<usize> {
        match algorithm {
            "aes-256-gcm" => Ok(32),
            "chacha20-poly1305" => Ok(32),
            _ => Err(GpuError::FallbackError {
                message: format!("Unknown key size for algorithm: {}", algorithm),
            }),
        }
    }

    fn get_private_key_size(&self, algorithm: &str) -> Result<usize> {
        match algorithm {
            "ed25519" => Ok(32),
            _ => Err(GpuError::FallbackError {
                message: format!("Unknown private key size for algorithm: {}", algorithm),
            }),
        }
    }

    fn get_public_key_size(&self, algorithm: &str) -> Result<usize> {
        match algorithm {
            "ed25519" => Ok(32),
            _ => Err(GpuError::FallbackError {
                message: format!("Unknown public key size for algorithm: {}", algorithm),
            }),
        }
    }

    fn get_signature_size(&self, algorithm: &str) -> Result<usize> {
        match algorithm {
            "ed25519" => Ok(64),
            _ => Err(GpuError::FallbackError {
                message: format!("Unknown signature size for algorithm: {}", algorithm),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_crypto_fallback_creation() {
        let fallback = CryptoFallback::new();
        // Just verify it doesn't panic
    }

    #[tokio::test]
    async fn test_single_hash_operations() {
        let fallback = CryptoFallback::new();
        let data = b"hello world";
        
        let sha256_result = fallback.single_hash("sha256", data).unwrap();
        assert_eq!(sha256_result.len(), 32);
        
        let sha3_result = fallback.single_hash("sha3-256", data).unwrap();
        assert_eq!(sha3_result.len(), 32);
        
        let blake2b_result = fallback.single_hash("blake2b", data).unwrap();
        assert_eq!(blake2b_result.len(), 64);
        
        // Test unsupported algorithm
        let result = fallback.single_hash("unknown", data);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_batch_hash() {
        let fallback = CryptoFallback::new();
        let data = vec![42u8; 1024]; // 1KB of data
        let batch_size = 4;
        
        let result = fallback.batch_hash("sha256", &data, batch_size).await.unwrap();
        assert_eq!(result.len(), 4 * 32); // 4 hashes of 32 bytes each
    }

    #[tokio::test]
    async fn test_encryption_decryption() {
        let fallback = CryptoFallback::new();
        let plaintext = b"secret message";
        let key = vec![1u8; 32]; // 32-byte key
        
        // Test AES-256-GCM
        let ciphertext = fallback.single_encrypt("aes-256-gcm", plaintext, &key).unwrap();
        assert_eq!(ciphertext.len(), plaintext.len() + 16); // +16 for GCM tag
        
        let decrypted = fallback.single_decrypt("aes-256-gcm", &ciphertext, &key).unwrap();
        assert_eq!(decrypted, plaintext);
        
        // Test ChaCha20-Poly1305
        let ciphertext = fallback.single_encrypt("chacha20-poly1305", plaintext, &key).unwrap();
        assert_eq!(ciphertext.len(), plaintext.len() + 16); // +16 for Poly1305 tag
        
        let decrypted = fallback.single_decrypt("chacha20-poly1305", &ciphertext, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[tokio::test]
    async fn test_batch_encryption() {
        let fallback = CryptoFallback::new();
        let data = vec![42u8; 256]; // 256 bytes of data
        let keys = vec![1u8; 4 * 32]; // 4 keys of 32 bytes each
        let batch_size = 4;
        
        let result = fallback.batch_encrypt("aes-256-gcm", &data, &keys, batch_size).await.unwrap();
        assert_eq!(result.len(), 4 * (64 + 16)); // 4 ciphertexts with tags
    }

    #[tokio::test]
    async fn test_signature_operations() {
        let fallback = CryptoFallback::new();
        let message = b"sign this message";
        let private_key = vec![2u8; 32];
        let public_key = vec![2u8; 32]; // Same as private key for this simplified implementation
        
        let signature = fallback.single_sign("ed25519", message, &private_key).unwrap();
        assert_eq!(signature.len(), 64);
        
        let valid = fallback.single_verify("ed25519", message, &signature, &public_key).unwrap();
        assert!(valid);
        
        // Test with wrong message
        let wrong_message = b"different message";
        let invalid = fallback.single_verify("ed25519", wrong_message, &signature, &public_key).unwrap();
        assert!(!invalid);
    }

    #[tokio::test]
    async fn test_batch_signature() {
        let fallback = CryptoFallback::new();
        let messages = vec![42u8; 128]; // 128 bytes of messages
        let private_keys = vec![3u8; 2 * 32]; // 2 private keys
        let batch_size = 2;
        
        let signatures = fallback.batch_sign("ed25519", &messages, &private_keys, batch_size).await.unwrap();
        assert_eq!(signatures.len(), 2 * 64); // 2 signatures of 64 bytes each
    }

    #[tokio::test]
    async fn test_batch_verification() {
        let fallback = CryptoFallback::new();
        let messages = vec![42u8; 128];
        let private_keys = vec![3u8; 2 * 32];
        let public_keys = vec![3u8; 2 * 32]; // Same as private keys
        let batch_size = 2;
        
        // First generate signatures
        let signatures = fallback.batch_sign("ed25519", &messages, &private_keys, batch_size).await.unwrap();
        
        // Then verify them
        let results = fallback.batch_verify("ed25519", &messages, &signatures, &public_keys, batch_size).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|&r| r)); // All should be valid
    }

    #[tokio::test]
    async fn test_key_size_validation() {
        let fallback = CryptoFallback::new();
        let data = b"test";
        let wrong_key = vec![1u8; 16]; // Wrong size
        
        let result = fallback.single_encrypt("aes-256-gcm", data, &wrong_key);
        assert!(result.is_err());
        
        let result = fallback.single_sign("ed25519", data, &wrong_key);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_deterministic_operations() {
        let fallback = CryptoFallback::new();
        let data = b"test data";
        
        // Hash should be deterministic
        let hash1 = fallback.single_hash("sha256", data).unwrap();
        let hash2 = fallback.single_hash("sha256", data).unwrap();
        assert_eq!(hash1, hash2);
        
        // Encryption with same key should be deterministic (in this simplified implementation)
        let key = vec![5u8; 32];
        let cipher1 = fallback.single_encrypt("aes-256-gcm", data, &key).unwrap();
        let cipher2 = fallback.single_encrypt("aes-256-gcm", data, &key).unwrap();
        assert_eq!(cipher1, cipher2);
    }
}