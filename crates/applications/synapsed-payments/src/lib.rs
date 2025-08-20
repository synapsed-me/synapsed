//! # Synapsed Payments
//!
//! A comprehensive payment processing library for the Synapsed framework, featuring:
//!
//! - **Multiple Payment Methods**: Credit cards, bank transfers, digital wallets, and cryptocurrency
//! - **Gateway Abstraction**: Support for multiple payment gateways with unified interface
//! - **Substrate Integration**: Native blockchain payment processing through synapsed-substrates
//! - **Risk Management**: Built-in fraud detection and risk assessment
//! - **Secure Storage**: Multiple storage backends with encryption support
//! - **Comprehensive Testing**: Mock implementations for development and testing
//! - **Zero-Knowledge Proofs**: Anonymous subscription verification with privacy preservation
//! - **DID Integration**: Decentralized identity support for user anonymity
//! - **PWA Optimization**: WebAssembly-optimized ZKP computation for browser applications
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use synapsed_payments::{PaymentManagerBuilder, Amount, Currency, FiatCurrency};
//! use rust_decimal::Decimal;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a payment manager with development defaults
//!     let manager = PaymentManagerBuilder::development()
//!         .build()?;
//!
//!     // Create a payment intent
//!     let amount = Amount::new(
//!         Decimal::new(10000, 2), // $100.00
//!         Currency::Fiat(FiatCurrency::USD),
//!     );
//!
//!     let payment = manager
//!         .create_payment(amount, "Test payment".to_string(), None)
//!         .await?;
//!
//!     println!("Created payment: {}", payment.id);
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! The library is organized into several key modules:
//!
//! - [`types`]: Core data structures and enums
//! - [`error`]: Error handling and result types
//! - [`processor`]: Main payment processing engine
//! - [`gateway`]: Payment gateway abstractions and implementations
//! - [`builder`]: Builder pattern for easy configuration
//! - [`storage`]: Data persistence layer
//! - [`substrate_integration`]: Blockchain payment processing
//!
//! ## Features
//!
//! - `default`: SQLite storage and HTTP gateway support
//! - `postgres`: PostgreSQL storage backend
//! - `substrate`: Substrate blockchain integration
//! - `http-gateway`: HTTP-based payment gateway client
//! - `crypto-advanced`: Advanced cryptographic features
//!
//! ## Payment Flow
//!
//! 1. **Create Payment Intent**: Define payment amount, currency, and description
//! 2. **Risk Assessment**: Evaluate transaction risk and fraud potential
//! 3. **Gateway Selection**: Choose appropriate payment gateway based on method
//! 4. **Process Payment**: Execute payment through selected gateway
//! 5. **Confirmation**: Wait for transaction confirmation and update status
//! 6. **Storage**: Persist transaction details and audit trail
//!
//! ## Security
//!
//! - Sensitive data is automatically zeroized from memory
//! - Payment credentials are encrypted at rest
//! - All transactions include cryptographic audit trails
//! - Risk assessment prevents fraudulent transactions
//! - Substrate integration provides blockchain-level security

pub mod builder;
pub mod error;
pub mod gateway;
pub mod processor;
pub mod storage;
pub mod substrate_integration;
pub mod types;

// Zero-knowledge proof and privacy modules
// Simplified ZK proof implementation for TDD
pub mod zkp_simple;

// Use simplified implementation as zkp module for now
pub use zkp_simple as zkp;

// Simplified DID integration for TDD
pub mod did_integration_simple;

// Use simplified implementation as did_integration module for now
pub use did_integration_simple as did_integration;

#[cfg(feature = "wasm-support")]
pub mod wasm_pwa;

// Re-export commonly used types for convenience
pub use builder::{PaymentManager, PaymentManagerBuilder};
pub use error::{PaymentError, PaymentResult};
pub use gateway::{GatewayConfig, PaymentGateway};
pub use processor::{PaymentProcessor, ProcessorConfig, RetryConfig, RiskEngine};
pub use storage::MemoryPaymentStorage;
pub use types::{
    Amount, Currency, Customer, FiatCurrency, PaymentIntent, PaymentMethod, PaymentStatus,
    Transaction, TransactionType,
};

// Re-export privacy and ZKP components
pub use zkp::{
    AnonymousSubscription, SubscriptionProof, SubscriptionTier, VerificationRequest,
    VerificationResult, ZKProofEngine,
};

pub use did_integration::{
    DIDAccessRequest, DIDAccessResponse, DIDManager, DIDSession, PortableSubscriptionProof,
    RotationReason,
};

#[cfg(feature = "wasm-support")]
pub use wasm_pwa::{
    BrowserVerificationRequest, BrowserVerificationResponse, PWACapabilities, WasmZKEngine,
};

// #[cfg(feature = "substrate")]
// pub use substrate_integration::{
//     SubstrateGatewayConfig, SubstratePaymentGateway, SubstratePaymentProcessor,
//     SubstrateTransaction,
// };

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Get library information
pub fn info() -> LibraryInfo {
    LibraryInfo {
        name: NAME,
        version: VERSION,
        features: get_enabled_features(),
    }
}

/// Library information structure
#[derive(Debug, Clone)]
pub struct LibraryInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub features: Vec<&'static str>,
}

/// Get list of enabled features
fn get_enabled_features() -> Vec<&'static str> {
    let mut features = Vec::new();

    #[cfg(feature = "sqlite")]
    features.push("sqlite");

    #[cfg(feature = "postgres")]
    features.push("postgres");

    #[cfg(feature = "substrate")]
    features.push("substrate");

    #[cfg(feature = "http-gateway")]
    features.push("http-gateway");

    #[cfg(feature = "crypto-advanced")]
    features.push("crypto-advanced");

    // Privacy and ZKP features
    #[cfg(feature = "zkp-payments")]
    features.push("zkp-payments");

    #[cfg(feature = "anonymous-subscriptions")]
    features.push("anonymous-subscriptions");

    #[cfg(feature = "did-integration")]
    features.push("did-integration");

    #[cfg(feature = "wasm-support")]
    features.push("wasm-support");

    features
}

/// Prelude module for common imports
// Re-export core types for better integration
pub use synapsed_core::{SynapsedError, SynapsedResult};
pub use synapsed_core::traits::{Observable, Configurable, Identifiable, Validatable};

// Map payment errors to synapsed-core errors
impl From<PaymentError> for SynapsedError {
    fn from(err: PaymentError) -> Self {
        match err {
            PaymentError::InvalidAmount { message } => SynapsedError::InvalidInput(message),
            PaymentError::ProcessingFailed { message, .. } => SynapsedError::Payment(message),
            PaymentError::GatewayError { message, .. } => SynapsedError::Payment(message),
            PaymentError::InsufficientFunds { requested, available } => {
                SynapsedError::Payment(format!("Insufficient funds: requested {}, available {}", requested, available))
            },
            PaymentError::PaymentNotFound { payment_id } => SynapsedError::NotFound(format!("Payment {}", payment_id)),
            PaymentError::TransactionNotFound { transaction_id } => SynapsedError::NotFound(format!("Transaction {}", transaction_id)),
            PaymentError::ConfigurationError { message } => SynapsedError::Configuration(message),
            PaymentError::NetworkError { message } => SynapsedError::Network(message),
            PaymentError::SerializationError { message } => SynapsedError::Serialization(message),
            PaymentError::DatabaseError { message } => SynapsedError::Storage(message),
            PaymentError::ValidationError { message, .. } => SynapsedError::InvalidInput(message),
            PaymentError::AuthenticationError { message } => SynapsedError::Authentication(message),
            PaymentError::InternalError { message } => SynapsedError::Internal(message),
            PaymentError::Timeout { operation } => SynapsedError::Timeout(format!("Operation {} timed out", operation)),
            PaymentError::CryptographyError { message } => SynapsedError::Cryptographic(message),
            PaymentError::UnsupportedCurrency { currency } => SynapsedError::InvalidInput(format!("Unsupported currency: {}", currency)),
            PaymentError::InvalidPaymentMethod { method } => SynapsedError::InvalidInput(format!("Invalid payment method: {}", method)),
            PaymentError::RiskBlocked { reason } => SynapsedError::PermissionDenied(format!("Risk blocked: {}", reason)),
            PaymentError::PaymentExpired { payment_id } => SynapsedError::InvalidInput(format!("Payment {} expired", payment_id)),
            PaymentError::DuplicateTransaction { transaction_id } => SynapsedError::InvalidInput(format!("Duplicate transaction: {}", transaction_id)),
            PaymentError::RefundError { message } => SynapsedError::Payment(format!("Refund error: {}", message)),
            PaymentError::SubstrateError { message } => SynapsedError::Internal(format!("Substrate error: {}", message)),
            PaymentError::RateLimitExceeded { message } => SynapsedError::InvalidInput(format!("Rate limit exceeded: {}", message)),
            PaymentError::CurrencyConversionFailed { from, to } => SynapsedError::InvalidInput(format!("Currency conversion failed from {} to {}", from, to)),
            PaymentError::TransactionAlreadyProcessed { transaction_id } => SynapsedError::InvalidInput(format!("Transaction {} already processed", transaction_id)),
            // ZKP and DID related errors
            PaymentError::ZKProofError { message } => SynapsedError::Cryptographic(format!("ZK proof error: {}", message)),
            PaymentError::InvalidDID { did } => SynapsedError::Did(format!("Invalid DID: {}", did)),
            PaymentError::DIDMismatch { expected, provided } => SynapsedError::Did(format!("DID mismatch: expected {}, provided {}", expected, provided)),
            PaymentError::InvalidSignature { message } => SynapsedError::Cryptographic(format!("Invalid signature: {}", message)),
            PaymentError::AccessDenied { resource } => SynapsedError::PermissionDenied(format!("Access denied to resource: {}", resource)),
            _ => SynapsedError::Internal(err.to_string()),
        }
    }
}

pub mod prelude {
    pub use crate::builder::{PaymentManager, PaymentManagerBuilder};
    pub use crate::error::{PaymentError, PaymentResult};
    pub use crate::types::{
        Amount, Currency, FiatCurrency, PaymentIntent, PaymentMethod, PaymentStatus,
    };
    
    // Re-export core types
    pub use synapsed_core::{SynapsedError, SynapsedResult};
    pub use synapsed_core::traits::{Observable, Configurable, Identifiable, Validatable};

    // ZKP and privacy features
    pub use crate::zkp::{SubscriptionTier, ZKProofEngine, VerificationRequest};

    pub use crate::did_integration::{DIDManager, DIDAccessRequest};

    #[cfg(feature = "wasm-support")]
    pub use crate::wasm_pwa::WasmZKEngine;
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use rust_decimal::Decimal;
    use crate::types::Amount;

    #[test]
    fn test_library_info() {
        let info = info();
        assert_eq!(info.name, "synapsed-payments");
        assert!(!info.version.is_empty());
        // Features might be empty if no features are enabled
    }

    #[tokio::test]
    async fn test_full_payment_flow() {
        // Create payment manager
        let manager = PaymentManagerBuilder::development()
            .build()
            .expect("Failed to build payment manager");

        // Create payment intent
        let amount = Amount::new(
            Decimal::new(5000, 2), // $50.00
            Currency::Fiat(FiatCurrency::USD),
        ).expect("Failed to create amount");

        let payment = manager
            .create_payment(amount, "Integration test payment".to_string(), None)
            .await
            .expect("Failed to create payment");

        assert_eq!(payment.status, PaymentStatus::Pending);
        assert_eq!(payment.description, "Integration test payment");

        // Check payment status
        let status = manager
            .get_payment_status(payment.id)
            .await
            .expect("Failed to get payment status");

        assert_eq!(status, PaymentStatus::Pending);
    }

    #[tokio::test]
    async fn test_mock_payment_processing() {
        use crate::types::PaymentMethod;

        let manager = PaymentManagerBuilder::development()
            .build()
            .expect("Failed to build payment manager");

        // Create payment
        let amount = Amount::new(
            Decimal::new(2500, 2), // $25.00
            Currency::Fiat(FiatCurrency::USD),
        ).expect("Failed to create amount");

        let payment = manager
            .create_payment(amount, "Mock payment test".to_string(), None)
            .await
            .expect("Failed to create payment");

        // Create mock payment method
        let payment_method = PaymentMethod::CreditCard {
            last_four: "4242".to_string(),
            brand: "Visa".to_string(),
            exp_month: 12,
            exp_year: 2025,
            holder_name: "John Doe".to_string(),
        };

        // Process payment (this will use mock gateway)
        let transaction = manager
            .process_payment(payment.id, payment_method)
            .await
            .expect("Failed to process payment");

        assert_eq!(transaction.payment_id, payment.id);
        assert_eq!(transaction.status, crate::types::TransactionStatus::Completed);
        assert!(transaction.gateway_transaction_id.is_some());
    }

    #[tokio::test]
    async fn test_payment_refund() {
        use crate::types::PaymentMethod;

        let manager = PaymentManagerBuilder::development()
            .build()
            .expect("Failed to build payment manager");

        // Create and process payment
        let amount = Amount::new(
            Decimal::new(7500, 2), // $75.00
            Currency::Fiat(FiatCurrency::USD),
        ).expect("Failed to create amount");

        let payment = manager
            .create_payment(amount.clone(), "Refund test payment".to_string(), None)
            .await
            .expect("Failed to create payment");

        let payment_method = PaymentMethod::CreditCard {
            last_four: "4242".to_string(),
            brand: "Visa".to_string(),
            exp_month: 12,
            exp_year: 2025,
            holder_name: "John Doe".to_string(),
        };

        manager
            .process_payment(payment.id, payment_method)
            .await
            .expect("Failed to process payment");

        // Process refund
        let partial_refund_amount = Amount::new(
            Decimal::new(2500, 2), // $25.00 partial refund
            Currency::Fiat(FiatCurrency::USD),
        ).expect("Failed to create refund amount");

        let refund = manager
            .refund_payment(
                payment.id,
                Some(partial_refund_amount),
                Some("Customer requested refund".to_string()),
            )
            .await
            .expect("Failed to process refund");

        assert_eq!(refund.payment_id, payment.id);
        assert_eq!(refund.status, PaymentStatus::Completed);
        assert_eq!(refund.reason, Some("Customer requested refund".to_string()));
    }

    #[tokio::test]
    async fn test_health_check() {
        let manager = PaymentManagerBuilder::development()
            .build()
            .expect("Failed to build payment manager");

        let health = manager
            .health_check()
            .await
            .expect("Health check failed");

        assert!(!health.is_empty());
        assert!(health.contains_key("gateway_mock_primary"));
    }

    #[test]
    fn test_amount_operations() {
        let amount1 = Amount::new(
            Decimal::new(10000, 2),
            Currency::Fiat(FiatCurrency::USD),
        ).expect("Failed to create amount");

        assert!(amount1.is_positive());
        assert!(!amount1.is_zero());

        let zero_amount = Amount::new(
            Decimal::ZERO,
            Currency::Fiat(FiatCurrency::USD),
        ).expect("Failed to create zero amount");

        assert!(!zero_amount.is_positive());
        assert!(zero_amount.is_zero());
    }

    #[test]
    fn test_currency_display() {
        let usd = Currency::Fiat(FiatCurrency::USD);
        assert_eq!(usd.to_string(), "USD");

        let btc = Currency::Crypto(crate::types::CryptoCurrency::Bitcoin);
        assert_eq!(btc.to_string(), "Bitcoin");

        let custom = Currency::Token("CUSTOM".to_string());
        assert_eq!(custom.to_string(), "CUSTOM");
    }

    #[test]
    fn test_payment_intent_expiry() {
        use chrono::{Duration, Utc};

        let amount = Amount::new(
            Decimal::new(10000, 2),
            Currency::Fiat(FiatCurrency::USD),
        ).expect("Failed to create amount");

        let mut payment = PaymentIntent::new(amount, "Test payment".to_string());

        // Not expired by default
        assert!(!payment.is_expired());
        assert!(payment.can_be_processed());

        // Set expiry in the past
        payment.expires_at = Some(Utc::now() - Duration::minutes(5));
        assert!(payment.is_expired());
        assert!(!payment.can_be_processed());

        // Set expiry in the future
        payment.expires_at = Some(Utc::now() + Duration::hours(1));
        assert!(!payment.is_expired());
        assert!(payment.can_be_processed());
    }
}