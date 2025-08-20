//! Serialization utilities and abstractions for the Synapsed ecosystem.
//!
//! This module provides common serialization patterns and utilities that can be
//! used across all Synapsed components for consistent data encoding/decoding.

use crate::{SynapsedError, SynapsedResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Serialization format enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SerializationFormat {
    /// JSON format
    Json,
    /// Binary format using bincode
    Binary,
    /// `MessagePack` format
    MessagePack,
    /// CBOR format
    Cbor,
    /// Custom format
    Custom(&'static str),
}

impl SerializationFormat {
    /// Get the MIME type for this format
    #[must_use] pub fn mime_type(&self) -> &'static str {
        match self {
            SerializationFormat::Json => "application/json",
            SerializationFormat::Binary => "application/octet-stream",
            SerializationFormat::MessagePack => "application/msgpack",
            SerializationFormat::Cbor => "application/cbor",
            SerializationFormat::Custom(name) => name,
        }
    }

    /// Get the file extension for this format
    #[must_use] pub fn file_extension(&self) -> &'static str {
        match self {
            SerializationFormat::Json => "json",
            SerializationFormat::Binary => "bin",
            SerializationFormat::MessagePack => "msgpack",
            SerializationFormat::Cbor => "cbor",
            SerializationFormat::Custom(_) => "dat",
        }
    }
}

impl std::fmt::Display for SerializationFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializationFormat::Json => write!(f, "JSON"),
            SerializationFormat::Binary => write!(f, "Binary"),
            SerializationFormat::MessagePack => write!(f, "MessagePack"),
            SerializationFormat::Cbor => write!(f, "CBOR"),
            SerializationFormat::Custom(name) => write!(f, "{name}"),
        }
    }
}

/// Trait for serializable data with format support
#[async_trait::async_trait]
pub trait FormatSerializer<T>: Send + Sync 
where
    T: Send + Sync + 'static,
{
    /// Serialize data to bytes using the specified format
    async fn serialize(&self, data: &T, format: SerializationFormat) -> SynapsedResult<Vec<u8>>;

    /// Deserialize data from bytes using the specified format
    async fn deserialize(&self, data: &[u8], format: SerializationFormat) -> SynapsedResult<T>;

    /// Get supported formats
    fn supported_formats(&self) -> Vec<SerializationFormat>;

    /// Get the default format
    fn default_format(&self) -> SerializationFormat {
        SerializationFormat::Json
    }
}

/// Default serializer implementation for types that implement Serialize/Deserialize
pub struct DefaultSerializer<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> DefaultSerializer<T> {
    /// Create a new default serializer
    #[must_use] pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T> Default for DefaultSerializer<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl<T> FormatSerializer<T> for DefaultSerializer<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    async fn serialize(&self, data: &T, format: SerializationFormat) -> SynapsedResult<Vec<u8>> {
        // Run CPU-intensive serialization in a blocking task
        let data_clone = serde_json::to_value(data)?; // First convert to Value for thread safety
        tokio::task::spawn_blocking(move || {
            match format {
                SerializationFormat::Json => {
                    serde_json::to_vec(&data_clone).map_err(SynapsedError::from)
                }
                SerializationFormat::Binary => {
                    bincode::serialize(&data_clone).map_err(SynapsedError::from)
                }
                SerializationFormat::MessagePack => {
                    Err(SynapsedError::serialization("MessagePack format not supported in default serializer"))
                }
                SerializationFormat::Cbor => {
                    Err(SynapsedError::serialization("CBOR format not supported in default serializer"))
                }
                SerializationFormat::Custom(name) => {
                    Err(SynapsedError::serialization(format!("Custom format '{name}' not supported")))
                }
            }
        }).await.map_err(|e| SynapsedError::internal(format!("Serialization task failed: {e}")))?
    }

    async fn deserialize(&self, data: &[u8], format: SerializationFormat) -> SynapsedResult<T> {
        // Clone data for thread safety
        let data = data.to_vec();
        tokio::task::spawn_blocking(move || {
            match format {
                SerializationFormat::Json => {
                    serde_json::from_slice(&data).map_err(SynapsedError::from)
                }
                SerializationFormat::Binary => {
                    bincode::deserialize(&data).map_err(SynapsedError::from)
                }
                SerializationFormat::MessagePack => {
                    Err(SynapsedError::serialization("MessagePack format not supported in default serializer"))
                }
                SerializationFormat::Cbor => {
                    Err(SynapsedError::serialization("CBOR format not supported in default serializer"))
                }
                SerializationFormat::Custom(name) => {
                    Err(SynapsedError::serialization(format!("Custom format '{name}' not supported")))
                }
            }
        }).await.map_err(|e| SynapsedError::internal(format!("Deserialization task failed: {e}")))?
    }

    fn supported_formats(&self) -> Vec<SerializationFormat> {
        vec![SerializationFormat::Json, SerializationFormat::Binary]
    }
}

/// Serialization context with metadata
#[derive(Debug, Clone)]
pub struct SerializationContext {
    /// Format used for serialization
    pub format: SerializationFormat,
    /// Schema version
    pub schema_version: u32,
    /// Compression used (if any)
    pub compression: Option<CompressionType>,
    /// Encoding metadata
    pub metadata: HashMap<String, String>,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl SerializationContext {
    /// Create a new serialization context
    #[must_use] pub fn new(format: SerializationFormat) -> Self {
        Self {
            format,
            schema_version: 1,
            compression: None,
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
        }
    }

    /// Set schema version
    #[must_use] pub fn with_schema_version(mut self, version: u32) -> Self {
        self.schema_version = version;
        self
    }

    /// Set compression type
    #[must_use] pub fn with_compression(mut self, compression: CompressionType) -> Self {
        self.compression = Some(compression);
        self
    }

    /// Add metadata
    pub fn with_metadata<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Compression types supported
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionType {
    /// No compression
    None,
    /// Gzip compression
    Gzip,
    /// LZ4 compression
    Lz4,
    /// Zstd compression
    Zstd,
}

impl std::fmt::Display for CompressionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressionType::None => write!(f, "None"),
            CompressionType::Gzip => write!(f, "Gzip"),
            CompressionType::Lz4 => write!(f, "LZ4"),
            CompressionType::Zstd => write!(f, "Zstd"),
        }
    }
}

/// Serialized data container with context
#[derive(Debug, Clone)]
pub struct SerializedData {
    /// Serialization context
    pub context: SerializationContext,
    /// Serialized data bytes
    pub data: Vec<u8>,
    /// Data checksum for integrity
    pub checksum: Option<String>,
}

impl SerializedData {
    /// Create new serialized data
    #[must_use] pub fn new(context: SerializationContext, data: Vec<u8>) -> Self {
        Self {
            context,
            data,
            checksum: None,
        }
    }

    /// Add checksum for integrity verification
    #[must_use] pub fn with_checksum(mut self, checksum: String) -> Self {
        self.checksum = Some(checksum);
        self
    }

    /// Verify checksum if present
    pub fn verify_checksum(&self) -> SynapsedResult<bool> {
        match &self.checksum {
            Some(expected) => {
                let actual = utils::calculate_checksum(&self.data);
                Ok(actual == *expected)
            }
            None => Ok(true), // No checksum to verify
        }
    }

    /// Get data size
    #[must_use] pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Trait for versioned serialization support
#[async_trait::async_trait]
pub trait VersionedSerializer<T>: Send + Sync {
    /// Serialize with version information
    async fn serialize_versioned(&self, data: &T, version: u32) -> SynapsedResult<SerializedData>;

    /// Deserialize with automatic version migration
    async fn deserialize_versioned(&self, serialized: &SerializedData) -> SynapsedResult<T>;

    /// Get current schema version
    fn current_version(&self) -> u32;

    /// Check if a version is supported
    fn supports_version(&self, version: u32) -> bool;

    /// Migrate data from one version to another
    async fn migrate_version(&self, data: &[u8], from_version: u32, to_version: u32) -> SynapsedResult<Vec<u8>>;
}

/// Batch serialization for multiple items
pub struct BatchSerializer<T> {
    #[allow(dead_code)]
    serializer: Box<dyn FormatSerializer<T> + Send + Sync>,
    #[allow(dead_code)]
    format: SerializationFormat,
}

impl<T> BatchSerializer<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
{
    /// Create a new batch serializer
    #[must_use] pub fn new(format: SerializationFormat) -> Self {
        Self {
            serializer: Box::new(DefaultSerializer::new()),
            format,
        }
    }

    /// Create with custom serializer
    #[must_use] pub fn with_serializer(serializer: Box<dyn FormatSerializer<T> + Send + Sync>, format: SerializationFormat) -> Self {
        Self { serializer, format }
    }

    /// Serialize multiple items
    pub async fn serialize_batch(&self, items: &[T]) -> SynapsedResult<Vec<u8>> {
        // For batch serialization, we serialize the entire slice
        let items_clone = items.to_vec(); // Clone for thread safety
        let format = self.format;
        tokio::task::spawn_blocking(move || {
            match format {
                SerializationFormat::Json => {
                    serde_json::to_vec(&items_clone).map_err(SynapsedError::from)
                }
                SerializationFormat::Binary => {
                    bincode::serialize(&items_clone).map_err(SynapsedError::from)
                }
                _ => Err(SynapsedError::serialization("Unsupported format for batch serialization"))
            }
        }).await.map_err(|e| SynapsedError::internal(format!("Batch serialization task failed: {e}")))?
    }

    /// Deserialize multiple items
    pub async fn deserialize_batch(&self, data: &[u8]) -> SynapsedResult<Vec<T>> {
        let data = data.to_vec(); // Clone for thread safety
        let format = self.format;
        tokio::task::spawn_blocking(move || {
            match format {
                SerializationFormat::Json => {
                    serde_json::from_slice(&data).map_err(SynapsedError::from)
                }
                SerializationFormat::Binary => {
                    bincode::deserialize(&data).map_err(SynapsedError::from)
                }
                _ => Err(SynapsedError::serialization("Unsupported format for batch deserialization"))
            }
        }).await.map_err(|e| SynapsedError::internal(format!("Batch deserialization task failed: {e}")))?
    }
}

/// Stream serialization for large datasets
pub struct StreamSerializer<T> {
    #[allow(dead_code)]
    serializer: Box<dyn FormatSerializer<T> + Send + Sync>,
    #[allow(dead_code)]
    format: SerializationFormat,
    buffer_size: usize,
}

impl<T> StreamSerializer<T>
where
    T: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone + 'static,
{
    /// Create a new stream serializer
    #[must_use] pub fn new(format: SerializationFormat, buffer_size: usize) -> Self {
        Self {
            serializer: Box::new(DefaultSerializer::new()),
            format,
            buffer_size,
        }
    }

    /// Get buffer size
    #[must_use] pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }
}

/// Utility functions for serialization
pub mod utils {
    use super::{CompressionType, SynapsedResult, SynapsedError, Serialize, Deserialize, SerializationFormat};
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    /// Calculate checksum for data integrity
    #[must_use] pub fn calculate_checksum(data: &[u8]) -> String {
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Compress data using the specified compression type
    pub fn compress_data(data: &[u8], compression: CompressionType) -> SynapsedResult<Vec<u8>> {
        match compression {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => {
                // In a real implementation, you would use flate2 or similar
                Err(SynapsedError::serialization("Gzip compression not implemented"))
            }
            CompressionType::Lz4 => {
                // In a real implementation, you would use lz4_flex or similar
                Err(SynapsedError::serialization("LZ4 compression not implemented"))
            }
            CompressionType::Zstd => {
                // In a real implementation, you would use zstd or similar
                Err(SynapsedError::serialization("Zstd compression not implemented"))
            }
        }
    }

    /// Decompress data using the specified compression type
    pub fn decompress_data(data: &[u8], compression: CompressionType) -> SynapsedResult<Vec<u8>> {
        match compression {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => {
                Err(SynapsedError::serialization("Gzip decompression not implemented"))
            }
            CompressionType::Lz4 => {
                Err(SynapsedError::serialization("LZ4 decompression not implemented"))
            }
            CompressionType::Zstd => {
                Err(SynapsedError::serialization("Zstd decompression not implemented"))
            }
        }
    }

    /// Serialize data with automatic format detection
    pub fn serialize_auto<T>(data: &T) -> SynapsedResult<Vec<u8>>
    where
        T: Serialize,
    {
        // Default to JSON for automatic serialization
        serde_json::to_vec(data).map_err(SynapsedError::from)
    }

    /// Deserialize data with automatic format detection
    pub fn deserialize_auto<T>(data: &[u8]) -> SynapsedResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        // Try JSON first (most common)
        if let Ok(result) = serde_json::from_slice(data) {
            return Ok(result);
        }

        // Try binary format
        if let Ok(result) = bincode::deserialize(data) {
            return Ok(result);
        }

        Err(SynapsedError::serialization("Could not determine serialization format"))
    }

    /// Get estimated serialized size for a type
    pub fn estimate_serialized_size<T>(data: &T, format: SerializationFormat) -> SynapsedResult<usize>
    where
        T: Serialize,
    {
        match format {
            SerializationFormat::Json => {
                let serialized = serde_json::to_vec(data)?;
                Ok(serialized.len())
            }
            SerializationFormat::Binary => {
                let serialized = bincode::serialize(data).map_err(SynapsedError::from)?;
                Ok(serialized.len())
            }
            _ => Err(SynapsedError::serialization("Size estimation not supported for this format")),
        }
    }
}

/// Macro for easy serialization/deserialization with format specification
#[macro_export]
macro_rules! serialize_with_format {
    ($data:expr, $format:expr) => {
        match $format {
            $crate::serialization::SerializationFormat::Json => {
                serde_json::to_vec($data).map_err($crate::SynapsedError::from)
            }
            $crate::serialization::SerializationFormat::Binary => {
                bincode::serialize($data).map_err($crate::SynapsedError::from)
            }
            _ => Err($crate::SynapsedError::serialization("Unsupported format")),
        }
    };
}

/// Macro for easy deserialization with format specification
#[macro_export]
/// Utility macro for deserializing data with format detection
macro_rules! deserialize_with_format {
    ($data:expr, $format:expr, $type:ty) => {
        match $format {
            $crate::serialization::SerializationFormat::Json => {
                serde_json::from_slice::<$type>($data).map_err($crate::SynapsedError::from)
            }
            $crate::serialization::SerializationFormat::Binary => {
                bincode::deserialize::<$type>($data).map_err($crate::SynapsedError::from)
            }
            _ => Err($crate::SynapsedError::serialization("Unsupported format")),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestData {
        id: u32,
        name: String,
        values: Vec<f64>,
    }

    #[test]
    fn test_serialization_format() {
        assert_eq!(SerializationFormat::Json.mime_type(), "application/json");
        assert_eq!(SerializationFormat::Binary.file_extension(), "bin");
        assert_eq!(SerializationFormat::Json.to_string(), "JSON");
    }

    #[tokio::test]
    async fn test_default_serializer() {
        let serializer = DefaultSerializer::<TestData>::new();
        let data = TestData {
            id: 42,
            name: "test".to_string(),
            values: vec![1.0, 2.0, 3.0],
        };

        // Test JSON serialization
        let json_bytes = serializer.serialize(&data, SerializationFormat::Json).await.unwrap();
        let deserialized: TestData = serializer.deserialize(&json_bytes, SerializationFormat::Json).await.unwrap();
        assert_eq!(data, deserialized);

        // Test binary serialization
        let binary_bytes = serializer.serialize(&data, SerializationFormat::Binary).await.unwrap();
        let deserialized: TestData = serializer.deserialize(&binary_bytes, SerializationFormat::Binary).await.unwrap();
        assert_eq!(data, deserialized);
    }

    #[test]
    fn test_serialization_context() {
        let context = SerializationContext::new(SerializationFormat::Json)
            .with_schema_version(2)
            .with_compression(CompressionType::Gzip)
            .with_metadata("author", "test");

        assert_eq!(context.format, SerializationFormat::Json);
        assert_eq!(context.schema_version, 2);
        assert_eq!(context.compression, Some(CompressionType::Gzip));
        assert_eq!(context.metadata.get("author"), Some(&"test".to_string()));
    }

    #[test]
    fn test_serialized_data() {
        let context = SerializationContext::new(SerializationFormat::Json);
        let data = vec![1, 2, 3, 4, 5];
        let checksum = utils::calculate_checksum(&data);
        
        let serialized = SerializedData::new(context, data.clone())
            .with_checksum(checksum);

        assert_eq!(serialized.data, data);
        assert!(serialized.verify_checksum().unwrap());
        assert_eq!(serialized.size(), 5);
    }

    #[tokio::test]
    async fn test_batch_serializer() {
        let serializer = BatchSerializer::<TestData>::new(SerializationFormat::Json);
        let items = vec![
            TestData { id: 1, name: "first".to_string(), values: vec![1.0] },
            TestData { id: 2, name: "second".to_string(), values: vec![2.0] },
        ];

        let serialized = serializer.serialize_batch(&items).await.unwrap();
        let deserialized = serializer.deserialize_batch(&serialized).await.unwrap();
        
        assert_eq!(items, deserialized);
    }

    #[test]
    fn test_utils() {
        let data = b"hello world";
        let checksum1 = utils::calculate_checksum(data);
        let checksum2 = utils::calculate_checksum(data);
        assert_eq!(checksum1, checksum2);

        let test_data = TestData {
            id: 42,
            name: "test".to_string(),
            values: vec![1.0, 2.0],
        };

        let serialized = utils::serialize_auto(&test_data).unwrap();
        let deserialized: TestData = utils::deserialize_auto(&serialized).unwrap();
        assert_eq!(test_data, deserialized);

        let size = utils::estimate_serialized_size(&test_data, SerializationFormat::Json).unwrap();
        assert!(size > 0);
    }

    #[test]
    fn test_macros() {
        let data = TestData {
            id: 42,
            name: "test".to_string(),
            values: vec![1.0, 2.0],
        };

        let serialized = serialize_with_format!(&data, SerializationFormat::Json).unwrap();
        let deserialized: TestData = deserialize_with_format!(&serialized, SerializationFormat::Json, TestData).unwrap();
        
        assert_eq!(data, deserialized);
    }
}