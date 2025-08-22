//! # Synapsed WASM Bindings
//! 
//! WebAssembly bindings for the Synapsed framework, enabling verifiable AI agent
//! systems to run in browsers and other WASM environments.

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

pub mod intent;
pub mod promise;
pub mod verify;
pub mod observability;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

/// Initialize panic hook for better error messages in browser console
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Version information for the WASM module
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Test that the WASM module is loaded correctly
#[wasm_bindgen]
pub fn hello_synapsed() -> String {
    "Hello from Synapsed WASM! Ready for verifiable AI agents.".to_string()
}

/// Result type for WASM operations
#[wasm_bindgen]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmResult {
    success: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

#[wasm_bindgen]
impl WasmResult {
    #[wasm_bindgen(constructor)]
    pub fn new(success: bool, message: String) -> Self {
        Self {
            success,
            message,
            data: None,
        }
    }
    
    #[wasm_bindgen(getter)]
    pub fn success(&self) -> bool {
        self.success
    }
    
    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.message.clone()
    }
    
    #[wasm_bindgen(getter)]
    pub fn data(&self) -> JsValue {
        match &self.data {
            Some(v) => serde_wasm_bindgen::to_value(v).unwrap_or(JsValue::NULL),
            None => JsValue::NULL,
        }
    }
}

impl WasmResult {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
        }
    }
    
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
        }
    }
    
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    
    wasm_bindgen_test_configure!(run_in_browser);
    
    #[wasm_bindgen_test]
    fn test_version() {
        let v = version();
        assert_eq!(v, "0.1.0");
    }
    
    #[wasm_bindgen_test]
    fn test_hello() {
        let msg = hello_synapsed();
        assert!(msg.contains("Synapsed WASM"));
    }
    
    #[wasm_bindgen_test]
    fn test_result() {
        let result = WasmResult::ok("Test passed");
        assert!(result.success());
        assert_eq!(result.message(), "Test passed");
    }
}