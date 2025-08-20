//! Host function management and registry for P2P platform
//!
//! This module provides P2P-specific host functions for WebRTC, CRDT, sync,
//! cryptographic operations, DID management, and PWA integration optimized
//! for browser execution environments.

use std::collections::HashMap;
use std::sync::Arc;
use wasmtime::{Caller, Linker};

use crate::error::{WasmError, WasmResult};
use crate::types::{ExecutionContext, HostFunction, WasmValue};

// Import P2P module functions
#[cfg(feature = "webrtc-modules")]
use crate::p2p::create_webrtc_host_functions;

#[cfg(feature = "crdt-modules")]
use crate::crdt::create_crdt_host_functions;

#[cfg(feature = "sync-modules")]
use crate::sync::create_sync_host_functions;

#[cfg(feature = "crypto-modules")]
use crate::crypto::create_crypto_host_functions;

#[cfg(feature = "zkp-modules")]
use crate::zkp::create_zkp_host_functions;

#[cfg(feature = "did-modules")]
use crate::did::create_did_host_functions;

#[cfg(feature = "service-worker")]
use crate::pwa::create_pwa_host_functions;

/// Host function manager for registering and managing host functions
pub struct HostFunctionManager {
    /// Registry of host functions
    functions: HashMap<String, HostFunction>,
    /// Module-specific host functions
    module_functions: HashMap<String, HashMap<String, HostFunction>>,
}

impl HostFunctionManager {
    /// Create a new host function manager
    pub fn new() -> Self {
        let mut manager = Self {
            functions: HashMap::new(),
            module_functions: HashMap::new(),
        };
        
        // Register default host functions
        manager.register_default_functions();
        
        // Register P2P-specific host functions
        manager.register_p2p_functions();
        
        manager
    }

    /// Register a global host function
    pub fn register_function<F>(&mut self, name: String, function: F) -> WasmResult<()>
    where
        F: Fn(&[WasmValue]) -> WasmResult<Vec<WasmValue>> + Send + Sync + 'static,
    {
        self.functions.insert(name.clone(), Arc::new(function));
        tracing::debug!("Registered global host function: {}", name);
        Ok(())
    }

    /// Register a module-specific host function
    pub fn register_module_function<F>(
        &mut self,
        module_name: String,
        function_name: String,
        function: F,
    ) -> WasmResult<()>
    where
        F: Fn(&[WasmValue]) -> WasmResult<Vec<WasmValue>> + Send + Sync + 'static,
    {
        self.module_functions
            .entry(module_name.clone())
            .or_default()
            .insert(function_name.clone(), Arc::new(function));
        
        tracing::debug!(
            "Registered module-specific host function: {}::{}",
            module_name,
            function_name
        );
        Ok(())
    }

    /// Add host functions to a linker
    pub fn add_to_linker(
        &self,
        linker: &mut Linker<ExecutionContext>,
        module_name: Option<&str>,
    ) -> WasmResult<()> {
        // Add global host functions
        for (name, func) in &self.functions {
            self.add_function_to_linker(linker, "env", name, func.clone())?;
        }

        // Add module-specific functions if module name is provided
        if let Some(module) = module_name {
            if let Some(module_funcs) = self.module_functions.get(module) {
                for (name, func) in module_funcs {
                    self.add_function_to_linker(linker, module, name, func.clone())?;
                }
            }
        }

        Ok(())
    }

    /// Add a single function to the linker
    fn add_function_to_linker(
        &self,
        linker: &mut Linker<ExecutionContext>,
        namespace: &str,
        name: &str,
        func: HostFunction,
    ) -> WasmResult<()> {
        // Create a simplified wrapper for demonstration
        // In a real implementation, you'd need proper type conversion
        linker
            .func_wrap(
                namespace,
                name,
                move |_caller: Caller<'_, ExecutionContext>, arg: i32| -> i32 {
                    match func(&[WasmValue::I32(arg)]) {
                        Ok(results) => {
                            if let Some(WasmValue::I32(result)) = results.first() {
                                *result
                            } else {
                                0
                            }
                        }
                        Err(_) => 0, // Return 0 on error for simplicity
                    }
                },
            )
            .map_err(WasmError::from)?;

        Ok(())
    }

    /// Get list of registered global functions
    pub fn get_global_functions(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }

    /// Get list of module-specific functions
    pub fn get_module_functions(&self, module_name: &str) -> Vec<String> {
        self.module_functions
            .get(module_name)
            .map(|funcs| funcs.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Remove a global host function
    pub fn remove_function(&mut self, name: &str) -> bool {
        self.functions.remove(name).is_some()
    }

    /// Remove all module-specific functions for a module
    pub fn remove_module_functions(&mut self, module_name: &str) -> bool {
        self.module_functions.remove(module_name).is_some()
    }

    /// Clear all host functions
    pub fn clear(&mut self) {
        self.functions.clear();
        self.module_functions.clear();
    }

    /// Register P2P platform-specific host functions
    fn register_p2p_functions(&mut self) {
        #[cfg(feature = "webrtc-modules")]
        {
            let webrtc_functions = create_webrtc_host_functions();
            for (name, func) in webrtc_functions {
                let _ = self.register_function(name, move |args| func(args));
            }
        }

        #[cfg(feature = "crdt-modules")]
        {
            let crdt_functions = create_crdt_host_functions();
            for (name, func) in crdt_functions {
                let _ = self.register_function(name, move |args| func(args));
            }
        }

        #[cfg(feature = "sync-modules")]
        {
            let sync_functions = create_sync_host_functions();
            for (name, func) in sync_functions {
                let _ = self.register_function(name, move |args| func(args));
            }
        }

        #[cfg(feature = "crypto-modules")]
        {
            let crypto_functions = create_crypto_host_functions();
            for (name, func) in crypto_functions {
                let _ = self.register_function(name, move |args| func(args));
            }
        }

        #[cfg(feature = "zkp-modules")]
        {
            let zkp_functions = create_zkp_host_functions();
            for (name, func) in zkp_functions {
                let _ = self.register_function(name, move |args| func(args));
            }
        }

        #[cfg(feature = "did-modules")]
        {
            let did_functions = create_did_host_functions();
            for (name, func) in did_functions {
                let _ = self.register_function(name, move |args| func(args));
            }
        }

        #[cfg(feature = "service-worker")]
        {
            let pwa_functions = create_pwa_host_functions();
            for (name, func) in pwa_functions {
                let _ = self.register_function(name, move |args| func(args));
            }
        }

        tracing::info!("P2P host functions registered");
    }

    /// Register default/built-in host functions (browser-optimized)
    fn register_default_functions(&mut self) {
        // Logging function
        let _ = self.register_function(
            "log".to_string(),
            |args| {
                if let Some(WasmValue::I32(level)) = args.first() {
                    match level {
                        0 => tracing::error!("WASM: log level 0"),
                        1 => tracing::warn!("WASM: log level 1"),
                        2 => tracing::info!("WASM: log level 2"),
                        _ => tracing::debug!("WASM: log level {}", level),
                    }
                }
                Ok(vec![WasmValue::I32(0)])
            },
        );

        // Timestamp function
        let _ = self.register_function(
            "timestamp".to_string(),
            |_args| {
                let timestamp = chrono::Utc::now().timestamp();
                Ok(vec![WasmValue::I64(timestamp)])
            },
        );

        // Random number generator
        let _ = self.register_function(
            "random".to_string(),
            |_args| {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let random_val: u32 = rng.gen();
                Ok(vec![WasmValue::I32(random_val as i32)])
            },
        );

        // Memory allocation helper
        let _ = self.register_function(
            "alloc".to_string(),
            |args| {
                if let Some(WasmValue::I32(size)) = args.first() {
                    if *size > 0 && *size < 1_000_000 { // Basic safety check
                        // In a real implementation, this would interact with WASM linear memory
                        Ok(vec![WasmValue::I32(1024)]) // Mock pointer
                    } else {
                        Err(WasmError::MemoryAllocation("Invalid size".to_string()))
                    }
                } else {
                    Err(WasmError::MemoryAllocation("No size provided".to_string()))
                }
            },
        );

        // Memory deallocation helper
        let _ = self.register_function(
            "dealloc".to_string(),
            |args| {
                if let Some(WasmValue::I32(_ptr)) = args.first() {
                    // In a real implementation, this would free WASM linear memory
                    Ok(vec![WasmValue::I32(0)]) // Success
                } else {
                    Ok(vec![WasmValue::I32(-1)]) // Error
                }
            },
        );

        // Browser-safe environment access (limited)
        let _ = self.register_function(
            "get_user_agent".to_string(),
            |_args| {
                // In browser, would access navigator.userAgent
                Ok(vec![WasmValue::String("P2P-Browser-WASM/1.0".to_string())])
            },
        );
        
        // Browser performance timing
        let _ = self.register_function(
            "performance_now".to_string(),
            |_args| {
                // In browser, would use performance.now()
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as f64;
                Ok(vec![WasmValue::F64(now)])
            },
        );
        
        // Browser storage quota check
        let _ = self.register_function(
            "storage_quota".to_string(),
            |_args| {
                // Mock storage quota - in browser would use navigator.storage.estimate()
                Ok(vec![WasmValue::I64(500 * 1024 * 1024)]) // 500MB
            },
        );

        // Performance counter
        let _ = self.register_function(
            "perf_counter".to_string(),
            |_args| {
                let now = std::time::Instant::now();
                // Return nanoseconds since some arbitrary point
                Ok(vec![WasmValue::I64(now.elapsed().as_nanos() as i64)])
            },
        );
    }
}

impl Default for HostFunctionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Pre-defined host function categories
pub mod prelude {
    use super::*;

    /// Create logging host functions
    pub fn create_logging_functions() -> HashMap<String, HostFunction> {
        let mut functions = HashMap::new();

        functions.insert(
            "log_error".to_string(),
            Arc::new(|args: &[WasmValue]| {
                if let Some(WasmValue::String(msg)) = args.first() {
                    tracing::error!("WASM: {}", msg);
                }
                Ok(vec![])
            }) as HostFunction,
        );

        functions.insert(
            "log_warn".to_string(),
            Arc::new(|args: &[WasmValue]| {
                if let Some(WasmValue::String(msg)) = args.first() {
                    tracing::warn!("WASM: {}", msg);
                }
                Ok(vec![])
            }) as HostFunction,
        );

        functions.insert(
            "log_info".to_string(),
            Arc::new(|args: &[WasmValue]| {
                if let Some(WasmValue::String(msg)) = args.first() {
                    tracing::info!("WASM: {}", msg);
                }
                Ok(vec![])
            }) as HostFunction,
        );

        functions
    }

    /// Create P2P communication host functions
    pub fn create_p2p_communication_functions() -> HashMap<String, HostFunction> {
        let mut functions = HashMap::new();

        // Peer connection management
        functions.insert(
            "p2p_connect_peer".to_string(),
            Arc::new(|args: &[WasmValue]| {
                if let Some(WasmValue::String(peer_id)) = args.first() {
                    tracing::info!(peer_id = %peer_id, "Connecting to peer");
                    Ok(vec![WasmValue::I32(1)]) // Success
                } else {
                    Err(WasmError::HostFunction("Peer ID required".to_string()))
                }
            }) as HostFunction,
        );

        // Send P2P message
        functions.insert(
            "p2p_send_message".to_string(),
            Arc::new(|args: &[WasmValue]| {
                match (args.get(0), args.get(1)) {
                    (Some(WasmValue::String(peer_id)), Some(WasmValue::Bytes(data))) => {
                        tracing::debug!(
                            peer_id = %peer_id,
                            data_len = data.len(),
                            "Sending P2P message"
                        );
                        Ok(vec![WasmValue::I32(1)]) // Success
                    }
                    _ => Err(WasmError::HostFunction("Invalid P2P message arguments".to_string()))
                }
            }) as HostFunction,
        );

        // Get peer list
        functions.insert(
            "p2p_get_peers".to_string(),
            Arc::new(|_args: &[WasmValue]| {
                // Return mock peer list
                Ok(vec![WasmValue::String("peer1,peer2,peer3".to_string())])
            }) as HostFunction,
        );

        functions
    }

    /// Create browser-optimized crypto functions
    pub fn create_browser_crypto_functions() -> HashMap<String, HostFunction> {
        let mut functions = HashMap::new();

        // Web Crypto API SHA-256
        functions.insert(
            "webcrypto_hash_sha256".to_string(),
            Arc::new(|args: &[WasmValue]| {
                if let Some(WasmValue::Bytes(data)) = args.first() {
                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    hasher.update(data);
                    let result = hasher.finalize();
                    Ok(vec![WasmValue::Bytes(result.to_vec())])
                } else {
                    Err(WasmError::HostFunction("Invalid input data for webcrypto_hash_sha256".to_string()))
                }
            }) as HostFunction,
        );

        // Generate secure random bytes
        functions.insert(
            "webcrypto_random_bytes".to_string(),
            Arc::new(|args: &[WasmValue]| {
                if let Some(WasmValue::I32(len)) = args.first() {
                    if *len > 0 && *len <= 1024 {
                        let mut bytes = vec![0u8; *len as usize];
                        if getrandom::getrandom(&mut bytes).is_ok() {
                            Ok(vec![WasmValue::Bytes(bytes)])
                        } else {
                            Err(WasmError::HostFunction("Random generation failed".to_string()))
                        }
                    } else {
                        Err(WasmError::HostFunction("Invalid length (1-1024 bytes)".to_string()))
                    }
                } else {
                    Err(WasmError::HostFunction("Length required".to_string()))
                }
            }) as HostFunction,
        );

        functions
    }

    /// Create real-time collaboration functions
    pub fn create_collaboration_functions() -> HashMap<String, HostFunction> {
        let mut functions = HashMap::new();

        // Create collaborative document
        functions.insert(
            "collab_create_document".to_string(),
            Arc::new(|args: &[WasmValue]| {
                if let Some(WasmValue::String(doc_id)) = args.first() {
                    tracing::info!(doc_id = %doc_id, "Creating collaborative document");
                    Ok(vec![WasmValue::String(doc_id.clone())])
                } else {
                    Err(WasmError::HostFunction("Document ID required".to_string()))
                }
            }) as HostFunction,
        );

        // Apply collaborative operation
        functions.insert(
            "collab_apply_operation".to_string(),
            Arc::new(|args: &[WasmValue]| {
                match (args.get(0), args.get(1), args.get(2)) {
                    (Some(WasmValue::String(doc_id)), 
                     Some(WasmValue::String(operation)), 
                     Some(WasmValue::Bytes(data))) => {
                        tracing::debug!(
                            doc_id = %doc_id,
                            operation = %operation,
                            data_len = data.len(),
                            "Applying collaborative operation"
                        );
                        Ok(vec![WasmValue::I32(1)]) // Success
                    }
                    _ => Err(WasmError::HostFunction("Invalid collaboration arguments".to_string()))
                }
            }) as HostFunction,
        );

        // Sync document state
        functions.insert(
            "collab_sync_document".to_string(),
            Arc::new(|args: &[WasmValue]| {
                if let Some(WasmValue::String(doc_id)) = args.first() {
                    tracing::debug!(doc_id = %doc_id, "Syncing collaborative document");
                    Ok(vec![WasmValue::Bytes(b"sync_delta".to_vec())])
                } else {
                    Err(WasmError::HostFunction("Document ID required".to_string()))
                }
            }) as HostFunction,
        );

        functions
    }

    /// Create browser storage functions (IndexedDB/localStorage compatible)
    pub fn create_browser_storage_functions() -> HashMap<String, HostFunction> {
        let mut functions = HashMap::new();

        // Store data in browser storage
        functions.insert(
            "storage_set_item".to_string(),
            Arc::new(|args: &[WasmValue]| {
                match (args.get(0), args.get(1)) {
                    (Some(WasmValue::String(key)), Some(WasmValue::Bytes(data))) => {
                        tracing::debug!(
                            key = %key,
                            data_len = data.len(),
                            "Storing data in browser storage"
                        );
                        Ok(vec![WasmValue::I32(1)]) // Success
                    }
                    _ => Err(WasmError::HostFunction("Invalid storage arguments".to_string()))
                }
            }) as HostFunction,
        );

        // Retrieve data from browser storage
        functions.insert(
            "storage_get_item".to_string(),
            Arc::new(|args: &[WasmValue]| {
                if let Some(WasmValue::String(key)) = args.first() {
                    tracing::debug!(key = %key, "Retrieving data from browser storage");
                    // Return mock data
                    Ok(vec![WasmValue::Bytes(b"stored_data".to_vec())])
                } else {
                    Err(WasmError::HostFunction("Storage key required".to_string()))
                }
            }) as HostFunction,
        );

        // Remove data from browser storage
        functions.insert(
            "storage_remove_item".to_string(),
            Arc::new(|args: &[WasmValue]| {
                if let Some(WasmValue::String(key)) = args.first() {
                    tracing::debug!(key = %key, "Removing data from browser storage");
                    Ok(vec![WasmValue::I32(1)]) // Success
                } else {
                    Err(WasmError::HostFunction("Storage key required".to_string()))
                }
            }) as HostFunction,
        );

        // Get storage quota information
        functions.insert(
            "storage_get_quota".to_string(),
            Arc::new(|_args: &[WasmValue]| {
                // Return mock quota info: used_bytes, available_bytes
                Ok(vec![
                    WasmValue::I64(50 * 1024 * 1024),  // 50MB used
                    WasmValue::I64(450 * 1024 * 1024), // 450MB available
                ])
            }) as HostFunction,
        );

        functions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_function_manager_creation() {
        let manager = HostFunctionManager::new();
        let global_functions = manager.get_global_functions();
        
        // Should have default functions
        assert!(!global_functions.is_empty());
        assert!(global_functions.contains(&"log".to_string()));
        assert!(global_functions.contains(&"timestamp".to_string()));
    }

    #[test]
    fn test_function_registration() {
        let mut manager = HostFunctionManager::new();
        
        let result = manager.register_function(
            "test_func".to_string(),
            |args| {
                if let Some(WasmValue::I32(val)) = args.first() {
                    Ok(vec![WasmValue::I32(val * 2)])
                } else {
                    Ok(vec![WasmValue::I32(0)])
                }
            },
        );
        
        assert!(result.is_ok());
        assert!(manager.get_global_functions().contains(&"test_func".to_string()));
    }

    #[test]
    fn test_module_specific_functions() {
        let mut manager = HostFunctionManager::new();
        
        let result = manager.register_module_function(
            "test_module".to_string(),
            "module_func".to_string(),
            |_args| Ok(vec![WasmValue::I32(42)]),
        );
        
        assert!(result.is_ok());
        assert!(manager
            .get_module_functions("test_module")
            .contains(&"module_func".to_string()));
    }

    #[test]
    fn test_function_removal() {
        let mut manager = HostFunctionManager::new();
        
        // Add a function
        manager.register_function(
            "temp_func".to_string(),
            |_args| Ok(vec![WasmValue::I32(1)]),
        ).unwrap();
        
        assert!(manager.get_global_functions().contains(&"temp_func".to_string()));
        
        // Remove it
        assert!(manager.remove_function("temp_func"));
        assert!(!manager.get_global_functions().contains(&"temp_func".to_string()));
        
        // Try to remove non-existent function
        assert!(!manager.remove_function("nonexistent"));
    }

    #[test]
    fn test_prelude_functions() {
        let logging_funcs = prelude::create_logging_functions();
        assert!(!logging_funcs.is_empty());
        assert!(logging_funcs.contains_key("log_error"));
        assert!(logging_funcs.contains_key("log_warn"));
        assert!(logging_funcs.contains_key("log_info"));

        let p2p_funcs = prelude::create_p2p_communication_functions();
        assert!(!p2p_funcs.is_empty());
        assert!(p2p_funcs.contains_key("p2p_connect_peer"));
        assert!(p2p_funcs.contains_key("p2p_send_message"));

        let crypto_funcs = prelude::create_browser_crypto_functions();
        assert!(!crypto_funcs.is_empty());
        assert!(crypto_funcs.contains_key("webcrypto_hash_sha256"));
        assert!(crypto_funcs.contains_key("webcrypto_random_bytes"));

        let storage_funcs = prelude::create_browser_storage_functions();
        assert!(!storage_funcs.is_empty());
        assert!(storage_funcs.contains_key("storage_set_item"));
        assert!(storage_funcs.contains_key("storage_get_item"));
        
        let collab_funcs = prelude::create_collaboration_functions();
        assert!(!collab_funcs.is_empty());
        assert!(collab_funcs.contains_key("collab_create_document"));
        assert!(collab_funcs.contains_key("collab_apply_operation"));
    }
}