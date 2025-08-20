//! Adaptive compression module that selects the best compression algorithm based on data characteristics

use bytes::Bytes;
use crate::error::StorageError;

/// Adaptive compression strategy
pub struct AdaptiveCompressor {
    /// Minimum size for compression
    min_size: usize,
    /// Compression level (1-9)
    level: u32,
}

impl Default for AdaptiveCompressor {
    fn default() -> Self {
        Self {
            min_size: 1024, // 1KB minimum
            level: 6,       // Default compression level
        }
    }
}

impl AdaptiveCompressor {
    /// Create a new adaptive compressor
    pub fn new(min_size: usize, level: u32) -> Self {
        Self {
            min_size,
            level: level.min(9).max(1),
        }
    }

    /// Compress data adaptively
    pub fn compress(&self, data: &[u8]) -> Result<Bytes, StorageError> {
        if data.len() < self.min_size {
            // Don't compress small data
            return Ok(Bytes::copy_from_slice(data));
        }

        // For now, just return uncompressed data
        // In a real implementation, we would analyze the data and choose
        // the best compression algorithm (LZ4, Zstd, etc.)
        Ok(Bytes::copy_from_slice(data))
    }

    /// Decompress data
    pub fn decompress(&self, data: &[u8]) -> Result<Bytes, StorageError> {
        // For now, just return the data as-is
        // In a real implementation, we would detect the compression format
        // and decompress accordingly
        Ok(Bytes::copy_from_slice(data))
    }

    /// Analyze data to determine the best compression algorithm
    pub fn analyze_data(&self, data: &[u8]) -> CompressionRecommendation {
        if data.len() < self.min_size {
            return CompressionRecommendation::None;
        }

        // Simple heuristic based on data entropy
        // In a real implementation, this would be more sophisticated
        let entropy = calculate_simple_entropy(data);
        
        // Use compression level to adjust thresholds
        let high_threshold = 0.5 + (9 - self.level) as f64 * 0.05;
        let fast_threshold = 0.8 + (9 - self.level) as f64 * 0.02;
        
        if entropy < high_threshold {
            CompressionRecommendation::HighCompression
        } else if entropy < fast_threshold {
            CompressionRecommendation::FastCompression
        } else {
            CompressionRecommendation::None
        }
    }
}

/// Compression recommendation based on data analysis
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressionRecommendation {
    /// No compression recommended
    None,
    /// Fast compression (e.g., LZ4)
    FastCompression,
    /// High compression ratio (e.g., Zstd)
    HighCompression,
}

/// Calculate simple entropy for data analysis
fn calculate_simple_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }

    let mut frequency = [0u64; 256];
    for &byte in data {
        frequency[byte as usize] += 1;
    }

    let len = data.len() as f64;
    let mut entropy = 0.0;

    for &count in &frequency {
        if count > 0 {
            let p = count as f64 / len;
            entropy -= p * p.log2();
        }
    }

    entropy / 8.0 // Normalize to 0-1 range
}