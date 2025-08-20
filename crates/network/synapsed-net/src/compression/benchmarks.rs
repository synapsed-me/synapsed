//! Benchmarking utilities for compression algorithms

use crate::compression::{
    CompressionEngine, Algorithm, ZstandardCompressor, Lz4Compressor,
    AdaptiveSelector, SelectionStrategy, CompressionStats,
};
use bytes::Bytes;
use std::sync::Arc;
use std::time::Instant;

/// Benchmark results for compression algorithm comparison
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub algorithm: Algorithm,
    pub data_size: usize,
    pub compression_ratio: f32,
    pub compression_time_us: u64,
    pub decompression_time_us: u64,
    pub throughput_mbps: f32,
}

impl BenchmarkResult {
    pub fn new(
        algorithm: Algorithm,
        data_size: usize,
        compression_ratio: f32,
        compression_time_us: u64,
        decompression_time_us: u64,
    ) -> Self {
        let total_time_s = (compression_time_us + decompression_time_us) as f32 / 1_000_000.0;
        let throughput_mbps = if total_time_s > 0.0 {
            (data_size as f32 / (1024.0 * 1024.0)) / total_time_s
        } else {
            0.0
        };
        
        Self {
            algorithm,
            data_size,
            compression_ratio,
            compression_time_us,
            decompression_time_us,
            throughput_mbps,
        }
    }
}

/// Comprehensive benchmark suite for compression algorithms
pub struct CompressionBenchmark {
    test_data: Vec<Bytes>,
    engines: Vec<(Algorithm, Arc<dyn CompressionEngine>)>,
}

impl CompressionBenchmark {
    pub fn new() -> Self {
        let mut engines: Vec<(Algorithm, Arc<dyn CompressionEngine>)> = Vec::new();
        engines.push((Algorithm::Zstd, Arc::new(ZstandardCompressor::new())));
        engines.push((Algorithm::Lz4, Arc::new(Lz4Compressor::new())));
        
        Self {
            test_data: Self::generate_test_data(),
            engines,
        }
    }
    
    fn generate_test_data() -> Vec<Bytes> {
        vec![
            // Highly compressible data
            Bytes::from(vec![b'A'; 1024]),
            
            // Mixed repetitive data
            Bytes::from("Hello World! ".repeat(100)),
            
            // JSON-like structured data
            Bytes::from(r#"{"name":"test","value":123,"items":[1,2,3,4,5]}"#.repeat(50)),
            
            // Random-like data (less compressible)
            Bytes::from((0..1024).map(|i| (i % 256) as u8).collect::<Vec<u8>>()),
            
            // Large text data
            Bytes::from("Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(200)),
        ]
    }
    
    /// Run comprehensive benchmarks on all algorithms
    pub fn run_all_benchmarks(&self) -> Vec<BenchmarkResult> {
        let mut results = Vec::new();
        
        for (algorithm, engine) in &self.engines {
            for data in &self.test_data {
                if let Ok(result) = self.benchmark_algorithm(engine.as_ref(), *algorithm, data) {
                    results.push(result);
                }
            }
        }
        
        results
    }
    
    /// Benchmark a specific algorithm with given data
    pub fn benchmark_algorithm(
        &self,
        engine: &dyn CompressionEngine,
        algorithm: Algorithm,
        data: &[u8],
    ) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
        // Benchmark compression
        let start = Instant::now();
        let compressed = engine.compress(data, None)?;
        let compression_time = start.elapsed().as_micros() as u64;
        
        // Benchmark decompression
        let start = Instant::now();
        let _decompressed = engine.decompress(&compressed)?;
        let decompression_time = start.elapsed().as_micros() as u64;
        
        let compression_ratio = 1.0 - (compressed.len() as f32 / data.len() as f32);
        
        Ok(BenchmarkResult::new(
            algorithm,
            data.len(),
            compression_ratio,
            compression_time,
            decompression_time,
        ))
    }
    
    /// Compare algorithms for specific data characteristics
    pub fn compare_algorithms(&self, data: &[u8]) -> Vec<BenchmarkResult> {
        let mut results = Vec::new();
        
        for (algorithm, engine) in &self.engines {
            if let Ok(result) = self.benchmark_algorithm(engine.as_ref(), *algorithm, data) {
                results.push(result);
            }
        }
        
        // Sort by throughput (descending)
        results.sort_by(|a, b| b.throughput_mbps.partial_cmp(&a.throughput_mbps).unwrap_or(std::cmp::Ordering::Equal));
        results
    }
    
    /// Get best algorithm for specific criteria
    pub fn get_best_algorithm(&self, data: &[u8], prefer_speed: bool) -> Option<Algorithm> {
        let results = self.compare_algorithms(data);
        
        if results.is_empty() {
            return None;
        }
        
        if prefer_speed {
            // Find algorithm with best throughput
            results.into_iter().max_by(|a, b| 
                a.throughput_mbps.partial_cmp(&b.throughput_mbps).unwrap_or(std::cmp::Ordering::Equal)
            ).map(|r| r.algorithm)
        } else {
            // Find algorithm with best compression ratio
            results.into_iter().max_by(|a, b| 
                a.compression_ratio.partial_cmp(&b.compression_ratio).unwrap_or(std::cmp::Ordering::Equal)
            ).map(|r| r.algorithm)
        }
    }
    
    /// Generate performance report
    pub fn generate_report(&self) -> String {
        let results = self.run_all_benchmarks();
        let mut report = String::new();
        
        report.push_str("# Compression Benchmark Report\n\n");
        
        // Group by algorithm
        let mut by_algorithm = std::collections::HashMap::new();
        for result in results {
            by_algorithm.entry(result.algorithm).or_insert_with(Vec::new).push(result);
        }
        
        for (algorithm, algorithm_results) in by_algorithm {
            report.push_str(&format!("## {}\n\n", algorithm));
            
            let avg_ratio: f32 = algorithm_results.iter().map(|r| r.compression_ratio).sum::<f32>() / algorithm_results.len() as f32;
            let avg_throughput: f32 = algorithm_results.iter().map(|r| r.throughput_mbps).sum::<f32>() / algorithm_results.len() as f32;
            
            report.push_str(&format!("- Average Compression Ratio: {:.2}%\n", avg_ratio * 100.0));
            report.push_str(&format!("- Average Throughput: {:.2} MB/s\n", avg_throughput));
            report.push_str(&format!("- Test Cases: {}\n\n", algorithm_results.len()));
        }
        
        report
    }
}

impl Default for CompressionBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_result_creation() {
        let result = BenchmarkResult::new(Algorithm::Zstd, 1000, 0.5, 1000, 500);
        assert_eq!(result.algorithm, Algorithm::Zstd);
        assert_eq!(result.data_size, 1000);
        assert_eq!(result.compression_ratio, 0.5);
        assert!(result.throughput_mbps > 0.0);
    }

    #[test]
    fn test_benchmark_suite_creation() {
        let benchmark = CompressionBenchmark::new();
        assert!(!benchmark.test_data.is_empty());
        assert_eq!(benchmark.engines.len(), 2); // Zstd and LZ4
    }

    #[test]
    fn test_generate_test_data() {
        let data = CompressionBenchmark::generate_test_data();
        assert!(data.len() >= 5);
        
        // Check that we have variety in data sizes and patterns
        let sizes: Vec<usize> = data.iter().map(|d| d.len()).collect();
        assert!(sizes.iter().any(|&s| s > 1000)); // At least one large dataset
        assert!(sizes.iter().any(|&s| s < 2000)); // At least one smaller dataset
    }

    #[test]
    fn test_benchmark_comparison() {
        let benchmark = CompressionBenchmark::new();
        let test_data = b"Hello World!".repeat(100);
        
        let results = benchmark.compare_algorithms(&test_data);
        assert!(!results.is_empty());
        
        // Results should be sorted by throughput
        for i in 1..results.len() {
            assert!(results[i-1].throughput_mbps >= results[i].throughput_mbps);
        }
    }

    #[test]
    fn test_best_algorithm_selection() {
        let benchmark = CompressionBenchmark::new();
        let test_data = b"A".repeat(1000); // Very compressible
        
        let best_speed = benchmark.get_best_algorithm(&test_data, true);
        let best_ratio = benchmark.get_best_algorithm(&test_data, false);
        
        assert!(best_speed.is_some());
        assert!(best_ratio.is_some());
        
        // For highly compressible data, the algorithms might be the same or different
        // depending on their specific characteristics
    }

    #[test]
    fn test_report_generation() {
        let benchmark = CompressionBenchmark::new();
        let report = benchmark.generate_report();
        
        assert!(report.contains("Compression Benchmark Report"));
        assert!(report.contains("zstd") || report.contains("Zstd"));
        assert!(report.contains("lz4") || report.contains("Lz4"));
        assert!(report.contains("Average Compression Ratio"));
        assert!(report.contains("Average Throughput"));
    }
}