# Synapsed Identity - DID-Based Identity System

A comprehensive DID (Decentralized Identifier) based identity and access management system implementing W3C DID Core v1.0 with advanced features for secure P2P communication platforms.

## 🚀 Features

### Core DID Support
- **W3C DID Core v1.0 Compliance**: Full specification compliance
- **Multiple DID Methods**: `did:key`, `did:web` support
- **DID Resolution**: Universal resolver with caching
- **Document Management**: Complete CRUD operations

### Advanced Security
- **Hierarchical Key Management**: DEK/KEK separation with rotation
- **Zero-Knowledge Proofs**: Anonymous credentials with selective disclosure
- **Post-Quantum Cryptography**: Future-proof security algorithms
- **Forward Secrecy**: Key rotation with historical preservation

### Local-First Architecture
- **Encrypted Local Storage**: All data encrypted at rest
- **Multi-Device Sync**: Secure synchronization across devices
- **Contact Vault**: Portable contact management
- **Offline-First**: Works without network connectivity

### PWA Support
- **WebAuthn Integration**: Passwordless biometric authentication
- **Service Worker**: Offline capability and caching
- **Browser Storage**: IndexedDB with encryption
- **Modern Web APIs**: Full PWA feature support

### Backward Compatibility
- **Existing API**: Traditional auth methods still supported
- **Feature Flags**: Modular activation via Cargo features
- **Migration Path**: Smooth transition from username/password

## 📦 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
synapsed-identity = { version = "0.1.0", features = ["did-core", "local-first"] }
```

### Feature Flags

- `did-core` - W3C DID Core v1.0 support (default)
- `did-key` - did:key method implementation (default)
- `did-web` - did:web method implementation
- `key-rotation` - Hierarchical key management (default)
- `zkp-support` - Zero-knowledge proof integration
- `local-first` - Encrypted local storage (default)
- `pwa-support` - Progressive Web App features
- `webauthn` - WebAuthn integration
- `full` - All features enabled

## 🎯 Quick Start

### Traditional Authentication

```rust
use synapsed_identity::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Traditional username/password authentication still works
    let manager = IdentityManager::builder()
        .with_storage(storage::MemoryIdentityStore::new())
        .with_authenticator(auth::PasswordAuthenticator::new())
        .with_authorizer(authorization::RbacAuthorizer::new())
        .build()
        .await?;

    // Create and authenticate user
    let identity = manager.authenticate(auth::PasswordCredentials {
        username: "user@example.com".to_string(),
        password: "secure_password".to_string(),
    }).await?;

    Ok(())
}
```

### DID-Based Identity

```rust
use synapsed_identity::*;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup components
    let temp_dir = TempDir::new()?;
    let key_manager = KeyRotationManager::new(
        key_management::RotationPolicy::default(),
        key_management::RecoveryMechanism::default(),
    );
    let storage = LocalFirstStorage::new(
        temp_dir.path(),
        "master_password",
        storage::StorageConfig::default(),
    )?;

    // Create DID identity manager
    let mut manager = IdentityManager::with_did_support()
        .with_key_manager(key_manager)
        .with_storage(storage)
        .build()
        .await?;

    // Create a new DID
    let did = manager.create_did("key").await?;
    println!("Created DID: {}", did);

    // Resolve DID to document
    let document = manager.resolve_did(&did).await?;
    println!("Resolved document with {} verification methods", 
             document.unwrap().verification_method.len());

    Ok(())
}
```

### Zero-Knowledge Proofs

```rust
use synapsed_identity::*;

#[tokio::main]
async fn main() -> Result<()> {
    let mut zkp_verifier = ZkpVerifier::new();

    // Create anonymous credential
    let mut attributes = std::collections::HashMap::new();
    attributes.insert("age".to_string(), zkp::AttributeValue::Number(25));
    attributes.insert("premium".to_string(), zkp::AttributeValue::Boolean(true));

    let credential = zkp::AnonymousCredential {
        id: "cred-123".to_string(),
        issuer: Did::new("issuer", "authority"),
        subject: Some(Did::new("user", "alice")),
        attributes,
        signature: zkp::CredentialSignature::BbsPlus {
            signature: vec![0u8; 64],
            public_key: vec![0u8; 32],
        },
        issued_at: chrono::Utc::now(),
        expires_at: None,
        revocation_registry_id: None,
    };

    // Create selective disclosure proof (reveal age, hide premium status)
    let proof_request = zkp::ProofRequest {
        name: "Age Verification".to_string(),
        version: "1.0".to_string(),
        nonce: vec![1, 2, 3, 4],
        requested_attributes: vec!["age".to_string()],
        requested_predicates: Vec::new(),
        non_revoked: None,
    };

    let disclosed = vec!["age".to_string()];
    let proof = credential.create_selective_disclosure_proof(&disclosed, &proof_request)?;

    // Verify the proof
    let public_inputs = b"verification_context";
    let is_valid = zkp_verifier.verify_proof(&proof, public_inputs)?;
    println!("Proof valid: {}", is_valid);

    Ok(())
}
```

### PWA Integration

```rust
use synapsed_identity::pwa::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Check browser capabilities
    let capabilities = BrowserCapabilities::detect().await;
    if !capabilities.webauthn_supported {
        println!("WebAuthn not supported in this browser");
        return Ok(());
    }

    // Create PWA DID manager
    let mut pwa_manager = PwaDidManager::new(PwaConfig::default()).await?;

    // Initialize identity with biometric authentication
    let identity = pwa_manager.initialize_identity("user@example.com").await?;
    println!("Created PWA identity: {}", identity.did);

    // Authenticate using biometrics
    let challenge = b"authentication_challenge";
    let auth_result = pwa_manager.authenticate(challenge).await?;
    
    if auth_result.success {
        println!("Biometric authentication successful!");
    }

    // Enable offline mode
    pwa_manager.enable_offline_mode().await?;

    Ok(())
}
```

## 🏗️ Architecture

### Core Components

1. **DID Core (`did/`)**: W3C DID specification implementation
   - `mod.rs` - DID URI parsing and core types
   - `document.rs` - DID Document structure and validation
   - `methods.rs` - DID method implementations (did:key, did:web)
   - `resolver.rs` - Universal DID resolver with caching

2. **Key Management (`did/key_management.rs`)**: Advanced key lifecycle
   - Hierarchical key structures (DEK/KEK)
   - Automatic rotation policies
   - Recovery mechanisms
   - Forward secrecy guarantee

3. **Zero-Knowledge Proofs (`did/zkp.rs`)**: Privacy-preserving verification
   - BBS+ signatures for selective disclosure
   - Bulletproofs for range proofs
   - Anonymous credential support
   - Browser-compatible verification

4. **Local-First Storage (`did/storage.rs`)**: Encrypted data persistence
   - ChaCha20Poly1305 encryption
   - Contact vault management
   - Multi-device synchronization
   - Backup/restore functionality

5. **PWA Support (`pwa/`)**: Modern web application features
   - WebAuthn integration
   - Service worker offline capability
   - IndexedDB storage
   - Browser API integration

### Security Features

- **Post-Quantum Ready**: ML-KEM and ML-DSA algorithm support
- **Zero-Trust Architecture**: Never store unencrypted sensitive data
- **Forward Secrecy**: Historical key compromise doesn't affect future communications
- **Anonymous Verification**: Prove attributes without revealing identity
- **Local-First Privacy**: Data never leaves device without explicit user consent

## 🔧 Configuration

### Storage Configuration

```rust
use synapsed_identity::storage::StorageConfig;

let config = StorageConfig {
    auto_sync: true,
    compression_enabled: true,
    backup_retention_days: 30,
    max_storage_size_mb: 1024,
};
```

### Key Rotation Policy

```rust
use synapsed_identity::key_management::{RotationPolicy, RecoveryMechanism};
use chrono::Duration;

let policy = RotationPolicy {
    max_key_age: Duration::days(90),
    rotate_on_device_change: true,
    rotate_on_compromise: true,
    rotation_schedule: Some("0 0 * * 0".to_string()), // Weekly
};

let recovery = RecoveryMechanism {
    recovery_phrase_length: 24, // BIP39 standard
    social_recovery_threshold: Some(3),
    hardware_recovery: true,
};
```

### PWA Configuration

```rust
use synapsed_identity::pwa::PwaConfig;

let config = PwaConfig {
    offline_mode: true,
    auto_sync_interval: 300, // 5 minutes
    webauthn: webauthn::WebAuthnConfig {
        rp_id: "your-domain.com".to_string(),
        rp_name: "Your App".to_string(),
        rp_origin: "https://your-domain.com".to_string(),
    },
    ..Default::default()
};
```

## 🧪 Testing

Run the comprehensive test suite:

```bash
# Test core DID functionality
cargo test --features did-core

# Test all features
cargo test --all-features

# Test specific components
cargo test did_tests --features did-core
cargo test pwa_tests --features pwa-support
cargo test zkp_tests --features zkp-support
```

## 🤝 Integration Examples

### Secure P2P Communication

```rust
// Use DIDs as peer identifiers
let peer_did = Did::parse("did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK")?;
let peer_document = resolver.resolve(&peer_did).await?;

// Extract encryption key for secure messaging
let encryption_key = peer_document.unwrap()
    .key_agreement
    .first()
    .expect("No encryption key found");
```

### Anonymous Subscription Verification

```rust
// Verify subscription without revealing payment details
let subscription_proof = create_subscription_proof(subscription_level, &challenge)?;
let is_premium = zkp_verifier.verify_subscription_proof(&subscription_proof)?;
```

### Cross-Device Identity Sync

```rust
// Export identity for device migration
let backup = storage.export_all("backup_password").await?;

// Import on new device
let mut new_storage = LocalFirstStorage::new(new_path, "master_password", config)?;
new_storage.import_backup(&backup, "backup_password").await?;
```

## 📚 Resources

- [W3C DID Core Specification](https://www.w3.org/TR/did-core/)
- [DID Method Registry](https://w3c.github.io/did-spec-registries/)
- [WebAuthn Specification](https://www.w3.org/TR/webauthn-2/)
- [Zero-Knowledge Proofs Guide](https://zkp.science/)
- [NIST Post-Quantum Cryptography](https://csrc.nist.gov/Projects/post-quantum-cryptography)

## 🛡️ Security Considerations

1. **Key Storage**: Master passwords should use strong entropy
2. **Network Security**: Always use HTTPS for did:web resolution
3. **Browser Security**: PWA features require secure origins
4. **Recovery Planning**: Implement multiple recovery mechanisms
5. **Regular Rotation**: Configure appropriate key rotation policies

## 📄 License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

---

**Note**: This implementation provides W3C DID Core v1.0 compliance with advanced features for secure P2P platforms. While the core cryptographic primitives are production-ready, some advanced features (especially PWA integration) may require additional implementation for full production use.