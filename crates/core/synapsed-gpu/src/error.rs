//! Error types and handling for GPU acceleration operations.

use thiserror::Error;

/// Result type alias for GPU operations.
pub type Result<T> = std::result::Result<T, GpuError>;

/// Comprehensive error types for GPU acceleration operations.
#[derive(Error, Debug, Clone)]
pub enum GpuError {
    /// No GPU devices are available on the system.
    #[error("No GPU devices available")]
    NoDevicesAvailable,

    /// CUDA-specific errors.
    #[error("CUDA error: {message}")]
    CudaError { message: String },

    /// OpenCL-specific errors.
    #[error("OpenCL error: {message}")]
    OpenClError { message: String },

    /// GPU memory allocation or management errors.
    #[error("GPU memory error: {message}")]
    MemoryError { message: String },

    /// Kernel compilation or execution errors.
    #[error("Kernel error: {message}")]
    KernelError { message: String },

    /// Device initialization or communication errors.
    #[error("Device error: {message}")]
    DeviceError { message: String },

    /// Batch processing errors.
    #[error("Batch processing error: {message}")]
    BatchError { message: String },

    /// Fallback processing errors.
    #[error("Fallback processing error: {message}")]
    FallbackError { message: String },

    /// Configuration or validation errors.
    #[error("Configuration error: {message}")]
    ConfigError { message: String },

    /// Resource exhaustion errors.
    #[error("Resource exhausted: {resource}")]
    ResourceExhausted { resource: String },

    /// Timeout errors for long-running operations.
    #[error("Operation timed out after {seconds} seconds")]
    Timeout { seconds: u64 },

    /// Integration errors with synapsed-crypto.
    #[error("Crypto integration error: {message}")]
    CryptoIntegrationError { message: String },

    /// Serialization/deserialization errors.
    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    /// IO errors (file access, etc.).
    #[error("IO error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },

    /// Generic internal errors.
    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl GpuError {
    /// Create a CUDA error with context.
    pub fn cuda(message: impl Into<String>) -> Self {
        Self::CudaError {
            message: message.into(),
        }
    }

    /// Create an OpenCL error with context.
    pub fn opencl(message: impl Into<String>) -> Self {
        Self::OpenClError {
            message: message.into(),
        }
    }

    /// Create a memory error with context.
    pub fn memory(message: impl Into<String>) -> Self {
        Self::MemoryError {
            message: message.into(),
        }
    }

    /// Create a kernel error with context.
    pub fn kernel(message: impl Into<String>) -> Self {
        Self::KernelError {
            message: message.into(),
        }
    }

    /// Create a device error with context.
    pub fn device(message: impl Into<String>) -> Self {
        Self::DeviceError {
            message: message.into(),
        }
    }

    /// Create a batch processing error with context.
    pub fn batch(message: impl Into<String>) -> Self {
        Self::BatchError {
            message: message.into(),
        }
    }

    /// Create a configuration error with context.
    pub fn config(message: impl Into<String>) -> Self {
        Self::ConfigError {
            message: message.into(),
        }
    }

    /// Create an internal error with context.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Check if this error indicates a recoverable condition.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            GpuError::MemoryError { .. }
                | GpuError::ResourceExhausted { .. }
                | GpuError::Timeout { .. }
                | GpuError::DeviceError { .. }
        )
    }

    /// Check if this error suggests fallback to CPU should be used.
    pub fn should_fallback(&self) -> bool {
        matches!(
            self,
            GpuError::NoDevicesAvailable
                | GpuError::DeviceError { .. }
                | GpuError::CudaError { .. }
                | GpuError::OpenClError { .. }
                | GpuError::ResourceExhausted { .. }
        )
    }

    /// Get error category for metrics and logging.
    pub fn category(&self) -> &'static str {
        match self {
            GpuError::NoDevicesAvailable => "device",
            GpuError::CudaError { .. } => "cuda",
            GpuError::OpenClError { .. } => "opencl",
            GpuError::MemoryError { .. } => "memory",
            GpuError::KernelError { .. } => "kernel",
            GpuError::DeviceError { .. } => "device",
            GpuError::BatchError { .. } => "batch",
            GpuError::FallbackError { .. } => "fallback",
            GpuError::ConfigError { .. } => "config",
            GpuError::ResourceExhausted { .. } => "resource",
            GpuError::Timeout { .. } => "timeout",
            GpuError::CryptoIntegrationError { .. } => "crypto",
            GpuError::SerializationError { .. } => "serialization",
            GpuError::IoError { .. } => "io",
            GpuError::Internal { .. } => "internal",
        }
    }
}

#[cfg(feature = "cuda")]
impl From<cudarc::driver::DriverError> for GpuError {
    fn from(err: cudarc::driver::DriverError) -> Self {
        GpuError::cuda(format!("{:?}", err))
    }
}

#[cfg(feature = "opencl")]
impl From<opencl3::error_codes::ClError> for GpuError {
    fn from(err: opencl3::error_codes::ClError) -> Self {
        GpuError::opencl(format!("{:?}", err))
    }
}

impl From<serde_json::Error> for GpuError {
    fn from(err: serde_json::Error) -> Self {
        GpuError::SerializationError {
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let cuda_err = GpuError::cuda("CUDA out of memory");
        assert!(matches!(cuda_err, GpuError::CudaError { .. }));
        assert_eq!(cuda_err.category(), "cuda");
        assert!(cuda_err.should_fallback());
    }

    #[test]
    fn test_error_recoverability() {
        let memory_err = GpuError::memory("Memory allocation failed");
        assert!(memory_err.is_recoverable());
        assert!(memory_err.should_fallback());

        let config_err = GpuError::config("Invalid configuration");
        assert!(!config_err.is_recoverable());
        assert!(!config_err.should_fallback());
    }

    #[test]
    fn test_error_categories() {
        assert_eq!(GpuError::NoDevicesAvailable.category(), "device");
        assert_eq!(GpuError::cuda("test").category(), "cuda");
        assert_eq!(GpuError::opencl("test").category(), "opencl");
        assert_eq!(GpuError::memory("test").category(), "memory");
    }

    #[test]
    fn test_fallback_conditions() {
        assert!(GpuError::NoDevicesAvailable.should_fallback());
        assert!(GpuError::cuda("error").should_fallback());
        assert!(GpuError::opencl("error").should_fallback());
        assert!(!GpuError::config("error").should_fallback());
    }
}