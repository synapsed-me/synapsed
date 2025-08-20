//! Error types for the synapsed-wasm crate


/// Result type for WASM operations
pub type WasmResult<T> = Result<T, WasmError>;

/// Comprehensive error types for WASM operations
#[derive(Debug, thiserror::Error)]
pub enum WasmError {
    /// Runtime initialization failed
    #[error("Runtime initialization failed: {0}")]
    RuntimeInit(String),

    /// Module loading failed
    #[error("Module loading failed: {0}")]
    ModuleLoad(String),

    /// Module compilation failed
    #[error("Module compilation failed: {0}")]
    ModuleCompilation(String),

    /// Module instantiation failed
    #[error("Module instantiation failed: {0}")]
    ModuleInstantiation(String),

    /// Function execution failed
    #[error("Function execution failed: {0}")]
    FunctionExecution(String),

    /// Function not found in module
    #[error("Function '{0}' not found in module")]
    FunctionNotFound(String),

    /// Invalid function signature
    #[error("Invalid function signature: expected {expected}, got {actual}")]
    InvalidSignature { 
        /// Expected signature
        expected: String, 
        /// Actual signature
        actual: String 
    },

    /// Memory access violation
    #[error("Memory access violation: {0}")]
    MemoryViolation(String),

    /// Memory allocation failed
    #[error("Memory allocation failed: {0}")]
    MemoryAllocation(String),

    /// Execution timeout
    #[error("Execution timed out after {seconds} seconds")]
    ExecutionTimeout { 
        /// Timeout duration in seconds
        seconds: u64 
    },

    /// Resource limit exceeded
    #[error("Resource limit exceeded: {resource} ({limit})")]
    ResourceLimitExceeded { 
        /// Resource type that exceeded limit
        resource: String, 
        /// The limit that was exceeded
        limit: String 
    },

    /// Invalid WASM bytecode
    #[error("Invalid WASM bytecode: {0}")]
    InvalidBytecode(String),

    /// Unsupported WASM feature
    #[error("Unsupported WASM feature: {0}")]
    UnsupportedFeature(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Type conversion error
    #[error("Type conversion error: {0}")]
    TypeConversion(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Host function error
    #[error("Host function error: {0}")]
    HostFunction(String),

    /// Security violation
    #[error("Security violation: {0}")]
    SecurityViolation(String),

    /// Cryptographic error
    #[cfg(feature = "crypto-modules")]
    #[error("Cryptographic error: {0}")]
    Cryptographic(String),

    /// Storage error
    #[cfg(feature = "storage-modules")]
    #[error("Storage error: {0}")]
    Storage(String),

    /// Network error
    #[cfg(feature = "network-modules")]
    #[error("Network error: {0}")]
    Network(String),

    /// Payment processing error
    #[cfg(feature = "payment-modules")]
    #[error("Payment processing error: {0}")]
    Payment(String),

    /// Substrate integration error
    #[cfg(feature = "substrate-modules")]
    #[error("Substrate integration error: {0}")]
    Substrate(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Wasmtime error
    #[error("Wasmtime error: {0}")]
    Wasmtime(#[from] wasmtime::Error),

    /// WASM parser error
    #[error("WASM parser error: {0}")]
    WasmParser(#[from] wasmparser::BinaryReaderError),

    /// Serde JSON error
    #[error("JSON serialization error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    /// Bincode error
    #[error("Binary serialization error: {0}")]
    Bincode(#[from] bincode::Error),

    /// Generic error for compatibility
    #[error("Generic error: {0}")]
    Generic(String),
}

impl WasmError {
    /// Create a new runtime initialization error
    pub fn runtime_init<S: Into<String>>(msg: S) -> Self {
        Self::RuntimeInit(msg.into())
    }

    /// Create a new module loading error
    pub fn module_load<S: Into<String>>(msg: S) -> Self {
        Self::ModuleLoad(msg.into())
    }

    /// Create a new function execution error
    pub fn function_execution<S: Into<String>>(msg: S) -> Self {
        Self::FunctionExecution(msg.into())
    }

    /// Create a new memory violation error
    pub fn memory_violation<S: Into<String>>(msg: S) -> Self {
        Self::MemoryViolation(msg.into())
    }

    /// Create a new execution timeout error
    pub fn execution_timeout(seconds: u64) -> Self {
        Self::ExecutionTimeout { seconds }
    }

    /// Create a new resource limit exceeded error
    pub fn resource_limit_exceeded<S: Into<String>>(resource: S, limit: S) -> Self {
        Self::ResourceLimitExceeded {
            resource: resource.into(),
            limit: limit.into(),
        }
    }

    /// Create a new security violation error
    pub fn security_violation<S: Into<String>>(msg: S) -> Self {
        Self::SecurityViolation(msg.into())
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            WasmError::ExecutionTimeout { .. }
                | WasmError::ResourceLimitExceeded { .. }
                | WasmError::FunctionExecution(_)
                | WasmError::MemoryViolation(_)
        )
    }

    /// Check if this error is related to security
    pub fn is_security_related(&self) -> bool {
        matches!(
            self,
            WasmError::SecurityViolation(_)
                | WasmError::MemoryViolation(_)
                | WasmError::ResourceLimitExceeded { .. }
        )
    }

    /// Get error category for logging/metrics
    pub fn category(&self) -> &'static str {
        match self {
            WasmError::RuntimeInit(_) => "runtime",
            WasmError::ModuleLoad(_) | WasmError::ModuleCompilation(_) | WasmError::ModuleInstantiation(_) => "module",
            WasmError::FunctionExecution(_) | WasmError::FunctionNotFound(_) | WasmError::InvalidSignature { .. } => "execution",
            WasmError::MemoryViolation(_) | WasmError::MemoryAllocation(_) => "memory",
            WasmError::ExecutionTimeout { .. } | WasmError::ResourceLimitExceeded { .. } => "limits",
            WasmError::SecurityViolation(_) => "security",
            WasmError::InvalidBytecode(_) | WasmError::UnsupportedFeature(_) => "validation",
            WasmError::Serialization(_) | WasmError::TypeConversion(_) => "serialization",
            WasmError::Configuration(_) => "config",
            WasmError::HostFunction(_) => "host",
            #[cfg(feature = "crypto-modules")]
            WasmError::Cryptographic(_) => "crypto",
            #[cfg(feature = "storage-modules")]
            WasmError::Storage(_) => "storage",
            #[cfg(feature = "network-modules")]
            WasmError::Network(_) => "network",
            #[cfg(feature = "payment-modules")]
            WasmError::Payment(_) => "payment",
            #[cfg(feature = "substrate-modules")]
            WasmError::Substrate(_) => "substrate",
            WasmError::Io(_) => "io",
            WasmError::Wasmtime(_) => "wasmtime",
            WasmError::WasmParser(_) => "parser",
            WasmError::SerdeJson(_) | WasmError::Bincode(_) => "serialization",
            WasmError::Generic(_) => "generic",
        }
    }
}

// Note: Display is implemented automatically by thiserror

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = WasmError::runtime_init("test error");
        assert!(matches!(err, WasmError::RuntimeInit(_)));
        assert_eq!(err.category(), "runtime");
    }

    #[test]
    fn test_error_recovery() {
        let recoverable = WasmError::execution_timeout(30);
        assert!(recoverable.is_recoverable());

        let non_recoverable = WasmError::runtime_init("critical error");
        assert!(!non_recoverable.is_recoverable());
    }

    #[test]
    fn test_security_classification() {
        let security_err = WasmError::security_violation("unauthorized access");
        assert!(security_err.is_security_related());

        let normal_err = WasmError::module_load("file not found");
        assert!(!normal_err.is_security_related());
    }

    #[test]
    fn test_error_categories() {
        assert_eq!(WasmError::runtime_init("test").category(), "runtime");
        assert_eq!(WasmError::module_load("test").category(), "module");
        assert_eq!(WasmError::function_execution("test").category(), "execution");
        assert_eq!(WasmError::memory_violation("test").category(), "memory");
        assert_eq!(WasmError::security_violation("test").category(), "security");
    }
}