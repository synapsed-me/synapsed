# Synapsed Payments

Privacy-preserving payment processing with zero-knowledge proofs and DID integration.

## Overview

`synapsed-payments` provides a complete payment infrastructure that prioritizes privacy and security through:
- Zero-knowledge proof verification for anonymous transactions
- Decentralized Identity (DID) integration for identity management
- Multi-gateway support with fallback mechanisms
- WASM-based PWA support for browser payments
- Substrate network integration for blockchain settlement

## Features

### Core Capabilities
- **Privacy-First Payments**: ZKP-based anonymous transactions
- **DID Integration**: Self-sovereign identity for payment authorization
- **Multi-Gateway Support**: Stripe, PayPal, crypto, and custom gateways
- **Progressive Web App**: Browser-based payment flows via WASM
- **Substrate Integration**: Native blockchain settlement layer

### Security Features
- Zero-knowledge proofs for transaction privacy
- Nullifier-based double-spend prevention
- Homomorphic encryption for amount hiding
- DID rotation for identity privacy
- Circuit-based verification

## Implementation Status

### Core Features
- ✅ Payment processor with gateway abstraction
- ✅ Zero-knowledge proof generation and verification
- ✅ DID integration with key rotation
- ✅ Storage layer with encryption
- ✅ Builder pattern for payment construction
- 🚧 Substrate network integration
- 🚧 WASM PWA compilation
- 📋 Production gateway implementations

### ZKP Components
- ✅ Anonymous payment proofs
- ✅ Nullifier generation and tracking
- ✅ Range proofs for amounts
- ✅ DID rotation proofs
- 🚧 Batch proof aggregation
- 📋 Recursive proof composition

### Testing
- ✅ Unit tests for all components
- ✅ Integration tests for payment flows
- ✅ ZKP verification tests
- 🚧 End-to-end gateway tests
- 📋 Performance benchmarks

## Usage

### Basic Payment
```rust
use synapsed_payments::{PaymentBuilder, PaymentProcessor};

// Create a payment
let payment = PaymentBuilder::new()
    .amount(100.0, "USD")
    .from_account("user123")
    .to_account("merchant456")
    .with_metadata("order_id", "ORDER-789")
    .build()?;

// Process the payment
let processor = PaymentProcessor::new(config);
let result = processor.process(payment).await?;
```

### Anonymous Payment with ZKP
```rust
use synapsed_payments::zkp::{AnonymousPayment, ZKPVerifier};

// Create anonymous payment
let anon_payment = AnonymousPayment::new(
    amount,
    sender_did,
    recipient_address
);

// Generate zero-knowledge proof
let proof = anon_payment.generate_proof(&witness)?;

// Verify without revealing sender
let verifier = ZKPVerifier::new();
verifier.verify_payment(&proof, &public_inputs)?;
```

### DID-Authenticated Payment
```rust
use synapsed_payments::did_integration::DIDPaymentAuth;

// Authenticate with DID
let auth = DIDPaymentAuth::new(did_document);
let payment_token = auth.create_payment_token(
    amount,
    recipient,
    Duration::minutes(5)
)?;

// Process with DID verification
processor.process_with_did(payment, payment_token).await?;
```

## Architecture

```
┌─────────────────────────────────────┐
│         Payment Request             │
└────────────┬────────────────────────┘
             │
      ┌──────▼──────┐
      │   Builder   │
      └──────┬──────┘
             │
      ┌──────▼──────┐
      │  Processor  │
      └──────┬──────┘
             │
    ┌────────┴────────┐
    │                 │
┌───▼───┐       ┌─────▼─────┐
│  ZKP  │       │    DID    │
│ Layer │       │   Auth    │
└───┬───┘       └─────┬─────┘
    │                 │
    └────────┬────────┘
             │
      ┌──────▼──────┐
      │   Gateway   │
      │  (Multiple) │
      └──────┬──────┘
             │
      ┌──────▼──────┐
      │   Storage   │
      │ (Encrypted) │
      └─────────────┘
```

## Configuration

```toml
[dependencies.synapsed-payments]
version = "0.1.0"
features = ["zkp", "did", "substrate", "wasm"]
```

## Examples

See the `examples/` directory for:
- `basic_payment.rs` - Simple payment processing
- `anonymous_subscription.rs` - Privacy-preserving subscriptions

## Testing

```bash
# Run all tests
cargo test -p synapsed-payments

# Run specific test suites
cargo test -p synapsed-payments zkp
cargo test -p synapsed-payments did
cargo test -p synapsed-payments integration
```

## Security Considerations

1. **Never log private keys or payment credentials**
2. **Always verify ZKP proofs before processing**
3. **Use secure storage for nullifier sets**
4. **Implement rate limiting on payment endpoints**
5. **Rotate DIDs regularly for privacy**

## License

Licensed under Apache 2.0 or MIT at your option.