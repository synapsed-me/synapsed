//! Storage WASM operations

#[cfg(feature = "storage-modules")]
use synapsed_storage::*;

use crate::error::{WasmError, WasmResult};
use crate::types::{HostFunction, WasmValue};
use std::collections::HashMap;
use std::sync::Arc;

/// Create storage host functions
pub fn create_storage_host_functions() -> HashMap<String, HostFunction> {
    let mut functions = HashMap::new();

    // Key-value storage
    functions.insert(
        "storage_set".to_string(),
        Arc::new(|args| {
            if let (Some(WasmValue::String(key)), Some(WasmValue::Bytes(value))) = 
                (args.get(0), args.get(1)) {
                // In a real implementation, this would use synapsed-storage
                tracing::info!("Storage set: {} -> {} bytes", key, value.len());
                Ok(vec![WasmValue::Bool(true)])
            } else {
                Err(WasmError::Storage("Invalid arguments".to_string()))
            }
        }) as HostFunction,
    );

    functions.insert(
        "storage_get".to_string(),
        Arc::new(|args| {
            if let Some(WasmValue::String(key)) = args.get(0) {
                // In a real implementation, this would retrieve from synapsed-storage
                tracing::info!("Storage get: {}", key);
                // Return empty bytes as placeholder
                Ok(vec![WasmValue::Bytes(vec![])])
            } else {
                Err(WasmError::Storage("Key required".to_string()))
            }
        }) as HostFunction,
    );

    functions
}

/// WASM-compatible storage operations
pub struct WasmStorage {
    /// In-memory storage for demonstration
    data: HashMap<String, Vec<u8>>,
}

impl WasmStorage {
    /// Create new WASM storage
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Set key-value pair
    pub fn set(&mut self, key: String, value: Vec<u8>) -> WasmResult<()> {
        self.data.insert(key, value);
        Ok(())
    }

    /// Get value by key
    pub fn get(&self, key: &str) -> WasmResult<Option<Vec<u8>>> {
        Ok(self.data.get(key).cloned())
    }

    /// Delete key
    pub fn delete(&mut self, key: &str) -> WasmResult<bool> {
        Ok(self.data.remove(key).is_some())
    }

    /// List all keys
    pub fn keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }

    /// Get storage size
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

impl Default for WasmStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_storage() {
        let mut storage = WasmStorage::new();
        
        // Test set/get
        storage.set("key1".to_string(), b"value1".to_vec()).unwrap();
        let value = storage.get("key1").unwrap().unwrap();
        assert_eq!(value, b"value1");

        // Test delete
        assert!(storage.delete("key1").unwrap());
        assert!(!storage.delete("key1").unwrap()); // Already deleted
        assert!(storage.get("key1").unwrap().is_none());

        // Test size
        storage.set("key2".to_string(), b"value2".to_vec()).unwrap();
        storage.set("key3".to_string(), b"value3".to_vec()).unwrap();
        assert_eq!(storage.size(), 2);

        // Test keys
        let keys = storage.keys();
        assert!(keys.contains(&"key2".to_string()));
        assert!(keys.contains(&"key3".to_string()));
    }

    #[test]
    fn test_storage_host_functions() {
        let functions = create_storage_host_functions();
        assert!(functions.contains_key("storage_set"));
        assert!(functions.contains_key("storage_get"));
    }
}