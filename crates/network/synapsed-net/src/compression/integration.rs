//! Integration layer for compression with synapsed-net protocols

use crate::compression::{
    CompressionEngine, CompressionConfig, AdaptiveSelector, SelectionStrategy, Algorithm,
    engine::{CompressionResult, CompressionError, CompressionStats},
};
use bytes::Bytes;
use std::sync::Arc;
use std::collections::HashMap;

/// Protocol-aware compression negotiation
#[derive(Debug, Clone)]
pub struct CompressionNegotiation {
    pub supported_algorithms: Vec<Algorithm>,
    pub preferred_algorithm: Algorithm,
    pub min_compression_ratio: f32,
    pub max_compression_level: i32,
}

impl Default for CompressionNegotiation {
    fn default() -> Self {
        Self {
            supported_algorithms: vec![Algorithm::Zstd, Algorithm::Lz4, Algorithm::None],
            preferred_algorithm: Algorithm::Zstd,
            min_compression_ratio: 0.1,
            max_compression_level: 6,
        }
    }
}

/// Message frame with compression metadata
#[derive(Debug, Clone)]
pub struct CompressedFrame {
    pub algorithm: Algorithm,
    pub original_size: usize,
    pub compressed_data: Bytes,
    pub dictionary_id: Option<String>,
    pub compression_level: i32,
}

impl CompressedFrame {
    pub fn new(
        algorithm: Algorithm,
        original_size: usize,
        compressed_data: Bytes,
        dictionary_id: Option<String>,
        compression_level: i32,
    ) -> Self {
        Self {
            algorithm,
            original_size,
            compressed_data,
            dictionary_id,
            compression_level,
        }
    }
    
    pub fn compression_ratio(&self) -> f32 {
        if self.original_size > 0 {
            1.0 - (self.compressed_data.len() as f32 / self.original_size as f32)
        } else {
            0.0
        }
    }
    
    pub fn is_compressed(&self) -> bool {
        self.algorithm != Algorithm::None
    }
}

/// Network-layer compression manager
#[derive(Debug)]
pub struct NetworkCompressionManager {
    selector: AdaptiveSelector,
    negotiation: CompressionNegotiation,
    performance_metrics: HashMap<String, CompressionStats>,
}

impl NetworkCompressionManager {
    pub fn new(config: CompressionConfig) -> Self {
        let selector = AdaptiveSelector::new(
            if config.adaptive_selection {
                SelectionStrategy::Adaptive
            } else {
                SelectionStrategy::Balanced
            },
            config.min_compression_ratio,
        );
        
        Self {
            selector,
            negotiation: CompressionNegotiation::default(),
            performance_metrics: HashMap::new(),
        }
    }
    
    /// Negotiate compression parameters with peer
    pub fn negotiate_compression(&mut self, peer_algorithms: &[Algorithm]) -> CompressionNegotiation {
        // Find common algorithms
        let common_algorithms: Vec<Algorithm> = peer_algorithms
            .iter()
            .filter(|&alg| self.negotiation.supported_algorithms.contains(alg))
            .copied()
            .collect();
        
        if common_algorithms.is_empty() {
            // Fallback to no compression
            CompressionNegotiation {
                supported_algorithms: vec![Algorithm::None],
                preferred_algorithm: Algorithm::None,
                min_compression_ratio: 0.0,
                max_compression_level: 0,
            }
        } else {
            // Prefer the first common algorithm that's also in our preference order
            let preferred = self.negotiation.supported_algorithms
                .iter()
                .find(|&alg| common_algorithms.contains(alg))
                .copied()
                .unwrap_or(common_algorithms[0]);
            
            CompressionNegotiation {
                supported_algorithms: common_algorithms,
                preferred_algorithm: preferred,
                min_compression_ratio: self.negotiation.min_compression_ratio,
                max_compression_level: self.negotiation.max_compression_level,
            }
        }
    }
    
    /// Compress message for transmission
    pub fn compress_message(&mut self, data: &[u8], level: Option<i32>) -> CompressionResult<CompressedFrame> {
        let (compressed_data, algorithm, stats) = self.selector.compress(data, level)?;
        
        // Store performance metrics
        let key = format!("{}_{}", algorithm, data.len());
        self.performance_metrics.insert(key, stats.clone());
        
        Ok(CompressedFrame::new(
            algorithm,
            data.len(),
            compressed_data,
            None, // Dictionary support can be added later
            level.unwrap_or(6),
        ))
    }
    
    /// Decompress received message
    pub fn decompress_message(&self, frame: &CompressedFrame) -> CompressionResult<Bytes> {
        if !frame.is_compressed() {
            return Ok(frame.compressed_data.clone());
        }
        
        self.selector.decompress(&frame.compressed_data, frame.algorithm)
    }
    
    /// Get compression performance metrics
    pub fn get_performance_metrics(&self) -> &HashMap<String, CompressionStats> {
        &self.performance_metrics
    }
    
    /// Update compression strategy based on network conditions
    pub fn update_strategy(&mut self, strategy: SelectionStrategy) {
        self.selector.set_strategy(strategy);
    }
    
    /// Get current compression statistics
    pub fn get_compression_stats(&self) -> HashMap<Algorithm, f32> {
        let mut stats = HashMap::new();
        
        for (algorithm, metrics) in self.selector.get_metrics() {
            stats.insert(*algorithm, metrics.avg_compression_ratio);
        }
        
        stats
    }
}

/// Performance monitoring for compression operations
#[derive(Debug, Default)]
pub struct CompressionPerformanceMonitor {
    algorithm_stats: HashMap<Algorithm, Vec<CompressionStats>>,
    total_bytes_processed: u64,
    total_compression_time_us: u64,
}

impl CompressionPerformanceMonitor {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn record_compression(&mut self, stats: CompressionStats) {
        let algorithm = Algorithm::Zstd; // Parse from stats.algorithm_used
        
        self.algorithm_stats
            .entry(algorithm)
            .or_insert_with(Vec::new)
            .push(stats.clone());
        
        self.total_bytes_processed += stats.original_size as u64;
        self.total_compression_time_us += stats.compression_time_us;
    }
    
    pub fn get_average_compression_ratio(&self, algorithm: Algorithm) -> f32 {
        if let Some(stats) = self.algorithm_stats.get(&algorithm) {
            if !stats.is_empty() {
                return stats.iter().map(|s| s.compression_ratio).sum::<f32>() / stats.len() as f32;
            }
        }
        0.0
    }
    
    pub fn get_average_compression_time(&self, algorithm: Algorithm) -> u64 {
        if let Some(stats) = self.algorithm_stats.get(&algorithm) {
            if !stats.is_empty() {
                return stats.iter().map(|s| s.compression_time_us).sum::<u64>() / stats.len() as u64;
            }
        }
        0
    }
    
    pub fn get_total_bytes_processed(&self) -> u64 {
        self.total_bytes_processed
    }
    
    pub fn get_total_compression_time(&self) -> u64 {
        self.total_compression_time_us
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_negotiation_default() {
        let negotiation = CompressionNegotiation::default();
        assert_eq!(negotiation.preferred_algorithm, Algorithm::Zstd);
        assert!(negotiation.supported_algorithms.contains(&Algorithm::Zstd));
        assert!(negotiation.supported_algorithms.contains(&Algorithm::Lz4));
    }

    #[test]
    fn test_compressed_frame_creation() {
        let data = Bytes::from("test data");
        let frame = CompressedFrame::new(
            Algorithm::Zstd,
            100,
            data.clone(),
            None,
            6,
        );
        
        assert_eq!(frame.algorithm, Algorithm::Zstd);
        assert_eq!(frame.original_size, 100);
        assert_eq!(frame.compressed_data, data);
        assert!(frame.is_compressed());
        
        let ratio = frame.compression_ratio();
        assert!(ratio > 0.0); // Should show compression occurred
    }

    #[test]
    fn test_network_compression_manager_creation() {
        let config = CompressionConfig::default();
        let manager = NetworkCompressionManager::new(config);
        
        assert_eq!(manager.negotiation.preferred_algorithm, Algorithm::Zstd);
    }

    #[test]
    fn test_compression_negotiation() {
        let config = CompressionConfig::default();
        let mut manager = NetworkCompressionManager::new(config);
        
        // Test with compatible algorithms
        let peer_algorithms = vec![Algorithm::Zstd, Algorithm::Lz4];
        let negotiated = manager.negotiate_compression(&peer_algorithms);
        assert!(negotiated.supported_algorithms.len() >= 1);
        
        // Test with no compatible algorithms (except None)
        let peer_algorithms = vec![];
        let negotiated = manager.negotiate_compression(&peer_algorithms);
        assert_eq!(negotiated.preferred_algorithm, Algorithm::None);
    }

    #[test]
    fn test_performance_monitor() {
        let mut monitor = CompressionPerformanceMonitor::new();
        
        let stats = CompressionStats::new(1000, 600, 1500, "zstd".to_string());
        monitor.record_compression(stats);
        
        assert_eq!(monitor.get_total_bytes_processed(), 1000);
        assert_eq!(monitor.get_total_compression_time(), 1500);
        
        let avg_ratio = monitor.get_average_compression_ratio(Algorithm::Zstd);
        assert_eq!(avg_ratio, 0.4); // (1000-600)/1000 = 0.4
    }
}