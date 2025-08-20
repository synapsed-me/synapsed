//! Module validation utilities

use crate::error::{WasmError, WasmResult};
use crate::types::ModuleMetadata;

/// Module validator
pub struct ModuleValidator;

impl ModuleValidator {
    /// Validate WASM module structure
    pub fn validate_structure(bytes: &[u8]) -> WasmResult<()> {
        wasmparser::validate(bytes).map(|_| ()).map_err(WasmError::from)
    }

    /// Validate module metadata
    pub fn validate_metadata(metadata: &ModuleMetadata) -> WasmResult<()> {
        if metadata.version.is_empty() {
            return Err(WasmError::Configuration("Version cannot be empty".to_string()));
        }

        if metadata.requirements.max_memory == 0 {
            return Err(WasmError::Configuration("Memory limit must be greater than zero".to_string()));
        }

        Ok(())
    }
}