//! Error types for the networking layer.

use std::fmt;
use thiserror::Error;

/// Type alias for Results in this crate.
pub type Result<T> = std::result::Result<T, NetworkError>;

/// Main error type for networking operations.
#[derive(Error, Debug)]
pub enum NetworkError {
    /// Transport-related errors
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),
    
    /// Security-related errors
    #[error("Security error: {0}")]
    Security(#[from] SecurityError),
    
    /// Privacy-related errors
    #[error("Privacy error: {0}")]
    Privacy(#[from] PrivacyError),
    
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    /// Connection errors
    #[error("Connection error: {0}")]
    Connection(String),
    
    /// Protocol errors
    #[error("Protocol error: {0}")]
    Protocol(String),
    
    /// Observability errors
    #[error("Observability error: {0}")]
    Observability(String),
    
    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Other errors
    #[error(transparent)]
    Other(#[from] anyhow::Error),
    
    /// Mock-related errors (for testing)
    #[error("Mock error: {0}")]
    Mock(String),
}

/// Transport-specific errors.
#[derive(Error, Debug)]
pub enum TransportError {
    #[error("QUIC error: {0}")]
    Quic(String),
    
    #[error("WebRTC error: {0}")]
    WebRtc(String),
    
    #[error("libp2p error: {0}")]
    Libp2p(String),
    
    #[error("TCP error: {0}")]
    Tcp(String),
    
    #[error("UDP error: {0}")]
    Udp(String),
    
    #[error("WebSocket error: {0}")]
    WebSocket(String),
    
    #[error("Signaling failed: {0}")]
    SignalingFailed(String),
    
    #[error("Transport not available: {0}")]
    NotAvailable(String),
    
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Connection timeout")]
    Timeout,
    
    #[error("Connection timeout: {0}")]
    TimeoutWithMsg(String),
    
    #[error("Connection limit reached: {0}")]
    ConnectionLimitReached(String),
    
    #[error("Not connected: {0}")]
    NotConnected(String),
    
    #[error("Not initialized: {0}")]
    NotInitialized(String),
    
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    
    #[error("All transports failed: {0}")]
    AllTransportsFailed(String),
}

/// Security-specific errors.
#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Encryption error: {0}")]
    Encryption(String),
    
    #[error("Decryption error: {0}")]
    Decryption(String),
    
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("Invalid signature")]
    InvalidSignature,
    
    #[error("Certificate error: {0}")]
    Certificate(String),
    
    #[error("Key exchange failed: {0}")]
    KeyExchange(String),
    
    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),
    
    #[error("Key generation failed: {0}")]
    KeyGeneration(String),
    
    #[error("Signature error: {0}")]
    Signature(String),
    
    #[error("Verification error: {0}")]
    Verification(String),
    
    #[error("Session expired: {0}")]
    SessionExpired(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Deserialization error: {0}")]
    Deserialization(String),
}

/// Privacy-specific errors.
#[derive(Error, Debug)]
pub enum PrivacyError {
    #[error("Anonymization failed: {0}")]
    AnonymizationFailed(String),
    
    #[error("Tor error: {0}")]
    Tor(String),
    
    #[error("Mix network error: {0}")]
    MixNetwork(String),
    
    #[error("Privacy policy violation: {0}")]
    PolicyViolation(String),
}

impl NetworkError {
    /// Returns the severity of this error for observability purposes.
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            NetworkError::Security(_) => ErrorSeverity::Critical,
            NetworkError::Privacy(_) => ErrorSeverity::Critical,
            NetworkError::Transport(TransportError::Timeout) => ErrorSeverity::Minor,
            NetworkError::Transport(_) => ErrorSeverity::Major,
            NetworkError::Configuration(_) => ErrorSeverity::Critical,
            NetworkError::Connection(_) => ErrorSeverity::Major,
            NetworkError::Protocol(_) => ErrorSeverity::Major,
            NetworkError::Observability(_) => ErrorSeverity::Minor,
            NetworkError::Io(_) => ErrorSeverity::Major,
            NetworkError::Other(_) => ErrorSeverity::Major,
            NetworkError::Mock(_) => ErrorSeverity::Minor,
        }
    }
    
    /// Returns an observable error type that doesn't leak sensitive information.
    pub fn observable_type(&self) -> &'static str {
        match self {
            NetworkError::Transport(_) => "transport",
            NetworkError::Security(_) => "security",
            NetworkError::Privacy(_) => "privacy",
            NetworkError::Configuration(_) => "configuration",
            NetworkError::Connection(_) => "connection",
            NetworkError::Protocol(_) => "protocol",
            NetworkError::Observability(_) => "observability",
            NetworkError::Io(_) => "io",
            NetworkError::Other(_) => "other",
            NetworkError::Mock(_) => "mock",
        }
    }
    
    /// Classifies the error for metrics without revealing details.
    pub fn classify(&self) -> ErrorClass {
        match self {
            NetworkError::Transport(TransportError::Timeout) => ErrorClass::Timeout,
            NetworkError::Transport(TransportError::TimeoutWithMsg(_)) => ErrorClass::Timeout,
            NetworkError::Transport(TransportError::ConnectionFailed(_)) => ErrorClass::ConnectionFailure,
            NetworkError::Security(SecurityError::AuthenticationFailed(_)) => ErrorClass::AuthFailure,
            NetworkError::Privacy(_) => ErrorClass::PrivacyViolation,
            NetworkError::Configuration(_) => ErrorClass::ConfigError,
            _ => ErrorClass::Other,
        }
    }
}

/// Error severity levels for monitoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Critical errors that require immediate attention
    Critical,
    /// Major errors that impact functionality
    Major,
    /// Minor errors that can be recovered from
    Minor,
}

/// Error classification for metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ErrorClass {
    /// Connection timeout
    Timeout,
    /// Connection failure
    ConnectionFailure,
    /// Authentication failure
    AuthFailure,
    /// Privacy violation
    PrivacyViolation,
    /// Configuration error
    ConfigError,
    /// Other error
    Other,
}

impl fmt::Display for ErrorClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorClass::Timeout => write!(f, "timeout"),
            ErrorClass::ConnectionFailure => write!(f, "connection_failure"),
            ErrorClass::AuthFailure => write!(f, "auth_failure"),
            ErrorClass::PrivacyViolation => write!(f, "privacy_violation"),
            ErrorClass::ConfigError => write!(f, "config_error"),
            ErrorClass::Other => write!(f, "other"),
        }
    }
}

// Additional From implementations for QUIC-specific errors
impl From<serde_json::Error> for NetworkError {
    fn from(err: serde_json::Error) -> Self {
        NetworkError::Transport(TransportError::ConnectionFailed(format!("Serialization error: {}", err)))
    }
}

impl From<quinn::ConnectError> for NetworkError {
    fn from(err: quinn::ConnectError) -> Self {
        NetworkError::Transport(TransportError::Quic(format!("QUIC connect error: {}", err)))
    }
}

impl From<quinn::ConnectionError> for NetworkError {
    fn from(err: quinn::ConnectionError) -> Self {
        NetworkError::Transport(TransportError::Quic(format!("QUIC connection error: {}", err)))
    }
}

impl From<quinn::WriteError> for NetworkError {
    fn from(err: quinn::WriteError) -> Self {
        NetworkError::Transport(TransportError::Quic(format!("QUIC write error: {}", err)))
    }
}

impl From<rustls::Error> for NetworkError {
    fn from(err: rustls::Error) -> Self {
        NetworkError::Security(SecurityError::Certificate(format!("TLS error: {}", err)))
    }
}

impl From<std::time::SystemTimeError> for NetworkError {
    fn from(err: std::time::SystemTimeError) -> Self {
        NetworkError::Other(anyhow::anyhow!("System time error: {}", err))
    }
}