//! CPU fallback implementation for Kyber768 operations.

use std::sync::Arc;
use rayon::prelude::*;
use tracing::{debug, info};

use crate::{Kyber768FallbackParams, Result, GpuError};

/// CPU fallback implementation for Kyber768 operations.
#[derive(Debug)]
pub struct KyberFallback {
    thread_pool_size: Option<usize>,
}

impl KyberFallback {
    /// Create a new Kyber fallback processor.
    pub fn new(thread_pool_size: Option<u32>) -> Self {
        info!("Creating Kyber768 CPU fallback processor");
        
        Self {
            thread_pool_size: thread_pool_size.map(|s| s as usize),
        }
    }

    /// Batch key generation on CPU.
    pub async fn batch_keygen(
        &self,
        seeds: &[u8],
        params: &Kyber768FallbackParams,
    ) -> Result<(Vec<u8>, Vec<u8>)> {
        debug!("Starting Kyber768 CPU batch keygen for {} keys", params.batch_size);

        if seeds.len() != (params.batch_size as usize * 32) {
            return Err(GpuError::FallbackError {
                message: "Invalid seed data size".to_string(),
            });
        }

        let key_pairs = if params.use_parallel && params.batch_size > 1 {
            self.parallel_keygen(seeds, params).await?
        } else {
            self.sequential_keygen(seeds, params).await?
        };

        // Separate public and secret keys
        let mut public_keys = Vec::with_capacity(params.batch_size as usize * 1184);
        let mut secret_keys = Vec::with_capacity(params.batch_size as usize * 2400);

        for (pk, sk) in key_pairs {
            public_keys.extend_from_slice(&pk);
            secret_keys.extend_from_slice(&sk);
        }

        info!("Completed Kyber768 CPU batch keygen");
        Ok((public_keys, secret_keys))
    }

    /// Batch encapsulation on CPU.
    pub async fn batch_encaps(
        &self,
        public_keys: &[u8],
        messages: &[u8],
        params: &Kyber768FallbackParams,
    ) -> Result<(Vec<u8>, Vec<u8>)> {
        debug!("Starting Kyber768 CPU batch encaps for {} operations", params.batch_size);

        let expected_pk_size = params.batch_size as usize * 1184;
        let expected_msg_size = params.batch_size as usize * 32;

        if public_keys.len() != expected_pk_size {
            return Err(GpuError::FallbackError {
                message: "Invalid public key data size".to_string(),
            });
        }

        if messages.len() != expected_msg_size {
            return Err(GpuError::FallbackError {
                message: "Invalid message data size".to_string(),
            });
        }

        let results = if params.use_parallel && params.batch_size > 1 {
            self.parallel_encaps(public_keys, messages, params).await?
        } else {
            self.sequential_encaps(public_keys, messages, params).await?
        };

        // Separate ciphertexts and shared secrets
        let mut ciphertexts = Vec::with_capacity(params.batch_size as usize * 1088);
        let mut shared_secrets = Vec::with_capacity(params.batch_size as usize * 32);

        for (ct, ss) in results {
            ciphertexts.extend_from_slice(&ct);
            shared_secrets.extend_from_slice(&ss);
        }

        info!("Completed Kyber768 CPU batch encaps");
        Ok((ciphertexts, shared_secrets))
    }

    /// Batch decapsulation on CPU.
    pub async fn batch_decaps(
        &self,
        secret_keys: &[u8],
        ciphertexts: &[u8],
        params: &Kyber768FallbackParams,
    ) -> Result<Vec<u8>> {
        debug!("Starting Kyber768 CPU batch decaps for {} operations", params.batch_size);

        let expected_sk_size = params.batch_size as usize * 2400;
        let expected_ct_size = params.batch_size as usize * 1088;

        if secret_keys.len() != expected_sk_size {
            return Err(GpuError::FallbackError {
                message: "Invalid secret key data size".to_string(),
            });
        }

        if ciphertexts.len() != expected_ct_size {
            return Err(GpuError::FallbackError {
                message: "Invalid ciphertext data size".to_string(),
            });
        }

        let shared_secrets = if params.use_parallel && params.batch_size > 1 {
            self.parallel_decaps(secret_keys, ciphertexts, params).await?
        } else {
            self.sequential_decaps(secret_keys, ciphertexts, params).await?
        };

        let mut result = Vec::with_capacity(params.batch_size as usize * 32);
        for ss in shared_secrets {
            result.extend_from_slice(&ss);
        }

        info!("Completed Kyber768 CPU batch decaps");
        Ok(result)
    }

    // Parallel implementations

    async fn parallel_keygen(
        &self,
        seeds: &[u8],
        params: &Kyber768FallbackParams,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let seed_chunks: Vec<&[u8]> = seeds.chunks(32).collect();
        
        let results: Result<Vec<_>> = if let Some(pool_size) = self.thread_pool_size {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(pool_size)
                .build()
                .map_err(|e| GpuError::FallbackError {
                    message: format!("Failed to create thread pool: {}", e),
                })?;

            pool.install(|| {
                seed_chunks
                    .par_iter()
                    .map(|seed| self.single_keygen(seed))
                    .collect()
            })
        } else {
            seed_chunks
                .par_iter()
                .map(|seed| self.single_keygen(seed))
                .collect()
        };

        results
    }

    async fn parallel_encaps(
        &self,
        public_keys: &[u8],
        messages: &[u8],
        params: &Kyber768FallbackParams,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let pk_chunks: Vec<&[u8]> = public_keys.chunks(1184).collect();
        let msg_chunks: Vec<&[u8]> = messages.chunks(32).collect();
        
        let pairs: Vec<_> = pk_chunks.into_iter().zip(msg_chunks).collect();

        let results: Result<Vec<_>> = if let Some(pool_size) = self.thread_pool_size {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(pool_size)
                .build()
                .map_err(|e| GpuError::FallbackError {
                    message: format!("Failed to create thread pool: {}", e),
                })?;

            pool.install(|| {
                pairs
                    .par_iter()
                    .map(|(pk, msg)| self.single_encaps(pk, msg))
                    .collect()
            })
        } else {
            pairs
                .par_iter()
                .map(|(pk, msg)| self.single_encaps(pk, msg))
                .collect()
        };

        results
    }

    async fn parallel_decaps(
        &self,
        secret_keys: &[u8],
        ciphertexts: &[u8],
        params: &Kyber768FallbackParams,
    ) -> Result<Vec<Vec<u8>>> {
        let sk_chunks: Vec<&[u8]> = secret_keys.chunks(2400).collect();
        let ct_chunks: Vec<&[u8]> = ciphertexts.chunks(1088).collect();
        
        let pairs: Vec<_> = sk_chunks.into_iter().zip(ct_chunks).collect();

        let results: Result<Vec<_>> = if let Some(pool_size) = self.thread_pool_size {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(pool_size)
                .build()
                .map_err(|e| GpuError::FallbackError {
                    message: format!("Failed to create thread pool: {}", e),
                })?;

            pool.install(|| {
                pairs
                    .par_iter()
                    .map(|(sk, ct)| self.single_decaps(sk, ct))
                    .collect()
            })
        } else {
            pairs
                .par_iter()
                .map(|(sk, ct)| self.single_decaps(sk, ct))
                .collect()
        };

        results
    }

    // Sequential implementations

    async fn sequential_keygen(
        &self,
        seeds: &[u8],
        params: &Kyber768FallbackParams,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut results = Vec::with_capacity(params.batch_size as usize);
        
        for seed_chunk in seeds.chunks(32) {
            let key_pair = self.single_keygen(seed_chunk)?;
            results.push(key_pair);
        }

        Ok(results)
    }

    async fn sequential_encaps(
        &self,
        public_keys: &[u8],
        messages: &[u8],
        params: &Kyber768FallbackParams,
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut results = Vec::with_capacity(params.batch_size as usize);
        
        let pk_chunks = public_keys.chunks(1184);
        let msg_chunks = messages.chunks(32);
        
        for (pk, msg) in pk_chunks.zip(msg_chunks) {
            let encaps_result = self.single_encaps(pk, msg)?;
            results.push(encaps_result);
        }

        Ok(results)
    }

    async fn sequential_decaps(
        &self,
        secret_keys: &[u8],
        ciphertexts: &[u8],
        params: &Kyber768FallbackParams,
    ) -> Result<Vec<Vec<u8>>> {
        let mut results = Vec::with_capacity(params.batch_size as usize);
        
        let sk_chunks = secret_keys.chunks(2400);
        let ct_chunks = ciphertexts.chunks(1088);
        
        for (sk, ct) in sk_chunks.zip(ct_chunks) {
            let shared_secret = self.single_decaps(sk, ct)?;
            results.push(shared_secret);
        }

        Ok(results)
    }

    // Single operation implementations (simplified for testing)

    fn single_keygen(&self, seed: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
        if seed.len() != 32 {
            return Err(GpuError::FallbackError {
                message: "Invalid seed size".to_string(),
            });
        }

        // Simplified Kyber768 key generation
        // In a real implementation, this would use the actual Kyber768 algorithm
        
        let mut public_key = vec![0u8; 1184];
        let mut secret_key = vec![0u8; 2400];
        
        // Use seed to generate deterministic keys
        let mut state = self.seed_to_state(seed);
        
        // Generate public key
        for i in 0..1184 {
            state = self.prng_next(state);
            public_key[i] = (state & 0xFF) as u8;
        }
        
        // Generate secret key
        for i in 0..2400 {
            state = self.prng_next(state);
            secret_key[i] = (state & 0xFF) as u8;
        }

        Ok((public_key, secret_key))
    }

    fn single_encaps(&self, public_key: &[u8], message: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
        if public_key.len() != 1184 {
            return Err(GpuError::FallbackError {
                message: "Invalid public key size".to_string(),
            });
        }
        
        if message.len() != 32 {
            return Err(GpuError::FallbackError {
                message: "Invalid message size".to_string(),
            });
        }

        // Simplified Kyber768 encapsulation
        let mut ciphertext = vec![0u8; 1088];
        let mut shared_secret = vec![0u8; 32];
        
        // Use message as seed for randomness
        let mut state = self.seed_to_state(message);
        
        // Generate ciphertext
        for i in 0..1088 {
            state = self.prng_next(state);
            ciphertext[i] = (state ^ public_key[i % public_key.len()] as u32) as u8;
        }
        
        // Generate shared secret
        for i in 0..32 {
            state = self.prng_next(state);
            shared_secret[i] = (state & 0xFF) as u8;
        }

        Ok((ciphertext, shared_secret))
    }

    fn single_decaps(&self, secret_key: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        if secret_key.len() != 2400 {
            return Err(GpuError::FallbackError {
                message: "Invalid secret key size".to_string(),
            });
        }
        
        if ciphertext.len() != 1088 {
            return Err(GpuError::FallbackError {
                message: "Invalid ciphertext size".to_string(),
            });
        }

        // Simplified Kyber768 decapsulation
        let mut shared_secret = vec![0u8; 32];
        
        // Use combination of secret key and ciphertext
        let mut state = 0u32;
        for i in 0..32 {
            state ^= secret_key[i] as u32;
            state ^= ciphertext[i] as u32;
            state = self.prng_next(state);
        }
        
        // Generate shared secret
        for i in 0..32 {
            state = self.prng_next(state);
            shared_secret[i] = (state & 0xFF) as u8;
        }

        Ok(shared_secret)
    }

    // Helper methods

    fn seed_to_state(&self, seed: &[u8]) -> u32 {
        let mut state = 0x12345678u32;
        for &byte in seed {
            state ^= byte as u32;
            state = self.prng_next(state);
        }
        state
    }

    fn prng_next(&self, state: u32) -> u32 {
        // Simple linear congruential generator
        state.wrapping_mul(1103515245).wrapping_add(12345)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_kyber_fallback() -> KyberFallback {
        KyberFallback::new(Some(4))
    }

    #[tokio::test]
    async fn test_kyber_fallback_creation() {
        let fallback = create_test_kyber_fallback();
        assert_eq!(fallback.thread_pool_size, Some(4));
    }

    #[tokio::test]
    async fn test_single_keygen() {
        let fallback = create_test_kyber_fallback();
        let seed = vec![1u8; 32];
        
        let result = fallback.single_keygen(&seed).unwrap();
        assert_eq!(result.0.len(), 1184); // Public key size
        assert_eq!(result.1.len(), 2400); // Secret key size
        
        // Test deterministic generation
        let result2 = fallback.single_keygen(&seed).unwrap();
        assert_eq!(result.0, result2.0);
        assert_eq!(result.1, result2.1);
    }

    #[tokio::test]
    async fn test_single_encaps() {
        let fallback = create_test_kyber_fallback();
        let public_key = vec![42u8; 1184];
        let message = vec![17u8; 32];
        
        let result = fallback.single_encaps(&public_key, &message).unwrap();
        assert_eq!(result.0.len(), 1088); // Ciphertext size
        assert_eq!(result.1.len(), 32);   // Shared secret size
    }

    #[tokio::test]
    async fn test_single_decaps() {
        let fallback = create_test_kyber_fallback();
        let secret_key = vec![99u8; 2400];
        let ciphertext = vec![55u8; 1088];
        
        let result = fallback.single_decaps(&secret_key, &ciphertext).unwrap();
        assert_eq!(result.len(), 32); // Shared secret size
    }

    #[tokio::test]
    async fn test_batch_keygen_sequential() {
        let fallback = create_test_kyber_fallback();
        let seeds = vec![1u8; 64]; // 2 seeds of 32 bytes each
        let mut params = Kyber768FallbackParams::default();
        params.batch_size = 2;
        params.use_parallel = false;
        
        let result = fallback.batch_keygen(&seeds, &params).await.unwrap();
        assert_eq!(result.0.len(), 2 * 1184); // 2 public keys
        assert_eq!(result.1.len(), 2 * 2400); // 2 secret keys
    }

    #[tokio::test]
    async fn test_batch_keygen_parallel() {
        let fallback = create_test_kyber_fallback();
        let seeds = vec![2u8; 96]; // 3 seeds of 32 bytes each
        let mut params = Kyber768FallbackParams::default();
        params.batch_size = 3;
        params.use_parallel = true;
        
        let result = fallback.batch_keygen(&seeds, &params).await.unwrap();
        assert_eq!(result.0.len(), 3 * 1184); // 3 public keys
        assert_eq!(result.1.len(), 3 * 2400); // 3 secret keys
    }

    #[tokio::test]
    async fn test_batch_encaps() {
        let fallback = create_test_kyber_fallback();
        let public_keys = vec![42u8; 2 * 1184]; // 2 public keys
        let messages = vec![17u8; 2 * 32];       // 2 messages
        let mut params = Kyber768FallbackParams::default();
        params.batch_size = 2;
        
        let result = fallback.batch_encaps(&public_keys, &messages, &params).await.unwrap();
        assert_eq!(result.0.len(), 2 * 1088); // 2 ciphertexts
        assert_eq!(result.1.len(), 2 * 32);   // 2 shared secrets
    }

    #[tokio::test]
    async fn test_batch_decaps() {
        let fallback = create_test_kyber_fallback();
        let secret_keys = vec![99u8; 2 * 2400]; // 2 secret keys
        let ciphertexts = vec![55u8; 2 * 1088]; // 2 ciphertexts
        let mut params = Kyber768FallbackParams::default();
        params.batch_size = 2;
        
        let result = fallback.batch_decaps(&secret_keys, &ciphertexts, &params).await.unwrap();
        assert_eq!(result.len(), 2 * 32); // 2 shared secrets
    }

    #[tokio::test]
    async fn test_invalid_input_sizes() {
        let fallback = create_test_kyber_fallback();
        
        // Test invalid seed size for keygen
        let invalid_seeds = vec![1u8; 30]; // Should be 32
        let mut params = Kyber768FallbackParams::default();
        params.batch_size = 1;
        
        let result = fallback.batch_keygen(&invalid_seeds, &params).await;
        assert!(result.is_err());
        
        // Test invalid public key size for encaps
        let invalid_pk = vec![42u8; 1000]; // Should be 1184
        let messages = vec![17u8; 32];
        
        let result = fallback.batch_encaps(&invalid_pk, &messages, &params).await;
        assert!(result.is_err());
        
        // Test invalid secret key size for decaps
        let invalid_sk = vec![99u8; 2000]; // Should be 2400
        let ciphertexts = vec![55u8; 1088];
        
        let result = fallback.batch_decaps(&invalid_sk, &ciphertexts, &params).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_prng_determinism() {
        let fallback = create_test_kyber_fallback();
        
        let state1 = fallback.prng_next(12345);
        let state2 = fallback.prng_next(12345);
        assert_eq!(state1, state2);
        
        let state3 = fallback.prng_next(state1);
        let state4 = fallback.prng_next(state2);
        assert_eq!(state3, state4);
    }

    #[tokio::test]
    async fn test_seed_to_state_consistency() {
        let fallback = create_test_kyber_fallback();
        let seed = vec![1, 2, 3, 4, 5];
        
        let state1 = fallback.seed_to_state(&seed);
        let state2 = fallback.seed_to_state(&seed);
        assert_eq!(state1, state2);
        
        let different_seed = vec![1, 2, 3, 4, 6];
        let state3 = fallback.seed_to_state(&different_seed);
        assert_ne!(state1, state3);
    }
}