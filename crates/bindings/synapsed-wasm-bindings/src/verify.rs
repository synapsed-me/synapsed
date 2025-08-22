//! Verification strategies for WASM

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

/// Verification result for WASM
#[wasm_bindgen]
pub struct WasmVerificationResult {
    verified: bool,
    strategy: String,
    confidence: f64,
    message: String,
    evidence: Vec<String>,
}

#[wasm_bindgen]
impl WasmVerificationResult {
    /// Create a new verification result
    #[wasm_bindgen(constructor)]
    pub fn new(verified: bool, strategy: String, confidence: f64, message: String) -> Self {
        Self {
            verified,
            strategy,
            confidence,
            message,
            evidence: Vec::new(),
        }
    }
    
    #[wasm_bindgen(getter)]
    pub fn verified(&self) -> bool {
        self.verified
    }
    
    #[wasm_bindgen(getter)]
    pub fn strategy(&self) -> String {
        self.strategy.clone()
    }
    
    #[wasm_bindgen(getter)]
    pub fn confidence(&self) -> f64 {
        self.confidence
    }
    
    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.message.clone()
    }
    
    /// Add evidence to the result
    pub fn add_evidence(&mut self, evidence: String) {
        self.evidence.push(evidence);
    }
    
    /// Get all evidence as JSON array
    pub fn get_evidence(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.evidence)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    
    /// Convert to JSON
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let json = serde_json::json!({
            "verified": self.verified,
            "strategy": self.strategy,
            "confidence": self.confidence,
            "message": self.message,
            "evidence": self.evidence,
        });
        
        serde_wasm_bindgen::to_value(&json)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

/// Command verifier for WASM
#[wasm_bindgen]
pub struct WasmCommandVerifier {
    allowed_commands: Vec<String>,
}

#[wasm_bindgen]
impl WasmCommandVerifier {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            allowed_commands: vec![
                "ls".to_string(),
                "pwd".to_string(),
                "echo".to_string(),
                "cat".to_string(),
            ],
        }
    }
    
    /// Add an allowed command
    pub fn allow_command(&mut self, command: String) {
        if !self.allowed_commands.contains(&command) {
            self.allowed_commands.push(command);
        }
    }
    
    /// Verify a command is safe to execute
    pub fn verify_command(&self, command: String) -> WasmVerificationResult {
        // Parse the base command
        let base_cmd = command.split_whitespace()
            .next()
            .unwrap_or("")
            .to_string();
        
        let verified = self.allowed_commands.contains(&base_cmd);
        let confidence = if verified { 1.0 } else { 0.0 };
        let message = if verified {
            format!("Command '{}' is allowed", base_cmd)
        } else {
            format!("Command '{}' is not in allowed list", base_cmd)
        };
        
        let mut result = WasmVerificationResult::new(
            verified,
            "command_allowlist".to_string(),
            confidence,
            message,
        );
        
        if verified {
            result.add_evidence(format!("Command '{}' found in allowlist", base_cmd));
        }
        
        result
    }
}

/// File system verifier for WASM
#[wasm_bindgen]
pub struct WasmFileSystemVerifier {
    expected_files: Vec<String>,
}

#[wasm_bindgen]
impl WasmFileSystemVerifier {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            expected_files: Vec::new(),
        }
    }
    
    /// Expect a file to exist
    pub fn expect_file(&mut self, path: String) {
        if !self.expected_files.contains(&path) {
            self.expected_files.push(path);
        }
    }
    
    /// Verify files exist (mock for WASM)
    pub fn verify_files(&self, existing_files: Vec<String>) -> WasmVerificationResult {
        let mut missing = Vec::new();
        let mut found = Vec::new();
        
        for expected in &self.expected_files {
            if existing_files.contains(expected) {
                found.push(expected.clone());
            } else {
                missing.push(expected.clone());
            }
        }
        
        let verified = missing.is_empty();
        let confidence = found.len() as f64 / self.expected_files.len().max(1) as f64;
        let message = if verified {
            "All expected files exist".to_string()
        } else {
            format!("{} files missing", missing.len())
        };
        
        let mut result = WasmVerificationResult::new(
            verified,
            "filesystem_state".to_string(),
            confidence,
            message,
        );
        
        for file in found {
            result.add_evidence(format!("File exists: {}", file));
        }
        for file in missing {
            result.add_evidence(format!("File missing: {}", file));
        }
        
        result
    }
}

/// API response verifier for WASM
#[wasm_bindgen]
pub struct WasmApiVerifier {
    expected_status: u16,
    required_fields: Vec<String>,
}

#[wasm_bindgen]
impl WasmApiVerifier {
    #[wasm_bindgen(constructor)]
    pub fn new(expected_status: u16) -> Self {
        Self {
            expected_status,
            required_fields: Vec::new(),
        }
    }
    
    /// Add a required field in the response
    pub fn require_field(&mut self, field: String) {
        if !self.required_fields.contains(&field) {
            self.required_fields.push(field);
        }
    }
    
    /// Verify an API response
    pub fn verify_response(&self, status: u16, response_json: JsValue) -> Result<WasmVerificationResult, JsValue> {
        let response: serde_json::Value = serde_wasm_bindgen::from_value(response_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        let status_ok = status == self.expected_status;
        let mut missing_fields = Vec::new();
        let mut found_fields = Vec::new();
        
        for field in &self.required_fields {
            if response.get(field).is_some() {
                found_fields.push(field.clone());
            } else {
                missing_fields.push(field.clone());
            }
        }
        
        let fields_ok = missing_fields.is_empty();
        let verified = status_ok && fields_ok;
        let confidence = if verified { 1.0 } else if status_ok { 0.5 } else { 0.0 };
        
        let message = match (status_ok, fields_ok) {
            (true, true) => "API response verified successfully".to_string(),
            (true, false) => format!("Status OK but {} fields missing", missing_fields.len()),
            (false, true) => format!("Fields OK but status {} != {}", status, self.expected_status),
            (false, false) => format!("Status wrong and {} fields missing", missing_fields.len()),
        };
        
        let mut result = WasmVerificationResult::new(
            verified,
            "api_response".to_string(),
            confidence,
            message,
        );
        
        if status_ok {
            result.add_evidence(format!("Status code {} matches expected", status));
        }
        for field in found_fields {
            result.add_evidence(format!("Field '{}' present", field));
        }
        for field in missing_fields {
            result.add_evidence(format!("Field '{}' missing", field));
        }
        
        Ok(result)
    }
}

/// Composite verifier that runs multiple strategies
#[wasm_bindgen]
pub struct WasmCompositeVerifier {
    results: Vec<WasmVerificationResult>,
}

#[wasm_bindgen]
impl WasmCompositeVerifier {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }
    
    /// Add a verification result
    pub fn add_result(&mut self, result: WasmVerificationResult) {
        self.results.push(result);
    }
    
    /// Get the overall verification result
    pub fn get_overall_result(&self) -> WasmVerificationResult {
        if self.results.is_empty() {
            return WasmVerificationResult::new(
                false,
                "composite".to_string(),
                0.0,
                "No verification strategies run".to_string(),
            );
        }
        
        let total = self.results.len() as f64;
        let verified_count = self.results.iter().filter(|r| r.verified).count() as f64;
        let avg_confidence = self.results.iter().map(|r| r.confidence).sum::<f64>() / total;
        
        let verified = verified_count == total;
        let message = format!(
            "{}/{} strategies passed, {:.1}% average confidence",
            verified_count as u32,
            total as u32,
            avg_confidence * 100.0
        );
        
        let mut result = WasmVerificationResult::new(
            verified,
            "composite".to_string(),
            avg_confidence,
            message,
        );
        
        for r in &self.results {
            result.add_evidence(format!(
                "{}: {} (confidence: {:.1}%)",
                r.strategy,
                if r.verified { "passed" } else { "failed" },
                r.confidence * 100.0
            ));
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    
    #[wasm_bindgen_test]
    fn test_command_verifier() {
        let verifier = WasmCommandVerifier::new();
        
        let result1 = verifier.verify_command("ls -la".to_string());
        assert!(result1.verified());
        
        let result2 = verifier.verify_command("rm -rf /".to_string());
        assert!(!result2.verified());
    }
    
    #[wasm_bindgen_test]
    fn test_filesystem_verifier() {
        let mut verifier = WasmFileSystemVerifier::new();
        verifier.expect_file("/tmp/test.txt".to_string());
        verifier.expect_file("/tmp/data.json".to_string());
        
        let existing = vec!["/tmp/test.txt".to_string()];
        let result = verifier.verify_files(existing);
        
        assert!(!result.verified()); // One file missing
        assert_eq!(result.confidence(), 0.5); // 1 of 2 files found
    }
}