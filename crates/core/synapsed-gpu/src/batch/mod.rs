//! Batch processing pipeline for efficient GPU utilization.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex, mpsc, Semaphore};
use tokio::time::{timeout, sleep};
use tracing::{debug, error, info, warn};
use serde::{Deserialize, Serialize};

use crate::{Device, MemoryManager, KernelManager, GpuBuffer, KernelParams, Result, GpuError};

pub mod queue;
pub mod scheduler;
pub mod pipeline;

pub use queue::{BatchQueue, QueueConfig};
pub use scheduler::{BatchScheduler, SchedulingStrategy};
pub use pipeline::{BatchPipeline, PipelineStage};

/// Batch processor for efficient GPU operations.
#[derive(Debug)]
pub struct BatchProcessor {
    device: Device,
    memory_manager: Arc<MemoryManager>,
    kernel_manager: Arc<KernelManager>,
    config: BatchConfig,
    queue: Arc<BatchQueue>,
    scheduler: Arc<BatchScheduler>,
    pipeline: Arc<BatchPipeline>,
    active_batches: Arc<RwLock<HashMap<String, ActiveBatch>>>,
    metrics: Arc<RwLock<BatchMetrics>>,
    shutdown_signal: Arc<Mutex<Option<mpsc::Sender<()>>>>,
}

/// Batch processing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Default batch size for operations.
    pub default_batch_size: u32,
    
    /// Maximum batch size allowed.
    pub max_batch_size: u32,
    
    /// Minimum batch size for efficiency.
    pub min_batch_size: u32,
    
    /// Batch timeout in milliseconds.
    pub batch_timeout_ms: u64,
    
    /// Maximum concurrent batches.
    pub max_concurrent_batches: u32,
    
    /// Enable dynamic batch sizing.
    pub enable_dynamic_sizing: bool,
    
    /// Queue capacity.
    pub queue_capacity: u32,
    
    /// Memory pooling configuration.
    pub enable_memory_pooling: bool,
    
    /// Pipeline depth.
    pub pipeline_depth: u32,
    
    /// Enable batch coalescing.
    pub enable_coalescing: bool,
    
    /// Coalescing window in milliseconds.
    pub coalescing_window_ms: u64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            default_batch_size: 1024,
            max_batch_size: 16384,
            min_batch_size: 32,
            batch_timeout_ms: 100,
            max_concurrent_batches: 8,
            enable_dynamic_sizing: true,
            queue_capacity: 10000,
            enable_memory_pooling: true,
            pipeline_depth: 4,
            enable_coalescing: true,
            coalescing_window_ms: 50,
        }
    }
}

/// Batch operation to be executed.
#[derive(Debug, Clone)]
pub struct BatchOperation {
    pub id: String,
    pub operation_type: String,
    pub kernel_name: String,
    pub kernel_params: KernelParams,
    pub input_buffers: Vec<String>,
    pub output_buffers: Vec<String>,
    pub priority: BatchPriority,
    pub submit_time: Instant,
    pub deadline: Option<Instant>,
    pub callback: Option<BatchCallback>,
}

/// Batch operation priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BatchPriority {
    Low = 0,
    Medium = 1,
    High = 2,
    Critical = 3,
}

/// Batch operation callback.
#[derive(Debug, Clone)]
pub struct BatchCallback {
    pub on_complete: Option<String>, // Serialized callback function
    pub on_error: Option<String>,
    pub context: HashMap<String, String>,
}

/// Active batch information.
#[derive(Debug, Clone)]
struct ActiveBatch {
    id: String,
    operations: Vec<BatchOperation>,
    start_time: Instant,
    status: BatchStatus,
    allocated_buffers: Vec<String>,
}

/// Batch execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BatchStatus {
    Queued,
    Preparing,
    Executing,
    Completed,
    Failed,
    Cancelled,
}

/// Batch processing result.
#[derive(Debug, Clone)]
pub struct BatchResult {
    pub batch_id: String,
    pub operation_results: Vec<OperationResult>,
    pub total_execution_time: Duration,
    pub queue_time: Duration,
    pub preparation_time: Duration,
    pub kernel_execution_time: Duration,
    pub memory_transfer_time: Duration,
    pub throughput_ops_per_sec: f64,
    pub efficiency_score: f64,
}

/// Individual operation result within a batch.
#[derive(Debug, Clone)]
pub struct OperationResult {
    pub operation_id: String,
    pub success: bool,
    pub execution_time: Duration,
    pub error_message: Option<String>,
    pub output_data: Option<Vec<u8>>,
}

/// Batch processing metrics.
#[derive(Debug, Clone, Default)]
struct BatchMetrics {
    total_batches_processed: u64,
    total_operations_processed: u64,
    average_batch_size: f64,
    average_execution_time_ms: f64,
    throughput_batches_per_sec: f64,
    throughput_ops_per_sec: f64,
    queue_utilization: f64,
    memory_utilization: f64,
    gpu_utilization: f64,
    error_rate: f64,
    cache_hit_rate: f64,
}

impl BatchProcessor {
    /// Create a new batch processor.
    pub async fn new(
        device: Device,
        memory_manager: Arc<MemoryManager>,
        kernel_manager: Arc<KernelManager>,
        config: BatchConfig,
    ) -> Result<Self> {
        info!("Creating batch processor for device: {}", device.info().id);

        let queue_config = QueueConfig {
            capacity: config.queue_capacity,
            enable_coalescing: config.enable_coalescing,
            coalescing_window_ms: config.coalescing_window_ms,
            priority_levels: 4,
        };
        
        let queue = Arc::new(BatchQueue::new(queue_config).await?);
        
        let scheduler = Arc::new(BatchScheduler::new(
            SchedulingStrategy::Adaptive,
            config.max_concurrent_batches,
        ).await?);
        
        let pipeline = Arc::new(BatchPipeline::new(
            config.pipeline_depth,
            device.clone(),
        ).await?);

        Ok(Self {
            device,
            memory_manager,
            kernel_manager,
            config,
            queue,
            scheduler,
            pipeline,
            active_batches: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(BatchMetrics::default())),
            shutdown_signal: Arc::new(Mutex::new(None)),
        })
    }

    /// Start the batch processor.
    pub async fn start(&self) -> Result<()> {
        info!("Starting batch processor");

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        {
            let mut signal = self.shutdown_signal.lock().await;
            *signal = Some(shutdown_tx);
        }

        // Start processing loop
        let processor = self.clone();
        tokio::spawn(async move {
            processor.processing_loop(shutdown_rx).await;
        });

        info!("Batch processor started");
        Ok(())
    }

    /// Submit a batch operation for processing.
    pub async fn submit_operation(&self, operation: BatchOperation) -> Result<String> {
        debug!("Submitting batch operation: {}", operation.id);

        // Validate operation
        self.validate_operation(&operation)?;

        // Submit to queue
        let batch_id = self.queue.enqueue(operation).await?;

        debug!("Operation queued with batch ID: {}", batch_id);
        Ok(batch_id)
    }

    /// Submit multiple operations as a single batch.
    pub async fn submit_batch(&self, operations: Vec<BatchOperation>) -> Result<String> {
        if operations.is_empty() {
            return Err(GpuError::batch("Cannot submit empty batch"));
        }

        debug!("Submitting batch with {} operations", operations.len());

        // Validate all operations
        for operation in &operations {
            self.validate_operation(operation)?;
        }

        // Submit to queue
        let batch_id = self.queue.enqueue_batch(operations).await?;

        debug!("Batch queued with ID: {}", batch_id);
        Ok(batch_id)
    }

    /// Get batch processing result.
    pub async fn get_result(&self, batch_id: &str) -> Result<Option<BatchResult>> {
        self.scheduler.get_result(batch_id).await
    }

    /// Wait for batch completion with timeout.
    pub async fn wait_for_completion(
        &self,
        batch_id: &str,
        timeout_ms: u64,
    ) -> Result<BatchResult> {
        let timeout_duration = Duration::from_millis(timeout_ms);
        
        match timeout(timeout_duration, self.wait_for_batch(batch_id)).await {
            Ok(result) => result,
            Err(_) => Err(GpuError::Timeout { seconds: timeout_ms / 1000 }),
        }
    }

    /// Cancel a pending or executing batch.
    pub async fn cancel_batch(&self, batch_id: &str) -> Result<bool> {
        info!("Cancelling batch: {}", batch_id);

        // Try to cancel in queue first
        if self.queue.cancel(batch_id).await? {
            return Ok(true);
        }

        // Try to cancel active batch
        let mut active_batches = self.active_batches.write().await;
        if let Some(active_batch) = active_batches.get_mut(batch_id) {
            active_batch.status = BatchStatus::Cancelled;
            return Ok(true);
        }

        Ok(false)
    }

    /// Get current processing statistics.
    pub async fn get_metrics(&self) -> BatchProcessingMetrics {
        let metrics = self.metrics.read().await.clone();
        let queue_stats = self.queue.get_stats().await;
        let scheduler_stats = self.scheduler.get_stats().await;

        BatchProcessingMetrics {
            total_batches_processed: metrics.total_batches_processed,
            total_operations_processed: metrics.total_operations_processed,
            average_batch_size: metrics.average_batch_size,
            average_execution_time_ms: metrics.average_execution_time_ms,
            throughput_batches_per_sec: metrics.throughput_batches_per_sec,
            throughput_ops_per_sec: metrics.throughput_ops_per_sec,
            queue_length: queue_stats.current_size,
            queue_utilization: queue_stats.utilization,
            active_batches: scheduler_stats.active_batches,
            memory_utilization: metrics.memory_utilization,
            gpu_utilization: metrics.gpu_utilization,
            error_rate: metrics.error_rate,
        }
    }

    /// Shutdown the batch processor.
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down batch processor");

        if let Some(sender) = self.shutdown_signal.lock().await.take() {
            let _ = sender.send(()).await;
        }

        // Wait for active batches to complete (with timeout)
        let timeout_duration = Duration::from_secs(30);
        let start_time = Instant::now();

        while start_time.elapsed() < timeout_duration {
            let active_count = self.active_batches.read().await.len();
            if active_count == 0 {
                break;
            }

            sleep(Duration::from_millis(100)).await;
        }

        info!("Batch processor shutdown complete");
        Ok(())
    }

    // Internal methods

    async fn processing_loop(&self, mut shutdown_rx: mpsc::Receiver<()>) {
        info!("Starting batch processing loop");

        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_batches as usize));

        loop {
            tokio::select! {
                // Check for shutdown signal
                _ = shutdown_rx.recv() => {
                    info!("Received shutdown signal");
                    break;
                }

                // Process next batch
                batch_opt = self.queue.dequeue() => {
                    if let Some(operations) = batch_opt {
                        let permit = semaphore.clone().acquire_owned().await.unwrap();
                        let processor = self.clone();
                        
                        tokio::spawn(async move {
                            let _permit = permit; // Hold permit for duration of processing
                            if let Err(e) = processor.process_operations(operations).await {
                                error!("Batch processing failed: {}", e);
                            }
                        });
                    } else {
                        // No batches available, wait a bit
                        sleep(Duration::from_millis(10)).await;
                    }
                }
            }
        }

        info!("Batch processing loop ended");
    }

    async fn process_operations(&self, operations: Vec<BatchOperation>) -> Result<()> {
        let batch_id = uuid::Uuid::new_v4().to_string();
        let start_time = Instant::now();

        debug!("Processing batch {} with {} operations", batch_id, operations.len());

        // Create active batch record
        let active_batch = ActiveBatch {
            id: batch_id.clone(),
            operations: operations.clone(),
            start_time,
            status: BatchStatus::Preparing,
            allocated_buffers: Vec::new(),
        };

        {
            let mut active_batches = self.active_batches.write().await;
            active_batches.insert(batch_id.clone(), active_batch);
        }

        // Process the batch
        let result = self.execute_batch(&batch_id, operations).await;

        // Update metrics and clean up
        self.update_metrics(&batch_id, &result).await;
        
        {
            let mut active_batches = self.active_batches.write().await;
            active_batches.remove(&batch_id);
        }

        // Store result
        match &result {
            Ok(batch_result) => {
                self.scheduler.store_result(&batch_id, batch_result.clone()).await?;
            }
            Err(e) => {
                error!("Batch {} failed: {}", batch_id, e);
            }
        }

        result.map(|_| ())
    }

    async fn execute_batch(
        &self,
        batch_id: &str,
        operations: Vec<BatchOperation>,
    ) -> Result<BatchResult> {
        
        let start_time = Instant::now();
        let mut operation_results = Vec::new();
        let mut preparation_time = Duration::ZERO;
        let mut kernel_execution_time = Duration::ZERO;
        let mut memory_transfer_time = Duration::ZERO;

        // Update status
        {
            let mut active_batches = self.active_batches.write().await;
            if let Some(batch) = active_batches.get_mut(batch_id) {
                batch.status = BatchStatus::Executing;
            }
        }

        // Process each operation
        for operation in &operations {
            let op_start = Instant::now();
            
            match self.execute_operation(operation).await {
                Ok((prep_time, exec_time, transfer_time, output)) => {
                    preparation_time += prep_time;
                    kernel_execution_time += exec_time;
                    memory_transfer_time += transfer_time;
                    
                    operation_results.push(OperationResult {
                        operation_id: operation.id.clone(),
                        success: true,
                        execution_time: op_start.elapsed(),
                        error_message: None,
                        output_data: output,
                    });
                }
                Err(e) => {
                    operation_results.push(OperationResult {
                        operation_id: operation.id.clone(),
                        success: false,
                        execution_time: op_start.elapsed(),
                        error_message: Some(e.to_string()),
                        output_data: None,
                    });
                }
            }
        }

        let total_execution_time = start_time.elapsed();
        let queue_time = operations.first()
            .map(|op| start_time.duration_since(op.submit_time))
            .unwrap_or_default();

        let successful_ops = operation_results.iter().filter(|r| r.success).count();
        let throughput = successful_ops as f64 / total_execution_time.as_secs_f64();
        let efficiency = successful_ops as f64 / operations.len() as f64;

        Ok(BatchResult {
            batch_id: batch_id.to_string(),
            operation_results,
            total_execution_time,
            queue_time,
            preparation_time,
            kernel_execution_time,
            memory_transfer_time,
            throughput_ops_per_sec: throughput,
            efficiency_score: efficiency,
        })
    }

    async fn execute_operation(
        &self,
        operation: &BatchOperation,
    ) -> Result<(Duration, Duration, Duration, Option<Vec<u8>>)> {
        
        let prep_start = Instant::now();
        
        // Prepare buffers (simplified)
        let buffers = HashMap::new(); // Would allocate actual buffers
        let preparation_time = prep_start.elapsed();

        let exec_start = Instant::now();
        
        // Execute kernel
        let _result = self.kernel_manager.execute_kernel(
            &operation.kernel_name,
            operation.kernel_params.clone(),
            &buffers,
        ).await?;
        
        let kernel_execution_time = exec_start.elapsed();

        let transfer_start = Instant::now();
        
        // Transfer results (simplified)
        let output_data = Some(vec![0u8; 1024]); // Mock output
        let memory_transfer_time = transfer_start.elapsed();

        Ok((preparation_time, kernel_execution_time, memory_transfer_time, output_data))
    }

    async fn validate_operation(&self, operation: &BatchOperation) -> Result<()> {
        if operation.id.is_empty() {
            return Err(GpuError::batch("Operation ID cannot be empty"));
        }

        if operation.kernel_name.is_empty() {
            return Err(GpuError::batch("Kernel name cannot be empty"));
        }

        if operation.kernel_params.global_work_size.0 == 0 {
            return Err(GpuError::batch("Global work size cannot be zero"));
        }

        Ok(())
    }

    async fn wait_for_batch(&self, batch_id: &str) -> Result<BatchResult> {
        loop {
            if let Some(result) = self.get_result(batch_id).await? {
                return Ok(result);
            }

            sleep(Duration::from_millis(10)).await;
        }
    }

    async fn update_metrics(&self, batch_id: &str, result: &Result<BatchResult>) {
        let mut metrics = self.metrics.write().await;
        
        metrics.total_batches_processed += 1;
        
        if let Ok(batch_result) = result {
            metrics.total_operations_processed += batch_result.operation_results.len() as u64;
            
            let batch_size = batch_result.operation_results.len() as f64;
            let execution_time = batch_result.total_execution_time.as_millis() as f64;
            
            // Update running averages
            let n = metrics.total_batches_processed as f64;
            metrics.average_batch_size = (metrics.average_batch_size * (n - 1.0) + batch_size) / n;
            metrics.average_execution_time_ms = (metrics.average_execution_time_ms * (n - 1.0) + execution_time) / n;
            
            // Update throughput
            if execution_time > 0.0 {
                let throughput = batch_size / (execution_time / 1000.0);
                metrics.throughput_ops_per_sec = (metrics.throughput_ops_per_sec * (n - 1.0) + throughput) / n;
            }
        }
    }
}

// Clone implementation for async spawning
impl Clone for BatchProcessor {
    fn clone(&self) -> Self {
        Self {
            device: self.device.clone(),
            memory_manager: self.memory_manager.clone(),
            kernel_manager: self.kernel_manager.clone(),
            config: self.config.clone(),
            queue: self.queue.clone(),
            scheduler: self.scheduler.clone(),
            pipeline: self.pipeline.clone(),
            active_batches: self.active_batches.clone(),
            metrics: self.metrics.clone(),
            shutdown_signal: self.shutdown_signal.clone(),
        }
    }
}

/// Public metrics structure.
#[derive(Debug, Clone)]
pub struct BatchProcessingMetrics {
    pub total_batches_processed: u64,
    pub total_operations_processed: u64,
    pub average_batch_size: f64,
    pub average_execution_time_ms: f64,
    pub throughput_batches_per_sec: f64,
    pub throughput_ops_per_sec: f64,
    pub queue_length: u32,
    pub queue_utilization: f64,
    pub active_batches: u32,
    pub memory_utilization: f64,
    pub gpu_utilization: f64,
    pub error_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceManager, DeviceConfig, MemoryConfig, KernelArg, ScalarValue};

    async fn create_test_batch_processor() -> Result<BatchProcessor> {
        let device_config = DeviceConfig::default();
        let device_manager = DeviceManager::new(device_config).await?;
        let device = device_manager.select_best_device().await?;
        
        let memory_config = MemoryConfig::default();
        let memory_manager = Arc::new(MemoryManager::new(device.clone(), memory_config).await?);
        let kernel_manager = Arc::new(KernelManager::new(device.clone()).await?);
        
        let batch_config = BatchConfig::default();
        
        BatchProcessor::new(device, memory_manager, kernel_manager, batch_config).await
    }

    #[tokio::test]
    async fn test_batch_processor_creation() {
        let processor = create_test_batch_processor().await.unwrap();
        assert_eq!(processor.config.default_batch_size, 1024);
    }

    #[tokio::test]
    async fn test_operation_submission() {
        let processor = create_test_batch_processor().await.unwrap();
        processor.start().await.unwrap();

        let operation = BatchOperation {
            id: "test-op-1".to_string(),
            operation_type: "test".to_string(),
            kernel_name: "test_kernel".to_string(),
            kernel_params: KernelParams {
                global_work_size: (1024, 1, 1),
                local_work_size: Some((64, 1, 1)),
                args: vec![KernelArg::Scalar(ScalarValue::U32(42))],
                shared_memory_bytes: 0,
            },
            input_buffers: vec!["input".to_string()],
            output_buffers: vec!["output".to_string()],
            priority: BatchPriority::Medium,
            submit_time: Instant::now(),
            deadline: None,
            callback: None,
        };

        let batch_id = processor.submit_operation(operation).await.unwrap();
        assert!(!batch_id.is_empty());
        
        processor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_batch_submission() {
        let processor = create_test_batch_processor().await.unwrap();
        processor.start().await.unwrap();

        let operations = vec![
            create_test_operation("op1"),
            create_test_operation("op2"),
            create_test_operation("op3"),
        ];

        let batch_id = processor.submit_batch(operations).await.unwrap();
        assert!(!batch_id.is_empty());
        
        processor.shutdown().await.unwrap();
    }

    fn create_test_operation(id: &str) -> BatchOperation {
        BatchOperation {
            id: id.to_string(),
            operation_type: "test".to_string(),
            kernel_name: "test_kernel".to_string(),
            kernel_params: KernelParams {
                global_work_size: (256, 1, 1),
                local_work_size: Some((32, 1, 1)),
                args: vec![],
                shared_memory_bytes: 0,
            },
            input_buffers: vec![],
            output_buffers: vec![],
            priority: BatchPriority::Medium,
            submit_time: Instant::now(),
            deadline: None,
            callback: None,
        }
    }

    #[tokio::test]
    async fn test_operation_validation() {
        let processor = create_test_batch_processor().await.unwrap();

        // Test empty ID
        let mut invalid_op = create_test_operation("");
        let result = processor.validate_operation(&invalid_op).await;
        assert!(result.is_err());

        // Test empty kernel name
        invalid_op = create_test_operation("test");
        invalid_op.kernel_name = "".to_string();
        let result = processor.validate_operation(&invalid_op).await;
        assert!(result.is_err());

        // Test zero work size
        invalid_op = create_test_operation("test");
        invalid_op.kernel_params.global_work_size = (0, 1, 1);
        let result = processor.validate_operation(&invalid_op).await;
        assert!(result.is_err());

        // Test valid operation
        let valid_op = create_test_operation("valid");
        let result = processor.validate_operation(&valid_op).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_metrics_collection() {
        let processor = create_test_batch_processor().await.unwrap();
        
        let metrics = processor.get_metrics().await;
        assert_eq!(metrics.total_batches_processed, 0);
        assert_eq!(metrics.total_operations_processed, 0);
    }

    #[tokio::test]
    async fn test_batch_priorities() {
        assert!(BatchPriority::Critical > BatchPriority::High);
        assert!(BatchPriority::High > BatchPriority::Medium);
        assert!(BatchPriority::Medium > BatchPriority::Low);
    }

    #[tokio::test]
    async fn test_batch_config_defaults() {
        let config = BatchConfig::default();
        assert_eq!(config.default_batch_size, 1024);
        assert_eq!(config.max_batch_size, 16384);
        assert_eq!(config.min_batch_size, 32);
        assert!(config.enable_dynamic_sizing);
        assert!(config.enable_memory_pooling);
        assert!(config.enable_coalescing);
    }
}