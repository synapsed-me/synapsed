//! Enhanced permission negotiation with adaptive trust and learning
//! 
//! This module extends the base permission negotiation with:
//! - Adaptive trust scoring based on agent behavior
//! - Learning from past decisions
//! - Context-aware permission granting
//! - Hierarchical permission delegation

use crate::{
    permission_negotiation::{
        PermissionRequest, PermissionResponse, Decision, Priority,
        RequestedPermissions, GrantedPermissions, Alternative
    },
    dynamic_agents::RiskLevel,
    memory::{HybridMemory, MemoryItem, MemoryContent, Episode, EpisodeOutcome, Event},
    Result, IntentError,
};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration, Timelike};

/// Adaptive permission negotiator with learning capabilities
pub struct AdaptivePermissionNegotiator {
    base_negotiator: Arc<RwLock<crate::permission_negotiation::PermissionNegotiator>>,
    memory: Arc<HybridMemory>,
    trust_scores: Arc<RwLock<HashMap<String, TrustScore>>>,
    decision_history: Arc<RwLock<Vec<DecisionRecord>>>,
    learning_engine: Arc<LearningEngine>,
    delegation_chain: Arc<RwLock<DelegationChain>>,
}

/// Trust score for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScore {
    pub agent_id: String,
    pub base_trust: f64,          // 0.0 to 1.0
    pub behavioral_trust: f64,    // Based on past behavior
    pub contextual_trust: f64,    // Based on current context
    pub temporal_decay: f64,      // How trust decays over time
    pub last_updated: DateTime<Utc>,
    pub violation_count: usize,
    pub success_count: usize,
}

impl TrustScore {
    pub fn calculate_effective_trust(&self) -> f64 {
        let time_since_update = (Utc::now() - self.last_updated).num_hours() as f64;
        let time_factor = (-time_since_update * self.temporal_decay / 24.0).exp();
        
        let weighted_trust = 
            self.base_trust * 0.3 +
            self.behavioral_trust * 0.5 +
            self.contextual_trust * 0.2;
        
        (weighted_trust * time_factor).min(1.0).max(0.0)
    }
    
    pub fn update_from_outcome(&mut self, success: bool) {
        if success {
            self.success_count += 1;
            self.behavioral_trust = (self.behavioral_trust + 0.1).min(1.0);
        } else {
            self.violation_count += 1;
            self.behavioral_trust = (self.behavioral_trust - 0.2).max(0.0);
        }
        self.last_updated = Utc::now();
    }
}

/// Record of a permission decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub request_id: Uuid,
    pub agent_id: String,
    pub decision: Decision,
    pub requested: RequestedPermissions,
    pub granted: Option<GrantedPermissions>,
    pub context_factors: ContextFactors,
    pub outcome: Option<DecisionOutcome>,
    pub timestamp: DateTime<Utc>,
}

/// Factors that influenced the decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFactors {
    pub trust_level: f64,
    pub risk_assessment: RiskLevel,
    pub resource_availability: ResourceState,
    pub concurrent_agents: usize,
    pub time_of_day: String,
    pub workload_intensity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceState {
    pub cpu_available: f64,
    pub memory_available_mb: usize,
    pub network_bandwidth_mbps: f64,
    pub storage_available_gb: f64,
}

/// Outcome of a permission decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOutcome {
    pub was_correct: bool,
    pub actual_usage: ActualUsage,
    pub violations: Vec<String>,
    pub feedback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActualUsage {
    pub commands_used: Vec<String>,
    pub paths_accessed: Vec<String>,
    pub memory_peak_mb: usize,
    pub cpu_seconds: u64,
    pub network_bytes: u64,
}

/// Learning engine for improving decisions
pub struct LearningEngine {
    patterns: Arc<RwLock<Vec<DecisionPattern>>>,
    success_rate_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionPattern {
    pub pattern_id: Uuid,
    pub conditions: Vec<PatternCondition>,
    pub recommended_decision: Decision,
    pub success_rate: f64,
    pub sample_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternCondition {
    TrustAbove(f64),
    TrustBelow(f64),
    RiskLevel(RiskLevel),
    TimeRange(String, String),
    ResourceConstraint(String, f64),
    AgentType(String),
}

/// Hierarchical delegation chain
pub struct DelegationChain {
    levels: Vec<DelegationLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationLevel {
    pub level: usize,
    pub name: String,
    pub authority: Authority,
    pub max_risk: RiskLevel,
    pub escalation_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authority {
    pub can_approve_commands: Vec<String>,
    pub can_approve_paths: Vec<String>,
    pub max_memory_mb: usize,
    pub max_cpu_seconds: u64,
    pub can_delegate: bool,
}

impl AdaptivePermissionNegotiator {
    pub fn new(
        base_negotiator: crate::permission_negotiation::PermissionNegotiator,
        memory: HybridMemory,
    ) -> Self {
        Self {
            base_negotiator: Arc::new(RwLock::new(base_negotiator)),
            memory: Arc::new(memory),
            trust_scores: Arc::new(RwLock::new(HashMap::new())),
            decision_history: Arc::new(RwLock::new(Vec::new())),
            learning_engine: Arc::new(LearningEngine::new()),
            delegation_chain: Arc::new(RwLock::new(DelegationChain::default())),
        }
    }
    
    /// Process a permission request with adaptive decision making
    pub async fn process_adaptive_request(
        &self,
        request: PermissionRequest,
    ) -> Result<PermissionResponse> {
        // Get or create trust score for agent
        let trust_score = self.get_or_create_trust_score(&request.agent_id).await;
        
        // Gather context factors
        let context_factors = self.gather_context_factors(&request).await;
        
        // Check if we have learned patterns that apply
        let pattern_recommendation = self.learning_engine
            .find_matching_pattern(&request, &context_factors)
            .await;
        
        // Make adaptive decision
        let decision = self.make_adaptive_decision(
            &request,
            trust_score,
            context_factors.clone(),
            pattern_recommendation,
        ).await?;
        
        // Record the decision
        self.record_decision(
            &request,
            &decision,
            context_factors,
        ).await;
        
        // Store in memory for future learning
        self.store_in_memory(&request, &decision).await?;
        
        Ok(decision)
    }
    
    async fn get_or_create_trust_score(&self, agent_id: &str) -> f64 {
        let mut scores = self.trust_scores.write().await;
        let score = scores.entry(agent_id.to_string()).or_insert_with(|| {
            TrustScore {
                agent_id: agent_id.to_string(),
                base_trust: 0.5,
                behavioral_trust: 0.5,
                contextual_trust: 0.5,
                temporal_decay: 0.1,
                last_updated: Utc::now(),
                violation_count: 0,
                success_count: 0,
            }
        });
        score.calculate_effective_trust()
    }
    
    async fn gather_context_factors(&self, request: &PermissionRequest) -> ContextFactors {
        // This would gather real system metrics
        ContextFactors {
            trust_level: self.get_or_create_trust_score(&request.agent_id).await,
            risk_assessment: self.assess_risk(request).await,
            resource_availability: ResourceState {
                cpu_available: 0.7,
                memory_available_mb: 2048,
                network_bandwidth_mbps: 100.0,
                storage_available_gb: 50.0,
            },
            concurrent_agents: 3,
            time_of_day: format!("{:02}:00", Utc::now().time().hour()),
            workload_intensity: 0.5,
        }
    }
    
    async fn assess_risk(&self, request: &PermissionRequest) -> RiskLevel {
        // Simple risk assessment based on requested permissions
        if request.requested_permissions.spawn_processes ||
           request.requested_permissions.network_access {
            RiskLevel::High
        } else if !request.requested_permissions.additional_commands.is_empty() {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        }
    }
    
    async fn make_adaptive_decision(
        &self,
        request: &PermissionRequest,
        trust_score: f64,
        context_factors: ContextFactors,
        pattern_recommendation: Option<Decision>,
    ) -> Result<PermissionResponse> {
        // Use pattern recommendation if available and trust is high
        if let Some(recommended) = pattern_recommendation {
            if trust_score > 0.7 {
                return Ok(PermissionResponse {
                    request_id: request.request_id,
                    decision: recommended.clone(),
                    granted_permissions: self.create_granted_permissions(request, &recommended),
                    reason: "Approved based on learned patterns".to_string(),
                    alternatives: vec![],
                    expires_at: Some(Utc::now() + Duration::hours(1)),
                });
            }
        }
        
        // Adaptive decision based on trust and context
        let decision = if trust_score > 0.8 && matches!(context_factors.risk_assessment, RiskLevel::Low) {
            Decision::Approved
        } else if trust_score > 0.5 && matches!(context_factors.risk_assessment, RiskLevel::Medium) {
            Decision::PartiallyApproved
        } else if trust_score < 0.3 {
            Decision::Denied
        } else {
            Decision::RequiresEscalation
        };
        
        Ok(PermissionResponse {
            request_id: request.request_id,
            decision: decision.clone(),
            granted_permissions: self.create_granted_permissions(request, &decision),
            reason: format!("Decision based on trust score: {:.2}", trust_score),
            alternatives: self.generate_alternatives(request, &context_factors).await,
            expires_at: Some(Utc::now() + Duration::minutes(30)),
        })
    }
    
    fn create_granted_permissions(
        &self,
        request: &PermissionRequest,
        decision: &Decision,
    ) -> Option<GrantedPermissions> {
        match decision {
            Decision::Approved => Some(GrantedPermissions {
                commands: request.requested_permissions.additional_commands.clone(),
                paths: request.requested_permissions.additional_paths.clone(),
                endpoints: request.requested_permissions.additional_endpoints.clone(),
                memory_mb: request.requested_permissions.increased_memory_mb,
                cpu_seconds: request.requested_permissions.increased_cpu_seconds,
                valid_until: Utc::now() + Duration::hours(1),
                revocable: true,
            }),
            Decision::PartiallyApproved => Some(GrantedPermissions {
                commands: request.requested_permissions.additional_commands
                    .iter()
                    .take(1)
                    .cloned()
                    .collect(),
                paths: vec![],
                endpoints: vec![],
                memory_mb: request.requested_permissions.increased_memory_mb.map(|m| m / 2),
                cpu_seconds: request.requested_permissions.increased_cpu_seconds.map(|c| c / 2),
                valid_until: Utc::now() + Duration::minutes(30),
                revocable: true,
            }),
            _ => None,
        }
    }
    
    async fn generate_alternatives(
        &self,
        request: &PermissionRequest,
        _context: &ContextFactors,
    ) -> Vec<Alternative> {
        let mut alternatives = vec![];
        
        // Suggest reduced permissions
        if !request.requested_permissions.additional_commands.is_empty() {
            alternatives.push(Alternative {
                description: "Request fewer commands".to_string(),
                modified_permissions: RequestedPermissions {
                    additional_commands: request.requested_permissions.additional_commands
                        .iter()
                        .take(1)
                        .cloned()
                        .collect(),
                    ..request.requested_permissions.clone()
                },
                likelihood_of_approval: 0.8,
            });
        }
        
        // Suggest time-limited permissions
        alternatives.push(Alternative {
            description: "Request for shorter duration".to_string(),
            modified_permissions: request.requested_permissions.clone(),
            likelihood_of_approval: 0.7,
        });
        
        alternatives
    }
    
    async fn record_decision(
        &self,
        request: &PermissionRequest,
        response: &PermissionResponse,
        context_factors: ContextFactors,
    ) {
        let record = DecisionRecord {
            request_id: request.request_id,
            agent_id: request.agent_id.clone(),
            decision: response.decision.clone(),
            requested: request.requested_permissions.clone(),
            granted: response.granted_permissions.clone(),
            context_factors,
            outcome: None,
            timestamp: Utc::now(),
        };
        
        let mut history = self.decision_history.write().await;
        history.push(record);
        
        // Keep only last 1000 records
        if history.len() > 1000 {
            history.drain(0..100);
        }
    }
    
    async fn store_in_memory(&self, request: &PermissionRequest, response: &PermissionResponse) -> Result<()> {
        let memory_item = MemoryItem {
            id: Uuid::new_v4(),
            content: MemoryContent::Episode {
                event_type: "permission_negotiation".to_string(),
                context: {
                    let mut ctx = HashMap::new();
                    ctx.insert("agent_id".to_string(), serde_json::json!(request.agent_id));
                    ctx.insert("decision".to_string(), serde_json::json!(response.decision));
                    ctx.insert("justification".to_string(), serde_json::json!(request.justification));
                    ctx
                },
                outcomes: vec![format!("{:?}", response.decision)],
            },
            timestamp: Utc::now(),
            access_count: 1,
            importance_score: match request.priority {
                Priority::Critical => 1.0,
                Priority::High => 0.8,
                Priority::Normal => 0.5,
                Priority::Low => 0.3,
            },
            decay_rate: 0.1,
            associations: vec![],
        };
        
        // Store in episodic memory
        let episode = Episode {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            duration: Duration::seconds(0),
            events: vec![],
            outcome: match response.decision {
                Decision::Approved => EpisodeOutcome::Success,
                Decision::Denied => EpisodeOutcome::Failure("Permission denied".to_string()),
                _ => EpisodeOutcome::Partial(0.5),
            },
            importance: memory_item.importance_score,
        };
        
        self.memory.episodic.add_episode(episode).await
            .map_err(|e| IntentError::Other(anyhow::anyhow!(e)))?;
        
        Ok(())
    }
    
    /// Update trust score based on actual outcome
    pub async fn update_from_outcome(
        &self,
        request_id: Uuid,
        outcome: DecisionOutcome,
    ) -> Result<()> {
        let mut history = self.decision_history.write().await;
        if let Some(record) = history.iter_mut().find(|r| r.request_id == request_id) {
            record.outcome = Some(outcome.clone());
            
            // Update trust score
            let mut scores = self.trust_scores.write().await;
            if let Some(score) = scores.get_mut(&record.agent_id) {
                score.update_from_outcome(outcome.was_correct);
            }
            
            // Update learning patterns
            self.learning_engine.update_patterns(record.clone()).await;
        }
        
        Ok(())
    }
}

impl LearningEngine {
    pub fn new() -> Self {
        Self {
            patterns: Arc::new(RwLock::new(Vec::new())),
            success_rate_threshold: 0.7,
        }
    }
    
    async fn find_matching_pattern(
        &self,
        request: &PermissionRequest,
        context: &ContextFactors,
    ) -> Option<Decision> {
        let patterns = self.patterns.read().await;
        
        for pattern in patterns.iter() {
            if pattern.success_rate > self.success_rate_threshold &&
               self.matches_conditions(&pattern.conditions, request, context) {
                return Some(pattern.recommended_decision.clone());
            }
        }
        
        None
    }
    
    fn matches_conditions(
        &self,
        conditions: &[PatternCondition],
        _request: &PermissionRequest,
        context: &ContextFactors,
    ) -> bool {
        conditions.iter().all(|condition| match condition {
            PatternCondition::TrustAbove(threshold) => context.trust_level > *threshold,
            PatternCondition::TrustBelow(threshold) => context.trust_level < *threshold,
            PatternCondition::RiskLevel(level) => context.risk_assessment == *level,
            _ => true, // Simplified for now
        })
    }
    
    async fn update_patterns(&self, record: DecisionRecord) {
        // This would implement pattern learning from outcomes
        // For now, just a placeholder
        let mut patterns = self.patterns.write().await;
        
        // Check if we should create a new pattern
        if let Some(outcome) = record.outcome {
            if outcome.was_correct {
                // Create or update pattern for successful decisions
                let pattern = DecisionPattern {
                    pattern_id: Uuid::new_v4(),
                    conditions: vec![
                        PatternCondition::TrustAbove(record.context_factors.trust_level - 0.1),
                        PatternCondition::RiskLevel(record.context_factors.risk_assessment),
                    ],
                    recommended_decision: record.decision,
                    success_rate: 1.0,
                    sample_count: 1,
                };
                patterns.push(pattern);
            }
        }
        
        // Keep only most recent patterns
        if patterns.len() > 100 {
            patterns.drain(0..20);
        }
    }
}

impl Default for DelegationChain {
    fn default() -> Self {
        Self {
            levels: vec![
                DelegationLevel {
                    level: 0,
                    name: "Automatic".to_string(),
                    authority: Authority {
                        can_approve_commands: vec!["ls".to_string(), "cat".to_string()],
                        can_approve_paths: vec!["/tmp/*".to_string()],
                        max_memory_mb: 100,
                        max_cpu_seconds: 10,
                        can_delegate: false,
                    },
                    max_risk: RiskLevel::Low,
                    escalation_threshold: 0.3,
                },
                DelegationLevel {
                    level: 1,
                    name: "Supervisor".to_string(),
                    authority: Authority {
                        can_approve_commands: vec!["git".to_string(), "npm".to_string()],
                        can_approve_paths: vec!["/workspace/*".to_string()],
                        max_memory_mb: 1000,
                        max_cpu_seconds: 60,
                        can_delegate: true,
                    },
                    max_risk: RiskLevel::Medium,
                    escalation_threshold: 0.6,
                },
                DelegationLevel {
                    level: 2,
                    name: "Administrator".to_string(),
                    authority: Authority {
                        can_approve_commands: vec!["*".to_string()],
                        can_approve_paths: vec!["*".to_string()],
                        max_memory_mb: 10000,
                        max_cpu_seconds: 3600,
                        can_delegate: true,
                    },
                    max_risk: RiskLevel::Critical,
                    escalation_threshold: 1.0,
                },
            ],
        }
    }
}