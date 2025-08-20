//! Module registry for managing WASM modules

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::WasmResult;
use crate::modules::WasmModule;
use crate::types::ModuleMetadata;

/// Registry for managing WASM modules
pub struct ModuleRegistry {
    /// Registered modules
    modules: Arc<RwLock<HashMap<String, Box<dyn WasmModule>>>>,
    /// Module metadata cache
    metadata_cache: Arc<RwLock<HashMap<String, ModuleMetadata>>>,
}

impl ModuleRegistry {
    /// Create a new module registry
    pub fn new() -> Self {
        Self {
            modules: Arc::new(RwLock::new(HashMap::new())),
            metadata_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a module
    pub async fn register(&self, module: Box<dyn WasmModule>) -> WasmResult<()> {
        let id = module.id().to_string();
        let metadata = module.metadata().clone();

        let mut modules = self.modules.write().await;
        let mut cache = self.metadata_cache.write().await;

        modules.insert(id.clone(), module);
        cache.insert(id, metadata);

        Ok(())
    }

    /// Get a module by ID
    pub async fn get(&self, id: &str) -> Option<ModuleMetadata> {
        let cache = self.metadata_cache.read().await;
        cache.get(id).cloned()
    }

    /// List all registered modules
    pub async fn list(&self) -> Vec<String> {
        let modules = self.modules.read().await;
        modules.keys().cloned().collect()
    }

    /// Remove a module
    pub async fn remove(&self, id: &str) -> WasmResult<()> {
        let mut modules = self.modules.write().await;
        let mut cache = self.metadata_cache.write().await;

        modules.remove(id);
        cache.remove(id);

        Ok(())
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}