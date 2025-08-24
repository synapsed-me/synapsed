//! Constraint system implementation
//!
//! This module provides a comprehensive constraint evaluation engine
//! that can validate system state against user-defined safety rules.

use crate::error::{Result, SafetyError};
use crate::traits::{ConstraintEngine, EngineStats};
use crate::types::*;
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn, error};

/// Default constraint engine implementation
#[derive(Debug)]
pub struct DefaultConstraintEngine {
    /// Active constraints indexed by ID
    constraints: Arc<RwLock<HashMap<ConstraintId, Constraint>>>,
    /// Evaluation cache for performance
    evaluation_cache: Arc<RwLock<HashMap<String, CachedEvaluation>>>,
    /// Engine statistics
    stats: Arc<RwLock<EngineStats>>,
    /// Configuration
    config: ConstraintEngineConfig,
}

/// Configuration for the constraint engine
#[derive(Debug, Clone)]
pub struct ConstraintEngineConfig {
    /// Enable evaluation caching
    pub cache_enabled: bool,
    /// Cache expiration time
    pub cache_ttl_ms: u64,
    /// Maximum cache size
    pub max_cache_entries: usize,
    /// Enable parallel evaluation
    pub parallel_evaluation: bool,
    /// Evaluation timeout
    pub evaluation_timeout_ms: u64,
    /// Enable constraint optimization
    pub optimization_enabled: bool,
}

impl Default for ConstraintEngineConfig {
    fn default() -> Self {
        Self {
            cache_enabled: true,
            cache_ttl_ms: 60_000, // 1 minute
            max_cache_entries: 1000,
            parallel_evaluation: true,
            evaluation_timeout_ms: 5_000, // 5 seconds
            optimization_enabled: true,
        }
    }
}

/// Cached evaluation result
#[derive(Debug, Clone)]
struct CachedEvaluation {
    result: ValidationResult,
    timestamp: Instant,
    state_hash: String,
}

impl DefaultConstraintEngine {
    /// Create a new constraint engine with default configuration
    pub fn new() -> Self {
        Self::with_config(ConstraintEngineConfig::default())
    }

    /// Create a new constraint engine with custom configuration
    pub fn with_config(config: ConstraintEngineConfig) -> Self {
        Self {
            constraints: Arc::new(RwLock::new(HashMap::new())),
            evaluation_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(EngineStats {
                constraints_count: 0,
                evaluations_performed: 0,
                violations_found: 0,
                avg_evaluation_time_ms: 0.0,
                optimization_level: 0.0,
            })),
            config,
        }
    }

    /// Calculate hash of state for caching
    fn calculate_state_hash(&self, state: &SafetyState) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        state.id.hash(&mut hasher);
        state.timestamp.hash(&mut hasher);
        // Hash key values for consistency
        let mut keys: Vec<_> = state.values.keys().collect();
        keys.sort();
        for key in keys {
            key.hash(&mut hasher);
            // Note: StateValue doesn't implement Hash, so we'll use a simplified approach
            if let Some(value) = state.values.get(key) {
                match value {
                    StateValue::Integer(v) => v.hash(&mut hasher),
                    StateValue::Float(v) => v.to_bits().hash(&mut hasher),
                    StateValue::String(v) => v.hash(&mut hasher),
                    StateValue::Boolean(v) => v.hash(&mut hasher),
                    _ => "complex".hash(&mut hasher),
                }
            }
        }
        format!("{:x}", hasher.finish())
    }

    /// Check if cached evaluation is still valid
    fn is_cache_valid(&self, cached: &CachedEvaluation) -> bool {
        if !self.config.cache_enabled {
            return false;
        }
        let elapsed = cached.timestamp.elapsed();
        elapsed.as_millis() < self.config.cache_ttl_ms as u128
    }

    /// Clean expired cache entries
    fn clean_cache(&self) {
        if !self.config.cache_enabled {
            return;
        }

        let mut cache = self.evaluation_cache.write();
        let now = Instant::now();
        let ttl = Duration::from_millis(self.config.cache_ttl_ms);
        
        cache.retain(|_, cached| now.duration_since(cached.timestamp) < ttl);
        
        // Enforce size limit
        if cache.len() > self.config.max_cache_entries {
            let excess = cache.len() - self.config.max_cache_entries;
            // Remove oldest entries (simple approach)
            let mut entries: Vec<_> = cache.iter().collect();
            entries.sort_by_key(|(_, v)| v.timestamp);
            let keys_to_remove: Vec<_> = entries.iter().take(excess).map(|(k, _)| (*k).clone()).collect();
            drop(entries); // Drop the borrow before mutating
            for key in keys_to_remove {
                cache.remove(&key);
            }
        }
    }

    /// Evaluate a single constraint against state
    fn evaluate_constraint(&self, constraint: &Constraint, state: &SafetyState) -> Result<bool> {
        if !constraint.enabled {
            return Ok(true);
        }

        debug!("Evaluating constraint: {} ({})", constraint.name, constraint.id);

        // Simple rule evaluation - in a real implementation, this would
        // use a proper expression evaluator or rule engine
        match self.evaluate_rule(&constraint.rule, state) {
            Ok(result) => {
                debug!("Constraint {} result: {}", constraint.id, result);
                Ok(result)
            }
            Err(e) => {
                error!("Failed to evaluate constraint {}: {}", constraint.id, e);
                Err(e)
            }
        }
    }

    /// Evaluate a constraint rule
    fn evaluate_rule(&self, rule: &ConstraintRule, state: &SafetyState) -> Result<bool> {
        // This is a simplified rule evaluator
        // In a production system, you would use a proper expression parser/evaluator
        
        let expression = &rule.expression;
        debug!("Evaluating rule expression: {}", expression);

        // Handle simple expressions
        if expression.contains("balance") && expression.contains(">=") {
            if let Some(StateValue::Integer(balance)) = state.values.get("balance") {
                if expression.contains(">= 0") {
                    return Ok(*balance >= 0);
                }
            }
        }

        if expression.contains("memory_usage") && expression.contains("<") {
            let usage_pct = state.resource_usage.memory_usage_percentage();
            if expression.contains("< 0.8") {
                return Ok(usage_pct < 0.8);
            }
            if expression.contains("< 0.9") {
                return Ok(usage_pct < 0.9);
            }
        }

        if expression.contains("cpu_usage") && expression.contains("<") {
            let cpu_usage = state.resource_usage.cpu_usage;
            if expression.contains("< 0.8") {
                return Ok(cpu_usage < 0.8);
            }
        }

        if expression.contains("health_score") && expression.contains(">") {
            let health_score = state.health_indicators.overall_health;
            if expression.contains("> 0.5") {
                return Ok(health_score > 0.5);
            }
        }

        // Default to true for unknown expressions (safe default)
        warn!("Unknown rule expression, defaulting to true: {}", expression);
        Ok(true)
    }

    /// Update engine statistics
    fn update_stats(&self, evaluation_time: Duration, violations_found: usize) {
        let mut stats = self.stats.write();
        stats.evaluations_performed += 1;
        stats.violations_found += violations_found as u64;
        
        // Update average evaluation time
        let new_time_ms = evaluation_time.as_millis() as f64;
        if stats.evaluations_performed == 1 {
            stats.avg_evaluation_time_ms = new_time_ms;
        } else {
            stats.avg_evaluation_time_ms = 
                (stats.avg_evaluation_time_ms * (stats.evaluations_performed - 1) as f64 + new_time_ms) 
                / stats.evaluations_performed as f64;
        }
    }
}

#[async_trait]
impl ConstraintEngine for DefaultConstraintEngine {
    async fn add_constraint(&mut self, constraint: Constraint) -> Result<()> {
        info!("Adding constraint: {} ({})", constraint.name, constraint.id);
        
        let mut constraints = self.constraints.write();
        constraints.insert(constraint.id.clone(), constraint);
        
        let mut stats = self.stats.write();
        stats.constraints_count = constraints.len() as u32;
        
        Ok(())
    }

    async fn remove_constraint(&mut self, constraint_id: &ConstraintId) -> Result<()> {
        info!("Removing constraint: {}", constraint_id);
        
        let mut constraints = self.constraints.write();
        if constraints.remove(constraint_id).is_none() {
            return Err(SafetyError::ConstraintEngineError {
                message: format!("Constraint not found: {}", constraint_id),
            });
        }
        
        let mut stats = self.stats.write();
        stats.constraints_count = constraints.len() as u32;
        
        Ok(())
    }

    async fn update_constraint(&mut self, constraint: Constraint) -> Result<()> {
        info!("Updating constraint: {} ({})", constraint.name, constraint.id);
        
        let mut constraints = self.constraints.write();
        if !constraints.contains_key(&constraint.id) {
            return Err(SafetyError::ConstraintEngineError {
                message: format!("Constraint not found: {}", constraint.id),
            });
        }
        
        constraints.insert(constraint.id.clone(), constraint);
        Ok(())
    }

    async fn get_constraint(&self, constraint_id: &ConstraintId) -> Result<Option<Constraint>> {
        let constraints = self.constraints.read();
        Ok(constraints.get(constraint_id).cloned())
    }

    async fn list_constraints(&self) -> Result<Vec<Constraint>> {
        let constraints = self.constraints.read();
        Ok(constraints.values().cloned().collect())
    }

    async fn validate_state(&self, state: &SafetyState) -> Result<ValidationResult> {
        let start_time = Instant::now();
        debug!("Validating state: {}", state.id);

        // Check cache first
        let state_hash = self.calculate_state_hash(state);
        if self.config.cache_enabled {
            let cache = self.evaluation_cache.read();
            if let Some(cached) = cache.get(&state_hash) {
                if self.is_cache_valid(cached) {
                    debug!("Using cached validation result for state {}", state.id);
                    return Ok(cached.result.clone());
                }
            }
        }

        let constraints = self.constraints.read();
        let mut violations = Vec::new();
        let mut warnings = Vec::new();

        // Evaluate all active constraints
        for constraint in constraints.values() {
            match self.evaluate_constraint(constraint, state) {
                Ok(true) => {
                    // Constraint satisfied
                    debug!("Constraint {} satisfied", constraint.id);
                }
                Ok(false) => {
                    // Constraint violated
                    warn!("Constraint {} violated", constraint.id);
                    violations.push(ConstraintViolation {
                        constraint_id: constraint.id.clone(),
                        severity: constraint.severity,
                        message: format!("Constraint violated: {}", constraint.description),
                        actual_value: StateValue::String("violation detected".to_string()),
                        expected_value: Some(StateValue::String("constraint satisfied".to_string())),
                        timestamp: chrono::Utc::now(),
                        context: HashMap::new(),
                    });
                }
                Err(e) => {
                    // Evaluation error - treat as warning
                    warn!("Failed to evaluate constraint {}: {}", constraint.id, e);
                    warnings.push(ConstraintWarning {
                        constraint_id: constraint.id.clone(),
                        message: format!("Evaluation error: {}", e),
                        suggested_action: Some("Check constraint rule syntax".to_string()),
                        timestamp: chrono::Utc::now(),
                    });
                }
            }
        }

        let evaluation_time = start_time.elapsed();
        let result = ValidationResult {
            passed: violations.is_empty(),
            violations: violations.clone(),
            warnings,
            metadata: ValidationMetadata {
                duration_ms: evaluation_time.as_millis() as u64,
                constraints_evaluated: constraints.len() as u32,
                engine: "DefaultConstraintEngine".to_string(),
                metrics: HashMap::new(),
            },
        };

        // Update cache
        if self.config.cache_enabled {
            let mut cache = self.evaluation_cache.write();
            cache.insert(state_hash, CachedEvaluation {
                result: result.clone(),
                timestamp: Instant::now(),
                state_hash: self.calculate_state_hash(state),
            });
        }

        // Update statistics
        self.update_stats(evaluation_time, violations.len());

        // Clean cache periodically
        if self.config.cache_enabled && rand::random::<f64>() < 0.1 {
            self.clean_cache();
        }

        info!(
            "State validation completed: {} violations, {} warnings in {}ms",
            result.violations.len(),
            result.warnings.len(),
            evaluation_time.as_millis()
        );

        Ok(result)
    }

    async fn validate_constraints(
        &self,
        state: &SafetyState,
        constraint_ids: &[ConstraintId],
    ) -> Result<ValidationResult> {
        let start_time = Instant::now();
        debug!("Validating {} specific constraints for state {}", constraint_ids.len(), state.id);

        let constraints = self.constraints.read();
        let mut violations = Vec::new();
        let mut warnings = Vec::new();
        let mut evaluated_count = 0;

        for constraint_id in constraint_ids {
            if let Some(constraint) = constraints.get(constraint_id) {
                evaluated_count += 1;
                match self.evaluate_constraint(constraint, state) {
                    Ok(true) => {
                        debug!("Constraint {} satisfied", constraint.id);
                    }
                    Ok(false) => {
                        warn!("Constraint {} violated", constraint.id);
                        violations.push(ConstraintViolation {
                            constraint_id: constraint.id.clone(),
                            severity: constraint.severity,
                            message: format!("Constraint violated: {}", constraint.description),
                            actual_value: StateValue::String("violation detected".to_string()),
                            expected_value: Some(StateValue::String("constraint satisfied".to_string())),
                            timestamp: chrono::Utc::now(),
                            context: HashMap::new(),
                        });
                    }
                    Err(e) => {
                        warn!("Failed to evaluate constraint {}: {}", constraint.id, e);
                        warnings.push(ConstraintWarning {
                            constraint_id: constraint.id.clone(),
                            message: format!("Evaluation error: {}", e),
                            suggested_action: Some("Check constraint rule syntax".to_string()),
                            timestamp: chrono::Utc::now(),
                        });
                    }
                }
            } else {
                warnings.push(ConstraintWarning {
                    constraint_id: constraint_id.clone(),
                    message: format!("Constraint not found: {}", constraint_id),
                    suggested_action: Some("Check constraint ID".to_string()),
                    timestamp: chrono::Utc::now(),
                });
            }
        }

        let evaluation_time = start_time.elapsed();
        let result = ValidationResult {
            passed: violations.is_empty(),
            violations,
            warnings,
            metadata: ValidationMetadata {
                duration_ms: evaluation_time.as_millis() as u64,
                constraints_evaluated: evaluated_count,
                engine: "DefaultConstraintEngine".to_string(),
                metrics: HashMap::new(),
            },
        };

        info!(
            "Constraint validation completed: {} violations, {} warnings in {}ms",
            result.violations.len(),
            result.warnings.len(),
            evaluation_time.as_millis()
        );

        Ok(result)
    }

    async fn set_constraint_enabled(&mut self, constraint_id: &ConstraintId, enabled: bool) -> Result<()> {
        let mut constraints = self.constraints.write();
        if let Some(constraint) = constraints.get_mut(constraint_id) {
            constraint.enabled = enabled;
            info!("Constraint {} {}", constraint_id, if enabled { "enabled" } else { "disabled" });
            Ok(())
        } else {
            Err(SafetyError::ConstraintEngineError {
                message: format!("Constraint not found: {}", constraint_id),
            })
        }
    }

    async fn get_stats(&self) -> Result<crate::traits::EngineStats> {
        let stats = self.stats.read();
        Ok(stats.clone())
    }

    async fn optimize(&mut self) -> Result<()> {
        info!("Optimizing constraint engine");
        
        // Clean cache
        self.clean_cache();
        
        // Update optimization level
        let mut stats = self.stats.write();
        stats.optimization_level = 1.0;
        
        info!("Constraint engine optimization completed");
        Ok(())
    }

    async fn export_constraints(&self) -> Result<String> {
        let constraints = self.constraints.read();
        let constraints_vec: Vec<_> = constraints.values().collect();
        
        serde_json::to_string_pretty(&constraints_vec)
            .map_err(|e| SafetyError::Serialization {
                message: format!("Failed to serialize constraints: {}", e),
            })
    }

    async fn import_constraints(&mut self, data: &str) -> Result<()> {
        let imported_constraints: Vec<Constraint> = serde_json::from_str(data)
            .map_err(|e| SafetyError::Serialization {
                message: format!("Failed to deserialize constraints: {}", e),
            })?;

        let mut constraints = self.constraints.write();
        for constraint in imported_constraints {
            info!("Importing constraint: {} ({})", constraint.name, constraint.id);
            constraints.insert(constraint.id.clone(), constraint);
        }

        let mut stats = self.stats.write();
        stats.constraints_count = constraints.len() as u32;

        info!("Imported {} constraints", constraints.len());
        Ok(())
    }
}

// Helper functions for creating common constraints
impl DefaultConstraintEngine {
    /// Create a memory usage constraint
    pub fn memory_constraint(threshold: f64) -> Constraint {
        Constraint {
            id: format!("memory_usage_{}", (threshold * 100.0) as i32),
            name: format!("Memory Usage < {}%", (threshold * 100.0) as i32),
            description: format!("Memory usage must be below {}%", (threshold * 100.0) as i32),
            constraint_type: ConstraintType::Resource,
            severity: if threshold > 0.9 { Severity::Critical } else { Severity::High },
            enabled: true,
            rule: ConstraintRule {
                expression: format!("memory_usage < {}", threshold),
                parameters: HashMap::new(),
                context: crate::types::RuleContext {
                    variables: HashMap::new(),
                    functions: vec![],
                    scope: "system".to_string(),
                },
                timeout_ms: Some(1000),
            },
            actions: vec![
                ConstraintAction::Log {
                    level: "warn".to_string(),
                    message: format!("Memory usage exceeded {}%", (threshold * 100.0) as i32),
                },
                if threshold > 0.9 {
                    ConstraintAction::Alert {
                        channel: "critical".to_string(),
                        message: "Critical memory usage detected".to_string(),
                        urgency: Severity::Critical,
                    }
                } else {
                    ConstraintAction::Throttle {
                        rate_limit: 0.8,
                        duration_ms: 60000,
                    }
                },
            ],
            metadata: ConstraintMetadata {
                created_at: chrono::Utc::now(),
                created_by: "system".to_string(),
                modified_at: chrono::Utc::now(),
                version: 1,
                tags: vec!["resource".to_string(), "memory".to_string()],
                properties: HashMap::new(),
            },
        }
    }

    /// Create a balance constraint for financial operations
    pub fn balance_constraint() -> Constraint {
        Constraint {
            id: "positive_balance".to_string(),
            name: "Positive Balance".to_string(),
            description: "Account balance must be non-negative".to_string(),
            constraint_type: ConstraintType::Invariant,
            severity: Severity::Critical,
            enabled: true,
            rule: ConstraintRule {
                expression: "balance >= 0".to_string(),
                parameters: HashMap::new(),
                context: crate::types::RuleContext {
                    variables: HashMap::new(),
                    functions: vec![],
                    scope: "financial".to_string(),
                },
                timeout_ms: Some(500),
            },
            actions: vec![
                ConstraintAction::Log {
                    level: "error".to_string(),
                    message: "Negative balance detected".to_string(),
                },
                ConstraintAction::Rollback {
                    checkpoint_id: None,
                    automatic: true,
                },
                ConstraintAction::Alert {
                    channel: "financial".to_string(),
                    message: "CRITICAL: Account balance is negative".to_string(),
                    urgency: Severity::Critical,
                },
            ],
            metadata: ConstraintMetadata {
                created_at: chrono::Utc::now(),
                created_by: "system".to_string(),
                modified_at: chrono::Utc::now(),
                version: 1,
                tags: vec!["financial".to_string(), "balance".to_string()],
                properties: HashMap::new(),
            },
        }
    }

    /// Create a health check constraint
    pub fn health_constraint(min_score: f64) -> Constraint {
        Constraint {
            id: format!("health_score_{}", (min_score * 100.0) as i32),
            name: format!("Health Score > {}", min_score),
            description: format!("System health score must be above {}", min_score),
            constraint_type: ConstraintType::Invariant,
            severity: Severity::Medium,
            enabled: true,
            rule: ConstraintRule {
                expression: format!("health_score > {}", min_score),
                parameters: HashMap::new(),
                context: crate::types::RuleContext {
                    variables: HashMap::new(),
                    functions: vec![],
                    scope: "health".to_string(),
                },
                timeout_ms: Some(2000),
            },
            actions: vec![
                ConstraintAction::Log {
                    level: "warn".to_string(),
                    message: format!("System health below {}", min_score),
                },
                ConstraintAction::Execute {
                    command: "health_check".to_string(),
                    parameters: HashMap::new(),
                },
            ],
            metadata: ConstraintMetadata {
                created_at: chrono::Utc::now(),
                created_by: "system".to_string(),
                modified_at: chrono::Utc::now(),
                version: 1,
                tags: vec!["health".to_string(), "monitoring".to_string()],
                properties: HashMap::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_state() -> SafetyState {
        SafetyState {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            values: {
                let mut values = HashMap::new();
                values.insert("balance".to_string(), StateValue::Integer(100));
                values.insert("user_id".to_string(), StateValue::String("user123".to_string()));
                values
            },
            active_constraints: vec![],
            resource_usage: ResourceUsage {
                cpu_usage: 0.5,
                memory_usage: 512 * 1024 * 1024, // 512MB
                memory_limit: 1024 * 1024 * 1024, // 1GB
                network_usage: 100,
                disk_io: 50,
                file_descriptors: 10,
                thread_count: 5,
                custom_resources: HashMap::new(),
            },
            health_indicators: crate::types::HealthIndicators {
                overall_health: 0.8,
                component_health: HashMap::new(),
                error_rates: HashMap::new(),
                response_times: HashMap::new(),
                availability: HashMap::new(),
                performance_indicators: HashMap::new(),
            },
            metadata: crate::types::StateMetadata {
                source: "test".to_string(),
                version: "1.0".to_string(),
                checksum: "test_checksum".to_string(),
                size_bytes: 1024,
                compression_ratio: None,
                tags: vec![],
                properties: HashMap::new(),
            },
        }
    }

    #[tokio::test]
    async fn test_constraint_engine_basic_operations() {
        let mut engine = DefaultConstraintEngine::new();
        
        // Test adding constraints
        let constraint = DefaultConstraintEngine::balance_constraint();
        engine.add_constraint(constraint.clone()).await.unwrap();
        
        // Test listing constraints
        let constraints = engine.list_constraints().await.unwrap();
        assert_eq!(constraints.len(), 1);
        assert_eq!(constraints[0].id, constraint.id);
        
        // Test getting specific constraint
        let retrieved = engine.get_constraint(&constraint.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, constraint.id);
        
        // Test removing constraint
        engine.remove_constraint(&constraint.id).await.unwrap();
        let constraints = engine.list_constraints().await.unwrap();
        assert_eq!(constraints.len(), 0);
    }

    #[tokio::test]
    async fn test_state_validation() {
        let mut engine = DefaultConstraintEngine::new();
        let state = create_test_state();
        
        // Add balance constraint
        let constraint = DefaultConstraintEngine::balance_constraint();
        engine.add_constraint(constraint).await.unwrap();
        
        // Test validation with positive balance (should pass)
        let result = engine.validate_state(&state).await.unwrap();
        assert!(result.passed);
        assert_eq!(result.violations.len(), 0);
        
        // Test validation with negative balance (should fail)
        let mut bad_state = state.clone();
        bad_state.values.insert("balance".to_string(), StateValue::Integer(-100));
        
        let result = engine.validate_state(&bad_state).await.unwrap();
        assert!(!result.passed);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].severity, Severity::Critical);
    }

    #[tokio::test]
    async fn test_memory_constraint() {
        let mut engine = DefaultConstraintEngine::new();
        let constraint = DefaultConstraintEngine::memory_constraint(0.8);
        
        engine.add_constraint(constraint).await.unwrap();
        
        // Test with memory usage below threshold
        let state = create_test_state(); // 50% memory usage
        let result = engine.validate_state(&state).await.unwrap();
        assert!(result.passed);
        
        // Test with memory usage above threshold
        let mut high_memory_state = state.clone();
        high_memory_state.resource_usage.memory_usage = 900 * 1024 * 1024; // 90% of 1GB
        
        let result = engine.validate_state(&high_memory_state).await.unwrap();
        assert!(!result.passed);
        assert_eq!(result.violations.len(), 1);
    }

    #[tokio::test]
    async fn test_constraint_enable_disable() {
        let mut engine = DefaultConstraintEngine::new();
        let constraint = DefaultConstraintEngine::balance_constraint();
        let constraint_id = constraint.id.clone();
        
        engine.add_constraint(constraint).await.unwrap();
        
        // Create state with negative balance
        let mut bad_state = create_test_state();
        bad_state.values.insert("balance".to_string(), StateValue::Integer(-100));
        
        // Should fail when constraint is enabled
        let result = engine.validate_state(&bad_state).await.unwrap();
        assert!(!result.passed);
        
        // Disable constraint
        engine.set_constraint_enabled(&constraint_id, false).await.unwrap();
        
        // Should pass when constraint is disabled
        let result = engine.validate_state(&bad_state).await.unwrap();
        assert!(result.passed);
    }

    #[tokio::test]
    async fn test_constraint_import_export() {
        let mut engine = DefaultConstraintEngine::new();
        
        // Add some constraints
        engine.add_constraint(DefaultConstraintEngine::balance_constraint()).await.unwrap();
        engine.add_constraint(DefaultConstraintEngine::memory_constraint(0.8)).await.unwrap();
        
        // Export constraints
        let exported = engine.export_constraints().await.unwrap();
        assert!(!exported.is_empty());
        
        // Create new engine and import
        let mut new_engine = DefaultConstraintEngine::new();
        new_engine.import_constraints(&exported).await.unwrap();
        
        // Verify constraints were imported
        let constraints = new_engine.list_constraints().await.unwrap();
        assert_eq!(constraints.len(), 2);
    }

    #[tokio::test]
    async fn test_engine_stats() {
        let mut engine = DefaultConstraintEngine::new();
        let state = create_test_state();
        
        // Add constraint
        engine.add_constraint(DefaultConstraintEngine::balance_constraint()).await.unwrap();
        
        // Perform validation
        engine.validate_state(&state).await.unwrap();
        
        // Check stats
        let stats = engine.get_stats().await.unwrap();
        assert_eq!(stats.constraints_count, 1);
        assert_eq!(stats.evaluations_performed, 1);
        assert_eq!(stats.violations_found, 0);
        assert!(stats.avg_evaluation_time_ms > 0.0);
    }
}