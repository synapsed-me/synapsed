//! WASM engine abstraction and management

use std::sync::Arc;
use wasmtime::{Engine, Config, OptLevel, Strategy};

use crate::error::{WasmError, WasmResult};
use crate::runtime::config::{RuntimeConfig, OptimizationLevel};

/// WASM engine abstraction
pub struct WasmEngine {
    /// Wasmtime engine instance
    inner: Engine,
    /// Engine configuration
    config: RuntimeConfig,
}

impl WasmEngine {
    /// Create a new WASM engine with the provided configuration
    pub fn new(config: RuntimeConfig) -> WasmResult<Self> {
        let engine = Self::create_wasmtime_engine(&config)?;
        
        Ok(Self {
            inner: engine,
            config,
        })
    }

    /// Get the underlying Wasmtime engine
    pub fn inner(&self) -> &Engine {
        &self.inner
    }

    /// Get engine configuration
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// Create Wasmtime engine from runtime configuration
    fn create_wasmtime_engine(config: &RuntimeConfig) -> WasmResult<Engine> {
        let mut wasmtime_config = Config::new();

        // Configure compilation strategy
        wasmtime_config.strategy(Strategy::Cranelift);

        // Configure WASM features
        wasmtime_config
            .wasm_threads(config.features.threads)
            .wasm_simd(config.features.simd)
            .wasm_multi_value(config.features.multi_value)
            .wasm_multi_memory(config.features.multi_memory)
            .wasm_bulk_memory(config.features.bulk_memory)
            .wasm_reference_types(config.features.reference_types)
            .wasm_component_model(config.features.component_model);

        // Configure limits
        wasmtime_config
            .max_wasm_stack(config.limits.max_stack_size)
            .consume_fuel(config.limits.enable_fuel)
            .epoch_interruption(config.limits.enable_epoch_interruption);

        // Configure security
        if config.security.enable_sandboxing {
            wasmtime_config.cranelift_nan_canonicalization(true);
        }

        // Configure optimizations
        let opt_level = match config.optimization.optimization_level {
            OptimizationLevel::None => OptLevel::None,
            OptimizationLevel::Size => OptLevel::SpeedAndSize,
            OptimizationLevel::Speed => OptLevel::Speed,
            OptimizationLevel::Balanced => OptLevel::SpeedAndSize,
        };
        wasmtime_config.cranelift_opt_level(opt_level);

        // Configure debug info
        wasmtime_config.debug_info(config.debug.enable_debug_info);

        // Configure parallel compilation
        if config.optimization.enable_parallel_compilation {
            wasmtime_config.parallel_compilation(true);
        }

        Engine::new(&wasmtime_config).map_err(WasmError::from)
    }

    /// Clone the engine (returns Arc for efficiency)
    pub fn clone_engine(&self) -> Arc<Engine> {
        Arc::new(self.inner.clone())
    }
}

impl Clone for WasmEngine {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let config = RuntimeConfig::default();
        let engine = WasmEngine::new(config).unwrap();
        assert!(engine.inner().check_compatible_with_native_host());
    }

    #[test]
    fn test_engine_with_different_configs() {
        // Test development config
        let dev_config = RuntimeConfig::development();
        let dev_engine = WasmEngine::new(dev_config).unwrap();
        assert!(dev_engine.config().debug.enable_debug_info);

        // Test production config
        let prod_config = RuntimeConfig::production();
        let prod_engine = WasmEngine::new(prod_config).unwrap();
        assert!(prod_engine.config().optimization.enable_optimizations);
    }

    #[test]
    fn test_engine_cloning() {
        let config = RuntimeConfig::default();
        let engine = WasmEngine::new(config).unwrap();
        let cloned = engine.clone();
        
        // Both engines should be compatible
        assert!(engine.inner().check_compatible_with_native_host());
        assert!(cloned.inner().check_compatible_with_native_host());
    }
}