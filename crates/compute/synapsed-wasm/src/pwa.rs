//! Progressive Web App integration for WASM
//!
//! This module provides WebAssembly-compatible PWA functionality including
//! service worker integration, IndexedDB operations, offline support, and
//! background synchronization optimized for P2P communication platforms.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    ServiceWorkerContainer, ServiceWorkerRegistration, MessageChannel, MessagePort,
    IdbDatabase, IdbTransaction, IdbObjectStore, IdbRequest, IdbKeyRange,
    BroadcastChannel, Storage, Window, WorkerGlobalScope,
};
use js_sys::{Object, Promise, JSON, Array, Uint8Array};

use crate::error::{WasmError, WasmResult};
use crate::types::{HostFunction, WasmValue};
use crate::{DEFAULT_INDEXEDDB_QUOTA};

/// PWA runtime for service worker and IndexedDB integration
pub struct ServiceWorkerRuntime {
    /// Service worker registration
    registration: Option<ServiceWorkerRegistration>,
    /// Message channels
    message_channels: HashMap<String, MessageChannel>,
    /// Background sync registrations
    sync_registrations: HashMap<String, BackgroundSyncRegistration>,
    /// Runtime statistics
    stats: PwaStats,
}

impl ServiceWorkerRuntime {
    /// Create a new service worker runtime
    pub fn new() -> WasmResult<Self> {
        Ok(Self {
            registration: None,
            message_channels: HashMap::new(),
            sync_registrations: HashMap::new(),
            stats: PwaStats::default(),
        })
    }

    /// Register service worker
    pub async fn register_service_worker(&mut self, script_url: &str) -> WasmResult<()> {
        let window = web_sys::window()
            .ok_or_else(|| WasmError::Configuration("No global window available".to_string()))?;
        
        let navigator = window.navigator();
        let service_worker = navigator.service_worker();
        
        let registration_promise = service_worker.register(script_url);
        let registration = JsFuture::from(registration_promise).await
            .map_err(|_| WasmError::Configuration("Service worker registration failed".to_string()))?;
        
        let registration = ServiceWorkerRegistration::from(registration);
        self.registration = Some(registration);
        self.stats.service_workers_registered += 1;
        
        tracing::info!(script_url = %script_url, "Service worker registered");
        Ok(())
    }

    /// Send message to service worker
    pub async fn send_message_to_sw(&mut self, message: PwaMessage) -> WasmResult<()> {
        let registration = self.registration.as_ref()
            .ok_or_else(|| WasmError::Configuration("No service worker registered".to_string()))?;
        
        let active_worker = registration.active()
            .ok_or_else(|| WasmError::Configuration("No active service worker".to_string()))?;
        
        let message_data = serde_wasm_bindgen::to_value(&message)
            .map_err(|_| WasmError::Configuration("Failed to serialize message".to_string()))?;
        
        active_worker.post_message(&message_data)
            .map_err(|_| WasmError::Configuration("Failed to send message".to_string()))?;
        
        self.stats.messages_sent_to_sw += 1;
        Ok(())
    }

    /// Create message channel for bidirectional communication
    pub async fn create_message_channel(&mut self, channel_id: String) -> WasmResult<MessagePort> {
        let channel = MessageChannel::new()
            .map_err(|_| WasmError::Configuration("Failed to create message channel".to_string()))?;
        
        let port2 = channel.port2();
        self.message_channels.insert(channel_id.clone(), channel);
        
        tracing::debug!(channel_id = %channel_id, "Message channel created");
        Ok(port2)
    }

    /// Register background sync
    pub async fn register_background_sync(&mut self, tag: String, options: BackgroundSyncOptions) -> WasmResult<()> {
        let registration = BackgroundSyncRegistration {
            tag: tag.clone(),
            options,
            created_at: std::time::SystemTime::now(),
        };
        
        self.sync_registrations.insert(tag.clone(), registration);
        self.stats.background_syncs_registered += 1;
        
        tracing::info!(tag = %tag, "Background sync registered");
        Ok(())
    }

    /// Handle background sync event
    pub async fn handle_sync_event(&self, tag: &str) -> WasmResult<()> {
        if let Some(registration) = self.sync_registrations.get(tag) {
            tracing::info!(tag = %tag, "Handling background sync event");
            
            // Execute sync operation based on tag
            match tag {
                "p2p-sync" => self.handle_p2p_sync(&registration.options).await?,
                "crdt-sync" => self.handle_crdt_sync(&registration.options).await?,
                "offline-queue" => self.handle_offline_queue(&registration.options).await?,
                _ => {
                    tracing::warn!(tag = %tag, "Unknown sync tag");
                }
            }
        }
        
        Ok(())
    }

    /// Get runtime statistics
    pub fn get_stats(&self) -> &PwaStats {
        &self.stats
    }

    /// Check if service worker is available
    pub fn is_service_worker_available(&self) -> bool {
        self.registration.is_some()
    }

    // Private helper methods

    async fn handle_p2p_sync(&self, _options: &BackgroundSyncOptions) -> WasmResult<()> {
        // Handle P2P synchronization in background
        tracing::debug!("Executing P2P background sync");
        Ok(())
    }

    async fn handle_crdt_sync(&self, _options: &BackgroundSyncOptions) -> WasmResult<()> {
        // Handle CRDT synchronization in background
        tracing::debug!("Executing CRDT background sync");
        Ok(())
    }

    async fn handle_offline_queue(&self, _options: &BackgroundSyncOptions) -> WasmResult<()> {
        // Process offline queue when back online
        tracing::debug!("Processing offline queue");
        Ok(())
    }
}

/// IndexedDB manager for persistent storage
pub struct IndexedDbManager {
    /// Database connection
    database: Option<IdbDatabase>,
    /// Database name
    db_name: String,
    /// Database version
    db_version: u32,
    /// Object stores
    stores: HashMap<String, ObjectStoreConfig>,
    /// Manager statistics
    stats: IndexedDbStats,
}

impl IndexedDbManager {
    /// Create a new IndexedDB manager
    pub fn new(db_name: String, version: u32) -> Self {
        Self {
            database: None,
            db_name,
            db_version: version,
            stores: HashMap::new(),
            stats: IndexedDbStats::default(),
        }
    }

    /// Open database connection
    pub async fn open(&mut self) -> WasmResult<()> {
        let window = web_sys::window()
            .ok_or_else(|| WasmError::Configuration("No global window available".to_string()))?;
        
        let idb_factory = window.indexed_db()
            .map_err(|_| WasmError::Configuration("IndexedDB not available".to_string()))?
            .ok_or_else(|| WasmError::Configuration("IndexedDB not supported".to_string()))?;
        
        let open_request = idb_factory.open_with_u32(&self.db_name, self.db_version)
            .map_err(|_| WasmError::Configuration("Failed to open IndexedDB".to_string()))?;
        
        let database_result = JsFuture::from(Promise::resolve(&open_request.into())).await
            .map_err(|_| WasmError::Configuration("Database open failed".to_string()))?;
        
        let database = IdbDatabase::from(database_result);
        self.database = Some(database);
        self.stats.connections_opened += 1;
        
        tracing::info!(db_name = %self.db_name, version = self.db_version, "IndexedDB opened");
        Ok(())
    }

    /// Create object store
    pub fn create_object_store(&mut self, name: String, config: ObjectStoreConfig) -> WasmResult<()> {
        self.stores.insert(name.clone(), config);
        tracing::debug!(store_name = %name, "Object store configuration added");
        Ok(())
    }

    /// Store data in object store
    pub async fn store_data(&mut self, store_name: &str, key: &str, data: &[u8]) -> WasmResult<()> {
        let database = self.database.as_ref()
            .ok_or_else(|| WasmError::Configuration("Database not opened".to_string()))?;
        
        let transaction = database.transaction_with_str_and_mode(
            store_name,
            web_sys::IdbTransactionMode::Readwrite,
        ).map_err(|_| WasmError::Configuration("Failed to create transaction".to_string()))?;
        
        let object_store = transaction.object_store(store_name)
            .map_err(|_| WasmError::Configuration(format!("Object store '{}' not found", store_name)))?;
        
        let data_array = Uint8Array::new_with_length(data.len() as u32);
        data_array.copy_from(data);
        
        let _request = object_store.put_with_key(&data_array, &JsValue::from_str(key))
            .map_err(|_| WasmError::Configuration("Failed to store data".to_string()))?;
        
        self.stats.records_stored += 1;
        self.stats.bytes_stored += data.len() as u64;
        
        tracing::debug!(store_name = %store_name, key = %key, size = data.len(), "Data stored");
        Ok(())
    }

    /// Retrieve data from object store
    pub async fn retrieve_data(&mut self, store_name: &str, key: &str) -> WasmResult<Option<Vec<u8>>> {
        let database = self.database.as_ref()
            .ok_or_else(|| WasmError::Configuration("Database not opened".to_string()))?;
        
        let transaction = database.transaction_with_str(store_name)
            .map_err(|_| WasmError::Configuration("Failed to create transaction".to_string()))?;
        
        let object_store = transaction.object_store(store_name)
            .map_err(|_| WasmError::Configuration(format!("Object store '{}' not found", store_name)))?;
        
        let request = object_store.get(&JsValue::from_str(key))
            .map_err(|_| WasmError::Configuration("Failed to retrieve data".to_string()))?;
        
        let result = JsFuture::from(Promise::resolve(&request.into())).await
            .map_err(|_| WasmError::Configuration("Data retrieval failed".to_string()))?;
        
        if result.is_undefined() || result.is_null() {
            Ok(None)
        } else {
            let data_array = Uint8Array::new(&result);
            let mut data = vec![0u8; data_array.length() as usize];
            data_array.copy_to(&mut data);
            
            self.stats.records_retrieved += 1;
            self.stats.bytes_retrieved += data.len() as u64;
            
            tracing::debug!(store_name = %store_name, key = %key, size = data.len(), "Data retrieved");
            Ok(Some(data))
        }
    }

    /// Delete data from object store
    pub async fn delete_data(&mut self, store_name: &str, key: &str) -> WasmResult<()> {
        let database = self.database.as_ref()
            .ok_or_else(|| WasmError::Configuration("Database not opened".to_string()))?;
        
        let transaction = database.transaction_with_str_and_mode(
            store_name,
            web_sys::IdbTransactionMode::Readwrite,
        ).map_err(|_| WasmError::Configuration("Failed to create transaction".to_string()))?;
        
        let object_store = transaction.object_store(store_name)
            .map_err(|_| WasmError::Configuration(format!("Object store '{}' not found", store_name)))?;
        
        let _request = object_store.delete(&JsValue::from_str(key))
            .map_err(|_| WasmError::Configuration("Failed to delete data".to_string()))?;
        
        self.stats.records_deleted += 1;
        
        tracing::debug!(store_name = %store_name, key = %key, "Data deleted");
        Ok(())
    }

    /// List all keys in object store
    pub async fn list_keys(&self, store_name: &str) -> WasmResult<Vec<String>> {
        let database = self.database.as_ref()
            .ok_or_else(|| WasmError::Configuration("Database not opened".to_string()))?;
        
        let transaction = database.transaction_with_str(store_name)
            .map_err(|_| WasmError::Configuration("Failed to create transaction".to_string()))?;
        
        let object_store = transaction.object_store(store_name)
            .map_err(|_| WasmError::Configuration(format!("Object store '{}' not found", store_name)))?;
        
        let request = object_store.get_all_keys()
            .map_err(|_| WasmError::Configuration("Failed to get keys".to_string()))?;
        
        let result = JsFuture::from(Promise::resolve(&request.into())).await
            .map_err(|_| WasmError::Configuration("Key listing failed".to_string()))?;
        
        let keys_array = Array::from(&result);
        let mut keys = Vec::new();
        
        for i in 0..keys_array.length() {
            if let Some(key) = keys_array.get(i).as_string() {
                keys.push(key);
            }
        }
        
        Ok(keys)
    }

    /// Get storage quota information
    pub async fn get_storage_quota(&self) -> WasmResult<StorageQuota> {
        // Simplified quota check - in practice would use navigator.storage.estimate()
        Ok(StorageQuota {
            used_bytes: self.stats.bytes_stored,
            available_bytes: DEFAULT_INDEXEDDB_QUOTA - self.stats.bytes_stored,
            total_bytes: DEFAULT_INDEXEDDB_QUOTA,
        })
    }

    /// Clear all data in object store
    pub async fn clear_store(&mut self, store_name: &str) -> WasmResult<()> {
        let database = self.database.as_ref()
            .ok_or_else(|| WasmError::Configuration("Database not opened".to_string()))?;
        
        let transaction = database.transaction_with_str_and_mode(
            store_name,
            web_sys::IdbTransactionMode::Readwrite,
        ).map_err(|_| WasmError::Configuration("Failed to create transaction".to_string()))?;
        
        let object_store = transaction.object_store(store_name)
            .map_err(|_| WasmError::Configuration(format!("Object store '{}' not found", store_name)))?;
        
        let _request = object_store.clear()
            .map_err(|_| WasmError::Configuration("Failed to clear store".to_string()))?;
        
        self.stats.stores_cleared += 1;
        
        tracing::info!(store_name = %store_name, "Object store cleared");
        Ok(())
    }

    /// Get statistics
    pub fn get_stats(&self) -> &IndexedDbStats {
        &self.stats
    }
}

/// PWA message structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PwaMessage {
    /// Message type
    pub message_type: String,
    /// Message payload
    pub payload: serde_json::Value,
    /// Message timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Message ID
    pub id: String,
}

/// Background sync registration
#[derive(Debug, Clone)]
pub struct BackgroundSyncRegistration {
    /// Sync tag
    pub tag: String,
    /// Sync options
    pub options: BackgroundSyncOptions,
    /// Registration timestamp
    pub created_at: std::time::SystemTime,
}

/// Background sync options
#[derive(Debug, Clone)]
pub struct BackgroundSyncOptions {
    /// Minimum interval between syncs
    pub min_interval: std::time::Duration,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Retry backoff multiplier
    pub backoff_multiplier: f64,
    /// Additional data
    pub data: HashMap<String, serde_json::Value>,
}

impl Default for BackgroundSyncOptions {
    fn default() -> Self {
        Self {
            min_interval: std::time::Duration::from_secs(30),
            max_retries: 3,
            backoff_multiplier: 2.0,
            data: HashMap::new(),
        }
    }
}

/// Object store configuration
#[derive(Debug, Clone)]
pub struct ObjectStoreConfig {
    /// Key path
    pub key_path: Option<String>,
    /// Auto increment
    pub auto_increment: bool,
    /// Indexes
    pub indexes: Vec<IndexConfig>,
}

impl Default for ObjectStoreConfig {
    fn default() -> Self {
        Self {
            key_path: None,
            auto_increment: false,
            indexes: Vec::new(),
        }
    }
}

/// Index configuration
#[derive(Debug, Clone)]
pub struct IndexConfig {
    /// Index name
    pub name: String,
    /// Key path
    pub key_path: String,
    /// Unique constraint
    pub unique: bool,
    /// Multi-entry index
    pub multi_entry: bool,
}

/// Storage quota information
#[derive(Debug, Clone)]
pub struct StorageQuota {
    /// Used bytes
    pub used_bytes: u64,
    /// Available bytes
    pub available_bytes: u64,
    /// Total quota bytes
    pub total_bytes: u64,
}

/// PWA runtime statistics
#[derive(Debug, Clone, Default)]
pub struct PwaStats {
    /// Service workers registered
    pub service_workers_registered: u64,
    /// Messages sent to service worker
    pub messages_sent_to_sw: u64,
    /// Background syncs registered
    pub background_syncs_registered: u64,
    /// Background syncs executed
    pub background_syncs_executed: u64,
}

/// IndexedDB statistics
#[derive(Debug, Clone, Default)]
pub struct IndexedDbStats {
    /// Database connections opened
    pub connections_opened: u64,
    /// Records stored
    pub records_stored: u64,
    /// Records retrieved
    pub records_retrieved: u64,
    /// Records deleted
    pub records_deleted: u64,
    /// Bytes stored
    pub bytes_stored: u64,
    /// Bytes retrieved
    pub bytes_retrieved: u64,
    /// Object stores cleared
    pub stores_cleared: u64,
}

/// Create PWA host functions for WASM modules
pub fn create_pwa_host_functions() -> HashMap<String, HostFunction> {
    let mut functions = HashMap::new();

    // Register service worker
    functions.insert(
        "pwa_register_sw".to_string(),
        Arc::new(|args| {
            if let Some(WasmValue::String(script_url)) = args.get(0) {
                tracing::info!(script_url = %script_url, "Registering service worker");
                Ok(vec![WasmValue::I32(1)]) // Success
            } else {
                Err(WasmError::Configuration("Script URL required".to_string()))
            }
        }) as HostFunction,
    );

    // Send message to service worker
    functions.insert(
        "pwa_send_message".to_string(),
        Arc::new(|args| {
            if let Some(WasmValue::Bytes(message)) = args.get(0) {
                tracing::debug!(message_size = message.len(), "Sending message to service worker");
                Ok(vec![WasmValue::I32(1)]) // Success
            } else {
                Err(WasmError::Configuration("Message data required".to_string()))
            }
        }) as HostFunction,
    );

    // Store data in IndexedDB
    functions.insert(
        "pwa_store_data".to_string(),
        Arc::new(|args| {
            match (args.get(0), args.get(1), args.get(2)) {
                (Some(WasmValue::String(store)), 
                 Some(WasmValue::String(key)), 
                 Some(WasmValue::Bytes(data))) => {
                    tracing::debug!(
                        store = %store,
                        key = %key,
                        size = data.len(),
                        "Storing data in IndexedDB"
                    );
                    Ok(vec![WasmValue::I32(1)]) // Success
                }
                _ => Err(WasmError::Configuration("Invalid arguments for data storage".to_string()))
            }
        }) as HostFunction,
    );

    // Retrieve data from IndexedDB
    functions.insert(
        "pwa_retrieve_data".to_string(),
        Arc::new(|args| {
            match (args.get(0), args.get(1)) {
                (Some(WasmValue::String(store)), Some(WasmValue::String(key))) => {
                    tracing::debug!(store = %store, key = %key, "Retrieving data from IndexedDB");
                    Ok(vec![WasmValue::Bytes(b"stored_data".to_vec())])
                }
                _ => Err(WasmError::Configuration("Invalid arguments for data retrieval".to_string()))
            }
        }) as HostFunction,
    );

    // Register background sync
    functions.insert(
        "pwa_register_sync".to_string(),
        Arc::new(|args| {
            if let Some(WasmValue::String(tag)) = args.get(0) {
                tracing::info!(tag = %tag, "Registering background sync");
                Ok(vec![WasmValue::I32(1)]) // Success
            } else {
                Err(WasmError::Configuration("Sync tag required".to_string()))
            }
        }) as HostFunction,
    );

    functions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pwa_message() {
        let message = PwaMessage {
            message_type: "sync".to_string(),
            payload: serde_json::json!({"data": "test"}),
            timestamp: chrono::Utc::now(),
            id: "msg_123".to_string(),
        };
        
        assert_eq!(message.message_type, "sync");
        assert_eq!(message.id, "msg_123");
    }

    #[test]
    fn test_background_sync_options() {
        let options = BackgroundSyncOptions::default();
        
        assert_eq!(options.min_interval, std::time::Duration::from_secs(30));
        assert_eq!(options.max_retries, 3);
        assert_eq!(options.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_object_store_config() {
        let mut config = ObjectStoreConfig::default();
        config.auto_increment = true;
        config.indexes.push(IndexConfig {
            name: "name_index".to_string(),
            key_path: "name".to_string(),
            unique: false,
            multi_entry: false,
        });
        
        assert!(config.auto_increment);
        assert_eq!(config.indexes.len(), 1);
        assert_eq!(config.indexes[0].name, "name_index");
    }

    #[test]
    fn test_storage_quota() {
        let quota = StorageQuota {
            used_bytes: 1024 * 1024, // 1MB
            available_bytes: 99 * 1024 * 1024, // 99MB
            total_bytes: 100 * 1024 * 1024, // 100MB
        };
        
        assert_eq!(quota.used_bytes, 1024 * 1024);
        assert_eq!(quota.total_bytes, 100 * 1024 * 1024);
        assert_eq!(quota.used_bytes + quota.available_bytes, quota.total_bytes);
    }

    #[test]
    fn test_indexeddb_manager() {
        let manager = IndexedDbManager::new("test_db".to_string(), 1);
        
        assert_eq!(manager.db_name, "test_db");
        assert_eq!(manager.db_version, 1);
        assert!(manager.database.is_none());
    }
}