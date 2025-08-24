//! Observability module for cryptographic operations
//!
//! Provides comprehensive monitoring and tracing for all cryptographic operations
//! including key generation, encryption, decryption, signing, and verification.

#[cfg(feature = "observability")]
use synapsed_substrates::{
    Subject, BasicSource, BasicSink,
    types::{Name, SubjectType},
};
#[cfg(feature = "observability")]
use synapsed_serventis::{BasicService, BasicProbe};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Crypto operation events
#[derive(Debug, Clone)]
pub enum CryptoEvent {
    /// Key generation started
    KeyGenStarted { algorithm: String },
    /// Key generation completed
    KeyGenCompleted { algorithm: String, duration: Duration },
    /// Encryption operation
    EncryptionPerformed { algorithm: String, data_size: usize, duration: Duration },
    /// Decryption operation
    DecryptionPerformed { algorithm: String, data_size: usize, duration: Duration },
    /// Signing operation
    SigningPerformed { algorithm: String, data_size: usize, duration: Duration },
    /// Verification operation
    VerificationPerformed { algorithm: String, success: bool, duration: Duration },
    /// Error occurred
    ErrorOccurred { operation: String, error: String },
}

/// Crypto performance metrics
#[derive(Debug, Clone)]
pub struct CryptoMetrics {
    /// Total operations performed
    pub total_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Average operation duration
    pub avg_duration_ms: f64,
    /// Key generations performed
    pub key_generations: u64,
    /// Encryptions performed
    pub encryptions: u64,
    /// Decryptions performed
    pub decryptions: u64,
    /// Signatures created
    pub signatures: u64,
    /// Verifications performed
    pub verifications: u64,
}

/// Observability context for crypto operations
pub struct CryptoObservability {
    #[cfg(feature = "observability")]
    subject: Arc<Subject>,
    #[cfg(feature = "observability")]
    event_source: Arc<BasicSource<CryptoEvent>>,
    #[cfg(feature = "observability")]
    metrics_sink: Arc<BasicSink<CryptoMetrics>>,
    #[cfg(feature = "observability")]
    service: Arc<BasicService>,
    #[cfg(feature = "observability")]
    probe: Arc<BasicProbe>,
    
    // Always track basic metrics even without full observability
    metrics: Arc<parking_lot::RwLock<CryptoMetrics>>,
}

impl CryptoObservability {
    /// Create a new observability context
    pub fn new(component: &str) -> Self {
        #[cfg(feature = "observability")]
        {
            let subject = Arc::new(Subject::new(
                Name::from(format!("crypto_{}", component)),
                SubjectType::Service,
            ));
            
            let event_source = Arc::new(BasicSource::new(
                Name::from(format!("crypto_{}_events", component))
            ));
            
            let metrics_sink = Arc::new(BasicSink::new(
                Name::from(format!("crypto_{}_metrics", component))
            ));
            
            let service = Arc::new(BasicService::new(
                Name::from(format!("crypto_{}_service", component))
            ));
            
            let probe = Arc::new(BasicProbe::new(
                Name::from(format!("crypto_{}_probe", component))
            ));
            
            Self {
                subject,
                event_source,
                metrics_sink,
                service,
                probe,
                metrics: Arc::new(parking_lot::RwLock::new(CryptoMetrics::default())),
            }
        }
        
        #[cfg(not(feature = "observability"))]
        {
            Self {
                metrics: Arc::new(parking_lot::RwLock::new(CryptoMetrics::default())),
            }
        }
    }
    
    /// Record a crypto operation
    pub fn record_operation(&self, event: CryptoEvent) {
        let mut metrics = self.metrics.write();
        metrics.total_operations += 1;
        
        match &event {
            CryptoEvent::KeyGenCompleted { duration, .. } => {
                metrics.key_generations += 1;
                self.update_avg_duration(&mut metrics, duration);
            }
            CryptoEvent::EncryptionPerformed { duration, .. } => {
                metrics.encryptions += 1;
                self.update_avg_duration(&mut metrics, duration);
            }
            CryptoEvent::DecryptionPerformed { duration, .. } => {
                metrics.decryptions += 1;
                self.update_avg_duration(&mut metrics, duration);
            }
            CryptoEvent::SigningPerformed { duration, .. } => {
                metrics.signatures += 1;
                self.update_avg_duration(&mut metrics, duration);
            }
            CryptoEvent::VerificationPerformed { duration, success, .. } => {
                metrics.verifications += 1;
                if !success {
                    metrics.failed_operations += 1;
                }
                self.update_avg_duration(&mut metrics, duration);
            }
            CryptoEvent::ErrorOccurred { .. } => {
                metrics.failed_operations += 1;
            }
            _ => {}
        }
        
        #[cfg(feature = "observability")]
        {
            // Emit event through Substrates
            if let Err(e) = self.event_source.emit(event) {
                tracing::warn!("Failed to emit crypto event: {}", e);
            }
        }
        
        #[cfg(feature = "tracing")]
        match event {
            CryptoEvent::KeyGenStarted { algorithm } => {
                tracing::debug!("Key generation started: {}", algorithm);
            }
            CryptoEvent::ErrorOccurred { operation, error } => {
                tracing::error!("Crypto operation failed: {} - {}", operation, error);
            }
            _ => {
                tracing::trace!("Crypto event: {:?}", event);
            }
        }
    }
    
    /// Start timing an operation
    pub fn start_operation(&self, operation: &str) -> OperationTimer {
        OperationTimer {
            operation: operation.to_string(),
            start: Instant::now(),
            observability: self,
        }
    }
    
    /// Get current metrics
    pub fn get_metrics(&self) -> CryptoMetrics {
        self.metrics.read().clone()
    }
    
    fn update_avg_duration(&self, metrics: &mut CryptoMetrics, duration: &Duration) {
        let new_duration_ms = duration.as_secs_f64() * 1000.0;
        if metrics.total_operations == 1 {
            metrics.avg_duration_ms = new_duration_ms;
        } else {
            let total = metrics.avg_duration_ms * (metrics.total_operations - 1) as f64;
            metrics.avg_duration_ms = (total + new_duration_ms) / metrics.total_operations as f64;
        }
    }
}

impl Default for CryptoMetrics {
    fn default() -> Self {
        Self {
            total_operations: 0,
            failed_operations: 0,
            avg_duration_ms: 0.0,
            key_generations: 0,
            encryptions: 0,
            decryptions: 0,
            signatures: 0,
            verifications: 0,
        }
    }
}

/// Timer for measuring operation duration
pub struct OperationTimer<'a> {
    operation: String,
    start: Instant,
    observability: &'a CryptoObservability,
}

impl<'a> OperationTimer<'a> {
    /// Complete the operation successfully
    pub fn complete(self, event_type: impl FnOnce(Duration) -> CryptoEvent) {
        let duration = self.start.elapsed();
        self.observability.record_operation(event_type(duration));
    }
    
    /// Mark the operation as failed
    pub fn failed(self, error: String) {
        self.observability.record_operation(CryptoEvent::ErrorOccurred {
            operation: self.operation,
            error,
        });
    }
}

/// Global observability instance for the crypto module
#[cfg(feature = "observability")]
lazy_static::lazy_static! {
    pub static ref CRYPTO_OBSERVABILITY: CryptoObservability = CryptoObservability::new("global");
}

#[cfg(not(feature = "observability"))]
pub fn get_observability() -> CryptoObservability {
    CryptoObservability::new("global")
}