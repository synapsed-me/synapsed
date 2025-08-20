//! WASM module management and registry

use std::path::Path;
use std::sync::Arc;
use async_trait::async_trait;
use wasmtime::{Engine, Linker, Module, Store};

use crate::error::{WasmError, WasmResult};
use crate::runtime::{HostFunctionManager, SecurityManager};
use crate::types::{ExecutionContext, ModuleMetadata, ModuleInstance, WasmValue};

pub mod registry;
pub mod loader;
pub mod compiler;
pub mod validator;

pub use registry::ModuleRegistry;
pub use loader::ModuleLoader;
pub use compiler::ModuleCompiler;
pub use validator::ModuleValidator;

/// WASM module trait for polymorphic module handling
#[async_trait]
pub trait WasmModule: Send + Sync {
    /// Get module ID
    fn id(&self) -> &str;

    /// Get module name
    fn name(&self) -> &str;

    /// Get module metadata
    fn metadata(&self) -> &ModuleMetadata;

    /// Check if module has a specific function
    fn has_function(&mut self, name: &str) -> bool;

    /// Get exported function names
    fn get_exported_functions(&self) -> Vec<String>;

    /// Execute a function
    async fn execute_function(
        &mut self,
        function_name: &str,
        args: &[WasmValue],
        context: ExecutionContext,
    ) -> WasmResult<Vec<WasmValue>>;

    /// Get module statistics
    fn get_stats(&self) -> ModuleStats;

    /// Validate module integrity
    async fn validate(&self) -> WasmResult<()>;

    /// Serialize module state for persistence
    async fn serialize_state(&self) -> WasmResult<Vec<u8>>;

    /// Deserialize module state from persistence
    async fn deserialize_state(&mut self, data: &[u8]) -> WasmResult<()>;
}

/// Standard WASM module implementation
pub struct StandardWasmModule {
    /// Module ID
    id: String,
    /// Module name
    name: String,
    /// Module metadata
    metadata: ModuleMetadata,
    /// Module instance
    instance: Option<ModuleInstance>,
    /// Module statistics
    stats: ModuleStats,
    /// Module bytecode (for recompilation if needed)
    bytecode: Vec<u8>,
    /// Creation timestamp
    created_at: std::time::SystemTime,
}

impl StandardWasmModule {
    /// Create a new standard WASM module
    pub fn new(
        id: String,
        name: String,
        metadata: ModuleMetadata,
        bytecode: Vec<u8>,
    ) -> Self {
        Self {
            id,
            name,
            metadata,
            instance: None,
            stats: ModuleStats::default(),
            bytecode,
            created_at: std::time::SystemTime::now(),
        }
    }

    /// Initialize the module with an engine
    pub async fn initialize(
        &mut self,
        engine: &Engine,
        host_functions: &HostFunctionManager,
        security_manager: &SecurityManager,
    ) -> WasmResult<()> {
        // Validate module security
        security_manager.validate_module(&self.bytecode, &self.metadata).await?;

        // Compile module
        let module = Module::new(engine, &self.bytecode)
            .map_err(|e| WasmError::ModuleCompilation(e.to_string()))?;

        // Create store
        let context = ExecutionContext::new()
            .with_memory_limit(self.metadata.requirements.max_memory)
            .with_timeout(std::time::Duration::from_secs(
                self.metadata.requirements.max_execution_time,
            ));

        let mut store = Store::new(engine, context);

        // Create linker and add host functions
        let mut linker = Linker::new(engine);
        host_functions.add_to_linker(&mut linker, Some(&self.name))?;

        // Instantiate module
        let instance = linker.instantiate(&mut store, &module)
            .map_err(|e| WasmError::ModuleInstantiation(e.to_string()))?;

        // Create module instance
        let module_instance = ModuleInstance::new(
            self.name.clone(),
            instance,
            store,
            self.metadata.clone(),
        );

        self.instance = Some(module_instance);
        self.stats.initialization_count += 1;

        tracing::info!(
            module_id = %self.id,
            module_name = %self.name,
            "Module initialized successfully"
        );

        Ok(())
    }

    /// Get the module instance
    fn get_instance(&mut self) -> WasmResult<&mut ModuleInstance> {
        self.instance.as_mut()
            .ok_or_else(|| WasmError::ModuleLoad("Module not initialized".to_string()))
    }

    /// Check if module is initialized
    pub fn is_initialized(&self) -> bool {
        self.instance.is_some()
    }

    /// Get module uptime
    pub fn uptime(&self) -> std::time::Duration {
        self.created_at.elapsed().unwrap_or_default()
    }

    /// Get module size
    pub fn size(&self) -> usize {
        self.bytecode.len()
    }
}

#[async_trait]
impl WasmModule for StandardWasmModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn metadata(&self) -> &ModuleMetadata {
        &self.metadata
    }

    fn has_function(&mut self, name: &str) -> bool {
        self.instance.as_mut()
            .map(|inst| inst.has_function(name))
            .unwrap_or(false)
    }

    fn get_exported_functions(&self) -> Vec<String> {
        self.instance.as_ref()
            .map(|inst| inst.exports.clone())
            .unwrap_or_default()
    }

    async fn execute_function(
        &mut self,
        function_name: &str,
        args: &[WasmValue],
        context: ExecutionContext,
    ) -> WasmResult<Vec<WasmValue>> {
        let start_time = std::time::Instant::now();
        
        let instance = self.get_instance()?;
        
        // Check if function exists
        if !instance.has_function(function_name) {
            return Err(WasmError::FunctionNotFound(function_name.to_string()));
        }

        // Get function
        let func = instance.instance
            .get_func(&mut instance.store, function_name)
            .ok_or_else(|| WasmError::FunctionNotFound(function_name.to_string()))?;

        // Convert arguments
        let wasmtime_args: Vec<wasmtime::Val> = args.iter()
            .map(|arg| arg.to_wasmtime_val())
            .collect();

        // Prepare results
        let results_len = func.ty(&instance.store).results().len();
        let mut results = vec![wasmtime::Val::I32(0); results_len];

        // Execute with timeout
        let execution_result = if context.timeout.is_zero() {
            func.call_async(&mut instance.store, &wasmtime_args, &mut results).await
        } else {
            tokio::time::timeout(
                context.timeout,
                func.call_async(&mut instance.store, &wasmtime_args, &mut results),
            )
            .await
            .map_err(|_| WasmError::execution_timeout(context.timeout.as_secs()))?
        };

        execution_result
            .map_err(|e| WasmError::FunctionExecution(e.to_string()))?;

        // Convert results
        let wasm_results: Vec<WasmValue> = results.iter()
            .map(WasmValue::from_wasmtime_val)
            .collect();

        // Update statistics
        instance.update_execution_stats();
        self.stats.function_calls += 1;
        self.stats.total_execution_time += start_time.elapsed();

        tracing::debug!(
            module_id = %self.id,
            function_name = %function_name,
            execution_time_ms = start_time.elapsed().as_millis(),
            "Function executed successfully"
        );

        Ok(wasm_results)
    }

    fn get_stats(&self) -> ModuleStats {
        let mut stats = self.stats.clone();
        
        if let Some(instance) = &self.instance {
            stats.memory_usage = instance.memory_size().unwrap_or(0);
            stats.uptime = self.uptime();
            stats.execution_count = instance.execution_count;
            stats.last_executed = instance.last_executed;
        }

        stats
    }

    async fn validate(&self) -> WasmResult<()> {
        // Validate bytecode
        if self.bytecode.is_empty() {
            return Err(WasmError::InvalidBytecode("Empty bytecode".to_string()));
        }

        // Check if module is properly initialized
        if !self.is_initialized() {
            return Err(WasmError::ModuleLoad("Module not initialized".to_string()));
        }

        // Validate metadata consistency
        if self.metadata.version.is_empty() {
            return Err(WasmError::Configuration("Invalid metadata version".to_string()));
        }

        Ok(())
    }

    async fn serialize_state(&self) -> WasmResult<Vec<u8>> {
        let state = ModuleState {
            id: self.id.clone(),
            name: self.name.clone(),
            metadata: self.metadata.clone(),
            stats: self.stats.clone(),
            bytecode: self.bytecode.clone(),
            created_at: self.created_at,
        };

        bincode::serialize(&state)
            .map_err(WasmError::from)
    }

    async fn deserialize_state(&mut self, data: &[u8]) -> WasmResult<()> {
        let state: ModuleState = bincode::deserialize(data)
            .map_err(WasmError::from)?;

        self.id = state.id;
        self.name = state.name;
        self.metadata = state.metadata;
        self.stats = state.stats;
        self.bytecode = state.bytecode;
        self.created_at = state.created_at;

        Ok(())
    }
}

/// Module statistics
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ModuleStats {
    /// Number of times the module was initialized
    pub initialization_count: u64,
    /// Number of function calls
    pub function_calls: u64,
    /// Total execution time
    pub total_execution_time: std::time::Duration,
    /// Current memory usage in bytes
    pub memory_usage: usize,
    /// Peak memory usage in bytes
    pub peak_memory_usage: usize,
    /// Module uptime
    pub uptime: std::time::Duration,
    /// Number of executions
    pub execution_count: u64,
    /// Last execution timestamp
    pub last_executed: Option<std::time::SystemTime>,
    /// Error count
    pub error_count: u64,
    /// Success rate (0.0 to 1.0)
    pub success_rate: f64,
}

impl ModuleStats {
    /// Update success rate based on execution results
    pub fn update_success_rate(&mut self, successful: bool) {
        let total_calls = self.function_calls + if successful { 0 } else { 1 };
        let successful_calls = if successful { self.function_calls } else { self.function_calls };
        
        self.success_rate = if total_calls == 0 {
            1.0
        } else {
            successful_calls as f64 / total_calls as f64
        };

        if !successful {
            self.error_count += 1;
        }
    }

    /// Get average execution time
    pub fn average_execution_time(&self) -> std::time::Duration {
        if self.function_calls == 0 {
            std::time::Duration::ZERO
        } else {
            self.total_execution_time / self.function_calls as u32
        }
    }

    /// Check if module is performing well
    pub fn is_healthy(&self) -> bool {
        self.success_rate > 0.95 && self.error_count < 10
    }
}

/// Serializable module state for persistence
#[derive(serde::Serialize, serde::Deserialize)]
struct ModuleState {
    id: String,
    name: String,
    metadata: ModuleMetadata,
    stats: ModuleStats,
    bytecode: Vec<u8>,
    created_at: std::time::SystemTime,
}

/// Module factory for creating different types of modules
pub struct ModuleFactory {
    /// Engine reference
    engine: Arc<Engine>,
    /// Host functions manager
    host_functions: Arc<HostFunctionManager>,
    /// Security manager
    security_manager: Arc<SecurityManager>,
}

impl ModuleFactory {
    /// Create a new module factory
    pub fn new(
        engine: Arc<Engine>,
        host_functions: Arc<HostFunctionManager>,
        security_manager: Arc<SecurityManager>,
    ) -> Self {
        Self {
            engine,
            host_functions,
            security_manager,
        }
    }

    /// Create a standard WASM module from bytecode
    pub async fn create_module(
        &self,
        id: String,
        name: String,
        bytecode: Vec<u8>,
        metadata: ModuleMetadata,
    ) -> WasmResult<Box<dyn WasmModule>> {
        let mut module = StandardWasmModule::new(id, name, metadata, bytecode);
        module.initialize(&self.engine, &self.host_functions, &self.security_manager).await?;
        Ok(Box::new(module))
    }

    /// Create a module from file
    pub async fn create_module_from_file<P: AsRef<Path>>(
        &self,
        id: String,
        name: String,
        path: P,
        metadata: ModuleMetadata,
    ) -> WasmResult<Box<dyn WasmModule>> {
        let bytecode = tokio::fs::read(path).await
            .map_err(WasmError::from)?;
        
        self.create_module(id, name, bytecode, metadata).await
    }

    /// Create a module from WAT (WebAssembly Text format)
    pub async fn create_module_from_wat(
        &self,
        id: String,
        name: String,
        wat: &str,
        metadata: ModuleMetadata,
    ) -> WasmResult<Box<dyn WasmModule>> {
        let bytecode = wat::parse_str(wat)
            .map_err(|e| WasmError::ModuleCompilation(e.to_string()))?;
        
        self.create_module(id, name, bytecode, metadata).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::config::SecurityConfig;

    fn create_test_metadata() -> ModuleMetadata {
        ModuleMetadata::new("1.0.0".to_string())
            .with_capability("test")
            .with_tag("testing")
    }

    #[test]
    fn test_standard_module_creation() {
        let module = StandardWasmModule::new(
            "test_id".to_string(),
            "test_module".to_string(),
            create_test_metadata(),
            vec![0, 97, 115, 109], // WASM magic number
        );

        assert_eq!(module.id(), "test_id");
        assert_eq!(module.name(), "test_module");
        assert!(!module.is_initialized());
        assert_eq!(module.size(), 4);
    }

    #[tokio::test]
    async fn test_module_validation() {
        let module = StandardWasmModule::new(
            "test_id".to_string(),
            "test_module".to_string(),
            create_test_metadata(),
            vec![0, 97, 115, 109],
        );

        // Should fail because module is not initialized
        let result = module.validate().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_module_serialization() {
        let module = StandardWasmModule::new(
            "test_id".to_string(),
            "test_module".to_string(),
            create_test_metadata(),
            vec![0, 97, 115, 109],
        );

        let serialized = module.serialize_state().await.unwrap();
        assert!(!serialized.is_empty());

        let mut new_module = StandardWasmModule::new(
            "temp".to_string(),
            "temp".to_string(),
            ModuleMetadata::default(),
            vec![],
        );

        new_module.deserialize_state(&serialized).await.unwrap();
        assert_eq!(new_module.id(), "test_id");
        assert_eq!(new_module.name(), "test_module");
    }

    #[test]
    fn test_module_stats() {
        let mut stats = ModuleStats::default();
        
        // Test success rate calculation
        stats.function_calls = 10;
        stats.update_success_rate(true);
        assert!(stats.success_rate > 0.9);

        stats.update_success_rate(false);
        assert!(stats.error_count > 0);
        assert!(stats.success_rate < 1.0);

        // Test health check
        stats.success_rate = 0.99;
        stats.error_count = 5;
        assert!(stats.is_healthy());

        stats.success_rate = 0.80;
        assert!(!stats.is_healthy());
    }

    #[test]
    fn test_average_execution_time() {
        let mut stats = ModuleStats::default();
        stats.function_calls = 5;
        stats.total_execution_time = std::time::Duration::from_millis(500);

        let avg = stats.average_execution_time();
        assert_eq!(avg, std::time::Duration::from_millis(100));

        // Test with no function calls
        stats.function_calls = 0;
        let avg = stats.average_execution_time();
        assert_eq!(avg, std::time::Duration::ZERO);
    }
}