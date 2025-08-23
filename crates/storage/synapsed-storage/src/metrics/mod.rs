//! Metrics collection and monitoring

use crate::{error::Result, traits::Storage, StorageError};
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable detailed metrics collection
    pub detailed: bool,
    /// Export metrics to Prometheus
    pub prometheus_export: bool,
    /// Metrics reporting interval in seconds
    pub report_interval_secs: u64,
}

/// Metrics collection layer
pub struct MetricsLayer<S: Storage + ?Sized> {
    inner: Arc<S>,
    config: MetricsConfig,
    get_count: AtomicU64,
    put_count: AtomicU64,
    delete_count: AtomicU64,
    get_latency_us: AtomicU64,
    put_latency_us: AtomicU64,
}

impl<S: Storage + ?Sized> MetricsLayer<S> {
    /// Create a new metrics layer
    pub fn new(inner: Arc<S>, config: MetricsConfig) -> Self {
        Self {
            inner,
            config,
            get_count: AtomicU64::new(0),
            put_count: AtomicU64::new(0),
            delete_count: AtomicU64::new(0),
            get_latency_us: AtomicU64::new(0),
            put_latency_us: AtomicU64::new(0),
        }
    }
}

#[async_trait]
impl<S: Storage + ?Sized> Storage for MetricsLayer<S> {
    type Error = StorageError;

    async fn get(&self, key: &[u8]) -> Result<Option<Bytes>> {
        let start = std::time::Instant::now();
        let result = self.inner.get(key).await.map_err(|_| StorageError::Backend(
            crate::error::BackendError::Other("Backend get failed".to_string())
        ));
        
        let duration = start.elapsed();
        self.get_count.fetch_add(1, Ordering::Relaxed);
        self.get_latency_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
        
        result
    }

    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let start = std::time::Instant::now();
        let result = self.inner.put(key, value).await.map_err(|_| StorageError::Backend(
            crate::error::BackendError::Other("Backend put failed".to_string())
        ));
        
        let duration = start.elapsed();
        self.put_count.fetch_add(1, Ordering::Relaxed);
        self.put_latency_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
        
        result
    }

    async fn delete(&self, key: &[u8]) -> Result<()> {
        self.delete_count.fetch_add(1, Ordering::Relaxed);
        self.inner.delete(key).await.map_err(|_| StorageError::Backend(
            crate::error::BackendError::Other("Backend delete failed".to_string())
        ))
    }
    
    async fn list(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>> {
        self.inner.list(prefix).await.map_err(|_| StorageError::Backend(
            crate::error::BackendError::Other("Backend list failed".to_string())
        ))
    }
}

pub mod collector;