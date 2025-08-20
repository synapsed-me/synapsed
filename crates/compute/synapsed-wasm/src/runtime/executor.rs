//! Module execution engine

use std::time::Duration;
use async_trait::async_trait;
use wasmtime::{Func, Instance, Store, Val};

use crate::error::{WasmError, WasmResult};
use crate::types::{ExecutionContext, WasmValue};

/// Module executor trait
#[async_trait]
pub trait ModuleExecutor: Send + Sync {
    /// Execute a function with the given arguments
    async fn execute_function(
        &mut self,
        function_name: &str,
        args: &[WasmValue],
        context: &ExecutionContext,
    ) -> WasmResult<Vec<WasmValue>>;

    /// Check if a function exists
    fn has_function(&mut self, function_name: &str) -> bool;

    /// Get function signature
    fn get_function_signature(&mut self, function_name: &str) -> WasmResult<Vec<String>>;

    /// Get exported functions
    fn get_exported_functions(&mut self) -> Vec<String>;
}

/// Default module executor implementation
pub struct DefaultModuleExecutor {
    /// WASM instance
    instance: Instance,
    /// WASM store
    store: Store<ExecutionContext>,
}

impl DefaultModuleExecutor {
    /// Create a new module executor
    pub fn new(instance: Instance, store: Store<ExecutionContext>) -> Self {
        Self { instance, store }
    }

    /// Get a function from the instance
    fn get_function(&mut self, name: &str) -> WasmResult<Func> {
        self.instance
            .get_func(&mut self.store, name)
            .ok_or_else(|| WasmError::FunctionNotFound(name.to_string()))
    }

    /// Convert WASM values to Wasmtime values
    fn convert_args_to_wasmtime(&self, args: &[WasmValue]) -> Vec<Val> {
        args.iter().map(|v| v.to_wasmtime_val()).collect()
    }

    /// Convert Wasmtime values to WASM values
    fn convert_results_from_wasmtime(&self, results: &[Val]) -> Vec<WasmValue> {
        results.iter().map(WasmValue::from_wasmtime_val).collect()
    }
}

#[async_trait]
impl ModuleExecutor for DefaultModuleExecutor {
    async fn execute_function(
        &mut self,
        function_name: &str,
        args: &[WasmValue],
        context: &ExecutionContext,
    ) -> WasmResult<Vec<WasmValue>> {
        let func = self.get_function(function_name)?;
        
        // Convert arguments
        let wasmtime_args = self.convert_args_to_wasmtime(args);
        
        // Prepare results buffer
        let results_len = func.ty(&self.store).results().len();
        let mut results = vec![Val::I32(0); results_len];
        
        // Execute with timeout if specified
        let execution_future = func.call_async(&mut self.store, &wasmtime_args, &mut results);
        
        let execution_result = if context.timeout.is_zero() {
            execution_future.await
        } else {
            tokio::time::timeout(context.timeout, execution_future)
                .await
                .map_err(|_| WasmError::execution_timeout(context.timeout.as_secs()))?
        };

        execution_result.map_err(|e| WasmError::FunctionExecution(e.to_string()))?;
        
        // Convert results
        Ok(self.convert_results_from_wasmtime(&results))
    }

    fn has_function(&mut self, function_name: &str) -> bool {
        self.instance.get_func(&mut self.store, function_name).is_some()
    }

    fn get_function_signature(&mut self, function_name: &str) -> WasmResult<Vec<String>> {
        let func = self.get_function(function_name)?;
        let ty = func.ty(&mut self.store);
        
        let mut signature = Vec::new();
        
        // Add parameter types
        for param in ty.params() {
            signature.push(format!("{param:?}"));
        }
        
        signature.push("->".to_string());
        
        // Add result types
        for result in ty.results() {
            signature.push(format!("{result:?}"));
        }
        
        Ok(signature)
    }

    fn get_exported_functions(&mut self) -> Vec<String> {
        self.instance
            .exports(&mut self.store)
            .filter_map(|export| {
                let name = export.name().to_string();
                if export.into_func().is_some() {
                    Some(name)
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Execution statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct ExecutionStats {
    /// Total executions
    pub total_executions: u64,
    /// Successful executions
    pub successful_executions: u64,
    /// Failed executions
    pub failed_executions: u64,
    /// Total execution time
    pub total_execution_time: Duration,
    /// Average execution time
    pub average_execution_time: Duration,
    /// Peak memory usage during execution
    pub peak_memory_usage: usize,
}

impl ExecutionStats {
    /// Record a successful execution
    pub fn record_success(&mut self, duration: Duration, memory_usage: usize) {
        self.total_executions += 1;
        self.successful_executions += 1;
        self.total_execution_time += duration;
        self.average_execution_time = self.total_execution_time / self.total_executions as u32;
        
        if memory_usage > self.peak_memory_usage {
            self.peak_memory_usage = memory_usage;
        }
    }

    /// Record a failed execution
    pub fn record_failure(&mut self, duration: Duration) {
        self.total_executions += 1;
        self.failed_executions += 1;
        self.total_execution_time += duration;
        self.average_execution_time = self.total_execution_time / self.total_executions as u32;
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_executions == 0 {
            0.0
        } else {
            (self.successful_executions as f64 / self.total_executions as f64) * 100.0
        }
    }

    /// Reset statistics
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Execution context builder for complex scenarios
pub struct ExecutionContextBuilder {
    context: ExecutionContext,
}

impl ExecutionContextBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            context: ExecutionContext::new(),
        }
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.context.timeout = timeout;
        self
    }

    /// Set memory limit
    pub fn with_memory_limit(mut self, limit: usize) -> Self {
        self.context.memory_limit = limit;
        self
    }

    /// Set gas limit
    pub fn with_gas_limit(mut self, limit: u64) -> Self {
        self.context.gas_limit = Some(limit);
        self
    }

    /// Add environment variable
    pub fn with_env<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.context.env.insert(key.into(), value.into());
        self
    }

    /// Add property
    pub fn with_property<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.context.properties.insert(key.into(), value.into());
        self
    }

    /// Set caller information
    pub fn with_caller<S: Into<String>>(mut self, caller: S) -> Self {
        self.context.caller = Some(caller.into());
        self
    }

    /// Build the execution context
    pub fn build(self) -> ExecutionContext {
        self.context
    }
}

impl Default for ExecutionContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_execution_stats() {
        let mut stats = ExecutionStats::default();
        
        // Record some executions
        stats.record_success(Duration::from_millis(100), 1024);
        stats.record_success(Duration::from_millis(200), 2048);
        stats.record_failure(Duration::from_millis(50));
        
        assert_eq!(stats.total_executions, 3);
        assert_eq!(stats.successful_executions, 2);
        assert_eq!(stats.failed_executions, 1);
        assert_eq!(stats.success_rate(), 200.0 / 3.0);
        assert_eq!(stats.peak_memory_usage, 2048);
    }

    #[test]
    fn test_execution_context_builder() {
        let context = ExecutionContextBuilder::new()
            .with_timeout(Duration::from_secs(60))
            .with_memory_limit(64 * 1024 * 1024)
            .with_gas_limit(1000000)
            .with_env("TEST_VAR", "test_value")
            .with_property("test_prop", "prop_value")
            .with_caller("test_caller")
            .build();

        assert_eq!(context.timeout, Duration::from_secs(60));
        assert_eq!(context.memory_limit, 64 * 1024 * 1024);
        assert_eq!(context.gas_limit, Some(1000000));
        assert_eq!(context.env.get("TEST_VAR"), Some(&"test_value".to_string()));
        assert_eq!(context.properties.get("test_prop"), Some(&"prop_value".to_string()));
        assert_eq!(context.caller, Some("test_caller".to_string()));
    }

    #[test]
    fn test_stats_reset() {
        let mut stats = ExecutionStats::default();
        stats.record_success(Duration::from_millis(100), 1024);
        
        assert_eq!(stats.total_executions, 1);
        
        stats.reset();
        assert_eq!(stats.total_executions, 0);
        assert_eq!(stats.successful_executions, 0);
    }
}