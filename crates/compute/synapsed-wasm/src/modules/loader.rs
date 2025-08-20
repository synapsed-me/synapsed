//! Module loading utilities

use std::path::Path;
use crate::error::{WasmError, WasmResult};

/// Module loader for different WASM sources
pub struct ModuleLoader;

impl ModuleLoader {
    /// Load WASM bytecode from file
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> WasmResult<Vec<u8>> {
        tokio::fs::read(path).await.map_err(WasmError::from)
    }

    /// Load WASM bytecode from WAT (WebAssembly Text format)
    pub fn load_from_wat(wat: &str) -> WasmResult<Vec<u8>> {
        wat::parse_str(wat).map_err(|e| WasmError::ModuleCompilation(e.to_string()))
    }

    /// Validate WASM bytecode
    pub fn validate_bytecode(bytes: &[u8]) -> WasmResult<()> {
        wasmparser::validate(bytes).map(|_| ()).map_err(WasmError::from)
    }
}