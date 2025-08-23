//! Compression layer implementations

use crate::{error::Result, traits::Storage, CompressionConfig, StorageError};
use async_trait::async_trait;
use bytes::Bytes;
use std::sync::Arc;

#[cfg(feature = "lz4")]
pub mod lz4;

#[cfg(feature = "zstd")]
pub mod zstd;

#[cfg(feature = "snap")]
pub mod snappy;

pub mod adaptive;

/// Compression layer that wraps a storage backend
pub struct CompressionLayer<S: Storage + ?Sized> {
    inner: Arc<S>,
    compressor: Arc<dyn Compressor>,
    config: CompressionConfig,
}

impl<S: Storage + ?Sized> CompressionLayer<S> {
    /// Create a new compression layer
    pub fn new(inner: Arc<S>, config: CompressionConfig) -> Result<Self> {
        let compressor: Arc<dyn Compressor> = match config.algorithm {
            #[cfg(feature = "lz4")]
            crate::config::CompressionAlgorithm::Lz4 => {
                Arc::new(lz4::Lz4Compressor::new(config.level))
            }
            #[cfg(feature = "zstd")]
            crate::config::CompressionAlgorithm::Zstd => {
                Arc::new(zstd::ZstdCompressor::new(config.level as i32))
            }
            crate::config::CompressionAlgorithm::None => Arc::new(NoopCompressor),
            _ => return Err(StorageError::Config("Unsupported compression algorithm".to_string())),
        };

        Ok(Self {
            inner,
            compressor,
            config,
        })
    }
}

#[async_trait]
impl<S: Storage + ?Sized> Storage for CompressionLayer<S> {
    type Error = StorageError;

    async fn get(&self, key: &[u8]) -> Result<Option<Bytes>> {
        let compressed = self.inner.get(key).await.map_err(|_| StorageError::Backend(
            crate::error::BackendError::Other("Backend get failed".to_string())
        ))?;

        match compressed {
            Some(data) => {
                if self.config.enabled && data.len() > 0 {
                    let decompressed = self.compressor.decompress(&data)?;
                    Ok(Some(decompressed))
                } else {
                    Ok(Some(data))
                }
            }
            None => Ok(None),
        }
    }

    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let data = if self.config.enabled && value.len() >= self.config.min_size {
            self.compressor.compress(value)?
        } else {
            Bytes::copy_from_slice(value)
        };

        self.inner.put(key, &data).await.map_err(|_| StorageError::Backend(
            crate::error::BackendError::Other("Backend put failed".to_string())
        ))
    }

    async fn delete(&self, key: &[u8]) -> Result<()> {
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

/// Trait for compression implementations
trait Compressor: Send + Sync {
    fn compress(&self, data: &[u8]) -> Result<Bytes>;
    fn decompress(&self, data: &[u8]) -> Result<Bytes>;
}

/// No-op compressor for when compression is disabled
struct NoopCompressor;

impl Compressor for NoopCompressor {
    fn compress(&self, data: &[u8]) -> Result<Bytes> {
        Ok(Bytes::copy_from_slice(data))
    }

    fn decompress(&self, data: &[u8]) -> Result<Bytes> {
        Ok(Bytes::copy_from_slice(data))
    }
}