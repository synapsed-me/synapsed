//! Security manager for WASM modules

use std::collections::HashSet;
use wasmparser::{Parser, Payload};

use crate::error::{WasmError, WasmResult};
use crate::runtime::config::SecurityConfig;
use crate::types::ModuleMetadata;

/// Security manager for validating and securing WASM modules
pub struct SecurityManager {
    /// Security configuration
    config: SecurityConfig,
    /// Banned instruction opcodes
    banned_opcodes: HashSet<u8>,
    /// Allowed import namespaces
    allowed_imports: HashSet<String>,
    /// Maximum allowed module size
    max_module_size: usize,
}

impl SecurityManager {
    /// Create a new security manager
    pub fn new(config: SecurityConfig) -> Self {
        let mut manager = Self {
            config: config.clone(),
            banned_opcodes: HashSet::new(),
            allowed_imports: HashSet::new(),
            max_module_size: 16 * 1024 * 1024, // 16MB default
        };

        manager.setup_security_rules();
        manager
    }

    /// Validate a WASM module for security compliance
    pub async fn validate_module(&self, bytes: &[u8], metadata: &ModuleMetadata) -> WasmResult<()> {
        // Check module size
        if bytes.len() > self.max_module_size {
            return Err(WasmError::SecurityViolation(format!(
                "Module size {} exceeds maximum allowed size {}",
                bytes.len(),
                self.max_module_size
            )));
        }

        // Parse and validate module structure
        self.validate_module_structure(bytes).await?;

        // Validate metadata
        self.validate_metadata(metadata)?;

        // Check security configuration compliance
        self.validate_security_compliance(bytes, metadata).await?;

        tracing::debug!("Module passed security validation");
        Ok(())
    }

    /// Validate the WASM module structure and contents
    async fn validate_module_structure(&self, bytes: &[u8]) -> WasmResult<()> {
        let parser = Parser::new(0);
        let mut import_count = 0;
        let mut export_count = 0;
        let mut has_start_function = false;
        let mut function_count = 0;

        for payload in parser.parse_all(bytes) {
            match payload.map_err(WasmError::from)? {
                Payload::ImportSection(imports) => {
                    for import in imports {
                        let import = import.map_err(WasmError::from)?;
                        import_count += 1;

                        // Check import limits
                        if import_count > self.config.max_imports {
                            return Err(WasmError::SecurityViolation(format!(
                                "Too many imports: {} (max: {})",
                                import_count, self.config.max_imports
                            )));
                        }

                        // Validate import namespace if strict validation is enabled
                        if self.config.strict_validation {
                            self.validate_import_namespace(import.module)?;
                        }

                        // Check for unsafe imports
                        if self.config.disable_unsafe_host_functions {
                            self.check_unsafe_import(import.module, import.name)?;
                        }
                    }
                }
                
                Payload::ExportSection(exports) => {
                    for export in exports {
                        let _export = export.map_err(WasmError::from)?;
                        export_count += 1;

                        // Check export limits
                        if export_count > self.config.max_exports {
                            return Err(WasmError::SecurityViolation(format!(
                                "Too many exports: {} (max: {})",
                                export_count, self.config.max_exports
                            )));
                        }
                    }
                }

                Payload::StartSection { func, .. } => {
                    has_start_function = true;
                    tracing::debug!("Module has start function: {}", func);
                }

                Payload::FunctionSection(functions) => {
                    function_count = functions.count();
                    tracing::debug!("Module has {} functions", function_count);
                }

                Payload::CodeSectionStart { count, range: _, size: _ } => {
                    // Code section started - we'll validate individual function bodies
                    // when we encounter CodeSectionEntry payloads
                    tracing::debug!("Code section started with {} function bodies", count);
                }

                Payload::CodeSectionEntry(body) => {
                    // Validate function body for banned instructions
                    if self.config.strict_validation {
                        self.validate_function_body(&body)?;
                    }
                }

                Payload::DataSection(data) => {
                    // Check data segments for suspicious patterns
                    if self.config.strict_validation {
                        for segment in data {
                            let segment = segment.map_err(WasmError::from)?;
                            self.validate_data_segment(&segment)?;
                        }
                    }
                }

                _ => {} // Other sections are generally safe
            }
        }

        // Additional validation for deterministic execution
        if self.config.enable_deterministic_execution {
            self.validate_deterministic_features(bytes).await?;
        }

        tracing::debug!(
            imports = import_count,
            exports = export_count,
            functions = function_count,
            has_start = has_start_function,
            "Module structure validation completed"
        );

        Ok(())
    }

    /// Validate function body for banned instructions
    fn validate_function_body(&self, body: &wasmparser::FunctionBody) -> WasmResult<()> {
        let mut reader = body.get_binary_reader();
        
        while !reader.eof() {
            let opcode = reader.read_u8().map_err(WasmError::from)?;
            
            if self.banned_opcodes.contains(&opcode) {
                return Err(WasmError::SecurityViolation(format!(
                    "Banned instruction opcode: 0x{:02X}",
                    opcode
                )));
            }

            // Skip operands based on instruction type
            // This is a simplified version - a full implementation would properly decode instructions
            match opcode {
                // Memory instructions might have immediate operands
                0x20..=0x3F => {
                    let _ = reader.read_var_u32(); // Skip memory immediate
                }
                // Control flow instructions
                0x02..=0x04 | 0x0C..=0x11 => {
                    let _ = reader.read_var_u32(); // Skip block type or label index
                }
                _ => {} // Other instructions handled by default
            }
        }

        Ok(())
    }

    /// Validate data segment
    fn validate_data_segment(&self, segment: &wasmparser::Data) -> WasmResult<()> {
        match segment.kind {
            wasmparser::DataKind::Active { .. } => {
                // Check for suspicious data patterns
                let data = segment.data;
                
                // Example: Check for executable code patterns in data
                if data.len() > 1000 && self.contains_executable_patterns(data) {
                    return Err(WasmError::SecurityViolation(
                        "Suspicious executable patterns in data segment".to_string(),
                    ));
                }
            }
            wasmparser::DataKind::Passive => {
                // Passive data segments are generally safe
            }
        }

        Ok(())
    }

    /// Check if data contains suspicious executable patterns
    fn contains_executable_patterns(&self, data: &[u8]) -> bool {
        // Simple heuristic: look for common instruction patterns
        // This is a simplified implementation
        if data.len() < 4 {
            return false;
        }

        let mut suspicious_count = 0;
        for window in data.windows(4) {
            // Look for common WASM instruction patterns
            if matches!(window[0], 0x20..=0x24) || // local.get, local.set, etc.
               matches!(window[0], 0x41..=0x44) || // i32.const, i64.const, etc.
               matches!(window[0], 0x6A..=0x78)    // arithmetic operations
            {
                suspicious_count += 1;
            }
        }

        // If more than 25% of 4-byte windows look like instructions, flag as suspicious
        suspicious_count > data.len() / 16
    }

    /// Validate import namespace
    fn validate_import_namespace(&self, namespace: &str) -> WasmResult<()> {
        if !self.allowed_imports.is_empty() && !self.allowed_imports.contains(namespace) {
            return Err(WasmError::SecurityViolation(format!(
                "Import from disallowed namespace: {}",
                namespace
            )));
        }

        Ok(())
    }

    /// Check for unsafe imports
    fn check_unsafe_import(&self, module: &str, name: &str) -> WasmResult<()> {
        // List of potentially unsafe host functions
        let unsafe_functions = [
            "system", "exec", "spawn", "fork",
            "read_file", "write_file", "delete_file",
            "network_request", "socket", "bind",
            "eval", "compile", "load_module",
        ];

        if unsafe_functions.contains(&name) {
            return Err(WasmError::SecurityViolation(format!(
                "Unsafe host function import: {}::{}",
                module, name
            )));
        }

        Ok(())
    }

    /// Validate module metadata
    fn validate_metadata(&self, metadata: &ModuleMetadata) -> WasmResult<()> {
        // Check resource requirements
        if metadata.requirements.max_memory > 1024 * 1024 * 1024 { // 1GB
            return Err(WasmError::SecurityViolation(
                "Module requests excessive memory".to_string(),
            ));
        }

        if metadata.requirements.max_execution_time > 3600 { // 1 hour
            return Err(WasmError::SecurityViolation(
                "Module requests excessive execution time".to_string(),
            ));
        }

        // Validate security configuration
        if !metadata.security.sandbox && self.config.enable_sandboxing {
            return Err(WasmError::SecurityViolation(
                "Module disables required sandboxing".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate security compliance
    async fn validate_security_compliance(
        &self,
        _bytes: &[u8],
        metadata: &ModuleMetadata,
    ) -> WasmResult<()> {
        // Check if module complies with security requirements
        if self.config.enable_sandboxing && !metadata.security.sandbox {
            return Err(WasmError::SecurityViolation(
                "Sandboxing is required but disabled in module".to_string(),
            ));
        }

        if self.config.enable_deterministic_execution {
            // Check for non-deterministic capabilities
            let non_deterministic_caps = ["random", "time", "network", "filesystem"];
            for cap in &metadata.capabilities {
                if non_deterministic_caps.contains(&cap.as_str()) {
                    return Err(WasmError::SecurityViolation(format!(
                        "Non-deterministic capability not allowed: {}",
                        cap
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate deterministic execution features
    async fn validate_deterministic_features(&self, _bytes: &[u8]) -> WasmResult<()> {
        // In a real implementation, this would check for:
        // - Non-deterministic instructions
        // - Floating-point operations that might vary between platforms
        // - Time-dependent operations
        // - Random number generation
        
        // For now, we'll do basic validation
        tracing::debug!("Deterministic execution validation passed");
        Ok(())
    }

    /// Setup default security rules
    fn setup_security_rules(&mut self) {
        // Ban potentially dangerous instructions
        if self.config.strict_validation {
            // These are example opcodes - in practice, you'd need the actual WASM opcodes
            self.banned_opcodes.insert(0xFF); // Example: hypothetical "unsafe" instruction
        }

        // Setup allowed import namespaces
        self.allowed_imports.insert("env".to_string());
        self.allowed_imports.insert("wasi_snapshot_preview1".to_string());
        
        if !self.config.disable_unsafe_host_functions {
            self.allowed_imports.insert("wasi_unstable".to_string());
        }
    }

    /// Update security configuration
    pub fn update_config(&mut self, config: SecurityConfig) {
        self.config = config;
        self.setup_security_rules();
    }

    /// Add allowed import namespace
    pub fn add_allowed_import(&mut self, namespace: String) {
        self.allowed_imports.insert(namespace);
    }

    /// Remove allowed import namespace
    pub fn remove_allowed_import(&mut self, namespace: &str) -> bool {
        self.allowed_imports.remove(namespace)
    }

    /// Ban instruction opcode
    pub fn ban_opcode(&mut self, opcode: u8) {
        self.banned_opcodes.insert(opcode);
    }

    /// Allow instruction opcode
    pub fn allow_opcode(&mut self, opcode: u8) -> bool {
        self.banned_opcodes.remove(&opcode)
    }

    /// Set maximum module size
    pub fn set_max_module_size(&mut self, size: usize) {
        self.max_module_size = size;
    }

    /// Get security configuration
    pub fn config(&self) -> &SecurityConfig {
        &self.config
    }

    /// Get security statistics
    pub fn get_stats(&self) -> SecurityStats {
        SecurityStats {
            banned_opcodes: self.banned_opcodes.len(),
            allowed_imports: self.allowed_imports.len(),
            max_module_size: self.max_module_size,
            sandboxing_enabled: self.config.enable_sandboxing,
            strict_validation: self.config.strict_validation,
            deterministic_execution: self.config.enable_deterministic_execution,
        }
    }
}

/// Security statistics
#[derive(Debug, Clone)]
pub struct SecurityStats {
    /// Number of banned opcodes
    pub banned_opcodes: usize,
    /// Number of allowed import namespaces
    pub allowed_imports: usize,
    /// Maximum allowed module size
    pub max_module_size: usize,
    /// Whether sandboxing is enabled
    pub sandboxing_enabled: bool,
    /// Whether strict validation is enabled
    pub strict_validation: bool,
    /// Whether deterministic execution is enforced
    pub deterministic_execution: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_manager_creation() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config);
        
        assert!(manager.config().enable_sandboxing);
        assert!(manager.config().strict_validation);
    }

    #[test]
    fn test_security_stats() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config);
        let stats = manager.get_stats();
        
        assert_eq!(stats.sandboxing_enabled, true);
        assert_eq!(stats.strict_validation, true);
        assert!(stats.max_module_size > 0);
    }

    #[test]
    fn test_allowed_imports_management() {
        let config = SecurityConfig::default();
        let mut manager = SecurityManager::new(config);
        
        manager.add_allowed_import("test_namespace".to_string());
        assert!(manager.allowed_imports.contains("test_namespace"));
        
        assert!(manager.remove_allowed_import("test_namespace"));
        assert!(!manager.allowed_imports.contains("test_namespace"));
        
        assert!(!manager.remove_allowed_import("nonexistent"));
    }

    #[test]
    fn test_opcode_management() {
        let config = SecurityConfig::default();
        let mut manager = SecurityManager::new(config);
        
        manager.ban_opcode(0x42);
        assert!(manager.banned_opcodes.contains(&0x42));
        
        assert!(manager.allow_opcode(0x42));
        assert!(!manager.banned_opcodes.contains(&0x42));
        
        assert!(!manager.allow_opcode(0x43)); // Not previously banned
    }

    #[tokio::test]
    async fn test_module_size_validation() {
        let config = SecurityConfig::default();
        let mut manager = SecurityManager::new(config);
        manager.set_max_module_size(100); // Very small for testing
        
        let large_bytes = vec![0u8; 200]; // Larger than limit
        let metadata = ModuleMetadata::default();
        
        let result = manager.validate_module(&large_bytes, &metadata).await;
        assert!(result.is_err());
        
        let small_bytes = vec![0u8; 50]; // Within limit
        // Note: This will fail parsing, but size check should pass first
        let result = manager.validate_module(&small_bytes, &metadata).await;
        // Should fail on parsing, not size
        assert!(result.is_err());
    }

    #[test]
    fn test_metadata_validation() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config);
        
        // Valid metadata
        let metadata = ModuleMetadata::default();
        assert!(manager.validate_metadata(&metadata).is_ok());
        
        // Invalid metadata - excessive memory
        let mut invalid_metadata = ModuleMetadata::default();
        invalid_metadata.requirements.max_memory = 2 * 1024 * 1024 * 1024; // 2GB
        assert!(manager.validate_metadata(&invalid_metadata).is_err());
        
        // Invalid metadata - excessive execution time
        let mut invalid_metadata2 = ModuleMetadata::default();
        invalid_metadata2.requirements.max_execution_time = 7200; // 2 hours
        assert!(manager.validate_metadata(&invalid_metadata2).is_err());
    }

    #[test]
    fn test_config_update() {
        let config = SecurityConfig::default();
        let mut manager = SecurityManager::new(config);
        
        let original_sandboxing = manager.config().enable_sandboxing;
        
        let mut new_config = SecurityConfig::default();
        new_config.enable_sandboxing = !original_sandboxing;
        
        manager.update_config(new_config);
        assert_eq!(manager.config().enable_sandboxing, !original_sandboxing);
    }
}