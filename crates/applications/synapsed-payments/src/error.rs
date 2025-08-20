use thiserror::Error;

/// Main error type for the payments crate
#[derive(Error, Debug)]
pub enum PaymentError {
    /// Payment processing errors
    #[error("Payment processing failed: {message}")]
    ProcessingFailed { message: String, code: Option<String> },

    /// Gateway communication errors
    #[error("Gateway error: {gateway} - {message}")]
    GatewayError { gateway: String, message: String },

    /// Validation errors
    #[error("Validation failed: {field} - {message}")]
    ValidationError { field: String, message: String },

    /// Configuration errors  
    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },

    /// Zero-knowledge proof error
    #[error("ZK proof error: {message}")]
    ZKProofError { message: String },

    /// Subscription not found
    #[error("Subscription not found: {subscription_id}")]
    SubscriptionNotFound { subscription_id: String },

    /// Subscription expired
    #[error("Subscription expired: {subscription_id}")]
    SubscriptionExpired { subscription_id: String },

    /// Invalid proof
    #[error("Invalid proof: {message}")]
    InvalidProof { message: String },

    /// Insufficient subscription tier
    #[error("Insufficient tier: required {required}, provided {provided}")]
    InsufficientTier { required: u32, provided: u32 },

    /// DID-related errors
    #[error("Invalid DID: {did}")]
    InvalidDID { did: String },

    /// DID mismatch
    #[error("DID mismatch: expected {expected}, provided {provided}")]
    DIDMismatch { expected: String, provided: String },

    /// Invalid signature
    #[error("Invalid signature: {message}")]
    InvalidSignature { message: String },

    /// Access denied
    #[error("Access denied to resource: {resource}")]
    AccessDenied { resource: String },

    /// Invalid recovery proof
    #[error("Invalid recovery proof for method: {method}")]
    InvalidRecoveryProof { method: String },

    /// Anonymous subscription error
    #[error("Anonymous subscription error: {message}")]
    AnonymousSubscriptionError { message: String },

    /// Authentication/authorization errors
    #[error("Authentication failed: {message}")]
    AuthenticationError { message: String },

    /// Insufficient funds
    #[error("Insufficient funds: requested {requested}, available {available}")]
    InsufficientFunds { requested: String, available: String },

    /// Payment not found
    #[error("Payment not found: {payment_id}")]
    PaymentNotFound { payment_id: String },

    /// Transaction not found
    #[error("Transaction not found: {transaction_id}")]
    TransactionNotFound { transaction_id: String },

    /// Invalid payment method
    #[error("Invalid payment method: {method}")]
    InvalidPaymentMethod { method: String },

    /// Currency not supported
    #[error("Currency not supported: {currency}")]
    UnsupportedCurrency { currency: String },

    /// Amount validation errors
    #[error("Invalid amount: {message}")]
    InvalidAmount { message: String },

    /// Expired payment
    #[error("Payment expired: {payment_id}")]
    PaymentExpired { payment_id: String },

    /// Duplicate transaction
    #[error("Duplicate transaction: {transaction_id}")]
    DuplicateTransaction { transaction_id: String },

    /// Risk management errors
    #[error("Payment blocked by risk management: {reason}")]
    RiskBlocked { reason: String },

    /// Refund errors
    #[error("Refund failed: {message}")]
    RefundError { message: String },

    /// Webhook errors
    #[error("Webhook processing failed: {message}")]
    WebhookError { message: String },

    /// Database errors
    #[error("Database error: {message}")]
    DatabaseError { message: String },

    /// Network/HTTP errors
    #[error("Network error: {message}")]
    NetworkError { message: String },

    /// Serialization/deserialization errors
    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    /// Cryptography errors
    #[error("Cryptographic error: {message}")]
    CryptographyError { message: String },

    /// Substrate integration errors
    #[error("Substrate error: {message}")]
    SubstrateError { message: String },

    /// Rate limiting errors
    #[error("Rate limit exceeded: {message}")]
    RateLimitExceeded { message: String },

    /// Timeout errors
    #[error("Operation timed out: {operation}")]
    Timeout { operation: String },

    /// Internal system errors
    #[error("Internal error: {message}")]
    InternalError { message: String },

    /// Currency conversion failed
    #[error("Currency conversion failed from {from} to {to}")]
    CurrencyConversionFailed { from: String, to: String },

    /// Transaction already processed
    #[error("Transaction already processed: {transaction_id}")]
    TransactionAlreadyProcessed { transaction_id: String },
}

/// Result type alias for payment operations
pub type PaymentResult<T> = Result<T, PaymentError>;

/// Gateway-specific error codes
#[derive(Debug, Clone, PartialEq)]
pub enum GatewayErrorCode {
    /// Card declined
    CardDeclined,
    /// Insufficient funds
    InsufficientFunds,
    /// Invalid card number
    InvalidCard,
    /// Expired card
    ExpiredCard,
    /// Invalid CVV
    InvalidCvv,
    /// Processing error
    ProcessingError,
    /// Authentication required (3DS)
    AuthenticationRequired,
    /// Gateway timeout
    Timeout,
    /// Unknown error
    Unknown(String),
}

/// Risk assessment error types
#[derive(Debug, Clone, PartialEq)]
pub enum RiskError {
    /// High risk transaction blocked
    HighRisk,
    /// Fraud detected
    FraudDetected,
    /// Velocity limit exceeded
    VelocityLimitExceeded,
    /// Geographic restriction
    GeographicRestriction,
    /// Unknown risk factor
    Unknown(String),
}

impl PaymentError {
    /// Create a processing error
    pub fn processing_failed(message: impl Into<String>) -> Self {
        Self::ProcessingFailed {
            message: message.into(),
            code: None,
        }
    }

    /// Create a processing error with code
    pub fn processing_failed_with_code(
        message: impl Into<String>, 
        code: impl Into<String>
    ) -> Self {
        Self::ProcessingFailed {
            message: message.into(),
            code: Some(code.into()),
        }
    }

    /// Create a gateway error
    pub fn gateway_error(gateway: impl Into<String>, message: impl Into<String>) -> Self {
        Self::GatewayError {
            gateway: gateway.into(),
            message: message.into(),
        }
    }

    /// Create a validation error
    pub fn validation_error(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ValidationError {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create a configuration error
    pub fn configuration_error(message: impl Into<String>) -> Self {
        Self::ConfigurationError {
            message: message.into(),
        }
    }

    /// Create an authentication error
    pub fn authentication_error(message: impl Into<String>) -> Self {
        Self::AuthenticationError {
            message: message.into(),
        }
    }

    /// Create an insufficient funds error
    pub fn insufficient_funds(requested: impl Into<String>, available: impl Into<String>) -> Self {
        Self::InsufficientFunds {
            requested: requested.into(),
            available: available.into(),
        }
    }

    /// Create a payment not found error
    pub fn payment_not_found(payment_id: impl Into<String>) -> Self {
        Self::PaymentNotFound {
            payment_id: payment_id.into(),
        }
    }

    /// Create a risk blocked error
    pub fn risk_blocked(reason: impl Into<String>) -> Self {
        Self::RiskBlocked {
            reason: reason.into(),
        }
    }

    /// Create an internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError {
            message: message.into(),
        }
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self, 
            PaymentError::NetworkError { .. } |
            PaymentError::Timeout { .. } |
            PaymentError::GatewayError { .. } |
            PaymentError::InternalError { .. }
        )
    }

    /// Check if error is permanent
    pub fn is_permanent(&self) -> bool {
        matches!(self,
            PaymentError::ValidationError { .. } |
            PaymentError::InvalidPaymentMethod { .. } |
            PaymentError::UnsupportedCurrency { .. } |
            PaymentError::InvalidAmount { .. } |
            PaymentError::PaymentExpired { .. } |
            PaymentError::DuplicateTransaction { .. } |
            PaymentError::AuthenticationError { .. }
        )
    }

    /// Get error code for external systems
    pub fn code(&self) -> &'static str {
        match self {
            PaymentError::ProcessingFailed { .. } => "PROCESSING_FAILED",
            PaymentError::GatewayError { .. } => "GATEWAY_ERROR",
            PaymentError::ValidationError { .. } => "VALIDATION_ERROR",
            PaymentError::ConfigurationError { .. } => "CONFIGURATION_ERROR",
            PaymentError::AuthenticationError { .. } => "AUTHENTICATION_ERROR",
            PaymentError::InsufficientFunds { .. } => "INSUFFICIENT_FUNDS",
            PaymentError::PaymentNotFound { .. } => "PAYMENT_NOT_FOUND",
            PaymentError::TransactionNotFound { .. } => "TRANSACTION_NOT_FOUND",
            PaymentError::InvalidPaymentMethod { .. } => "INVALID_PAYMENT_METHOD",
            PaymentError::UnsupportedCurrency { .. } => "UNSUPPORTED_CURRENCY",
            PaymentError::InvalidAmount { .. } => "INVALID_AMOUNT",
            PaymentError::PaymentExpired { .. } => "PAYMENT_EXPIRED",
            PaymentError::DuplicateTransaction { .. } => "DUPLICATE_TRANSACTION",
            PaymentError::RiskBlocked { .. } => "RISK_BLOCKED",
            PaymentError::RefundError { .. } => "REFUND_ERROR",
            PaymentError::WebhookError { .. } => "WEBHOOK_ERROR",
            PaymentError::DatabaseError { .. } => "DATABASE_ERROR",
            PaymentError::NetworkError { .. } => "NETWORK_ERROR",
            PaymentError::SerializationError { .. } => "SERIALIZATION_ERROR",
            PaymentError::CryptographyError { .. } => "CRYPTOGRAPHY_ERROR",
            PaymentError::SubstrateError { .. } => "SUBSTRATE_ERROR",
            PaymentError::RateLimitExceeded { .. } => "RATE_LIMIT_EXCEEDED",
            PaymentError::Timeout { .. } => "TIMEOUT",
            PaymentError::InternalError { .. } => "INTERNAL_ERROR",
            PaymentError::CurrencyConversionFailed { .. } => "CURRENCY_CONVERSION_FAILED",
            PaymentError::TransactionAlreadyProcessed { .. } => "TRANSACTION_ALREADY_PROCESSED",
            PaymentError::ZKProofError { .. } => "ZK_PROOF_ERROR",
            PaymentError::SubscriptionNotFound { .. } => "SUBSCRIPTION_NOT_FOUND",
            PaymentError::SubscriptionExpired { .. } => "SUBSCRIPTION_EXPIRED",
            PaymentError::InvalidProof { .. } => "INVALID_PROOF",
            PaymentError::InsufficientTier { .. } => "INSUFFICIENT_TIER",
            PaymentError::InvalidDID { .. } => "INVALID_DID",
            PaymentError::DIDMismatch { .. } => "DID_MISMATCH",
            PaymentError::InvalidSignature { .. } => "INVALID_SIGNATURE",
            PaymentError::AccessDenied { .. } => "ACCESS_DENIED",
            PaymentError::InvalidRecoveryProof { .. } => "INVALID_RECOVERY_PROOF",
            PaymentError::AnonymousSubscriptionError { .. } => "ANONYMOUS_SUBSCRIPTION_ERROR",
        }
    }
}

impl From<serde_json::Error> for PaymentError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError {
            message: err.to_string(),
        }
    }
}

impl From<validator::ValidationErrors> for PaymentError {
    fn from(err: validator::ValidationErrors) -> Self {
        Self::ValidationError {
            field: "multiple".to_string(),
            message: err.to_string(),
        }
    }
}

#[cfg(feature = "sqlx")]
impl From<sqlx::Error> for PaymentError {
    fn from(err: sqlx::Error) -> Self {
        Self::DatabaseError {
            message: err.to_string(),
        }
    }
}

#[cfg(feature = "http-gateway")]
impl From<reqwest::Error> for PaymentError {
    fn from(err: reqwest::Error) -> Self {
        Self::NetworkError {
            message: err.to_string(),
        }
    }
}

impl From<synapsed_crypto::error::Error> for PaymentError {
    fn from(err: synapsed_crypto::error::Error) -> Self {
        Self::CryptographyError {
            message: format!("{:?}", err),
        }
    }
}

impl GatewayErrorCode {
    /// Convert error code to string
    pub fn as_str(&self) -> &str {
        match self {
            GatewayErrorCode::CardDeclined => "CARD_DECLINED",
            GatewayErrorCode::InsufficientFunds => "INSUFFICIENT_FUNDS",
            GatewayErrorCode::InvalidCard => "INVALID_CARD",
            GatewayErrorCode::ExpiredCard => "EXPIRED_CARD",
            GatewayErrorCode::InvalidCvv => "INVALID_CVV",
            GatewayErrorCode::ProcessingError => "PROCESSING_ERROR",
            GatewayErrorCode::AuthenticationRequired => "AUTHENTICATION_REQUIRED",
            GatewayErrorCode::Timeout => "TIMEOUT",
            GatewayErrorCode::Unknown(code) => code,
        }
    }

    /// Check if error code indicates a retryable error
    pub fn is_retryable(&self) -> bool {
        matches!(self, 
            GatewayErrorCode::ProcessingError |
            GatewayErrorCode::Timeout
        )
    }
}

impl RiskError {
    /// Convert risk error to string
    pub fn as_str(&self) -> &str {
        match self {
            RiskError::HighRisk => "HIGH_RISK",
            RiskError::FraudDetected => "FRAUD_DETECTED", 
            RiskError::VelocityLimitExceeded => "VELOCITY_LIMIT_EXCEEDED",
            RiskError::GeographicRestriction => "GEOGRAPHIC_RESTRICTION",
            RiskError::Unknown(code) => code,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_error_creation() {
        let error = PaymentError::processing_failed("Test error");
        assert_eq!(error.code(), "PROCESSING_FAILED");
        assert!(!error.is_permanent());
    }

    #[test]
    fn test_error_retryable() {
        let network_error = PaymentError::NetworkError {
            message: "Connection failed".to_string(),
        };
        assert!(network_error.is_retryable());

        let validation_error = PaymentError::ValidationError {
            field: "amount".to_string(),
            message: "Invalid".to_string(),
        };
        assert!(!validation_error.is_retryable());
        assert!(validation_error.is_permanent());
    }

    #[test]
    fn test_gateway_error_code() {
        let code = GatewayErrorCode::CardDeclined;
        assert_eq!(code.as_str(), "CARD_DECLINED");
        assert!(!code.is_retryable());

        let processing_error = GatewayErrorCode::ProcessingError;
        assert!(processing_error.is_retryable());
    }
}