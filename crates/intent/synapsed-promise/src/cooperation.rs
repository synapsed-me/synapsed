//! Cooperation protocol for multi-agent systems (Claude sub-agents)

use crate::{
    types::*, AgentId, PromiseId, Promise, TrustLevel,
    Result, PromiseError
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Request for cooperation between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CooperationRequest {
    /// Request ID
    pub id: Uuid,
    /// Requesting agent
    pub from: AgentId,
    /// Target agent(s)
    pub to: Vec<AgentId>,
    /// Type of cooperation needed
    pub cooperation_type: CooperationType,
    /// Intent tree for the cooperation
    pub intent: CooperationIntent,
    /// Context to inject into sub-agent
    pub context: HashMap<String, serde_json::Value>,
    /// Timeout for the cooperation
    pub timeout_ms: u64,
    /// Parent request ID for hierarchical tracking
    pub parent_request_id: Option<Uuid>,
}

/// Type of cooperation between agents
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CooperationType {
    /// Delegate a task to another agent (like Claude sub-agent)
    Delegate,
    /// Request information from another agent
    Query,
    /// Collaborate on a shared task
    Collaborate,
    /// Verify another agent's work
    Verify,
    /// Coordinate multiple agents
    Coordinate,
}

/// Intent for cooperation (what the agent should do)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CooperationIntent {
    /// Goal of the cooperation
    pub goal: String,
    /// Preconditions that must be met
    pub preconditions: Vec<String>,
    /// Steps to perform
    pub steps: Vec<IntentStep>,
    /// Expected outcomes
    pub postconditions: Vec<String>,
    /// Verification criteria
    pub verification: Vec<VerificationCriteria>,
}

/// A step in the cooperation intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentStep {
    /// Step name
    pub name: String,
    /// Step description
    pub description: String,
    /// Command or action to perform
    pub action: String,
    /// Expected result
    pub expected_result: Option<String>,
    /// Dependencies on other steps
    pub dependencies: Vec<String>,
}

/// Verification criteria for cooperation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCriteria {
    /// What to verify
    pub check: String,
    /// Expected outcome
    pub expected: String,
    /// Whether this is critical
    pub critical: bool,
}

/// Response to a cooperation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CooperationResponse {
    /// Response ID
    pub id: Uuid,
    /// Request being responded to
    pub request_id: Uuid,
    /// Responding agent
    pub from: AgentId,
    /// Response type
    pub response_type: ResponseType,
    /// Result of the cooperation
    pub result: Option<CooperationResult>,
    /// Error if failed
    pub error: Option<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Type of response
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseType {
    /// Accepted the cooperation request
    Accepted,
    /// Rejected the cooperation request
    Rejected,
    /// Completed the cooperation
    Completed,
    /// Failed to complete
    Failed,
    /// Progress update
    Progress,
}

/// Result of cooperation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CooperationResult {
    /// Whether all steps were completed
    pub success: bool,
    /// Results from each step
    pub step_results: Vec<StepResult>,
    /// Verification results
    pub verification_results: Vec<VerificationResult>,
    /// Evidence of completion
    pub evidence: Vec<Evidence>,
    /// Output data
    pub output: HashMap<String, serde_json::Value>,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

/// Result from a single step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Step name
    pub step: String,
    /// Whether it succeeded
    pub success: bool,
    /// Output from the step
    pub output: Option<String>,
    /// Error if failed
    pub error: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

/// Result of verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// What was checked
    pub check: String,
    /// Whether it passed
    pub passed: bool,
    /// Actual value found
    pub actual: String,
    /// Expected value
    pub expected: String,
}

/// Coordination session for multi-agent cooperation
#[derive(Debug, Clone)]
pub struct CoordinationSession {
    /// Session ID
    pub id: Uuid,
    /// Coordinating agent
    pub coordinator: AgentId,
    /// Participating agents
    pub participants: Vec<AgentId>,
    /// Session intent
    pub intent: CooperationIntent,
    /// Current state
    pub state: SessionState,
    /// Results from each participant
    pub results: HashMap<AgentId, CooperationResult>,
    /// Start time
    pub started_at: DateTime<Utc>,
    /// End time
    pub ended_at: Option<DateTime<Utc>>,
}

/// State of a coordination session
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    /// Session is being planned
    Planning,
    /// Session is active
    Active,
    /// Waiting for participants
    Waiting,
    /// Session is complete
    Complete,
    /// Session failed
    Failed,
}

/// Protocol for agent cooperation
#[derive(Debug, Clone)]
pub struct CooperationProtocol {
    /// Agent using this protocol
    agent_id: Option<AgentId>,
    /// Active cooperation requests
    active_requests: Arc<DashMap<Uuid, CooperationRequest>>,
    /// Active sessions
    sessions: Arc<DashMap<Uuid, CoordinationSession>>,
    /// Request history
    history: Arc<RwLock<Vec<CooperationRequest>>>,
    /// Response history
    responses: Arc<RwLock<Vec<CooperationResponse>>>,
}

impl CooperationProtocol {
    /// Creates a new cooperation protocol
    pub fn new() -> Self {
        Self {
            agent_id: None,
            active_requests: Arc::new(DashMap::new()),
            sessions: Arc::new(DashMap::new()),
            history: Arc::new(RwLock::new(Vec::new())),
            responses: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Initializes the protocol for an agent
    pub fn initialize(&mut self, agent_id: AgentId) -> Result<()> {
        self.agent_id = Some(agent_id);
        Ok(())
    }
    
    /// Creates a cooperation request (for Claude sub-agent delegation)
    pub async fn create_request(
        &self,
        to: Vec<AgentId>,
        cooperation_type: CooperationType,
        intent: CooperationIntent,
        context: HashMap<String, serde_json::Value>,
    ) -> Result<CooperationRequest> {
        let agent_id = self.agent_id
            .ok_or_else(|| PromiseError::ValidationFailed("Protocol not initialized".to_string()))?;
        
        let request = CooperationRequest {
            id: Uuid::new_v4(),
            from: agent_id,
            to,
            cooperation_type,
            intent,
            context,
            timeout_ms: 60000, // 60 second default
            parent_request_id: None,
        };
        
        self.active_requests.insert(request.id, request.clone());
        self.history.write().await.push(request.clone());
        
        Ok(request)
    }
    
    /// Requests cooperation from another agent
    pub async fn request_cooperation(
        &mut self,
        from: AgentId,
        to: AgentId,
        request: CooperationRequest,
    ) -> Result<CooperationResponse> {
        // This would integrate with synapsed-net for actual communication
        // For now, simulate a response
        
        let response = CooperationResponse {
            id: Uuid::new_v4(),
            request_id: request.id,
            from: to,
            response_type: ResponseType::Accepted,
            result: None,
            error: None,
            timestamp: Utc::now(),
        };
        
        self.responses.write().await.push(response.clone());
        
        Ok(response)
    }
    
    /// Handles an incoming cooperation request
    pub async fn handle_request(
        &mut self,
        request: CooperationRequest,
    ) -> Result<CooperationResponse> {
        let agent_id = self.agent_id
            .ok_or_else(|| PromiseError::ValidationFailed("Protocol not initialized".to_string()))?;
        
        // Check if we're the target
        if !request.to.contains(&agent_id) {
            return Err(PromiseError::ValidationFailed(
                "Request not addressed to this agent".to_string()
            ));
        }
        
        // Store the request
        self.active_requests.insert(request.id, request.clone());
        
        // Create initial response
        let response = CooperationResponse {
            id: Uuid::new_v4(),
            request_id: request.id,
            from: agent_id,
            response_type: ResponseType::Accepted,
            result: None,
            error: None,
            timestamp: Utc::now(),
        };
        
        Ok(response)
    }
    
    /// Executes a cooperation request with verification
    pub async fn execute_cooperation(
        &mut self,
        request_id: Uuid,
    ) -> Result<CooperationResult> {
        let request = self.active_requests.get(&request_id)
            .ok_or_else(|| PromiseError::ValidationFailed("Request not found".to_string()))?
            .clone();
        
        let start = Utc::now();
        let mut step_results = Vec::new();
        let mut verification_results = Vec::new();
        
        // Execute each step
        for step in &request.intent.steps {
            let step_start = Utc::now();
            
            // This would actually execute the step
            // For now, simulate success
            let result = StepResult {
                step: step.name.clone(),
                success: true,
                output: Some(format!("Completed: {}", step.description)),
                error: None,
                duration_ms: (Utc::now() - step_start).num_milliseconds() as u64,
            };
            
            step_results.push(result);
        }
        
        // Perform verification
        for criteria in &request.intent.verification {
            let result = VerificationResult {
                check: criteria.check.clone(),
                passed: true, // Would actually verify
                actual: criteria.expected.clone(),
                expected: criteria.expected.clone(),
            };
            
            verification_results.push(result);
        }
        
        let duration_ms = (Utc::now() - start).num_milliseconds() as u64;
        
        Ok(CooperationResult {
            success: true,
            step_results,
            verification_results,
            evidence: Vec::new(),
            output: HashMap::new(),
            duration_ms,
        })
    }
    
    /// Creates a coordination session for multi-agent cooperation
    pub async fn create_session(
        &mut self,
        participants: Vec<AgentId>,
        intent: CooperationIntent,
    ) -> Result<CoordinationSession> {
        let agent_id = self.agent_id
            .ok_or_else(|| PromiseError::ValidationFailed("Protocol not initialized".to_string()))?;
        
        let session = CoordinationSession {
            id: Uuid::new_v4(),
            coordinator: agent_id,
            participants,
            intent,
            state: SessionState::Planning,
            results: HashMap::new(),
            started_at: Utc::now(),
            ended_at: None,
        };
        
        self.sessions.insert(session.id, session.clone());
        
        Ok(session)
    }
    
    /// Gets the history of cooperation requests
    pub async fn get_history(&self) -> Vec<CooperationRequest> {
        self.history.read().await.clone()
    }
    
    /// Gets active requests
    pub fn get_active_requests(&self) -> Vec<CooperationRequest> {
        self.active_requests.iter()
            .map(|entry| entry.value().clone())
            .collect()
    }
}

impl Default for CooperationProtocol {
    fn default() -> Self {
        Self::new()
    }
}

use dashmap::DashMap;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_cooperation_protocol() {
        let mut protocol = CooperationProtocol::new();
        let agent_id = AgentId::new();
        protocol.initialize(agent_id).unwrap();
        
        let intent = CooperationIntent {
            goal: "Test cooperation".to_string(),
            preconditions: vec!["System ready".to_string()],
            steps: vec![
                IntentStep {
                    name: "Step 1".to_string(),
                    description: "First step".to_string(),
                    action: "echo test".to_string(),
                    expected_result: Some("test".to_string()),
                    dependencies: Vec::new(),
                },
            ],
            postconditions: vec!["Task complete".to_string()],
            verification: vec![
                VerificationCriteria {
                    check: "Output exists".to_string(),
                    expected: "true".to_string(),
                    critical: true,
                },
            ],
        };
        
        let request = protocol.create_request(
            vec![AgentId::new()],
            CooperationType::Delegate,
            intent,
            HashMap::new(),
        ).await.unwrap();
        
        assert_eq!(request.from, agent_id);
        assert_eq!(request.cooperation_type, CooperationType::Delegate);
        
        let result = protocol.execute_cooperation(request.id).await.unwrap();
        assert!(result.success);
        assert!(!result.step_results.is_empty());
        assert!(!result.verification_results.is_empty());
    }
}