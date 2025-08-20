//! Compression algorithm implementations

use crate::compression::engine::{CompressionEngine, CompressionResult, CompressionError};
use async_trait::async_trait;
use bytes::Bytes;
use std::fmt;

/// Supported compression algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Algorithm {
    Zstd,
    Lz4,
    None,
}

impl Algorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            Algorithm::Zstd => "zstd",
            Algorithm::Lz4 => "lz4",
            Algorithm::None => "none",
        }
    }
}

impl fmt::Display for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Zstandard compression implementation
#[derive(Debug, Default)]
pub struct ZstandardCompressor {
    default_level: i32,
}

impl ZstandardCompressor {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_level(level: i32) -> Self {
        Self {
            default_level: level,
        }
    }
}

#[async_trait]
impl CompressionEngine for ZstandardCompressor {
    fn compress(&self, data: &[u8], level: Option<i32>) -> CompressionResult<Bytes> {
        let compression_level = level.unwrap_or(self.default_level);
        
        // Validate compression level
        if !self.supported_levels().contains(&compression_level) {
            return Err(CompressionError::InvalidLevel { level: compression_level });
        }
        
        zstd::bulk::compress(data, compression_level)
            .map(Bytes::from)
            .map_err(|e| CompressionError::CompressionFailed {
                reason: format!("Zstd compression failed: {}", e),
            })
    }
    
    fn decompress(&self, data: &[u8]) -> CompressionResult<Bytes> {
        zstd::bulk::decompress(data, 1024 * 1024) // 1MB max decompressed size for safety
            .map(Bytes::from)
            .map_err(|e| CompressionError::DecompressionFailed {
                reason: format!("Zstd decompression failed: {}", e),
            })
    }
    
    fn algorithm_name(&self) -> &'static str {
        "zstd"
    }
    
    fn supported_levels(&self) -> std::ops::Range<i32> {
        1..22 // Zstd supports levels 1-21
    }
    
    fn estimate_compression_ratio(&self, data: &[u8]) -> f32 {
        // Simple heuristic - will be improved in GREEN phase
        if data.len() < 32 {
            return 0.0;
        }
        
        // Estimate based on entropy (simplified)
        let mut byte_counts = [0u32; 256];
        for &byte in data {
            byte_counts[byte as usize] += 1;
        }
        
        let len = data.len() as f32;
        let mut entropy = 0.0;
        for &count in &byte_counts {
            if count > 0 {
                let p = count as f32 / len;
                entropy -= p * p.log2();
            }
        }
        
        // Convert entropy to estimated compression ratio
        (8.0 - entropy).max(0.0) / 8.0
    }
}

/// LZ4 compression implementation
#[derive(Debug, Default)]
pub struct Lz4Compressor {
    high_compression: bool,
}

impl Lz4Compressor {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_high_compression(high_compression: bool) -> Self {
        Self { high_compression }
    }
}

#[async_trait]
impl CompressionEngine for Lz4Compressor {
    fn compress(&self, data: &[u8], _level: Option<i32>) -> CompressionResult<Bytes> {
        let compressed = if self.high_compression {
            lz4_flex::compress(data)
        } else {
            lz4_flex::compress(data)
        };
        
        Ok(Bytes::from(compressed))
    }
    
    fn decompress(&self, data: &[u8]) -> CompressionResult<Bytes> {
        // We need to know the original size for LZ4 decompression
        // For now, we'll use safe_decompress with a reasonable limit
        lz4_flex::decompress(data, 1024 * 1024) // 1MB max decompressed size
            .map(Bytes::from)
            .map_err(|e| CompressionError::DecompressionFailed {
                reason: format!("LZ4 decompression failed: {}", e),
            })
    }
    
    fn algorithm_name(&self) -> &'static str {
        "lz4"
    }
    
    fn supported_levels(&self) -> std::ops::Range<i32> {
        0..2 // LZ4 has standard (0) and high compression (1)
    }
    
    fn estimate_compression_ratio(&self, data: &[u8]) -> f32 {
        // LZ4 is optimized for speed, typically lower compression ratios
        if data.len() < 16 {
            return 0.0;
        }
        
        // Simple duplicate detection heuristic
        let mut seen = std::collections::HashSet::new();
        let mut duplicates = 0;
        
        for chunk in data.chunks(4) {
            if chunk.len() == 4 {
                if !seen.insert(chunk) {
                    duplicates += 1;
                }
            }
        }
        
        (duplicates as f32 / (data.len() / 4) as f32) * 0.5 // Conservative estimate
    }
}

/// No-op compressor for testing and fallback
#[derive(Debug, Default)]
pub struct NoopCompressor;

#[async_trait]
impl CompressionEngine for NoopCompressor {
    fn compress(&self, data: &[u8], _level: Option<i32>) -> CompressionResult<Bytes> {
        Ok(Bytes::copy_from_slice(data))
    }
    
    fn decompress(&self, data: &[u8]) -> CompressionResult<Bytes> {
        Ok(Bytes::copy_from_slice(data))
    }
    
    fn algorithm_name(&self) -> &'static str {
        "none"
    }
    
    fn supported_levels(&self) -> std::ops::Range<i32> {
        0..1
    }
    
    fn estimate_compression_ratio(&self, _data: &[u8]) -> f32 {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_algorithm_display() {
        assert_eq!(Algorithm::Zstd.to_string(), "zstd");
        assert_eq!(Algorithm::Lz4.to_string(), "lz4");
        assert_eq!(Algorithm::None.to_string(), "none");
    }

    #[test]  
    fn test_zstd_compressor_creation() {
        let compressor = ZstandardCompressor::new();
        assert_eq!(compressor.algorithm_name(), "zstd");
        assert_eq!(compressor.supported_levels(), 1..22);
    }

    #[test]
    fn test_lz4_compressor_creation() {
        let compressor = Lz4Compressor::new();
        assert_eq!(compressor.algorithm_name(), "lz4");
        assert_eq!(compressor.supported_levels(), 0..2);
    }

    #[test]
    fn test_noop_compressor() {
        let compressor = NoopCompressor;
        let data = b"test data";
        let compressed = compressor.compress(data, None).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();
        
        assert_eq!(compressed.as_ref(), data);
        assert_eq!(decompressed.as_ref(), data);
        assert_eq!(compressor.estimate_compression_ratio(data), 0.0);
    }

    #[test]
    #[should_panic(expected = "Zstd compression not implemented yet")]
    fn test_zstd_compression_not_implemented() {
        let compressor = ZstandardCompressor::new();
        let _result = compressor.compress(b"test", None);
    }

    #[test]
    #[should_panic(expected = "LZ4 compression not implemented yet")]
    fn test_lz4_compression_not_implemented() {
        let compressor = Lz4Compressor::new();
        let _result = compressor.compress(b"test", None);
    }

    #[test]
    fn test_compression_ratio_estimation() {
        let compressor = ZstandardCompressor::new();
        
        // Test with small data - should return 0
        let small_data = b"x";
        assert_eq!(compressor.estimate_compression_ratio(small_data), 0.0);
        
        // Test with repetitive data - should have good compression ratio
        let repetitive_data = vec![b'A'; 1000];
        let ratio = compressor.estimate_compression_ratio(&repetitive_data);
        assert!(ratio > 0.5); // Should be highly compressible
        
        // Test with random-like data - should have lower compression ratio
        let random_data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let ratio = compressor.estimate_compression_ratio(&random_data);
        assert!(ratio < 0.3); // Should be less compressible
    }
}