//! Self-healing mechanisms for automatic recovery
//!
//! This module provides self-healing capabilities that automatically
//! detect and recover from safety violations without manual intervention.

use crate::error::{Result, SafetyError};
use crate::types::*;
use crate::traits::SafetyMonitor;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

/// Self-healing engine
pub struct SelfHealingEngine {
    strategies: Arc<RwLock<HashMap<String, Box<dyn HealingStrategy>>>>,
    history: Arc<RwLock<Vec<HealingEvent>>>,
    config: HealingConfig,
}

/// Healing strategy trait
#[async_trait::async_trait]
pub trait HealingStrategy: Send + Sync {
    /// Name of the strategy
    fn name(&self) -> &str;
    
    /// Check if this strategy can handle the violation
    async fn can_handle(&self, violation: &ConstraintViolation) -> bool;
    
    /// Apply healing action
    async fn heal(&mut self, violation: &ConstraintViolation, state: &SafetyState) -> Result<HealingAction>;
    
    /// Verify healing was successful
    async fn verify_healing(&self, state: &SafetyState) -> Result<bool>;
}

/// Healing action taken by the system
#[derive(Debug, Clone)]
pub struct HealingAction {
    pub action_type: HealingActionType,
    pub description: String,
    pub success: bool,
    pub timestamp: DateTime<Utc>,
}

/// Types of healing actions
#[derive(Debug, Clone)]
pub enum HealingActionType {
    /// Restart a component
    RestartComponent(String),
    /// Scale resources
    ScaleResources(ResourceScaling),
    /// Rollback to checkpoint
    Rollback(uuid::Uuid),
    /// Reconfigure system
    Reconfigure(HashMap<String, String>),
    /// Custom action
    Custom(String),
}

/// Resource scaling parameters
#[derive(Debug, Clone)]
pub struct ResourceScaling {
    pub resource_type: String,
    pub current_value: f64,
    pub new_value: f64,
}

/// Healing event in history
#[derive(Debug, Clone)]
pub struct HealingEvent {
    pub id: uuid::Uuid,
    pub violation: ConstraintViolation,
    pub action: HealingAction,
    pub timestamp: DateTime<Utc>,
}

/// Configuration for self-healing
#[derive(Debug, Clone)]
pub struct HealingConfig {
    pub enabled: bool,
    pub max_attempts: usize,
    pub retry_delay_ms: u64,
    pub learning_enabled: bool,
}

impl SelfHealingEngine {
    /// Create a new self-healing engine
    pub fn new(config: HealingConfig) -> Self {
        Self {
            strategies: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }
    
    /// Register a healing strategy
    pub async fn register_strategy(&mut self, strategy: Box<dyn HealingStrategy>) {
        let mut strategies = self.strategies.write().await;
        strategies.insert(strategy.name().to_string(), strategy);
    }
    
    /// Handle a constraint violation
    pub async fn handle_violation(
        &mut self,
        violation: &ConstraintViolation,
        state: &SafetyState,
    ) -> Result<HealingAction> {
        if !self.config.enabled {
            return Err(SafetyError::healing_disabled());
        }
        
        let strategies = self.strategies.read().await;
        
        // Find a strategy that can handle this violation
        for (_, strategy) in strategies.iter() {
            if strategy.can_handle(violation).await {
                drop(strategies);
                let mut strategies = self.strategies.write().await;
                
                if let Some(mut strategy) = strategies.remove(strategy.name()) {
                    let action = strategy.heal(violation, state).await?;
                    
                    // Record healing event
                    let event = HealingEvent {
                        id: uuid::Uuid::new_v4(),
                        violation: violation.clone(),
                        action: action.clone(),
                        timestamp: Utc::now(),
                    };
                    
                    let mut history = self.history.write().await;
                    history.push(event);
                    
                    // Re-insert strategy
                    strategies.insert(strategy.name().to_string(), strategy);
                    
                    return Ok(action);
                }
            }
        }
        
        Err(SafetyError::no_healing_strategy())
    }
    
    /// Get healing history
    pub async fn get_history(&self) -> Vec<HealingEvent> {
        self.history.read().await.clone()
    }
}

/// Basic restart strategy
pub struct RestartStrategy {
    max_restarts: usize,
    restart_count: HashMap<String, usize>,
}

impl RestartStrategy {
    pub fn new(max_restarts: usize) -> Self {
        Self {
            max_restarts,
            restart_count: HashMap::new(),
        }
    }
}

#[async_trait::async_trait]
impl HealingStrategy for RestartStrategy {
    fn name(&self) -> &str {
        "restart"
    }
    
    async fn can_handle(&self, violation: &ConstraintViolation) -> bool {
        // Can handle component failures
        violation.constraint_id.contains("component") ||
        violation.constraint_id.contains("health")
    }
    
    async fn heal(&mut self, violation: &ConstraintViolation, _state: &SafetyState) -> Result<HealingAction> {
        let component = violation.constraint_id.clone();
        let count = self.restart_count.entry(component.clone()).or_insert(0);
        
        if *count >= self.max_restarts {
            return Err(SafetyError::max_restarts_exceeded());
        }
        
        *count += 1;
        
        Ok(HealingAction {
            action_type: HealingActionType::RestartComponent(component),
            description: format!("Restarting component due to {}", violation.message),
            success: true,
            timestamp: Utc::now(),
        })
    }
    
    async fn verify_healing(&self, _state: &SafetyState) -> Result<bool> {
        // Would check if component is healthy
        Ok(true)
    }
}

impl Default for HealingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 3,
            retry_delay_ms: 1000,
            learning_enabled: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_self_healing_engine() {
        let config = HealingConfig::default();
        let mut engine = SelfHealingEngine::new(config);
        
        let strategy = Box::new(RestartStrategy::new(3));
        engine.register_strategy(strategy).await;
        
        let violation = ConstraintViolation {
            constraint_id: "component_health".to_string(),
            severity: Severity::High,
            message: "Component unhealthy".to_string(),
            timestamp: Utc::now(),
            context: HashMap::new(),
        };
        
        let state = SafetyState {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            values: HashMap::new(),
            active_constraints: vec![],
            resource_usage: ResourceUsage::default(),
            health_indicators: HealthIndicators::default(),
            metadata: StateMetadata::default(),
        };
        
        let action = engine.handle_violation(&violation, &state).await;
        assert!(action.is_ok());
    }
}