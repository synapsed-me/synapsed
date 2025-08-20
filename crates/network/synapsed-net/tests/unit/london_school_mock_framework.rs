//! London School TDD Mock Framework
//! Comprehensive mock framework following interaction-based testing principles

use synapsed_net::error::{NetworkError, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::fmt::Debug;

/// Generic mock expectation system for London School TDD
#[derive(Debug, Clone)]
pub struct MockExpectation {
    pub method_name: String,
    pub expected_parameters: Vec<String>, 
    pub return_value: MockReturnValue,
    pub call_count: usize,
}

#[derive(Debug, Clone)]
pub enum MockReturnValue {
    Success(String),
    Error(String),
    Custom(Box<dyn Fn() -> Result<Vec<u8>> + Send + Sync>),
}

/// Mock interaction recorder for verifying object collaborations
#[derive(Debug, Default)]
pub struct InteractionRecorder {
    interactions: Arc<Mutex<Vec<Interaction>>>,
    expectations: Arc<Mutex<Vec<MockExpectation>>>,
}

#[derive(Debug, Clone)]
pub struct Interaction {
    pub object: String,
    pub method: String,
    pub parameters: Vec<String>,
    pub timestamp: std::time::Instant,
    pub result: String,
}

impl InteractionRecorder {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Records an interaction between objects
    pub fn record_interaction(
        &self, 
        object: &str, 
        method: &str, 
        parameters: Vec<String>,
        result: &str
    ) {
        let interaction = Interaction {
            object: object.to_string(),
            method: method.to_string(),
            parameters,
            timestamp: std::time::Instant::now(),
            result: result.to_string(),
        };
        
        let mut interactions = self.interactions.lock().unwrap();
        interactions.push(interaction);
    }
    
    /// Sets up expectations for method calls
    pub fn expect_call(&self, expectation: MockExpectation) {
        let mut expectations = self.expectations.lock().unwrap();
        expectations.push(expectation);
    }
    
    /// Verifies that all expectations were met
    pub fn verify_expectations(&self) -> Result<()> {
        let interactions = self.interactions.lock().unwrap();
        let expectations = self.expectations.lock().unwrap();
        
        for expectation in expectations.iter() {
            let matching_calls = interactions.iter()
                .filter(|i| i.method == expectation.method_name)
                .count();
                
            if matching_calls != expectation.call_count {
                return Err(NetworkError::Mock(format!(
                    "Expected {} calls to '{}', but got {}",
                    expectation.call_count, expectation.method_name, matching_calls
                )));
            }
        }
        
        Ok(())
    }
    
    /// Gets all recorded interactions
    pub fn get_interactions(&self) -> Vec<Interaction> {
        self.interactions.lock().unwrap().clone()
    }
    
    /// Verifies interaction ordering (London School focus)
    pub fn verify_interaction_order(&self, expected_sequence: Vec<(&str, &str)>) -> Result<()> {
        let interactions = self.interactions.lock().unwrap();
        
        let actual_sequence: Vec<(String, String)> = interactions
            .iter()
            .map(|i| (i.object.clone(), i.method.clone()))
            .collect();
            
        let expected_sequence: Vec<(String, String)> = expected_sequence
            .into_iter()
            .map(|(obj, method)| (obj.to_string(), method.to_string()))
            .collect();
            
        if actual_sequence != expected_sequence {
            return Err(NetworkError::Mock(format!(
                "Expected interaction sequence {:?}, but got {:?}",
                expected_sequence, actual_sequence
            )));
        }
        
        Ok(())
    }
}

/// Mock object trait for implementing mock behaviors
pub trait MockObject: Debug + Send + Sync {
    fn name(&self) -> &str;
    fn record_call(&self, method: &str, parameters: Vec<String>, result: &str);
    fn get_call_count(&self, method: &str) -> usize;
    fn was_called(&self, method: &str) -> bool;
}

/// Generic mock implementation
#[derive(Debug)]
pub struct GenericMock {
    name: String,
    call_counts: Arc<Mutex<HashMap<String, usize>>>,
    recorder: Arc<InteractionRecorder>,
    behaviors: Arc<Mutex<HashMap<String, Box<dyn Fn(&[String]) -> Result<String> + Send + Sync>>>>,
}

impl GenericMock {
    pub fn new(name: &str, recorder: Arc<InteractionRecorder>) -> Self {
        Self {
            name: name.to_string(),
            call_counts: Arc::new(Mutex::new(HashMap::new())),
            recorder,
            behaviors: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Configures mock behavior for a method
    pub fn configure_behavior<F>(&self, method: &str, behavior: F)
    where 
        F: Fn(&[String]) -> Result<String> + Send + Sync + 'static
    {
        let mut behaviors = self.behaviors.lock().unwrap();
        behaviors.insert(method.to_string(), Box::new(behavior));
    }
    
    /// Executes a mock method call
    pub fn call_method(&self, method: &str, parameters: Vec<String>) -> Result<String> {
        // Record the call
        self.record_call(method, parameters.clone(), "pending");
        
        // Execute configured behavior
        let behaviors = self.behaviors.lock().unwrap();
        if let Some(behavior) = behaviors.get(method) {
            let result = behavior(&parameters)?;
            
            // Update interaction record with result
            self.recorder.record_interaction(&self.name, method, parameters, &result);
            
            Ok(result)
        } else {
            let error_msg = format!("No behavior configured for method '{}'", method);
            self.recorder.record_interaction(&self.name, method, parameters, &error_msg);
            Err(NetworkError::Mock(error_msg))
        }
    }
}

impl MockObject for GenericMock {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn record_call(&self, method: &str, parameters: Vec<String>, result: &str) {
        let mut counts = self.call_counts.lock().unwrap();
        *counts.entry(method.to_string()).or_insert(0) += 1;
        
        self.recorder.record_interaction(&self.name, method, parameters, result);
    }
    
    fn get_call_count(&self, method: &str) -> usize {
        let counts = self.call_counts.lock().unwrap();
        counts.get(method).copied().unwrap_or(0)
    }
    
    fn was_called(&self, method: &str) -> bool {
        self.get_call_count(method) > 0
    }
}

/// Mock factory for creating standardized mocks
pub struct MockFactory {
    recorder: Arc<InteractionRecorder>,
}

impl MockFactory {
    pub fn new() -> Self {
        Self {
            recorder: Arc::new(InteractionRecorder::new()),
        }
    }
    
    pub fn get_recorder(&self) -> Arc<InteractionRecorder> {
        self.recorder.clone()
    }
    
    /// Creates a mock certificate validator
    pub fn create_certificate_validator_mock(&self) -> GenericMock {
        let mock = GenericMock::new("CertificateValidator", self.recorder.clone());
        
        // Configure default behaviors
        mock.configure_behavior("validate_chain", |params| {
            if params.is_empty() {
                Err(NetworkError::Mock("Empty certificate chain".to_string()))
            } else {
                Ok("validation_success".to_string())
            }
        });
        
        mock.configure_behavior("add_root_certificate", |_params| {
            Ok("certificate_added".to_string())
        });
        
        mock
    }
    
    /// Creates a mock security manager
    pub fn create_security_manager_mock(&self) -> GenericMock {
        let mock = GenericMock::new("EnhancedSecurityManager", self.recorder.clone());
        
        // Configure default behaviors
        mock.configure_behavior("secure_handshake", |params| {
            if params.len() >= 1 {
                Ok(format!("session_{}", uuid::Uuid::new_v4()))
            } else {
                Err(NetworkError::Mock("Invalid handshake parameters".to_string()))
            }
        });
        
        mock.configure_behavior("encrypt_secure", |params| {
            if params.len() >= 2 {
                Ok(format!("encrypted_{}", params[0]))
            } else {
                Err(NetworkError::Mock("Invalid encryption parameters".to_string()))
            }
        });
        
        mock.configure_behavior("decrypt_secure", |params| {
            if params.len() >= 2 && params[0].starts_with("encrypted_") {
                Ok(params[0].replace("encrypted_", "decrypted_"))
            } else {
                Err(NetworkError::Mock("Invalid decryption parameters".to_string()))
            }
        });
        
        mock
    }
    
    /// Creates a mock transport layer
    pub fn create_transport_mock(&self) -> GenericMock {
        let mock = GenericMock::new("Transport", self.recorder.clone());
        
        mock.configure_behavior("connect", |params| {
            if !params.is_empty() {
                Ok(format!("connection_{}", params[0]))
            } else {
                Err(NetworkError::Mock("No address provided".to_string()))
            }
        });
        
        mock.configure_behavior("send", |params| {
            if params.len() >= 2 {
                Ok("message_sent".to_string())
            } else {
                Err(NetworkError::Mock("Invalid send parameters".to_string()))
            }
        });
        
        mock.configure_behavior("receive", |_params| {
            Ok("message_received".to_string())
        });
        
        mock
    }
}

/// Contract verification for London School TDD
#[derive(Debug)]
pub struct ContractVerifier {
    pub interface_name: String,
    pub required_methods: Vec<String>,
    pub method_signatures: HashMap<String, Vec<String>>,
}

impl ContractVerifier {
    pub fn new(interface_name: &str) -> Self {
        Self {
            interface_name: interface_name.to_string(),
            required_methods: Vec::new(),
            method_signatures: HashMap::new(),
        }
    }
    
    pub fn add_required_method(&mut self, method: &str, signature: Vec<String>) {
        self.required_methods.push(method.to_string());
        self.method_signatures.insert(method.to_string(), signature);
    }
    
    /// Verifies that a mock satisfies the contract
    pub fn verify_contract(&self, mock: &dyn MockObject) -> Result<()> {
        for method in &self.required_methods {
            if !mock.was_called(method) {
                return Err(NetworkError::Mock(format!(
                    "Required method '{}' was not called on {}",
                    method, mock.name()
                )));
            }
        }
        Ok(())
    }
}

/// Test orchestration for complex interaction scenarios
#[derive(Debug)]
pub struct InteractionOrchestrator {
    mocks: HashMap<String, Box<dyn MockObject>>,
    recorder: Arc<InteractionRecorder>,
}

impl InteractionOrchestrator {
    pub fn new(recorder: Arc<InteractionRecorder>) -> Self {
        Self {
            mocks: HashMap::new(),
            recorder,
        }
    }
    
    pub fn add_mock(&mut self, name: String, mock: Box<dyn MockObject>) {
        self.mocks.insert(name, mock);
    }
    
    /// Orchestrates a complex interaction scenario
    pub fn orchestrate_scenario(&self, scenario: &[(&str, &str, Vec<String>)]) -> Result<Vec<String>> {
        let mut results = Vec::new();
        
        for (mock_name, method, params) in scenario {
            if let Some(mock) = self.mocks.get(*mock_name) {
                mock.record_call(method, params.clone(), "orchestrated");
                results.push(format!("{}::{} called", mock_name, method));
            } else {
                return Err(NetworkError::Mock(format!("Mock '{}' not found", mock_name)));
            }
        }
        
        Ok(results)
    }
    
    /// Verifies the orchestrated scenario completed correctly
    pub fn verify_scenario_completion(&self) -> Result<()> {
        self.recorder.verify_expectations()
    }
}

#[cfg(test)]
mod mock_framework_tests {
    use super::*;
    
    #[test]
    fn test_interaction_recorder() {
        let recorder = InteractionRecorder::new();
        
        recorder.record_interaction("MockObject", "method1", vec!["param1".to_string()], "success");
        recorder.record_interaction("MockObject", "method2", vec!["param2".to_string()], "success");
        
        let interactions = recorder.get_interactions();
        assert_eq!(interactions.len(), 2);
        assert_eq!(interactions[0].method, "method1");
        assert_eq!(interactions[1].method, "method2");
    }
    
    #[test]
    fn test_mock_factory() {
        let factory = MockFactory::new();
        let cert_mock = factory.create_certificate_validator_mock();
        
        // Test behavior configuration
        let result = cert_mock.call_method("validate_chain", vec!["test_cert".to_string()]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "validation_success");
        
        // Verify interaction was recorded
        assert!(cert_mock.was_called("validate_chain"));
        assert_eq!(cert_mock.get_call_count("validate_chain"), 1);
    }
    
    #[test]
    fn test_contract_verification() {
        let mut verifier = ContractVerifier::new("TestInterface");
        verifier.add_required_method("required_method", vec!["param".to_string()]);
        
        let factory = MockFactory::new();
        let mock = factory.create_certificate_validator_mock();
        
        // This should fail because required_method wasn't called
        let result = verifier.verify_contract(&mock);
        assert!(result.is_err());
        
        // Call the required method
        let _ = mock.call_method("required_method", vec!["test".to_string()]);
        
        // Now verification should pass
        let result = verifier.verify_contract(&mock);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_interaction_ordering() {
        let recorder = InteractionRecorder::new();
        
        recorder.record_interaction("ObjectA", "method1", vec![], "success");
        recorder.record_interaction("ObjectB", "method2", vec![], "success");
        recorder.record_interaction("ObjectA", "method3", vec![], "success");
        
        // Verify correct ordering
        let result = recorder.verify_interaction_order(vec![
            ("ObjectA", "method1"),
            ("ObjectB", "method2"),
            ("ObjectA", "method3"),
        ]);
        assert!(result.is_ok());
        
        // Verify incorrect ordering fails
        let result = recorder.verify_interaction_order(vec![
            ("ObjectB", "method2"),
            ("ObjectA", "method1"),
        ]);
        assert!(result.is_err());
    }
}

// Extend the error types to include mock-specific errors
impl From<NetworkError> for NetworkError {
    fn from(error: NetworkError) -> Self {
        error
    }
}

// Add mock error variant to NetworkError (would normally be in error.rs)
// This is just for testing - in real implementation, add to the main error enum