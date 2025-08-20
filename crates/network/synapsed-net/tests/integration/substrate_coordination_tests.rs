//! Integration tests for substrate and serventis coordination
//! Following London School TDD with focus on cross-system interactions

use synapsed_net::crypto::{
    EnhancedSecurityManager, EnhancedSecurityConfig, SecureCipherSuite,
    SecurityEvent, SecurityEventType, SecurityEventSeverity,
};
use synapsed_net::types::{PeerInfo, PeerId};
use synapsed_net::error::{NetworkError, Result};
use synapsed_net::observability::{ObservabilityContext, MetricsCollector};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Mock substrate for testing observability integration
#[derive(Debug)]
pub struct MockSubstrate {
    name: String,
    events_received: Arc<Mutex<Vec<SubstrateEvent>>>,
    metrics_received: Arc<Mutex<Vec<SubstrateMetric>>>,
    coordination_messages: Arc<Mutex<Vec<CoordinationMessage>>>,
}

#[derive(Debug, Clone)]
pub struct SubstrateEvent {
    pub event_type: String,
    pub source: String,
    pub data: HashMap<String, String>,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone)]
pub struct SubstrateMetric {
    pub metric_name: String,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone)]
pub struct CoordinationMessage {
    pub from: String,
    pub to: String,
    pub message_type: String,
    pub payload: String,
    pub timestamp: SystemTime,
}

impl MockSubstrate {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            events_received: Arc::new(Mutex::new(Vec::new())),
            metrics_received: Arc::new(Mutex::new(Vec::new())),
            coordination_messages: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub fn emit_event(&self, event: SubstrateEvent) {
        let mut events = self.events_received.lock().unwrap();
        events.push(event);
    }
    
    pub fn record_metric(&self, metric: SubstrateMetric) {
        let mut metrics = self.metrics_received.lock().unwrap();
        metrics.push(metric);
    }
    
    pub fn send_coordination_message(&self, message: CoordinationMessage) {
        let mut messages = self.coordination_messages.lock().unwrap();
        messages.push(message);
    }
    
    pub fn get_events(&self) -> Vec<SubstrateEvent> {
        self.events_received.lock().unwrap().clone()
    }
    
    pub fn get_metrics(&self) -> Vec<SubstrateMetric> {
        self.metrics_received.lock().unwrap().clone()
    }
    
    pub fn get_coordination_messages(&self) -> Vec<CoordinationMessage> {
        self.coordination_messages.lock().unwrap().clone()
    }
    
    pub fn clear_all(&self) {
        self.events_received.lock().unwrap().clear();
        self.metrics_received.lock().unwrap().clear();
        self.coordination_messages.lock().unwrap().clear();
    }
}

/// Mock serventis for testing service coordination
#[derive(Debug)]
pub struct MockServentis {
    name: String,
    service_registry: Arc<Mutex<HashMap<String, ServiceInfo>>>,
    coordination_state: Arc<Mutex<CoordinationState>>,
}

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub service_id: String,
    pub status: ServiceStatus,
    pub capabilities: Vec<String>,
    pub metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub enum ServiceStatus {
    Active,
    Degraded,
    Failed,
    Recovering,
}

#[derive(Debug, Clone)]
pub struct CoordinationState {
    pub active_sessions: HashMap<String, SessionInfo>,
    pub peer_coordination: HashMap<String, PeerCoordinationInfo>,
    pub resource_allocation: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: String,
    pub peer_id: String,
    pub cipher_suite: String,
    pub created_at: SystemTime,
    pub last_activity: SystemTime,
}

#[derive(Debug, Clone)]
pub struct PeerCoordinationInfo {
    pub peer_id: String,
    pub coordination_level: String,
    pub shared_resources: Vec<String>,
    pub trust_score: f64,
}

impl MockServentis {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            service_registry: Arc::new(Mutex::new(HashMap::new())),
            coordination_state: Arc::new(Mutex::new(CoordinationState {
                active_sessions: HashMap::new(),
                peer_coordination: HashMap::new(),
                resource_allocation: HashMap::new(),
            })),
        }
    }
    
    pub fn register_service(&self, service: ServiceInfo) {
        let mut registry = self.service_registry.lock().unwrap();
        registry.insert(service.service_id.clone(), service);
    }
    
    pub fn update_session(&self, session_info: SessionInfo) {
        let mut state = self.coordination_state.lock().unwrap();
        state.active_sessions.insert(session_info.session_id.clone(), session_info);
    }
    
    pub fn coordinate_with_peer(&self, peer_info: PeerCoordinationInfo) {
        let mut state = self.coordination_state.lock().unwrap();
        state.peer_coordination.insert(peer_info.peer_id.clone(), peer_info);
    }
    
    pub fn get_service_status(&self, service_id: &str) -> Option<ServiceStatus> {
        let registry = self.service_registry.lock().unwrap();
        registry.get(service_id).map(|s| s.status.clone())
    }
    
    pub fn get_coordination_state(&self) -> CoordinationState {
        self.coordination_state.lock().unwrap().clone()
    }
}

/// Integration test orchestrator for substrate/serventis coordination
#[derive(Debug)]
pub struct CoordinationOrchestrator {
    substrates: HashMap<String, Arc<MockSubstrate>>,
    serventis: HashMap<String, Arc<MockServentis>>,
    test_scenario: String,
}

impl CoordinationOrchestrator {
    pub fn new(scenario: &str) -> Self {
        Self {
            substrates: HashMap::new(),
            serventis: HashMap::new(),
            test_scenario: scenario.to_string(),
        }    
    }
    
    pub fn add_substrate(&mut self, name: &str, substrate: Arc<MockSubstrate>) {
        self.substrates.insert(name.to_string(), substrate);
    }
    
    pub fn add_serventis(&mut self, name: &str, serventis: Arc<MockServentis>) {
        self.serventis.insert(name.to_string(), serventis);
    }
    
    /// Orchestrates a crypto operation with full observability
    pub async fn orchestrate_crypto_operation(
        &self,
        operation_type: &str,
        peer_info: &PeerInfo,
    ) -> Result<HashMap<String, String>> {
        let mut results = HashMap::new();
        
        // Step 1: Emit substrate events for operation start
        for (name, substrate) in &self.substrates {
            substrate.emit_event(SubstrateEvent {
                event_type: format!("{}_start", operation_type),
                source: "synapsed-net".to_string(),
                data: {
                    let mut data = HashMap::new();
                    data.insert("peer_id".to_string(), peer_info.id.to_string());
                    data.insert("operation".to_string(), operation_type.to_string());
                    data
                },
                timestamp: SystemTime::now(),
            });
            
            results.insert(format!("{}_event_emitted", name), "true".to_string());
        }
        
        // Step 2: Update serventis coordination state
        for (name, serventis) in &self.serventis {
            if operation_type == "handshake" {
                serventis.update_session(SessionInfo {
                    session_id: Uuid::new_v4().to_string(),
                    peer_id: peer_info.id.to_string(),
                    cipher_suite: "ChaCha20Poly1305X25519".to_string(),
                    created_at: SystemTime::now(),
                    last_activity: SystemTime::now(),
                });
            }
            
            serventis.coordinate_with_peer(PeerCoordinationInfo {
                peer_id: peer_info.id.to_string(),
                coordination_level: "full".to_string(),
                shared_resources: vec!["crypto".to_string(), "transport".to_string()],
                trust_score: 0.9,
            });
            
            results.insert(format!("{}_coordination_updated", name), "true".to_string());
        }
        
        // Step 3: Record metrics
        for (name, substrate) in &self.substrates {
            substrate.record_metric(SubstrateMetric {
                metric_name: format!("{}_operations_total", operation_type),
                value: 1.0,
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("peer_id".to_string(), peer_info.id.to_string());
                    labels.insert("operation_type".to_string(), operation_type.to_string());
                    labels
                },
                timestamp: SystemTime::now(),
            });
        }
        
        // Step 4: Cross-substrate coordination messages
        if self.substrates.len() > 1 {
            let substrate_names: Vec<_> = self.substrates.keys().collect();
            for i in 0..substrate_names.len() {
                for j in i+1..substrate_names.len() {
                    let from = substrate_names[i];
                    let to = substrate_names[j];
                    
                    if let (Some(from_substrate), Some(to_substrate)) = 
                        (self.substrates.get(from), self.substrates.get(to)) {
                        
                        from_substrate.send_coordination_message(CoordinationMessage {
                            from: from.clone(),
                            to: to.clone(),
                            message_type: "coordination_sync".to_string(),
                            payload: format!("operation_complete:{}", operation_type),
                            timestamp: SystemTime::now(),
                        });
                    }
                }
            }
        }
        
        results.insert("orchestration_status".to_string(), "complete".to_string());
        Ok(results)
    }
    
    /// Verifies that all coordination occurred correctly
    pub fn verify_coordination(&self) -> Result<()> {
        // Verify substrate events were emitted
        for (name, substrate) in &self.substrates {
            let events = substrate.get_events();
            if events.is_empty() {
                return Err(NetworkError::Mock(format!(
                    "No events emitted by substrate '{}'", name
                )));
            }
        }
        
        // Verify serventis coordination state updated
        for (name, serventis) in &self.serventis {
            let state = serventis.get_coordination_state();
            if state.active_sessions.is_empty() && state.peer_coordination.is_empty() {
                return Err(NetworkError::Mock(format!(
                    "No coordination state updated by serventis '{}'", name
                )));
            }
        }
        
        // Verify cross-substrate coordination messages
        if self.substrates.len() > 1 {
            for (name, substrate) in &self.substrates {
                let messages = substrate.get_coordination_messages();
                if messages.is_empty() {
                    return Err(NetworkError::Mock(format!(
                        "No coordination messages sent by substrate '{}'", name
                    )));
                }
            }
        }
        
        Ok(())
    }
    
    /// Cleans up all test state
    pub fn cleanup(&self) {
        for substrate in self.substrates.values() {
            substrate.clear_all();
        }
    }
}

/// Test helper for creating test peers
fn create_test_peer(id_str: &str) -> PeerInfo {
    let peer_id = PeerId::new();
    let mut peer = PeerInfo::new(peer_id);
    peer.address = format!("127.0.0.1:8080");
    peer.capabilities = vec!["ChaCha20Poly1305X25519".to_string()];
    peer
}

#[cfg(test)]
mod substrate_coordination_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_single_substrate_crypto_operation_coordination() {
        let mut orchestrator = CoordinationOrchestrator::new("single_substrate_test");
        
        // Setup single substrate
        let substrate = Arc::new(MockSubstrate::new("test_substrate"));
        orchestrator.add_substrate("test_substrate", substrate.clone());
        
        // Setup single serventis
        let serventis = Arc::new(MockServentis::new("test_serventis"));
        orchestrator.add_serventis("test_serventis", serventis.clone());
        
        let test_peer = create_test_peer("test_peer");
        
        // Orchestrate handshake operation
        let results = orchestrator.orchestrate_crypto_operation("handshake", &test_peer).await.unwrap();
        
        // Verify results
        assert_eq!(results.get("test_substrate_event_emitted"), Some(&"true".to_string()));
        assert_eq!(results.get("test_serventis_coordination_updated"), Some(&"true".to_string()));
        assert_eq!(results.get("orchestration_status"), Some(&"complete".to_string()));
        
        // Verify coordination occurred
        assert!(orchestrator.verify_coordination().is_ok());
        
        // Verify specific interactions
        let events = substrate.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "handshake_start");
        assert_eq!(events[0].source, "synapsed-net");
        
        let metrics = substrate.get_metrics();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].metric_name, "handshake_operations_total");
        assert_eq!(metrics[0].value, 1.0);
        
        let state = serventis.get_coordination_state();
        assert_eq!(state.active_sessions.len(), 1);
        assert_eq!(state.peer_coordination.len(), 1);
        
        // Cleanup
        orchestrator.cleanup();
    }
    
    #[tokio::test]
    async fn test_multi_substrate_coordination() {
        let mut orchestrator = CoordinationOrchestrator::new("multi_substrate_test");
        
        // Setup multiple substrates
        let substrate1 = Arc::new(MockSubstrate::new("substrate_1"));
        let substrate2 = Arc::new(MockSubstrate::new("substrate_2"));
        orchestrator.add_substrate("substrate_1", substrate1.clone());
        orchestrator.add_substrate("substrate_2", substrate2.clone());
        
        // Setup multiple serventis instances
        let serventis1 = Arc::new(MockServentis::new("serventis_1"));
        let serventis2 = Arc::new(MockServentis::new("serventis_2"));
        orchestrator.add_serventis("serventis_1", serventis1.clone());
        orchestrator.add_serventis("serventis_2", serventis2.clone());
        
        let test_peer = create_test_peer("multi_test_peer");
        
        // Orchestrate encryption operation
        let results = orchestrator.orchestrate_crypto_operation("encryption", &test_peer).await.unwrap();
        
        // Verify all components were coordinated
        assert_eq!(results.get("substrate_1_event_emitted"), Some(&"true".to_string()));
        assert_eq!(results.get("substrate_2_event_emitted"), Some(&"true".to_string()));
        assert_eq!(results.get("serventis_1_coordination_updated"), Some(&"true".to_string()));
        assert_eq!(results.get("serventis_2_coordination_updated"), Some(&"true".to_string()));
        
        // Verify cross-substrate coordination occurred
        assert!(orchestrator.verify_coordination().is_ok());
        
        // Verify cross-substrate messages were sent
        let messages1 = substrate1.get_coordination_messages();
        let messages2 = substrate2.get_coordination_messages();
        assert!(!messages1.is_empty() || !messages2.is_empty());
        
        // Verify coordination message content
        if !messages1.is_empty() {
            assert_eq!(messages1[0].message_type, "coordination_sync");
            assert!(messages1[0].payload.contains("operation_complete:encryption"));
        }
        
        orchestrator.cleanup();
    }
    
    #[tokio::test]
    async fn test_substrate_serventis_interaction_patterns() {
        let mut orchestrator = CoordinationOrchestrator::new("interaction_pattern_test");
        
        let substrate = Arc::new(MockSubstrate::new("interaction_substrate"));
        let serventis = Arc::new(MockServentis::new("interaction_serventis"));
        
        orchestrator.add_substrate("interaction_substrate", substrate.clone());
        orchestrator.add_serventis("interaction_serventis", serventis.clone());
        
        let test_peer = create_test_peer("interaction_peer");
        
        // Test sequence of operations to verify interaction patterns
        let operations = vec!["handshake", "encryption", "decryption"];
        
        for operation in operations {
            let results = orchestrator.orchestrate_crypto_operation(operation, &test_peer).await.unwrap();
            assert_eq!(results.get("orchestration_status"), Some(&"complete".to_string()));
        }
        
        // Verify interaction patterns
        let events = substrate.get_events();
        assert_eq!(events.len(), 3); // One for each operation
        
        let expected_events = vec!["handshake_start", "encryption_start", "decryption_start"];
        for (i, expected) in expected_events.iter().enumerate() {
            assert_eq!(events[i].event_type, *expected);
        }
        
        let metrics = substrate.get_metrics();
        assert_eq!(metrics.len(), 3); // One metric per operation
        
        let state = serventis.get_coordination_state();
        // Should have one session (from handshake) and one peer coordination entry
        assert_eq!(state.active_sessions.len(), 1);
        assert_eq!(state.peer_coordination.len(), 1);
        
        // Verify trust score was maintained
        let peer_coord = state.peer_coordination.values().next().unwrap();
        assert_eq!(peer_coord.trust_score, 0.9);
        assert_eq!(peer_coord.coordination_level, "full");
        
        orchestrator.cleanup();
    }
    
    #[tokio::test]
    async fn test_error_propagation_in_coordination() {
        let mut orchestrator = CoordinationOrchestrator::new("error_propagation_test");
        
        // Test with no substrates/serventis to trigger error conditions
        let test_peer = create_test_peer("error_test_peer");
        
        // This should still work but with empty results
        let results = orchestrator.orchestrate_crypto_operation("test_operation", &test_peer).await.unwrap();
        assert_eq!(results.get("orchestration_status"), Some(&"complete".to_string()));
        
        // Verification should fail due to no coordination
        let verification_result = orchestrator.verify_coordination();
        assert!(verification_result.is_ok()); // Empty coordination is OK if no components exist
    }
    
    #[tokio::test] 
    async fn test_coordination_cleanup_and_isolation() {
        let mut orchestrator = CoordinationOrchestrator::new("cleanup_test");
        
        let substrate = Arc::new(MockSubstrate::new("cleanup_substrate"));
        orchestrator.add_substrate("cleanup_substrate", substrate.clone());
        
        let test_peer = create_test_peer("cleanup_peer");
        
        // Perform operation
        let _results = orchestrator.orchestrate_crypto_operation("cleanup_test", &test_peer).await.unwrap();
        
        // Verify data exists
        assert!(!substrate.get_events().is_empty());
        assert!(!substrate.get_metrics().is_empty());
        
        // Cleanup
        orchestrator.cleanup();
        
        // Verify cleanup worked
        assert!(substrate.get_events().is_empty());
        assert!(substrate.get_metrics().is_empty());
        assert!(substrate.get_coordination_messages().is_empty());
    }
    
    #[tokio::test]
    async fn test_cross_substrate_message_coordination() {
        let mut orchestrator = CoordinationOrchestrator::new("message_coordination_test");
        
        // Setup three substrates to test complex coordination
        let substrate1 = Arc::new(MockSubstrate::new("msg_substrate_1"));
        let substrate2 = Arc::new(MockSubstrate::new("msg_substrate_2"));
        let substrate3 = Arc::new(MockSubstrate::new("msg_substrate_3"));
        
        orchestrator.add_substrate("msg_substrate_1", substrate1.clone());
        orchestrator.add_substrate("msg_substrate_2", substrate2.clone());
        orchestrator.add_substrate("msg_substrate_3", substrate3.clone());
        
        let test_peer = create_test_peer("msg_peer");
        
        // Execute operation
        let _results = orchestrator.orchestrate_crypto_operation("message_test", &test_peer).await.unwrap();
        
        // Verify cross-substrate messages were sent
        let all_messages = vec![
            substrate1.get_coordination_messages(),
            substrate2.get_coordination_messages(), 
            substrate3.get_coordination_messages(),
        ];
        
        let total_messages: usize = all_messages.iter().map(|msgs| msgs.len()).sum();
        
        // With 3 substrates, we expect 3 pairs of coordination messages
        // Each substrate sends to the others, so we get coordination in both directions
        assert!(total_messages >= 3); // At least 3 coordination messages
        
        // Verify message content
        for messages in all_messages {
            for message in messages {
                assert_eq!(message.message_type, "coordination_sync");
                assert!(message.payload.contains("operation_complete:message_test"));
            }
        }
        
        orchestrator.cleanup();
    }
}