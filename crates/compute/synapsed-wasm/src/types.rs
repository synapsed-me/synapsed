//! Type definitions for the synapsed-wasm crate

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use uuid::Uuid;
use wasmtime::{Instance, Memory, Store};

use crate::error::WasmResult;

/// WASM value types that can be passed to/from functions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WasmValue {
    /// 32-bit integer
    I32(i32),
    /// 64-bit integer
    I64(i64),
    /// 32-bit float
    F32(f32),
    /// 64-bit float
    F64(f64),
    /// Vector of 128 bits (SIMD)
    V128([u8; 16]),
    /// Reference type (externref or funcref)
    Ref(Option<u32>),
    /// Byte array
    Bytes(Vec<u8>),
    /// UTF-8 string
    String(String),
    /// Boolean value
    Bool(bool),
    /// Null/None value
    Null,
}

impl WasmValue {
    /// Convert to wasmtime::Val
    pub fn to_wasmtime_val(&self) -> wasmtime::Val {
        match self {
            WasmValue::I32(v) => wasmtime::Val::I32(*v),
            WasmValue::I64(v) => wasmtime::Val::I64(*v),
            WasmValue::F32(v) => wasmtime::Val::F32(v.to_bits()),
            WasmValue::F64(v) => wasmtime::Val::F64(v.to_bits()),
            WasmValue::V128(v) => wasmtime::Val::V128(wasmtime::V128::from(u128::from_le_bytes(*v))),
            WasmValue::Ref(Some(_v)) => wasmtime::Val::ExternRef(None), // Simplified for now
            WasmValue::Ref(None) => wasmtime::Val::ExternRef(None),
            // For complex types, we'll need to serialize and pass as memory reference
            _ => wasmtime::Val::I32(0), // Placeholder - should be handled differently
        }
    }

    /// Create from wasmtime::Val
    pub fn from_wasmtime_val(val: &wasmtime::Val) -> Self {
        match val {
            wasmtime::Val::I32(v) => WasmValue::I32(*v),
            wasmtime::Val::I64(v) => WasmValue::I64(*v),
            wasmtime::Val::F32(bits) => WasmValue::F32(f32::from_bits(*bits)),
            wasmtime::Val::F64(bits) => WasmValue::F64(f64::from_bits(*bits)),
            wasmtime::Val::V128(v) => {
                // V128 doesn't have to_le_bytes, convert through u128
                let v128_bits = v.as_u128();
                WasmValue::V128(v128_bits.to_le_bytes())
            }
            wasmtime::Val::ExternRef(Some(_)) => WasmValue::Ref(Some(0)), // Simplified
            wasmtime::Val::ExternRef(None) => WasmValue::Ref(None),
            wasmtime::Val::FuncRef(_) => WasmValue::Ref(Some(0)), // Simplified
            wasmtime::Val::AnyRef(_) => WasmValue::Ref(Some(0)), // Simplified
        }
    }

    /// Get the WASM type name
    pub fn type_name(&self) -> &'static str {
        match self {
            WasmValue::I32(_) => "i32",
            WasmValue::I64(_) => "i64",
            WasmValue::F32(_) => "f32",
            WasmValue::F64(_) => "f64",
            WasmValue::V128(_) => "v128",
            WasmValue::Ref(_) => "externref",
            WasmValue::Bytes(_) => "bytes",
            WasmValue::String(_) => "string",
            WasmValue::Bool(_) => "bool",
            WasmValue::Null => "null",
        }
    }

    /// Check if the value is numeric
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            WasmValue::I32(_) | WasmValue::I64(_) | WasmValue::F32(_) | WasmValue::F64(_)
        )
    }
}

/// Execution context for WASM function calls
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Unique execution ID
    pub id: Uuid,
    /// Maximum execution time
    pub timeout: Duration,
    /// Memory limit in bytes
    pub memory_limit: usize,
    /// Gas limit for execution (for blockchain contexts)
    pub gas_limit: Option<u64>,
    /// Custom properties
    pub properties: HashMap<String, String>,
    /// Execution start time
    pub started_at: SystemTime,
    /// Caller information
    pub caller: Option<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            timeout: Duration::from_secs(30),
            memory_limit: 64 * 1024 * 1024, // 64MB default
            gas_limit: None,
            properties: HashMap::new(),
            started_at: SystemTime::now(),
            caller: None,
            env: HashMap::new(),
        }
    }

    /// Create with custom timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Create with memory limit
    pub fn with_memory_limit(mut self, memory_limit: usize) -> Self {
        self.memory_limit = memory_limit;
        self
    }

    /// Create with gas limit (for blockchain contexts)
    pub fn with_gas_limit(mut self, gas_limit: u64) -> Self {
        self.gas_limit = Some(gas_limit);
        self
    }

    /// Set caller information
    pub fn with_caller<S: Into<String>>(mut self, caller: S) -> Self {
        self.caller = Some(caller.into());
        self
    }

    /// Add environment variable
    pub fn with_env<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Add custom property
    pub fn with_property<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }

    /// Get elapsed execution time
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed().unwrap_or_default()
    }

    /// Check if execution has timed out
    pub fn is_timed_out(&self) -> bool {
        self.elapsed() > self.timeout
    }

    /// Get remaining execution time
    pub fn remaining_time(&self) -> Duration {
        self.timeout.saturating_sub(self.elapsed())
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

/// WASM module instance with execution state
pub struct ModuleInstance {
    /// Unique instance ID
    pub id: Uuid,
    /// Module name/identifier
    pub name: String,
    /// Wasmtime instance
    pub instance: Instance,
    /// Wasmtime store
    pub store: Store<ExecutionContext>,
    /// Module memory (if exported)
    pub memory: Option<Memory>,
    /// Exported function names
    pub exports: Vec<String>,
    /// Module metadata
    pub metadata: ModuleMetadata,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Last execution timestamp
    pub last_executed: Option<SystemTime>,
    /// Execution count
    pub execution_count: u64,
}

impl ModuleInstance {
    /// Create a new module instance
    pub fn new(
        name: String,
        instance: Instance,
        mut store: Store<ExecutionContext>,
        metadata: ModuleMetadata,
    ) -> Self {
        let exports = instance
            .exports(&mut store)
            .map(|export| export.name().to_string())
            .collect();

        let memory = instance.get_memory(&mut store, "memory");

        Self {
            id: Uuid::new_v4(),
            name,
            instance,
            store,
            memory,
            exports,
            metadata,
            created_at: SystemTime::now(),
            last_executed: None,
            execution_count: 0,
        }
    }

    /// Check if function exists
    pub fn has_function(&mut self, name: &str) -> bool {
        self.instance.get_func(&mut self.store, name).is_some()
    }

    /// Get function signature information
    pub fn get_function_signature(&mut self, name: &str) -> Option<FunctionSignature> {
        let func = self.instance.get_func(&mut self.store, name)?;
        let ty = func.ty(&mut self.store);
        
        let params = ty.params().map(|vt| format!("{vt:?}")).collect();
        let results = ty.results().map(|vt| format!("{vt:?}")).collect();

        Some(FunctionSignature {
            name: name.to_string(),
            params,
            results,
        })
    }

    /// Update execution statistics
    pub fn update_execution_stats(&mut self) {
        self.last_executed = Some(SystemTime::now());
        self.execution_count += 1;
    }

    /// Get memory size in bytes
    pub fn memory_size(&self) -> Option<usize> {
        self.memory.map(|mem| mem.data_size(&self.store))
    }

    /// Get uptime duration
    pub fn uptime(&self) -> Duration {
        self.created_at.elapsed().unwrap_or_default()
    }
}

/// Function signature information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    /// Function name
    pub name: String,
    /// Parameter types
    pub params: Vec<String>,
    /// Return types
    pub results: Vec<String>,
}

/// Module metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetadata {
    /// Module version
    pub version: String,
    /// Module description
    pub description: Option<String>,
    /// Module author
    pub author: Option<String>,
    /// Module license
    pub license: Option<String>,
    /// Module tags
    pub tags: Vec<String>,
    /// Module capabilities
    pub capabilities: Vec<String>,
    /// Resource requirements
    pub requirements: ResourceRequirements,
    /// Security configuration
    pub security: SecurityConfig,
    /// Custom metadata
    pub custom: HashMap<String, serde_json::Value>,
}

impl ModuleMetadata {
    /// Create new metadata
    pub fn new(version: String) -> Self {
        Self {
            version,
            description: None,
            author: None,
            license: None,
            tags: Vec::new(),
            capabilities: Vec::new(),
            requirements: ResourceRequirements::default(),
            security: SecurityConfig::default(),
            custom: HashMap::new(),
        }
    }

    /// Add capability
    pub fn with_capability<S: Into<String>>(mut self, capability: S) -> Self {
        self.capabilities.push(capability.into());
        self
    }

    /// Add tag
    pub fn with_tag<S: Into<String>>(mut self, tag: S) -> Self {
        self.tags.push(tag.into());
        self
    }
}

impl Default for ModuleMetadata {
    fn default() -> Self {
        Self::new("1.0.0".to_string())
    }
}

/// Resource requirements for WASM modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// Maximum memory in bytes
    pub max_memory: usize,
    /// Maximum execution time in seconds
    pub max_execution_time: u64,
    /// Maximum stack size
    pub max_stack_size: Option<usize>,
    /// CPU affinity requirements
    pub cpu_features: Vec<String>,
    /// Required host functions
    pub required_imports: Vec<String>,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            max_memory: 64 * 1024 * 1024, // 64MB
            max_execution_time: 30, // 30 seconds
            max_stack_size: None,
            cpu_features: Vec::new(),
            required_imports: Vec::new(),
        }
    }
}

/// Security configuration for WASM modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable sandboxing
    pub sandbox: bool,
    /// Allowed system calls
    pub allowed_syscalls: Vec<String>,
    /// Network access permissions
    pub network_access: NetworkPermissions,
    /// File system access permissions
    pub filesystem_access: FilesystemPermissions,
    /// Memory protection settings
    pub memory_protection: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            sandbox: true,
            allowed_syscalls: Vec::new(),
            network_access: NetworkPermissions::None,
            filesystem_access: FilesystemPermissions::None,
            memory_protection: true,
        }
    }
}

/// Network access permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkPermissions {
    /// No network access
    None,
    /// Allow outbound connections only
    OutboundOnly,
    /// Allow specific hosts
    AllowedHosts(Vec<String>),
    /// Full network access (not recommended)
    Full,
}

/// Filesystem access permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilesystemPermissions {
    /// No filesystem access
    None,
    /// Read-only access to specific directories
    ReadOnly(Vec<String>),
    /// Read-write access to specific directories
    ReadWrite(Vec<String>),
    /// Full filesystem access (not recommended)
    Full,
}

/// WASM module compilation target
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompilationTarget {
    /// Native compilation
    Native,
    /// Web (via wasm-bindgen)
    Web,
    /// WASI (WebAssembly System Interface)
    Wasi,
    /// Substrate runtime
    Substrate,
}

impl CompilationTarget {
    /// Get target name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            CompilationTarget::Native => "native",
            CompilationTarget::Web => "web",
            CompilationTarget::Wasi => "wasi",
            CompilationTarget::Substrate => "substrate",
        }
    }
}

/// Host function interface for extending WASM modules
pub type HostFunction = Arc<
    dyn Fn(&[WasmValue]) -> WasmResult<Vec<WasmValue>> + Send + Sync
>;

/// Registry of host functions
pub type HostFunctionRegistry = HashMap<String, HostFunction>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_value_types() {
        let val = WasmValue::I32(42);
        assert_eq!(val.type_name(), "i32");
        assert!(val.is_numeric());

        let val = WasmValue::String("hello".to_string());
        assert_eq!(val.type_name(), "string");
        assert!(!val.is_numeric());
    }

    #[test]
    fn test_execution_context() {
        let ctx = ExecutionContext::new()
            .with_timeout(Duration::from_secs(60))
            .with_memory_limit(128 * 1024 * 1024)
            .with_caller("test_caller")
            .with_env("KEY", "value")
            .with_property("test", "prop");

        assert_eq!(ctx.timeout, Duration::from_secs(60));
        assert_eq!(ctx.memory_limit, 128 * 1024 * 1024);
        assert_eq!(ctx.caller, Some("test_caller".to_string()));
        assert_eq!(ctx.env.get("KEY"), Some(&"value".to_string()));
        assert_eq!(ctx.properties.get("test"), Some(&"prop".to_string()));
    }

    #[test]
    fn test_module_metadata() {
        let metadata = ModuleMetadata::new("1.0.0".to_string())
            .with_capability("crypto")
            .with_capability("storage")
            .with_tag("experimental");

        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.capabilities, vec!["crypto", "storage"]);
        assert_eq!(metadata.tags, vec!["experimental"]);
    }

    #[test]
    fn test_compilation_target() {
        assert_eq!(CompilationTarget::Native.as_str(), "native");
        assert_eq!(CompilationTarget::Web.as_str(), "web");
        assert_eq!(CompilationTarget::Wasi.as_str(), "wasi");
        assert_eq!(CompilationTarget::Substrate.as_str(), "substrate");
    }
}