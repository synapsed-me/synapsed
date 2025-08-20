//! Error types for cryptographic operations

use core::fmt;

/// Result type alias using our Error type
pub type Result<T> = core::result::Result<T, Error>;

/// Errors that can occur during cryptographic operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Invalid key size
    InvalidKeySize,
    
    /// Invalid size
    InvalidSize,
    
    /// Invalid ciphertext
    InvalidCiphertext,
    
    /// Invalid signature
    InvalidSignature,
    
    /// Invalid parameter
    InvalidParameter,
    
    /// Invalid input
    InvalidInput,
    
    /// Invalid encoding (e.g., hex decode error)
    InvalidEncoding,
    
    /// Random number generator failure
    RandomnessError,
    
    /// Polynomial arithmetic error
    PolynomialError,
    
    /// NTT (Number Theoretic Transform) error
    NttError,
    
    /// Hash function error
    HashError,
    
    /// Serialization/deserialization error
    SerializationError,
    
    /// Generic crypto error
    CryptoError,
    
    /// Unsupported compression parameter
    UnsupportedCompression,
    
    /// Unsupported modulus for sampling
    UnsupportedModulus,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidKeySize => write!(f, "Invalid key size"),
            Error::InvalidSize => write!(f, "Invalid size"),
            Error::InvalidCiphertext => write!(f, "Invalid ciphertext"),
            Error::InvalidSignature => write!(f, "Invalid signature"),
            Error::InvalidParameter => write!(f, "Invalid parameter"),
            Error::InvalidInput => write!(f, "Invalid input"),
            Error::InvalidEncoding => write!(f, "Invalid encoding"),
            Error::RandomnessError => write!(f, "Random number generator error"),
            Error::PolynomialError => write!(f, "Polynomial arithmetic error"),
            Error::NttError => write!(f, "NTT error"),
            Error::HashError => write!(f, "Hash function error"),
            Error::SerializationError => write!(f, "Serialization error"),
            Error::CryptoError => write!(f, "Cryptographic error"),
            Error::UnsupportedCompression => write!(f, "Unsupported compression parameter"),
            Error::UnsupportedModulus => write!(f, "Unsupported modulus for sampling"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}