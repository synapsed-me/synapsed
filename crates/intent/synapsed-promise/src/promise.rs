//! Promise contract and execution implementation

use crate::{
    types::*, Result, PromiseError
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// State of a promise
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromiseState {
    /// Promise has been declared but not yet activated
    Declared,
    /// Promise is active and being fulfilled
    Active,
    /// Promise is being fulfilled
    Fulfilling,
    /// Promise has been fulfilled successfully
    Fulfilled,
    /// Promise was broken/failed
    Broken,
    /// Promise was cancelled
    Cancelled,
    /// Promise has expired
    Expired,
}

/// Outcome of a promise
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromiseOutcome {
    /// Final state of the promise
    pub state: PromiseState,
    /// Quality of fulfillment (0.0 to 1.0)
    pub quality: f64,
    /// Evidence of fulfillment
    pub evidence: Vec<Evidence>,
    /// Time taken to fulfill
    pub duration_ms: Option<u64>,
    /// Any error that occurred
    pub error: Option<String>,
}

/// A promise contract between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromiseContract {
    /// Conditions that must be met before the promise activates
    pub preconditions: Vec<Constraint>,
    /// The promise body
    pub body: PromiseBody,
    /// Conditions that must be met for successful fulfillment
    pub postconditions: Vec<Constraint>,
    /// Conditions that must remain true throughout
    pub invariants: Vec<Constraint>,
    /// Timeout for the promise
    pub timeout_ms: Option<u64>,
    /// Dependencies on other promises
    pub dependencies: Vec<PromiseId>,
}

/// A promise made by an agent
#[derive(Debug, Clone)]
pub struct Promise {
    /// Unique promise ID
    id: PromiseId,
    /// Agent making the promise
    promisor: AgentId,
    /// Type of promise
    promise_type: PromiseType,
    /// Scope of the promise
    scope: PromiseScope,
    /// Contract details
    contract: Arc<PromiseContract>,
    /// Current state
    state: Arc<RwLock<PromiseState>>,
    /// Creation timestamp
    created_at: DateTime<Utc>,
    /// Activation timestamp
    activated_at: Arc<RwLock<Option<DateTime<Utc>>>>,
    /// Completion timestamp
    completed_at: Arc<RwLock<Option<DateTime<Utc>>>>,
    /// Promise outcome
    outcome: Arc<RwLock<Option<PromiseOutcome>>>,
    /// Assessments of this promise
    assessments: Arc<RwLock<Vec<Assessment>>>,
}

impl Promise {
    /// Creates a new promise
    pub fn new(
        promisor: AgentId,
        promise_type: PromiseType,
        scope: PromiseScope,
        body: PromiseBody,
    ) -> Self {
        let contract = PromiseContract {
            preconditions: Vec::new(),
            body,
            postconditions: Vec::new(),
            invariants: Vec::new(),
            timeout_ms: Some(60000), // Default 60 second timeout
            dependencies: Vec::new(),
        };
        
        Self {
            id: PromiseId::new(),
            promisor,
            promise_type,
            scope,
            contract: Arc::new(contract),
            state: Arc::new(RwLock::new(PromiseState::Declared)),
            created_at: Utc::now(),
            activated_at: Arc::new(RwLock::new(None)),
            completed_at: Arc::new(RwLock::new(None)),
            outcome: Arc::new(RwLock::new(None)),
            assessments: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Creates a promise from an imposition
    pub fn from_imposition(acceptor: AgentId, imposition: Imposition) -> Self {
        let contract = PromiseContract {
            preconditions: Vec::new(),
            body: imposition.body,
            postconditions: Vec::new(),
            invariants: Vec::new(),
            timeout_ms: Some(60000),
            dependencies: Vec::new(),
        };
        
        Self {
            id: PromiseId::new(),
            promisor: acceptor,
            promise_type: PromiseType::Accept,
            scope: PromiseScope::Agent(imposition.from),
            contract: Arc::new(contract),
            state: Arc::new(RwLock::new(PromiseState::Declared)),
            created_at: Utc::now(),
            activated_at: Arc::new(RwLock::new(None)),
            completed_at: Arc::new(RwLock::new(None)),
            outcome: Arc::new(RwLock::new(None)),
            assessments: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Creates a promise with a full contract
    pub fn with_contract(
        promisor: AgentId,
        promise_type: PromiseType,
        scope: PromiseScope,
        contract: PromiseContract,
    ) -> Self {
        Self {
            id: PromiseId::new(),
            promisor,
            promise_type,
            scope,
            contract: Arc::new(contract),
            state: Arc::new(RwLock::new(PromiseState::Declared)),
            created_at: Utc::now(),
            activated_at: Arc::new(RwLock::new(None)),
            completed_at: Arc::new(RwLock::new(None)),
            outcome: Arc::new(RwLock::new(None)),
            assessments: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Gets the promise ID
    pub fn id(&self) -> PromiseId {
        self.id
    }
    
    /// Gets the promisor agent ID
    pub fn promisor(&self) -> Option<AgentId> {
        Some(self.promisor)
    }
    
    /// Gets the promise type
    pub fn promise_type(&self) -> &PromiseType {
        &self.promise_type
    }
    
    /// Gets the promise scope
    pub fn scope(&self) -> &PromiseScope {
        &self.scope
    }
    
    /// Gets the current state
    pub async fn state(&self) -> PromiseState {
        self.state.read().await.clone()
    }
    
    /// Checks if the promise is active
    pub async fn is_active(&self) -> bool {
        matches!(
            *self.state.read().await,
            PromiseState::Active | PromiseState::Fulfilling
        )
    }
    
    /// Checks if the promise is complete
    pub async fn is_complete(&self) -> bool {
        matches!(
            *self.state.read().await,
            PromiseState::Fulfilled | PromiseState::Broken | PromiseState::Cancelled | PromiseState::Expired
        )
    }
    
    /// Activates the promise
    pub async fn activate(&self) -> Result<()> {
        let mut state = self.state.write().await;
        if *state != PromiseState::Declared {
            return Err(PromiseError::ValidationFailed(
                format!("Cannot activate promise in state {:?}", *state)
            ));
        }
        
        *state = PromiseState::Active;
        *self.activated_at.write().await = Some(Utc::now());
        
        Ok(())
    }
    
    /// Starts fulfilling the promise
    pub async fn start_fulfilling(&self) -> Result<()> {
        let mut state = self.state.write().await;
        if *state != PromiseState::Active {
            return Err(PromiseError::ValidationFailed(
                format!("Cannot start fulfilling promise in state {:?}", *state)
            ));
        }
        
        *state = PromiseState::Fulfilling;
        
        Ok(())
    }
    
    /// Marks the promise as fulfilled
    pub async fn fulfill(&self, evidence: Vec<Evidence>) -> Result<()> {
        let mut state = self.state.write().await;
        if !matches!(*state, PromiseState::Active | PromiseState::Fulfilling) {
            return Err(PromiseError::ValidationFailed(
                format!("Cannot fulfill promise in state {:?}", *state)
            ));
        }
        
        *state = PromiseState::Fulfilled;
        *self.completed_at.write().await = Some(Utc::now());
        
        let duration_ms = if let Some(activated) = *self.activated_at.read().await {
            Some((Utc::now() - activated).num_milliseconds() as u64)
        } else {
            None
        };
        
        *self.outcome.write().await = Some(PromiseOutcome {
            state: PromiseState::Fulfilled,
            quality: 1.0,
            evidence,
            duration_ms,
            error: None,
        });
        
        Ok(())
    }
    
    /// Marks the promise as broken
    pub async fn break_promise(&self, reason: String) -> Result<()> {
        let mut state = self.state.write().await;
        if self.is_complete().await {
            return Err(PromiseError::ValidationFailed(
                "Promise is already complete".to_string()
            ));
        }
        
        *state = PromiseState::Broken;
        *self.completed_at.write().await = Some(Utc::now());
        
        *self.outcome.write().await = Some(PromiseOutcome {
            state: PromiseState::Broken,
            quality: 0.0,
            evidence: Vec::new(),
            duration_ms: None,
            error: Some(reason),
        });
        
        Ok(())
    }
    
    /// Cancels the promise
    pub async fn cancel(&self) -> Result<()> {
        let mut state = self.state.write().await;
        if self.is_complete().await {
            return Err(PromiseError::ValidationFailed(
                "Cannot cancel completed promise".to_string()
            ));
        }
        
        *state = PromiseState::Cancelled;
        *self.completed_at.write().await = Some(Utc::now());
        
        *self.outcome.write().await = Some(PromiseOutcome {
            state: PromiseState::Cancelled,
            quality: 0.0,
            evidence: Vec::new(),
            duration_ms: None,
            error: Some("Promise cancelled".to_string()),
        });
        
        Ok(())
    }
    
    /// Checks if the promise has expired
    pub async fn check_expiry(&self) -> Result<bool> {
        if self.is_complete().await {
            return Ok(false);
        }
        
        if let Some(timeout_ms) = self.contract.timeout_ms {
            if let Some(activated) = *self.activated_at.read().await {
                let elapsed = (Utc::now() - activated).num_milliseconds() as u64;
                if elapsed > timeout_ms {
                    let mut state = self.state.write().await;
                    *state = PromiseState::Expired;
                    *self.completed_at.write().await = Some(Utc::now());
                    
                    *self.outcome.write().await = Some(PromiseOutcome {
                        state: PromiseState::Expired,
                        quality: 0.0,
                        evidence: Vec::new(),
                        duration_ms: Some(elapsed),
                        error: Some("Promise expired".to_string()),
                    });
                    
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
    
    /// Adds an assessment to the promise
    pub async fn add_assessment(&self, assessment: Assessment) -> Result<()> {
        if assessment.promise_id != self.id {
            return Err(PromiseError::ValidationFailed(
                "Assessment is for a different promise".to_string()
            ));
        }
        
        self.assessments.write().await.push(assessment);
        
        Ok(())
    }
    
    /// Gets all assessments
    pub async fn assessments(&self) -> Vec<Assessment> {
        self.assessments.read().await.clone()
    }
    
    /// Gets the promise outcome if complete
    pub async fn outcome(&self) -> Option<PromiseOutcome> {
        self.outcome.read().await.clone()
    }
    
    /// Validates preconditions
    pub async fn validate_preconditions(&self) -> Result<bool> {
        // This would integrate with synapsed-verify crate
        // For now, return true if no preconditions
        Ok(self.contract.preconditions.is_empty())
    }
    
    /// Validates postconditions
    pub async fn validate_postconditions(&self) -> Result<bool> {
        // This would integrate with synapsed-verify crate
        // For now, return true if no postconditions
        Ok(self.contract.postconditions.is_empty())
    }
    
    /// Validates invariants
    pub async fn validate_invariants(&self) -> Result<bool> {
        // This would integrate with synapsed-verify crate
        // For now, return true if no invariants
        Ok(self.contract.invariants.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_promise_lifecycle() {
        let agent_id = AgentId::new();
        let body = PromiseBody {
            content: "Test promise".to_string(),
            constraints: Vec::new(),
            qos: None,
            metadata: Default::default(),
        };
        
        let promise = Promise::new(
            agent_id,
            PromiseType::Offer,
            PromiseScope::Universal,
            body,
        );
        
        // Check initial state
        assert_eq!(promise.state().await, PromiseState::Declared);
        assert!(!promise.is_active().await);
        assert!(!promise.is_complete().await);
        
        // Activate
        promise.activate().await.unwrap();
        assert_eq!(promise.state().await, PromiseState::Active);
        assert!(promise.is_active().await);
        
        // Start fulfilling
        promise.start_fulfilling().await.unwrap();
        assert_eq!(promise.state().await, PromiseState::Fulfilling);
        
        // Fulfill
        let evidence = vec![Evidence {
            evidence_type: EvidenceType::Log,
            data: serde_json::json!("Promise fulfilled"),
            proof: None,
        }];
        promise.fulfill(evidence).await.unwrap();
        assert_eq!(promise.state().await, PromiseState::Fulfilled);
        assert!(promise.is_complete().await);
        
        // Check outcome
        let outcome = promise.outcome().await.unwrap();
        assert_eq!(outcome.state, PromiseState::Fulfilled);
        assert_eq!(outcome.quality, 1.0);
    }
}