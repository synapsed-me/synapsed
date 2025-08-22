//! Autonomous agent implementation for Promise Theory

use crate::{
    types::*, Promise, PromiseContract, TrustModel, CooperationProtocol,
    Result, PromiseError
};
use async_trait::async_trait;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;
use synapsed_core::traits::{Identifiable, Observable};

/// State of an autonomous agent
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentState {
    /// Agent is initializing
    Initializing,
    /// Agent is ready to operate
    Ready,
    /// Agent is actively processing
    Active,
    /// Agent is cooperating with others
    Cooperating,
    /// Agent is in degraded state
    Degraded,
    /// Agent is shutting down
    ShuttingDown,
    /// Agent has stopped
    Stopped,
}

/// Capabilities that an agent can promise
#[derive(Debug, Clone)]
pub struct AgentCapabilities {
    /// Services the agent can provide
    pub services: Vec<String>,
    /// Resources the agent has
    pub resources: Vec<String>,
    /// Protocols the agent supports
    pub protocols: Vec<String>,
    /// Quality guarantees the agent can make
    pub quality: QualityOfService,
}

/// Configuration for an autonomous agent
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Agent name
    pub name: String,
    /// Agent capabilities
    pub capabilities: AgentCapabilities,
    /// Trust model to use
    pub trust_model: TrustModel,
    /// Cooperation protocol
    pub cooperation_protocol: CooperationProtocol,
    /// Maximum concurrent promises
    pub max_promises: usize,
    /// Promise timeout in seconds
    pub promise_timeout_secs: u64,
}

/// An autonomous agent in the Promise Theory model
pub struct AutonomousAgent {
    /// Unique agent ID
    id: AgentId,
    /// Agent configuration
    config: Arc<AgentConfig>,
    /// Current state
    state: Arc<RwLock<AgentState>>,
    /// Active promises made by this agent
    promises_made: Arc<DashMap<PromiseId, Promise>>,
    /// Promises accepted from other agents
    promises_accepted: Arc<DashMap<PromiseId, Promise>>,
    /// Impositions from other agents (not yet accepted)
    impositions: Arc<DashMap<Uuid, Imposition>>,
    /// Trust model instance
    trust_model: Arc<RwLock<TrustModel>>,
    /// Cooperation protocol instance
    cooperation: Arc<RwLock<CooperationProtocol>>,
    /// Observable substrate for monitoring
    substrate: Arc<synapsed_substrates::Subject>,
}

impl AutonomousAgent {
    /// Creates a new autonomous agent
    pub fn new(config: AgentConfig) -> Self {
        let id = AgentId::new();
        use synapsed_substrates::types::{Name, SubjectType};
        let substrate = synapsed_substrates::Subject::new(
            Name::from(format!("agent.{}", id.0).as_str()),
            SubjectType::Source
        );
        
        Self {
            id,
            config: Arc::new(config.clone()),
            state: Arc::new(RwLock::new(AgentState::Initializing)),
            promises_made: Arc::new(DashMap::new()),
            promises_accepted: Arc::new(DashMap::new()),
            impositions: Arc::new(DashMap::new()),
            trust_model: Arc::new(RwLock::new(config.trust_model)),
            cooperation: Arc::new(RwLock::new(config.cooperation_protocol)),
            substrate: Arc::new(substrate),
        }
    }
    
    /// Gets the agent's ID
    pub fn id(&self) -> AgentId {
        self.id
    }
    
    /// Gets the agent's current state
    pub async fn state(&self) -> AgentState {
        self.state.read().await.clone()
    }
    
    /// Initializes the agent
    pub async fn initialize(&self) -> Result<()> {
        let mut state = self.state.write().await;
        if *state != AgentState::Initializing {
            return Err(PromiseError::ValidationFailed(
                "Agent already initialized".to_string()
            ));
        }
        
        // Initialize trust model
        self.trust_model.write().await.initialize()?;
        
        // Initialize cooperation protocol
        self.cooperation.write().await.initialize(self.id)?;
        
        *state = AgentState::Ready;
        
        // Emit observability event
        // TODO: Emit event through circuit/channel when available
        let _event = serde_json::json!({
            "event": "initialized",
            "agent_id": self.id.0,
            "capabilities": self.config.capabilities.services.clone(),
        });
        
        Ok(())
    }
    
    /// Makes a promise to other agents
    pub async fn make_promise(
        &self,
        promise_type: PromiseType,
        scope: PromiseScope,
        body: PromiseBody,
    ) -> Result<Promise> {
        // Check if we're in a valid state
        let state = self.state.read().await;
        if !matches!(*state, AgentState::Ready | AgentState::Active | AgentState::Cooperating) {
            return Err(PromiseError::ValidationFailed(
                format!("Cannot make promise in state {:?}", *state)
            ));
        }
        drop(state);
        
        // Check if we've reached promise limit
        if self.promises_made.len() >= self.config.max_promises {
            return Err(PromiseError::ValidationFailed(
                "Maximum promise limit reached".to_string()
            ));
        }
        
        // Validate promise against capabilities
        self.validate_promise_capability(&promise_type, &body)?;
        
        // Create the promise
        let promise = Promise::new(
            self.id,
            promise_type,
            scope,
            body,
        );
        
        // Store the promise
        self.promises_made.insert(promise.id(), promise.clone());
        
        // Update state if needed
        let mut state = self.state.write().await;
        if *state == AgentState::Ready {
            *state = AgentState::Active;
        }
        
        // Emit observability event
        // TODO: Emit event through circuit/channel when available
        let _event = serde_json::json!({
            "event": "promise_made",
            "promise_id": promise.id().0,
            "promise_type": format!("{:?}", promise.promise_type()),
            "scope": format!("{:?}", promise.scope()),
        });
        
        Ok(promise)
    }
    
    /// Receives an imposition from another agent
    pub async fn receive_imposition(&self, imposition: Imposition) -> Result<()> {
        // Validate the imposition
        if imposition.to != self.id {
            return Err(PromiseError::ValidationFailed(
                "Imposition not addressed to this agent".to_string()
            ));
        }
        
        // Check trust level of imposing agent
        let trust_level = self.trust_model.read().await
            .get_trust_level(imposition.from).await?;
        
        // Store the imposition for evaluation
        self.impositions.insert(imposition.id, imposition.clone());
        
        // Emit observability event
        // TODO: Emit event through circuit/channel when available
        let _event = serde_json::json!({
            "event": "imposition_received",
            "imposition_id": imposition.id,
            "from_agent": imposition.from.0,
            "trust_level": format!("{:?}", trust_level),
        });
        
        Ok(())
    }
    
    /// Evaluates and potentially accepts an imposition as a promise
    pub async fn evaluate_imposition(&self, imposition_id: Uuid) -> Result<Option<Promise>> {
        let imposition = self.impositions.get(&imposition_id)
            .ok_or_else(|| PromiseError::ValidationFailed("Imposition not found".to_string()))?
            .clone();
        
        // Check trust level
        let trust_level = self.trust_model.read().await
            .get_trust_level(imposition.from).await?;
        
        // Check if we can fulfill this
        let can_fulfill = self.can_fulfill_imposition(&imposition).await?;
        
        if !trust_level.is_sufficient() || !can_fulfill {
            // Reject the imposition
            self.impositions.remove(&imposition_id);
            
            // TODO: Emit event through circuit/channel when available
        let _event = serde_json::json!({
            "event": "imposition_rejected",
                "imposition_id": imposition_id,
                "reason": if !trust_level.is_sufficient() { "insufficient_trust" } else { "cannot_fulfill" },
            });
            
            return Ok(None);
        }
        
        // Accept the imposition as a promise
        let promise = Promise::from_imposition(self.id, imposition.clone());
        
        self.promises_accepted.insert(promise.id(), promise.clone());
        self.impositions.remove(&imposition_id);
        
        // TODO: Emit event through circuit/channel when available
        let _event = serde_json::json!({
            "event": "imposition_accepted",
            "imposition_id": imposition_id,
            "promise_id": promise.id().0,
        });
        
        Ok(Some(promise))
    }
    
    /// Requests cooperation from another agent
    pub async fn request_cooperation(
        &self,
        target: AgentId,
        request: crate::CooperationRequest,
    ) -> Result<crate::CooperationResponse> {
        // Update state
        let mut state = self.state.write().await;
        let prev_state = state.clone();
        *state = AgentState::Cooperating;
        drop(state);
        
        // Use cooperation protocol
        let response = self.cooperation.write().await
            .request_cooperation(self.id, target, request).await?;
        
        // Restore previous state if cooperation is complete
        let mut state = self.state.write().await;
        *state = prev_state;
        
        Ok(response)
    }
    
    /// Assesses whether a promise was kept
    pub async fn assess_promise(
        &self,
        promise_id: PromiseId,
        evidence: Vec<Evidence>,
    ) -> Result<Assessment> {
        // Find the promise
        let promise = self.promises_accepted.get(&promise_id)
            .or_else(|| self.promises_made.get(&promise_id))
            .ok_or_else(|| PromiseError::ValidationFailed("Promise not found".to_string()))?
            .clone();
        
        // Evaluate the evidence
        let (kept, quality) = self.evaluate_evidence(&promise, &evidence).await?;
        
        // Create assessment
        let assessment = Assessment {
            id: Uuid::new_v4(),
            promise_id,
            assessor: self.id,
            kept,
            quality,
            evidence,
            timestamp: Utc::now(),
        };
        
        // Update trust model based on assessment
        if let Some(promisor) = promise.promisor() {
            self.trust_model.write().await
                .update_trust(promisor, kept, quality).await?;
        }
        
        // TODO: Emit event through circuit/channel when available
        let _event = serde_json::json!({
            "event": "promise_assessed",
            "promise_id": promise_id.0,
            "kept": kept,
            "quality": quality,
        });
        
        Ok(assessment)
    }
    
    /// Shuts down the agent
    pub async fn shutdown(&self) -> Result<()> {
        let mut state = self.state.write().await;
        *state = AgentState::ShuttingDown;
        
        // Clean up promises
        self.promises_made.clear();
        self.promises_accepted.clear();
        self.impositions.clear();
        
        *state = AgentState::Stopped;
        
        // TODO: Emit event through circuit/channel when available
        let _event = serde_json::json!({
            "event": "shutdown",
            "agent_id": self.id.0,
        });
        
        Ok(())
    }
    
    // Helper methods
    
    fn validate_promise_capability(
        &self,
        promise_type: &PromiseType,
        body: &PromiseBody,
    ) -> Result<()> {
        // Check if the promise aligns with our capabilities
        match promise_type {
            PromiseType::Offer => {
                // Check if we have the service/resource to offer
                let content = &body.content;
                let has_capability = self.config.capabilities.services.iter()
                    .any(|s| content.contains(s)) ||
                    self.config.capabilities.resources.iter()
                    .any(|r| content.contains(r));
                
                if !has_capability {
                    return Err(PromiseError::ValidationFailed(
                        "Promise exceeds agent capabilities".to_string()
                    ));
                }
            },
            _ => {
                // Other promise types don't require capability validation
            }
        }
        
        Ok(())
    }
    
    async fn can_fulfill_imposition(&self, imposition: &Imposition) -> Result<bool> {
        // Check if we have the capabilities to fulfill this imposition
        let content = &imposition.body.content;
        
        let has_capability = self.config.capabilities.services.iter()
            .any(|s| content.contains(s)) ||
            self.config.capabilities.resources.iter()
            .any(|r| content.contains(r));
        
        Ok(has_capability)
    }
    
    async fn evaluate_evidence(
        &self,
        _promise: &Promise,
        evidence: &[Evidence],
    ) -> Result<(bool, f64)> {
        // Simple evaluation: if we have cryptographic proof, it's kept
        let has_proof = evidence.iter()
            .any(|e| e.evidence_type == EvidenceType::CryptographicProof && e.proof.is_some());
        
        if has_proof {
            return Ok((true, 1.0));
        }
        
        // Otherwise, calculate quality based on evidence count and types
        let quality = (evidence.len() as f64 / 5.0).min(1.0);
        let kept = quality >= 0.5;
        
        Ok((kept, quality))
    }
}

impl Identifiable for AutonomousAgent {
    fn id(&self) -> Uuid {
        self.id.0
    }
    
    fn name(&self) -> &str {
        &self.config.name
    }
    
    fn type_name(&self) -> &'static str {
        "AutonomousAgent"
    }
}

#[async_trait]
impl Observable for AutonomousAgent {
    async fn status(&self) -> synapsed_core::SynapsedResult<synapsed_core::traits::ObservableStatus> {
        use synapsed_core::traits::*;
        use std::collections::HashMap;
        
        let state = self.state.read().await;
        let obs_state = match *state {
            AgentState::Ready | AgentState::Active | AgentState::Cooperating => ObservableState::Running,
            AgentState::Initializing => ObservableState::Initializing,
            AgentState::ShuttingDown => ObservableState::ShuttingDown,
            AgentState::Stopped => ObservableState::Stopped,
            AgentState::Degraded => ObservableState::Degraded,
        };
        
        let mut metadata = HashMap::new();
        metadata.insert("agent_id".to_string(), self.id.0.to_string());
        metadata.insert("promises_made".to_string(), self.promises_made.len().to_string());
        metadata.insert("promises_accepted".to_string(), self.promises_accepted.len().to_string());
        metadata.insert("impositions_pending".to_string(), self.impositions.len().to_string());
        
        Ok(ObservableStatus {
            state: obs_state,
            last_updated: chrono::Utc::now(),
            metadata,
        })
    }
    
    async fn health(&self) -> synapsed_core::SynapsedResult<synapsed_core::traits::HealthStatus> {
        use synapsed_core::traits::*;
        use std::collections::HashMap;
        
        let mut checks = HashMap::new();
        let state = self.state.read().await;
        
        let state_check = match *state {
            AgentState::Ready | AgentState::Active | AgentState::Cooperating => {
                HealthCheck {
                    level: HealthLevel::Healthy,
                    message: format!("Agent is {:?}", *state),
                    timestamp: chrono::Utc::now(),
                }
            },
            AgentState::Degraded => {
                HealthCheck {
                    level: HealthLevel::Warning,
                    message: "Agent is in degraded state".to_string(),
                    timestamp: chrono::Utc::now(),
                }
            },
            _ => {
                HealthCheck {
                    level: HealthLevel::Critical,
                    message: format!("Agent is {:?}", *state),
                    timestamp: chrono::Utc::now(),
                }
            }
        };
        checks.insert("state".to_string(), state_check);
        
        let promise_check = if self.promises_made.len() < self.config.max_promises {
            HealthCheck {
                level: HealthLevel::Healthy,
                message: format!("{}/{} promises", self.promises_made.len(), self.config.max_promises),
                timestamp: chrono::Utc::now(),
            }
        } else {
            HealthCheck {
                level: HealthLevel::Warning,
                message: "At maximum promise capacity".to_string(),
                timestamp: chrono::Utc::now(),
            }
        };
        checks.insert("promises".to_string(), promise_check);
        
        let overall = if checks.values().any(|c| c.level == HealthLevel::Critical) {
            HealthLevel::Critical
        } else if checks.values().any(|c| c.level == HealthLevel::Warning) {
            HealthLevel::Warning
        } else {
            HealthLevel::Healthy
        };
        
        Ok(HealthStatus {
            overall,
            checks,
            last_check: chrono::Utc::now(),
        })
    }
    
    async fn metrics(&self) -> synapsed_core::SynapsedResult<HashMap<String, f64>> {
        let mut metrics = HashMap::new();
        
        metrics.insert("promises_made".to_string(), self.promises_made.len() as f64);
        metrics.insert("promises_accepted".to_string(), self.promises_accepted.len() as f64);
        metrics.insert("impositions_pending".to_string(), self.impositions.len() as f64);
        metrics.insert("promise_capacity_used".to_string(), 
            self.promises_made.len() as f64 / self.config.max_promises as f64);
        
        Ok(metrics)
    }
    
    fn describe(&self) -> String {
        format!("AutonomousAgent '{}' ({})", self.config.name, self.id.0)
    }
}