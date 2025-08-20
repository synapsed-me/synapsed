//! Network compression module for the Synapsed ecosystem
//! 
//! This module provides adaptive compression capabilities for network communications,
//! supporting multiple algorithms and intelligent selection based on data characteristics.

pub mod engine;
pub mod algorithms;
pub mod adaptive;
pub mod dictionary;
pub mod stream;
pub mod integration;

#[cfg(test)]
pub mod benchmarks;

pub use engine::{CompressionEngine, CompressionResult, CompressionError, CompressionStats};
pub use algorithms::{Algorithm, ZstandardCompressor, Lz4Compressor};
pub use adaptive::{AdaptiveSelector, SelectionStrategy};
pub use dictionary::{DictionaryManager, Dictionary};
pub use stream::{CompressedStream, StreamCompressor};
pub use integration::{NetworkCompressionManager, CompressedFrame, CompressionNegotiation};

/// Compression configuration for network layer
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Minimum compression ratio to apply compression (0.0 to 1.0)
    pub min_compression_ratio: f32,
    /// Maximum compression level (algorithm-specific)
    pub max_level: i32,
    /// Enable adaptive algorithm selection
    pub adaptive_selection: bool,
    /// Dictionary size for dictionary-based compression
    pub dictionary_size: usize,
    /// Stream chunk size for streaming compression
    pub stream_chunk_size: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            min_compression_ratio: 0.1,
            max_level: 6,
            adaptive_selection: true,
            dictionary_size: 64 * 1024, // 64KB
            stream_chunk_size: 32 * 1024, // 32KB
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CompressionConfig::default();
        assert_eq!(config.min_compression_ratio, 0.1);
        assert_eq!(config.max_level, 6);
        assert!(config.adaptive_selection);
        assert_eq!(config.dictionary_size, 64 * 1024);
        assert_eq!(config.stream_chunk_size, 32 * 1024);
    }
}