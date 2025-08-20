//! WASM runtime management and execution engine

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::RwLock;
use wasmtime::*;

use crate::error::{WasmError, WasmResult};
use crate::types::{
    CompilationTarget, ExecutionContext, HostFunctionRegistry, ModuleInstance, 
    ModuleMetadata, WasmValue
};

pub mod config;
pub mod engine;
pub mod executor;
pub mod host_functions;
pub mod memory_manager;
pub mod security;

pub use config::RuntimeConfig;
pub use engine::WasmEngine;
pub use executor::ModuleExecutor;
pub use host_functions::HostFunctionManager;
pub use memory_manager::MemoryManager;
pub use security::SecurityManager;

/// High-level WASM runtime interface
#[async_trait]
pub trait WasmRuntimeTrait: Send + Sync {
    /// Load a WASM module from bytes
    async fn load_module(&self, name: String, bytes: &[u8], metadata: ModuleMetadata) -> WasmResult<String>;

    /// Execute a function in a loaded module
    async fn execute_function(
        &self,
        module_id: &str,
        function_name: &str,
        args: &[WasmValue],
        context: ExecutionContext,
    ) -> WasmResult<Vec<WasmValue>>;

    /// Unload a module
    async fn unload_module(&self, module_id: &str) -> WasmResult<()>;

    /// List loaded modules
    async fn list_modules(&self) -> WasmResult<Vec<String>>;

    /// Get module information
    async fn get_module_info(&self, module_id: &str) -> WasmResult<ModuleMetadata>;

    /// Register host function
    async fn register_host_function<F>(&self, name: String, func: F) -> WasmResult<()>
    where
        F: Fn(&[WasmValue]) -> WasmResult<Vec<WasmValue>> + Send + Sync + 'static;
}

/// Main WASM runtime implementation
pub struct WasmRuntime {
    /// Wasmtime engine
    engine: Engine,
    /// Runtime configuration
    config: RuntimeConfig,
    /// Module instances
    modules: Arc<RwLock<HashMap<String, ModuleInstance>>>,
    /// Host function registry
    host_functions: Arc<RwLock<HostFunctionRegistry>>,
    /// Security manager
    security_manager: SecurityManager,
    /// Memory manager
    memory_manager: MemoryManager,
    /// Execution statistics
    stats: Arc<RwLock<RuntimeStats>>,
}

impl WasmRuntime {
    /// Create a new WASM runtime with default configuration
    pub async fn new() -> WasmResult<Self> {
        Self::with_config(RuntimeConfig::default()).await
    }

    /// Create a new WASM runtime with custom configuration
    pub async fn with_config(config: RuntimeConfig) -> WasmResult<Self> {
        let engine = Self::create_engine(&config)?;
        
        Ok(Self {
            engine,
            config: config.clone(),
            modules: Arc::new(RwLock::new(HashMap::new())),
            host_functions: Arc::new(RwLock::new(HashMap::new())),
            security_manager: SecurityManager::new(config.security),
            memory_manager: MemoryManager::new(config.memory),
            stats: Arc::new(RwLock::new(RuntimeStats::default())),
        })
    }

    /// Create Wasmtime engine with configuration
    fn create_engine(config: &RuntimeConfig) -> WasmResult<Engine> {
        let mut wasmtime_config = Config::new();
        
        // Configure compilation strategy
        match config.compilation.target {
            CompilationTarget::Native => {
                wasmtime_config.strategy(Strategy::Cranelift);
            }
            CompilationTarget::Web => {
                wasmtime_config.strategy(Strategy::Cranelift);
                // Web-specific optimizations
            }
            CompilationTarget::Wasi => {
                wasmtime_config.strategy(Strategy::Cranelift);
                wasmtime_config.wasm_component_model(true);
            }
            CompilationTarget::Substrate => {
                wasmtime_config.strategy(Strategy::Cranelift);
                // Substrate-specific optimizations
            }
        }

        // Configure WASM features
        wasmtime_config
            .wasm_threads(config.features.threads)
            .wasm_simd(config.features.simd)
            .wasm_multi_value(config.features.multi_value)
            .wasm_multi_memory(config.features.multi_memory)
            .wasm_bulk_memory(config.features.bulk_memory)
            .wasm_reference_types(config.features.reference_types);

        // Configure memory and limits
        wasmtime_config
            .max_wasm_stack(config.limits.max_stack_size)
            .consume_fuel(config.limits.enable_fuel)
            .epoch_interruption(config.limits.enable_epoch_interruption);

        // Configure security
        if config.security.enable_sandboxing {
            wasmtime_config.cranelift_nan_canonicalization(true);
        }

        // Configure optimizations
        wasmtime_config
            .cranelift_opt_level(if config.optimization.enable_optimizations {
                OptLevel::Speed
            } else {
                OptLevel::None
            })
            .debug_info(config.debug.enable_debug_info);

        Engine::new(&wasmtime_config).map_err(WasmError::from)
    }

    /// Load WASM module from file
    pub async fn load_module_from_file<P: AsRef<Path>>(
        &self,
        name: String,
        path: P,
        metadata: ModuleMetadata,
    ) -> WasmResult<String> {
        let bytes = tokio::fs::read(path)
            .await
            .map_err(WasmError::from)?;
        
        self.load_module(name, &bytes, metadata).await
    }

    /// Execute WASM module directly (convenience method)
    pub async fn execute_module(
        &self,
        bytes: &[u8],
        function_name: &str,
        args: &[WasmValue],
    ) -> WasmResult<Vec<WasmValue>> {
        let module_id = format!("temp_{}", uuid::Uuid::new_v4());
        let metadata = ModuleMetadata::default();
        
        // Load module
        self.load_module(module_id.clone(), bytes, metadata).await?;
        
        // Execute function
        let context = ExecutionContext::new();
        let result = self.execute_function(&module_id, function_name, args, context).await;
        
        // Cleanup
        let _ = self.unload_module(&module_id).await;
        
        result
    }

    /// Get runtime statistics
    pub async fn get_stats(&self) -> RuntimeStats {
        self.stats.read().await.clone()
    }

    /// Reset runtime statistics
    pub async fn reset_stats(&self) -> WasmResult<()> {
        let mut stats = self.stats.write().await;
        *stats = RuntimeStats::default();
        Ok(())
    }

    /// Validate WASM bytecode
    pub fn validate_bytes(&self, bytes: &[u8]) -> WasmResult<()> {
        Module::validate(&self.engine, bytes)
            .map_err(WasmError::from)
    }

    /// Get runtime configuration
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// Shutdown runtime and cleanup resources
    pub async fn shutdown(&self) -> WasmResult<()> {
        let mut modules = self.modules.write().await;
        modules.clear();
        Ok(())
    }

    /// Check if module exists
    pub async fn has_module(&self, module_id: &str) -> bool {
        self.modules.read().await.contains_key(module_id)
    }

    /// Check if module exists (helper method)
    async fn module_exists(&self, module_id: &str) -> bool {
        let modules = self.modules.read().await;
        modules.contains_key(module_id)
    }
}

#[async_trait]
impl WasmRuntimeTrait for WasmRuntime {
    async fn load_module(
        &self,
        name: String,
        bytes: &[u8],
        metadata: ModuleMetadata,
    ) -> WasmResult<String> {
        let start_time = Instant::now();
        
        // Validate bytecode
        self.validate_bytes(bytes)?;
        
        // Security check
        self.security_manager.validate_module(bytes, &metadata).await?;
        
        // Compile module
        let module = Module::new(&self.engine, bytes)
            .map_err(|e| WasmError::ModuleCompilation(e.to_string()))?;
        
        // Create store with execution context
        let context = ExecutionContext::new()
            .with_memory_limit(metadata.requirements.max_memory)
            .with_timeout(Duration::from_secs(metadata.requirements.max_execution_time));
        
        let mut store = Store::new(&self.engine, context);
        
        // Configure store limits
        if self.config.limits.enable_fuel {
            store.fuel_async_yield_interval(Some(1000))?;
            store.set_fuel(self.config.limits.default_fuel)?;
        }
        
        // Set up host functions
        let host_functions = self.host_functions.read().await;
        let mut linker = Linker::new(&self.engine);
        
        // Add default host functions
        self.add_default_host_functions(&mut linker)?;
        
        // Add custom host functions
        for (name, _func) in host_functions.iter() {
            linker.func_wrap(
                "env",
                name,
                |args: i32| -> i32 {
                    // Simplified host function wrapper
                    args // Echo for now
                },
            )?;
        }
        drop(host_functions);
        
        // Instantiate module
        let instance = linker.instantiate_async(&mut store, &module)
            .await
            .map_err(|e| WasmError::ModuleInstantiation(e.to_string()))?;
            
        // Create module instance
        let module_instance = ModuleInstance::new(name.clone(), instance, store, metadata);
        let module_id = module_instance.id.to_string();
        
        // Store module
        let mut modules = self.modules.write().await;
        modules.insert(module_id.clone(), module_instance);
        drop(modules);
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.modules_loaded += 1;
        stats.total_load_time += start_time.elapsed();
        
        tracing::info!(
            module_id = %module_id,
            name = %name,
            load_time_ms = start_time.elapsed().as_millis(),
            "Module loaded successfully"
        );
        
        Ok(module_id)
    }

    async fn execute_function(
        &self,
        module_id: &str,
        function_name: &str,
        args: &[WasmValue],
        context: ExecutionContext,
    ) -> WasmResult<Vec<WasmValue>> {
        let start_time = Instant::now();
        
        // Get and check module
        let mut modules = self.modules.write().await;
        let mut module = modules.remove(module_id)
            .ok_or_else(|| WasmError::ModuleLoad(format!("Module '{}' not found", module_id)))?;
        
        // Check if function exists
        if !module.has_function(function_name) {
            // Put module back before returning error
            modules.insert(module_id.to_string(), module);
            return Err(WasmError::FunctionNotFound(function_name.to_string()));
        }
        
        // Get function
        let func = module.instance
            .get_func(&mut module.store, function_name)
            .ok_or_else(|| WasmError::FunctionNotFound(function_name.to_string()))?;
        
        // Convert arguments
        let wasmtime_args: Vec<wasmtime::Val> = args.iter()
            .map(|arg| arg.to_wasmtime_val())
            .collect();
        
        // Prepare results buffer
        let results_len = func.ty(&mut module.store).results().len();
        let mut results = vec![wasmtime::Val::I32(0); results_len];
        
        // Execute with timeout
        let execution_future = func.call_async(&mut module.store, &wasmtime_args, &mut results);
        
        let execution_result = if context.timeout.is_zero() {
            execution_future.await
        } else {
            tokio::time::timeout(context.timeout, execution_future).await
                .map_err(|_| WasmError::execution_timeout(context.timeout.as_secs()))?
        };
        
        execution_result
            .map_err(|e| WasmError::FunctionExecution(e.to_string()))?;
        
        // Convert results
        let wasm_results: Vec<WasmValue> = results.iter()
            .map(WasmValue::from_wasmtime_val)
            .collect();
        
        // Update module statistics
        module.update_execution_stats();
        
        // Store updated module back
        modules.insert(module_id.to_string(), module);
        drop(modules);
        
        // Update runtime statistics
        let mut stats = self.stats.write().await;
        stats.functions_executed += 1;
        stats.total_execution_time += start_time.elapsed();
        
        tracing::debug!(
            module_id = %module_id,
            function_name = %function_name,
            execution_time_ms = start_time.elapsed().as_millis(),
            "Function executed successfully"
        );
        
        Ok(wasm_results)
    }

    async fn unload_module(&self, module_id: &str) -> WasmResult<()> {
        let mut modules = self.modules.write().await;
        
        if modules.remove(module_id).is_some() {
            let mut stats = self.stats.write().await;
            stats.modules_unloaded += 1;
            
            tracing::info!(module_id = %module_id, "Module unloaded");
            Ok(())
        } else {
            Err(WasmError::ModuleLoad(format!("Module '{}' not found", module_id)))
        }
    }

    async fn list_modules(&self) -> WasmResult<Vec<String>> {
        let modules = self.modules.read().await;
        Ok(modules.keys().cloned().collect())
    }

    async fn get_module_info(&self, module_id: &str) -> WasmResult<ModuleMetadata> {
        let modules = self.modules.read().await;
        let module = modules.get(module_id)
            .ok_or_else(|| WasmError::ModuleLoad(format!("Module '{}' not found", module_id)))?;
        
        Ok(module.metadata.clone())
    }

    async fn register_host_function<F>(&self, name: String, func: F) -> WasmResult<()>
    where
        F: Fn(&[WasmValue]) -> WasmResult<Vec<WasmValue>> + Send + Sync + 'static,
    {
        let mut host_functions = self.host_functions.write().await;
        host_functions.insert(name.clone(), Arc::new(func));
        
        tracing::info!(function_name = %name, "Host function registered");
        Ok(())
    }
}

impl WasmRuntime {
    /// Add default host functions
    fn add_default_host_functions(&self, linker: &mut Linker<ExecutionContext>) -> WasmResult<()> {
        // Add basic logging function
        linker.func_wrap(
            "env",
            "log",
            |_caller: Caller<'_, ExecutionContext>, ptr: i32, len: i32| {
                tracing::info!("WASM log: ptr={}, len={}", ptr, len);
            },
        )?;
        
        // Add timestamp function
        linker.func_wrap(
            "env",
            "timestamp",
            |_caller: Caller<'_, ExecutionContext>| -> i64 {
                chrono::Utc::now().timestamp()
            },
        )?;
        
        Ok(())
    }
}

/// Runtime execution statistics
#[derive(Debug, Clone, Default)]
pub struct RuntimeStats {
    /// Number of modules loaded
    pub modules_loaded: u64,
    /// Number of modules unloaded
    pub modules_unloaded: u64,
    /// Number of functions executed
    pub functions_executed: u64,
    /// Total time spent loading modules
    pub total_load_time: Duration,
    /// Total time spent executing functions
    pub total_execution_time: Duration,
    /// Peak memory usage
    pub peak_memory_usage: usize,
    /// Current memory usage
    pub current_memory_usage: usize,
}

impl RuntimeStats {
    /// Get average load time per module
    pub fn average_load_time(&self) -> Duration {
        if self.modules_loaded > 0 {
            self.total_load_time / self.modules_loaded as u32
        } else {
            Duration::ZERO
        }
    }
    
    /// Get average execution time per function
    pub fn average_execution_time(&self) -> Duration {
        if self.functions_executed > 0 {
            self.total_execution_time / self.functions_executed as u32
        } else {
            Duration::ZERO
        }
    }
    
    /// Get current number of loaded modules
    pub fn current_modules(&self) -> u64 {
        self.modules_loaded.saturating_sub(self.modules_unloaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_runtime_creation() {
        let runtime = WasmRuntime::new().await.unwrap();
        assert_eq!(runtime.config().compilation.target, CompilationTarget::Native);
    }

    #[tokio::test]
    async fn test_module_management() {
        let runtime = WasmRuntime::new().await.unwrap();
        
        // Initially no modules
        let modules = runtime.list_modules().await.unwrap();
        assert!(modules.is_empty());
        
        // Test non-existent module
        assert!(!runtime.has_module("nonexistent").await);
    }

    #[tokio::test]
    async fn test_host_function_registration() {
        let runtime = WasmRuntime::new().await.unwrap();
        
        let result = runtime.register_host_function(
            "test_func".to_string(),
            |args| {
                if let Some(WasmValue::I32(val)) = args.first() {
                    Ok(vec![WasmValue::I32(val * 2)])
                } else {
                    Ok(vec![WasmValue::I32(0)])
                }
            }
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_runtime_stats() {
        let runtime = WasmRuntime::new().await.unwrap();
        let stats = runtime.get_stats().await;
        
        assert_eq!(stats.modules_loaded, 0);
        assert_eq!(stats.functions_executed, 0);
        assert_eq!(stats.current_modules(), 0);
    }
}