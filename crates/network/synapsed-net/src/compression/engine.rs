//! Core compression engine traits and types

use std::fmt;
use async_trait::async_trait;
use bytes::Bytes;
use thiserror::Error;

/// Result type for compression operations
pub type CompressionResult<T> = Result<T, CompressionError>;

/// Errors that can occur during compression operations
#[derive(Error, Debug, Clone)]
pub enum CompressionError {
    #[error("Compression failed: {reason}")]
    CompressionFailed { reason: String },
    
    #[error("Decompression failed: {reason}")]
    DecompressionFailed { reason: String },
    
    #[error("Invalid compression level: {level}")]
    InvalidLevel { level: i32 },
    
    #[error("Dictionary error: {reason}")]
    DictionaryError { reason: String },
    
    #[error("Stream error: {reason}")]
    StreamError { reason: String },
    
    #[error("Configuration error: {reason}")]
    ConfigError { reason: String },
}

/// Compression statistics
#[derive(Debug, Clone, PartialEq)]
pub struct CompressionStats {
    pub original_size: usize,
    pub compressed_size: usize,
    pub compression_ratio: f32,
    pub compression_time_us: u64,
    pub algorithm_used: String,
}

impl CompressionStats {
    pub fn new(original_size: usize, compressed_size: usize, time_us: u64, algorithm: String) -> Self {
        let ratio = if original_size > 0 {
            1.0 - (compressed_size as f32 / original_size as f32)
        } else {
            0.0
        };
        
        Self {
            original_size,
            compressed_size,
            compression_ratio: ratio,
            compression_time_us: time_us,
            algorithm_used: algorithm,
        }
    }
}

/// Core compression engine trait
#[async_trait]
pub trait CompressionEngine: Send + Sync + fmt::Debug {
    /// Compress data synchronously
    fn compress(&self, data: &[u8], level: Option<i32>) -> CompressionResult<Bytes>;
    
    /// Decompress data synchronously
    fn decompress(&self, data: &[u8]) -> CompressionResult<Bytes>;
    
    /// Compress data asynchronously
    async fn compress_async(&self, data: &[u8], level: Option<i32>) -> CompressionResult<Bytes> {
        // Default implementation uses sync version
        self.compress(data, level)
    }
    
    /// Decompress data asynchronously
    async fn decompress_async(&self, data: &[u8]) -> CompressionResult<Bytes> {
        // Default implementation uses sync version
        self.decompress(data)
    }
    
    /// Get algorithm name
    fn algorithm_name(&self) -> &'static str;
    
    /// Get supported compression levels
    fn supported_levels(&self) -> std::ops::Range<i32>;
    
    /// Estimate compression ratio for given data
    fn estimate_compression_ratio(&self, data: &[u8]) -> f32;
    
    /// Check if compression is beneficial for this data
    fn should_compress(&self, data: &[u8], min_ratio: f32) -> bool {
        data.len() >= 32 && self.estimate_compression_ratio(data) >= min_ratio
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_stats() {
        let stats = CompressionStats::new(1000, 400, 1500, "zstd".to_string());
        assert_eq!(stats.original_size, 1000);
        assert_eq!(stats.compressed_size, 400);
        assert_eq!(stats.compression_ratio, 0.6); // 60% compression
        assert_eq!(stats.compression_time_us, 1500);
        assert_eq!(stats.algorithm_used, "zstd");
    }

    #[test]
    fn test_compression_stats_zero_size() {
        let stats = CompressionStats::new(0, 0, 100, "zstd".to_string());
        assert_eq!(stats.compression_ratio, 0.0);
    }
}