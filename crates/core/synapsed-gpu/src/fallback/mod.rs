//! CPU fallback system for GPU operations.

use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use serde::{Deserialize, Serialize};

use crate::{FallbackConfig, GpuError, Result};

pub mod crypto;
pub mod kyber;
pub mod compute;

pub use crypto::CryptoFallback;
pub use kyber::KyberFallback;
pub use compute::ComputeFallback;

/// CPU fallback processor for GPU operations.
#[derive(Debug)]
pub struct FallbackProcessor {
    config: FallbackConfig,
    crypto_fallback: Arc<CryptoFallback>,
    kyber_fallback: Arc<KyberFallback>,
    compute_fallback: Arc<ComputeFallback>,
    active_fallbacks: Arc<RwLock<HashMap<String, FallbackOperation>>>,
    metrics: Arc<RwLock<FallbackMetrics>>,
}

/// Reason for falling back to CPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FallbackReason {
    /// No GPU devices available.
    NoGpuAvailable,
    
    /// GPU device error or failure.
    GpuError,
    
    /// GPU memory exhausted.
    OutOfMemory,
    
    /// GPU kernel compilation failed.
    KernelCompilationFailed,
    
    /// GPU operation timeout.
    Timeout,
    
    /// Performance is better on CPU for small workloads.
    BetterCpuPerformance,
    
    /// Explicit user request.
    UserRequested,
    
    /// Testing purposes.
    Testing,
}

/// Fallback operation tracking.
#[derive(Debug, Clone)]
struct FallbackOperation {
    id: String,
    operation_type: String,
    reason: FallbackReason,
    start_time: Instant,
    status: FallbackStatus,
}

/// Fallback operation status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FallbackStatus {
    Queued,
    Executing,
    Completed,
    Failed,
}

/// Fallback processing metrics.
#[derive(Debug, Clone, Default)]
struct FallbackMetrics {
    total_fallbacks: u64,
    successful_fallbacks: u64,
    failed_fallbacks: u64,
    average_execution_time_ms: f64,
    fallback_reasons: HashMap<FallbackReason, u64>,
    performance_comparison: HashMap<String, PerformanceComparison>,
}

/// Performance comparison between GPU and CPU.
#[derive(Debug, Clone)]
struct PerformanceComparison {
    gpu_avg_time_ms: f64,
    cpu_avg_time_ms: f64,
    speedup_ratio: f64,
    sample_count: u64,
}

/// Fallback operation result.
#[derive(Debug, Clone)]
pub struct FallbackResult<T> {
    pub data: T,
    pub execution_time: Duration,
    pub reason: FallbackReason,
    pub performance_score: f64,
}

/// Kyber768 fallback parameters.
#[derive(Debug, Clone)]
pub struct Kyber768FallbackParams {
    pub batch_size: u32,
    pub use_parallel: bool,
    pub thread_count: Option<usize>,
}

impl Default for Kyber768FallbackParams {
    fn default() -> Self {
        Self {
            batch_size: 1,
            use_parallel: true,
            thread_count: None, // Use system default
        }
    }
}

impl FallbackProcessor {
    /// Create a new fallback processor.
    pub fn new(config: FallbackConfig) -> Self {
        info!("Creating CPU fallback processor");

        let crypto_fallback = Arc::new(CryptoFallback::new());
        let kyber_fallback = Arc::new(KyberFallback::new(config.cpu_thread_pool_size));
        let compute_fallback = Arc::new(ComputeFallback::new());

        Self {
            config,
            crypto_fallback,
            kyber_fallback,
            compute_fallback,
            active_fallbacks: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(FallbackMetrics::default())),
        }
    }

    /// Check if fallback should be used for the given workload size.
    pub async fn should_use_fallback(&self, operation_type: &str, workload_size: u64) -> bool {
        if !self.config.enable_auto_fallback {
            return false;
        }

        // Check performance history
        let metrics = self.metrics.read().await;
        if let Some(comparison) = metrics.performance_comparison.get(operation_type) {
            // Use CPU if it's faster for this operation type
            if comparison.cpu_avg_time_ms < comparison.gpu_avg_time_ms * 0.8 {
                return true;
            }
        }

        // Use CPU for very small workloads
        match operation_type {
            "kyber768_keygen" | "kyber768_encaps" | "kyber768_decaps" => workload_size < 16,
            "sha256" | "sha3" => workload_size < 1024,
            "aes_encrypt" | "aes_decrypt" => workload_size < 256,
            _ => workload_size < 64,
        }
    }

    /// Perform Kyber768 key generation fallback.
    pub async fn kyber768_keygen_fallback(
        &self,
        seeds: &[u8],
        params: &Kyber768FallbackParams,
        reason: FallbackReason,
    ) -> Result<FallbackResult<(Vec<u8>, Vec<u8>)>> {
        let operation_id = uuid::Uuid::new_v4().to_string();
        let start_time = Instant::now();

        debug!("Starting Kyber768 keygen fallback (reason: {:?})", reason);

        // Track operation
        self.track_fallback_operation(&operation_id, "kyber768_keygen", reason).await;

        // Perform CPU-based key generation
        let result = self.kyber_fallback.batch_keygen(seeds, params).await;

        let execution_time = start_time.elapsed();
        self.complete_fallback_operation(&operation_id, result.is_ok()).await;

        match result {
            Ok((public_keys, secret_keys)) => {
                let performance_score = self.calculate_performance_score(
                    "kyber768_keygen",
                    execution_time,
                    params.batch_size as u64,
                ).await;

                Ok(FallbackResult {
                    data: (public_keys, secret_keys),
                    execution_time,
                    reason,
                    performance_score,
                })
            }
            Err(e) => {
                error!("Kyber768 keygen fallback failed: {}", e);
                Err(GpuError::FallbackError { message: e.to_string() })
            }
        }
    }

    /// Perform Kyber768 encapsulation fallback.
    pub async fn kyber768_encaps_fallback(
        &self,
        public_keys: &[u8],
        messages: &[u8],
        params: &Kyber768FallbackParams,
        reason: FallbackReason,
    ) -> Result<FallbackResult<(Vec<u8>, Vec<u8>)>> {
        let operation_id = uuid::Uuid::new_v4().to_string();
        let start_time = Instant::now();

        debug!("Starting Kyber768 encaps fallback (reason: {:?})", reason);

        self.track_fallback_operation(&operation_id, "kyber768_encaps", reason).await;

        let result = self.kyber_fallback.batch_encaps(public_keys, messages, params).await;

        let execution_time = start_time.elapsed();
        self.complete_fallback_operation(&operation_id, result.is_ok()).await;

        match result {
            Ok((ciphertexts, shared_secrets)) => {
                let performance_score = self.calculate_performance_score(
                    "kyber768_encaps",
                    execution_time,
                    params.batch_size as u64,
                ).await;

                Ok(FallbackResult {
                    data: (ciphertexts, shared_secrets),
                    execution_time,
                    reason,
                    performance_score,
                })
            }
            Err(e) => {
                error!("Kyber768 encaps fallback failed: {}", e);
                Err(GpuError::FallbackError { message: e.to_string() })
            }
        }
    }

    /// Perform Kyber768 decapsulation fallback.
    pub async fn kyber768_decaps_fallback(
        &self,
        secret_keys: &[u8],
        ciphertexts: &[u8],
        params: &Kyber768FallbackParams,
        reason: FallbackReason,
    ) -> Result<FallbackResult<Vec<u8>>> {
        let operation_id = uuid::Uuid::new_v4().to_string();
        let start_time = Instant::now();

        debug!("Starting Kyber768 decaps fallback (reason: {:?})", reason);

        self.track_fallback_operation(&operation_id, "kyber768_decaps", reason).await;

        let result = self.kyber_fallback.batch_decaps(secret_keys, ciphertexts, params).await;

        let execution_time = start_time.elapsed();
        self.complete_fallback_operation(&operation_id, result.is_ok()).await;

        match result {
            Ok(shared_secrets) => {
                let performance_score = self.calculate_performance_score(
                    "kyber768_decaps",
                    execution_time,
                    params.batch_size as u64,
                ).await;

                Ok(FallbackResult {
                    data: shared_secrets,
                    execution_time,
                    reason,
                    performance_score,
                })
            }
            Err(e) => {
                error!("Kyber768 decaps fallback failed: {}", e);
                Err(GpuError::FallbackError { message: e.to_string() })
            }
        }
    }

    /// Perform cryptographic hash fallback.
    pub async fn hash_fallback(
        &self,
        algorithm: &str,
        data: &[u8],
        batch_size: u32,
        reason: FallbackReason,
    ) -> Result<FallbackResult<Vec<u8>>> {
        let operation_id = uuid::Uuid::new_v4().to_string();
        let start_time = Instant::now();

        debug!("Starting {} hash fallback (reason: {:?})", algorithm, reason);

        self.track_fallback_operation(&operation_id, &format!("{}_hash", algorithm), reason).await;

        let result = self.crypto_fallback.batch_hash(algorithm, data, batch_size).await;

        let execution_time = start_time.elapsed();
        self.complete_fallback_operation(&operation_id, result.is_ok()).await;

        match result {
            Ok(hashes) => {
                let performance_score = self.calculate_performance_score(
                    &format!("{}_hash", algorithm),
                    execution_time,
                    batch_size as u64,
                ).await;

                Ok(FallbackResult {
                    data: hashes,
                    execution_time,
                    reason,
                    performance_score,
                })
            }
            Err(e) => {
                error!("{} hash fallback failed: {}", algorithm, e);
                Err(GpuError::FallbackError { message: e.to_string() })
            }
        }
    }

    /// Perform symmetric encryption fallback.
    pub async fn encrypt_fallback(
        &self,
        algorithm: &str,
        data: &[u8],
        keys: &[u8],
        batch_size: u32,
        reason: FallbackReason,
    ) -> Result<FallbackResult<Vec<u8>>> {
        let operation_id = uuid::Uuid::new_v4().to_string();
        let start_time = Instant::now();

        debug!("Starting {} encrypt fallback (reason: {:?})", algorithm, reason);

        self.track_fallback_operation(&operation_id, &format!("{}_encrypt", algorithm), reason).await;

        let result = self.crypto_fallback.batch_encrypt(algorithm, data, keys, batch_size).await;

        let execution_time = start_time.elapsed();
        self.complete_fallback_operation(&operation_id, result.is_ok()).await;

        match result {
            Ok(ciphertexts) => {
                let performance_score = self.calculate_performance_score(
                    &format!("{}_encrypt", algorithm),
                    execution_time,
                    batch_size as u64,
                ).await;

                Ok(FallbackResult {
                    data: ciphertexts,
                    execution_time,
                    reason,
                    performance_score,
                })
            }
            Err(e) => {
                error!("{} encrypt fallback failed: {}", algorithm, e);
                Err(GpuError::FallbackError { message: e.to_string() })
            }
        }
    }

    /// Get fallback processing statistics.
    pub async fn get_fallback_metrics(&self) -> FallbackStatistics {
        let metrics = self.metrics.read().await;
        let active_count = self.active_fallbacks.read().await.len() as u64;

        let success_rate = if metrics.total_fallbacks > 0 {
            metrics.successful_fallbacks as f64 / metrics.total_fallbacks as f64
        } else {
            0.0
        };

        FallbackStatistics {
            total_fallbacks: metrics.total_fallbacks,
            successful_fallbacks: metrics.successful_fallbacks,
            failed_fallbacks: metrics.failed_fallbacks,
            active_fallbacks: active_count,
            success_rate,
            average_execution_time_ms: metrics.average_execution_time_ms,
            fallback_reasons: metrics.fallback_reasons.clone(),
        }
    }

    /// Update performance comparison data.
    pub async fn update_performance_comparison(
        &self,
        operation_type: &str,
        gpu_time: Duration,
        cpu_time: Duration,
    ) {
        let mut metrics = self.metrics.write().await;
        
        let comparison = metrics.performance_comparison
            .entry(operation_type.to_string())
            .or_insert(PerformanceComparison {
                gpu_avg_time_ms: 0.0,
                cpu_avg_time_ms: 0.0,
                speedup_ratio: 1.0,
                sample_count: 0,
            });

        let n = comparison.sample_count as f64;
        let gpu_time_ms = gpu_time.as_millis() as f64;
        let cpu_time_ms = cpu_time.as_millis() as f64;

        // Update running averages
        comparison.gpu_avg_time_ms = (comparison.gpu_avg_time_ms * n + gpu_time_ms) / (n + 1.0);
        comparison.cpu_avg_time_ms = (comparison.cpu_avg_time_ms * n + cpu_time_ms) / (n + 1.0);
        comparison.sample_count += 1;

        // Calculate speedup ratio (GPU time / CPU time)
        if comparison.cpu_avg_time_ms > 0.0 {
            comparison.speedup_ratio = comparison.gpu_avg_time_ms / comparison.cpu_avg_time_ms;
        }

        debug!(
            "Performance comparison for {}: GPU={:.2}ms, CPU={:.2}ms, Speedup={:.2}x",
            operation_type,
            comparison.gpu_avg_time_ms,
            comparison.cpu_avg_time_ms,
            comparison.speedup_ratio
        );
    }

    // Internal methods

    async fn track_fallback_operation(
        &self,
        operation_id: &str,
        operation_type: &str,
        reason: FallbackReason,
    ) {
        let operation = FallbackOperation {
            id: operation_id.to_string(),
            operation_type: operation_type.to_string(),
            reason,
            start_time: Instant::now(),
            status: FallbackStatus::Executing,
        };

        let mut active_fallbacks = self.active_fallbacks.write().await;
        active_fallbacks.insert(operation_id.to_string(), operation);

        let mut metrics = self.metrics.write().await;
        metrics.total_fallbacks += 1;
        *metrics.fallback_reasons.entry(reason).or_insert(0) += 1;
    }

    async fn complete_fallback_operation(&self, operation_id: &str, success: bool) {
        let mut active_fallbacks = self.active_fallbacks.write().await;
        if let Some(operation) = active_fallbacks.remove(operation_id) {
            let execution_time = operation.start_time.elapsed();
            
            let mut metrics = self.metrics.write().await;
            
            if success {
                metrics.successful_fallbacks += 1;
            } else {
                metrics.failed_fallbacks += 1;
            }

            // Update average execution time
            let n = metrics.total_fallbacks as f64;
            let new_time = execution_time.as_millis() as f64;
            metrics.average_execution_time_ms = 
                (metrics.average_execution_time_ms * (n - 1.0) + new_time) / n;
        }
    }

    async fn calculate_performance_score(
        &self,
        operation_type: &str,
        execution_time: Duration,
        workload_size: u64,
    ) -> f64 {
        let metrics = self.metrics.read().await;
        
        if let Some(comparison) = metrics.performance_comparison.get(operation_type) {
            let time_ms = execution_time.as_millis() as f64;
            let relative_performance = comparison.gpu_avg_time_ms / time_ms.max(1.0);
            
            // Adjust for workload size (smaller workloads favor CPU)
            let size_factor = (workload_size as f64 / 1000.0).min(1.0);
            
            relative_performance * (0.5 + 0.5 * size_factor)
        } else {
            // No comparison data available
            1.0
        }
    }
}

/// Public fallback statistics.
#[derive(Debug, Clone)]
pub struct FallbackStatistics {
    pub total_fallbacks: u64,
    pub successful_fallbacks: u64,
    pub failed_fallbacks: u64,
    pub active_fallbacks: u64,
    pub success_rate: f64,
    pub average_execution_time_ms: f64,
    pub fallback_reasons: HashMap<FallbackReason, u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FallbackConfig;

    fn create_test_fallback_processor() -> FallbackProcessor {
        let config = FallbackConfig::default();
        FallbackProcessor::new(config)
    }

    #[tokio::test]
    async fn test_fallback_processor_creation() {
        let processor = create_test_fallback_processor();
        let stats = processor.get_fallback_metrics().await;
        
        assert_eq!(stats.total_fallbacks, 0);
        assert_eq!(stats.successful_fallbacks, 0);
        assert_eq!(stats.failed_fallbacks, 0);
    }

    #[tokio::test]
    async fn test_should_use_fallback() {
        let processor = create_test_fallback_processor();
        
        // Small workloads should use fallback
        assert!(processor.should_use_fallback("kyber768_keygen", 1).await);
        assert!(processor.should_use_fallback("sha256", 100).await);
        
        // Large workloads should prefer GPU
        assert!(!processor.should_use_fallback("kyber768_keygen", 1000).await);
        assert!(!processor.should_use_fallback("sha256", 10000).await);
    }

    #[tokio::test]
    async fn test_kyber768_fallback_params() {
        let params = Kyber768FallbackParams::default();
        assert_eq!(params.batch_size, 1);
        assert!(params.use_parallel);
        assert!(params.thread_count.is_none());
    }

    #[tokio::test]
    async fn test_fallback_reasons() {
        // Test that all fallback reasons are distinct
        let reasons = vec![
            FallbackReason::NoGpuAvailable,
            FallbackReason::GpuError,
            FallbackReason::OutOfMemory,
            FallbackReason::KernelCompilationFailed,
            FallbackReason::Timeout,
            FallbackReason::BetterCpuPerformance,
            FallbackReason::UserRequested,
            FallbackReason::Testing,
        ];
        
        for (i, reason1) in reasons.iter().enumerate() {
            for (j, reason2) in reasons.iter().enumerate() {
                if i != j {
                    assert_ne!(reason1, reason2);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_performance_comparison_update() {
        let processor = create_test_fallback_processor();
        
        let gpu_time = Duration::from_millis(100);
        let cpu_time = Duration::from_millis(200);
        
        processor.update_performance_comparison("test_op", gpu_time, cpu_time).await;
        
        let metrics = processor.metrics.read().await;
        let comparison = metrics.performance_comparison.get("test_op").unwrap();
        
        assert_eq!(comparison.gpu_avg_time_ms, 100.0);
        assert_eq!(comparison.cpu_avg_time_ms, 200.0);
        assert_eq!(comparison.speedup_ratio, 0.5); // GPU is 2x faster
        assert_eq!(comparison.sample_count, 1);
    }

    #[tokio::test]
    async fn test_performance_score_calculation() {
        let processor = create_test_fallback_processor();
        
        // Add some performance comparison data
        processor.update_performance_comparison(
            "test_op",
            Duration::from_millis(50),
            Duration::from_millis(100),
        ).await;
        
        let score = processor.calculate_performance_score(
            "test_op",
            Duration::from_millis(80),
            1000,
        ).await;
        
        assert!(score > 0.0);
        assert!(score < 2.0);
    }

    #[tokio::test]
    async fn test_fallback_operation_tracking() {
        let processor = create_test_fallback_processor();
        
        let operation_id = "test-op-1";
        processor.track_fallback_operation(
            operation_id,
            "test_operation",
            FallbackReason::Testing,
        ).await;
        
        let stats = processor.get_fallback_metrics().await;
        assert_eq!(stats.total_fallbacks, 1);
        assert_eq!(stats.active_fallbacks, 1);
        assert_eq!(stats.fallback_reasons.get(&FallbackReason::Testing), Some(&1));
        
        processor.complete_fallback_operation(operation_id, true).await;
        
        let stats = processor.get_fallback_metrics().await;
        assert_eq!(stats.successful_fallbacks, 1);
        assert_eq!(stats.active_fallbacks, 0);
        assert_eq!(stats.success_rate, 1.0);
    }
}