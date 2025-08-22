//! Adaptive compression algorithm selection

use crate::compression::{
    engine::{CompressionEngine, CompressionResult, CompressionStats, CompressionError},
    algorithms::{Algorithm, ZstandardCompressor, Lz4Compressor, NoopCompressor},
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use bytes::Bytes;

/// Strategy for selecting compression algorithms
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionStrategy {
    /// Always use the fastest algorithm
    Speed,
    /// Always use the best compression ratio
    Ratio,
    /// Balance between speed and compression ratio
    Balanced,
    /// Adapt based on data characteristics and performance history
    Adaptive,
}

/// Performance metrics for algorithm selection
#[derive(Debug, Clone)]
pub struct AlgorithmMetrics {
    pub algorithm: Algorithm,
    pub avg_compression_ratio: f32,
    pub avg_compression_time_us: u64,
    pub usage_count: u64,
    pub success_rate: f32,
}

impl AlgorithmMetrics {
    pub fn new(algorithm: Algorithm) -> Self {
        Self {
            algorithm,
            avg_compression_ratio: 0.0,
            avg_compression_time_us: 0,
            usage_count: 0,
            success_rate: 1.0,
        }
    }
    
    pub fn update(&mut self, stats: &CompressionStats, success: bool) {
        let count = self.usage_count as f32;
        let new_count = count + 1.0;
        
        // Update moving averages
        self.avg_compression_ratio = (self.avg_compression_ratio * count + stats.compression_ratio) / new_count;
        self.avg_compression_time_us = ((self.avg_compression_time_us as f32 * count) + stats.compression_time_us as f32) as u64 / new_count as u64;
        self.success_rate = (self.success_rate * count + if success { 1.0 } else { 0.0 }) / new_count;
        
        self.usage_count += 1;
    }
    
    pub fn score(&self, strategy: SelectionStrategy) -> f32 {
        match strategy {
            SelectionStrategy::Speed => {
                // Lower time = higher score, factor in success rate
                let time_score = 1.0 / (1.0 + self.avg_compression_time_us as f32 / 1000.0);
                time_score * self.success_rate
            }
            SelectionStrategy::Ratio => {
                // Higher ratio = higher score, factor in success rate
                self.avg_compression_ratio * self.success_rate
            }
            SelectionStrategy::Balanced => {
                // Balance between speed and ratio
                let time_score = 1.0 / (1.0 + self.avg_compression_time_us as f32 / 1000.0);
                let ratio_score = self.avg_compression_ratio;
                (time_score * 0.4 + ratio_score * 0.6) * self.success_rate
            }
            SelectionStrategy::Adaptive => {
                // Consider all factors with usage-based weighting
                let time_score = 1.0 / (1.0 + self.avg_compression_time_us as f32 / 1000.0);
                let ratio_score = self.avg_compression_ratio;
                let usage_weight = (self.usage_count as f32).log2().max(1.0) / 10.0;
                
                ((time_score * 0.3 + ratio_score * 0.5 + usage_weight * 0.2) * self.success_rate)
            }
        }
    }
}

/// Adaptive compression algorithm selector
#[derive(Debug)]
pub struct AdaptiveSelector {
    strategy: SelectionStrategy,
    engines: HashMap<Algorithm, Arc<dyn CompressionEngine>>,
    metrics: HashMap<Algorithm, AlgorithmMetrics>,
    min_compression_ratio: f32,
}

impl AdaptiveSelector {
    pub fn new(strategy: SelectionStrategy, min_compression_ratio: f32) -> Self {
        let mut engines: HashMap<Algorithm, Arc<dyn CompressionEngine>> = HashMap::new();
        let mut metrics = HashMap::new();
        
        // Initialize engines and metrics
        engines.insert(Algorithm::Zstd, Arc::new(ZstandardCompressor::new()));
        engines.insert(Algorithm::Lz4, Arc::new(Lz4Compressor::new()));
        engines.insert(Algorithm::None, Arc::new(NoopCompressor));
        
        metrics.insert(Algorithm::Zstd, AlgorithmMetrics::new(Algorithm::Zstd));
        metrics.insert(Algorithm::Lz4, AlgorithmMetrics::new(Algorithm::Lz4));
        metrics.insert(Algorithm::None, AlgorithmMetrics::new(Algorithm::None));
        
        Self {
            strategy,
            engines,
            metrics,
            min_compression_ratio,
        }
    }
    
    /// Select the best algorithm for the given data
    pub fn select_algorithm(&self, data: &[u8]) -> Algorithm {
        // For very small data, don't compress
        if data.len() < 32 {
            return Algorithm::None;
        }
        
        match self.strategy {
            SelectionStrategy::Speed => {
                // LZ4 is generally fastest
                if data.len() < 1024 {
                    Algorithm::None
                } else {
                    Algorithm::Lz4
                }
            }
            SelectionStrategy::Ratio => {
                // Zstd generally has better compression ratios
                Algorithm::Zstd
            }
            SelectionStrategy::Balanced => {
                // Choose based on data size
                if data.len() < 1024 {
                    Algorithm::Lz4
                } else {
                    Algorithm::Zstd
                }
            }
            SelectionStrategy::Adaptive => {
                // Use metrics to decide
                let mut best_algorithm = Algorithm::Zstd;
                let mut best_score = 0.0;
                
                for (algorithm, metrics) in &self.metrics {
                    let score = metrics.score(self.strategy);
                    if score > best_score {
                        best_score = score;
                        best_algorithm = *algorithm;
                    }
                }
                
                // If no metrics yet, use balanced approach
                if best_score == 0.0 {
                    if data.len() < 1024 {
                        Algorithm::Lz4
                    } else {
                        Algorithm::Zstd
                    }
                } else {
                    best_algorithm
                }
            }
        }
    }
    
    /// Compress data using the best selected algorithm
    pub fn compress(&mut self, data: &[u8], level: Option<i32>) -> CompressionResult<(Bytes, Algorithm, CompressionStats)> {
        let algorithm = self.select_algorithm(data);
        let engine = self.engines.get(&algorithm).ok_or_else(|| {
            CompressionError::ConfigError {
                reason: format!("Engine not found for algorithm: {}", algorithm),
            }
        })?;
        
        let start_time = Instant::now();
        let result = engine.compress(data, level);
        let compression_time = start_time.elapsed().as_micros() as u64;
        
        match result {
            Ok(compressed_data) => {
                let stats = CompressionStats::new(
                    data.len(),
                    compressed_data.len(),
                    compression_time,
                    algorithm.to_string(),
                );
                
                // Update metrics
                if let Some(metrics) = self.metrics.get_mut(&algorithm) {
                    metrics.update(&stats, true);
                }
                
                // Check if compression is beneficial
                if stats.compression_ratio < self.min_compression_ratio {
                    // Compression not beneficial, return original data
                    let no_compress_stats = CompressionStats::new(
                        data.len(),
                        data.len(),
                        0,
                        "none".to_string(),
                    );
                    Ok((Bytes::copy_from_slice(data), Algorithm::None, no_compress_stats))
                } else {
                    Ok((compressed_data, algorithm, stats))
                }
            }
            Err(e) => {
                // Update metrics with failure
                if let Some(metrics) = self.metrics.get_mut(&algorithm) {
                    let failed_stats = CompressionStats::new(
                        data.len(),
                        data.len(),
                        compression_time,
                        algorithm.to_string(),
                    );
                    metrics.update(&failed_stats, false);
                }
                Err(e)
            }
        }
    }
    
    /// Decompress data using the specified algorithm
    pub fn decompress(&self, data: &[u8], algorithm: Algorithm) -> CompressionResult<Bytes> {
        let engine = self.engines.get(&algorithm).ok_or_else(|| {
            CompressionError::ConfigError {
                reason: format!("Engine not found for algorithm: {}", algorithm),
            }
        })?;
        
        engine.decompress(data)
    }
    
    /// Get performance metrics for all algorithms
    pub fn get_metrics(&self) -> &HashMap<Algorithm, AlgorithmMetrics> {
        &self.metrics
    }
    
    /// Update strategy
    pub fn set_strategy(&mut self, strategy: SelectionStrategy) {
        self.strategy = strategy;
    }
    
    /// Get current strategy
    pub fn strategy(&self) -> SelectionStrategy {
        self.strategy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_algorithm_metrics_creation() {
        let metrics = AlgorithmMetrics::new(Algorithm::Zstd);
        assert_eq!(metrics.algorithm, Algorithm::Zstd);
        assert_eq!(metrics.avg_compression_ratio, 0.0);
        assert_eq!(metrics.avg_compression_time_us, 0);
        assert_eq!(metrics.usage_count, 0);
        assert_eq!(metrics.success_rate, 1.0);
    }

    #[test]
    fn test_algorithm_metrics_update() {
        let mut metrics = AlgorithmMetrics::new(Algorithm::Zstd);
        let stats = CompressionStats::new(1000, 400, 1500, "zstd".to_string());
        
        metrics.update(&stats, true);
        
        assert_eq!(metrics.usage_count, 1);
        assert_eq!(metrics.avg_compression_ratio, 0.6);
        assert_eq!(metrics.avg_compression_time_us, 1500);
        assert_eq!(metrics.success_rate, 1.0);
        
        // Update with failure
        let stats2 = CompressionStats::new(1000, 500, 2000, "zstd".to_string());
        metrics.update(&stats2, false);
        
        assert_eq!(metrics.usage_count, 2);
        assert_eq!(metrics.success_rate, 0.5); // 1 success, 1 failure
    }

    #[test]
    fn test_algorithm_metrics_score() {
        let mut metrics = AlgorithmMetrics::new(Algorithm::Zstd);
        let stats = CompressionStats::new(1000, 300, 1000, "zstd".to_string());
        metrics.update(&stats, true);
        
        let speed_score = metrics.score(SelectionStrategy::Speed);
        let ratio_score = metrics.score(SelectionStrategy::Ratio);
        let balanced_score = metrics.score(SelectionStrategy::Balanced);
        let adaptive_score = metrics.score(SelectionStrategy::Adaptive);
        
        assert!(speed_score > 0.0);
        assert!(ratio_score > 0.0);
        assert!(balanced_score > 0.0);
        assert!(adaptive_score > 0.0);
        
        // Ratio score should equal compression ratio for this strategy
        assert_eq!(ratio_score, 0.7); // (1000-300)/1000 = 0.7
    }

    #[test]
    fn test_adaptive_selector_creation() {
        let selector = AdaptiveSelector::new(SelectionStrategy::Balanced, 0.1);
        assert_eq!(selector.strategy(), SelectionStrategy::Balanced);
        assert_eq!(selector.min_compression_ratio, 0.1);
        
        // Should have all three algorithms
        assert!(selector.engines.contains_key(&Algorithm::Zstd));
        assert!(selector.engines.contains_key(&Algorithm::Lz4));
        assert!(selector.engines.contains_key(&Algorithm::None));
    }

    #[test]
    fn test_adaptive_selector_strategy_update() {
        let mut selector = AdaptiveSelector::new(SelectionStrategy::Speed, 0.1);
        assert_eq!(selector.strategy(), SelectionStrategy::Speed);
        
        selector.set_strategy(SelectionStrategy::Ratio);
        assert_eq!(selector.strategy(), SelectionStrategy::Ratio);
    }

    #[test]
    #[should_panic(expected = "Algorithm selection not implemented yet")]
    fn test_algorithm_selection_not_implemented() {
        let selector = AdaptiveSelector::new(SelectionStrategy::Balanced, 0.1);
        let _algorithm = selector.select_algorithm(b"test data");
    }

    #[test]
    #[should_panic(expected = "Adaptive compression not implemented yet")]
    fn test_adaptive_compression_not_implemented() {
        let mut selector = AdaptiveSelector::new(SelectionStrategy::Balanced, 0.1);
        let _result = selector.compress(b"test data", None);
    }

    #[test]
    #[should_panic(expected = "Adaptive decompression not implemented yet")]
    fn test_adaptive_decompression_not_implemented() {
        let selector = AdaptiveSelector::new(SelectionStrategy::Balanced, 0.1);
        let _result = selector.decompress(b"compressed", Algorithm::Zstd);
    }

    #[test]
    fn test_selection_strategy_values() {
        // Test that all enum values are distinct
        let strategies = vec![
            SelectionStrategy::Speed,
            SelectionStrategy::Ratio,
            SelectionStrategy::Balanced,
            SelectionStrategy::Adaptive,
        ];
        
        for (i, strategy1) in strategies.iter().enumerate() {
            for (j, strategy2) in strategies.iter().enumerate() {
                if i != j {
                    assert_ne!(strategy1, strategy2);
                }
            }
        }
    }
}