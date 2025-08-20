// Unit tests for Post-Quantum Key Management

#![cfg(test)]

use synapsed_identity::key_management::*;
use synapsed_crypto::{ml_kem::*, ml_dsa::*};
use crate::test_framework::{*, generators::*, performance::*, security::*};

#[cfg(test)]
mod key_generation_tests {
    use super::*;

    #[test]
    fn test_generate_ml_kem_keypair() {
        // Test ML-KEM (Kyber) key generation
        let result = generate_ml_kem_keypair();
        assert!(result.is_ok(), "Failed to generate ML-KEM keypair");
        
        let (public_key, secret_key) = result.unwrap();
        assert!(!public_key.is_empty(), "Public key should not be empty");
        assert!(!secret_key.is_empty(), "Secret key should not be empty");
        
        // Verify key sizes match expected values
        assert_eq!(public_key.len(), ML_KEM_768_PUBLIC_KEY_SIZE);
        assert_eq!(secret_key.len(), ML_KEM_768_SECRET_KEY_SIZE);
    }

    #[test]
    fn test_generate_ml_dsa_keypair() {
        // Test ML-DSA (Dilithium) key generation
        let result = generate_ml_dsa_keypair();
        assert!(result.is_ok(), "Failed to generate ML-DSA keypair");
        
        let (public_key, secret_key) = result.unwrap();
        assert!(!public_key.is_empty(), "Public key should not be empty");
        assert!(!secret_key.is_empty(), "Secret key should not be empty");
    }

    #[test]
    fn test_hybrid_keypair_generation() {
        // Test hybrid classical-quantum key generation
        let result = generate_hybrid_keypair();
        assert!(result.is_ok(), "Failed to generate hybrid keypair");
        
        let hybrid_keys = result.unwrap();
        assert!(hybrid_keys.has_classical(), "Missing classical keys");
        assert!(hybrid_keys.has_quantum(), "Missing quantum keys");
    }

    #[test]
    fn test_key_generation_deterministic() {
        // Test deterministic key generation with seed
        let seed = [42u8; 32];
        
        let keys1 = generate_ml_kem_keypair_from_seed(&seed).unwrap();
        let keys2 = generate_ml_kem_keypair_from_seed(&seed).unwrap();
        
        assert_eq!(keys1.0, keys2.0, "Public keys should match for same seed");
        assert_eq!(keys1.1, keys2.1, "Secret keys should match for same seed");
    }
}

#[cfg(test)]
mod key_storage_tests {
    use super::*;

    #[test]
    fn test_secure_key_storage() {
        let mut storage = SecureKeyStorage::new();
        let (public_key, secret_key) = generate_ml_kem_keypair().unwrap();
        let key_id = "test-key-001";
        
        // Store key
        let result = storage.store_keypair(key_id, &public_key, &secret_key);
        assert!(result.is_ok(), "Failed to store keypair");
        
        // Retrieve key
        let retrieved = storage.retrieve_keypair(key_id);
        assert!(retrieved.is_ok(), "Failed to retrieve keypair");
        
        let (retrieved_public, retrieved_secret) = retrieved.unwrap();
        assert_eq!(public_key, retrieved_public, "Public key mismatch");
        assert_eq!(secret_key, retrieved_secret, "Secret key mismatch");
    }

    #[test]
    fn test_encrypted_key_storage() {
        let mut storage = EncryptedKeyStorage::new();
        let master_key = [0u8; 32]; // Test master key
        storage.set_master_key(&master_key);
        
        let (public_key, secret_key) = generate_ml_kem_keypair().unwrap();
        let key_id = "encrypted-key-001";
        
        // Store encrypted
        let result = storage.store_encrypted(key_id, &public_key, &secret_key);
        assert!(result.is_ok(), "Failed to store encrypted keypair");
        
        // Retrieve and decrypt
        let retrieved = storage.retrieve_decrypted(key_id);
        assert!(retrieved.is_ok(), "Failed to retrieve encrypted keypair");
    }

    #[test]
    fn test_key_deletion() {
        let mut storage = SecureKeyStorage::new();
        let (public_key, secret_key) = generate_ml_kem_keypair().unwrap();
        let key_id = "delete-test-key";
        
        storage.store_keypair(key_id, &public_key, &secret_key).unwrap();
        assert!(storage.key_exists(key_id), "Key should exist after storage");
        
        // Delete key
        let result = storage.delete_keypair(key_id);
        assert!(result.is_ok(), "Failed to delete keypair");
        assert!(!storage.key_exists(key_id), "Key should not exist after deletion");
    }
}

#[cfg(test)]
mod key_rotation_tests {
    use super::*;

    #[test]
    fn test_key_rotation_basic() {
        let mut key_manager = KeyManager::new();
        let identity_id = "test-identity";
        
        // Generate initial keys
        let result = key_manager.initialize_keys(identity_id);
        assert!(result.is_ok(), "Failed to initialize keys");
        
        let initial_version = key_manager.get_current_key_version(identity_id).unwrap();
        
        // Rotate keys
        let rotation_result = key_manager.rotate_keys(identity_id);
        assert!(rotation_result.is_ok(), "Failed to rotate keys");
        
        let new_version = key_manager.get_current_key_version(identity_id).unwrap();
        assert!(new_version > initial_version, "Key version should increase after rotation");
    }

    #[test]
    fn test_key_rotation_with_grace_period() {
        let mut key_manager = KeyManager::new();
        let identity_id = "grace-period-test";
        
        key_manager.initialize_keys(identity_id).unwrap();
        let old_keys = key_manager.get_current_keys(identity_id).unwrap();
        
        // Rotate with grace period
        key_manager.rotate_keys_with_grace_period(identity_id, 3600).unwrap();
        
        // Old keys should still be accessible during grace period
        let old_keys_check = key_manager.get_keys_by_version(identity_id, 0);
        assert!(old_keys_check.is_ok(), "Old keys should be accessible during grace period");
    }

    #[test]
    fn test_key_migration_strategy() {
        let mut key_manager = KeyManager::new();
        let identity_id = "migration-test";
        
        // Initialize with classical keys
        key_manager.initialize_classical_keys(identity_id).unwrap();
        
        // Migrate to hybrid
        let migration_result = key_manager.migrate_to_hybrid(identity_id);
        assert!(migration_result.is_ok(), "Failed to migrate to hybrid keys");
        
        // Verify both key types exist
        let current_keys = key_manager.get_current_keys(identity_id).unwrap();
        assert!(current_keys.has_classical(), "Should have classical keys after migration");
        assert!(current_keys.has_quantum(), "Should have quantum keys after migration");
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use criterion::black_box;

    #[test]
    fn test_key_generation_performance() {
        assert_performance!(
            || { generate_ml_kem_keypair().unwrap() },
            constants::PERFORMANCE_THRESHOLD_MS
        );
    }

    #[test]
    fn test_key_storage_performance() {
        let mut storage = SecureKeyStorage::new();
        let (public_key, secret_key) = generate_ml_kem_keypair().unwrap();
        
        assert_performance!(
            || {
                storage.store_keypair("perf-test", &public_key, &secret_key).unwrap();
            },
            50 // 50ms threshold for storage
        );
    }

    #[test]
    fn test_bulk_key_operations() {
        let mut key_manager = KeyManager::new();
        let num_identities = 100;
        
        let (_, elapsed) = measure_time(|| {
            for i in 0..num_identities {
                let identity_id = format!("bulk-test-{}", i);
                key_manager.initialize_keys(&identity_id).unwrap();
            }
        });
        
        let avg_time = elapsed / num_identities;
        assert!(
            avg_time < 10,
            "Average key generation time too high: {} ms",
            avg_time
        );
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;

    #[test]
    fn test_constant_time_key_operations() {
        let storage = SecureKeyStorage::new();
        
        assert_constant_time!(|input| {
            // Simulate key comparison
            let key1 = &input[..16];
            let key2 = &input[16..];
            constant_time_compare(key1, key2)
        });
    }

    #[test]
    fn test_key_zeroization() {
        let mut secret_key = vec![42u8; 32];
        let key_ptr = secret_key.as_ptr();
        
        // Zeroize the key
        zeroize_key(&mut secret_key);
        
        // Verify memory is cleared
        assert!(secret_key.iter().all(|&b| b == 0), "Key not properly zeroized");
    }

    #[test]
    fn test_secure_random_generation() {
        let random1 = generate_secure_random(32);
        let random2 = generate_secure_random(32);
        
        assert_ne!(random1, random2, "Random values should not be identical");
        assert!(random1.len() == 32, "Random value has incorrect length");
    }
}