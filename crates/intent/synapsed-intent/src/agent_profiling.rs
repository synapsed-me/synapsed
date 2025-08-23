//! Agent profiling system
//! 
//! This module builds behavioral profiles from agent execution patterns,
//! learning what tools agents actually use vs. declared and adapting permissions.

use crate::{
    dynamic_agents::SubAgentDefinition,
    Result,
};
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;
use chrono::{DateTime, Utc, Timelike};

/// Agent profiling system that tracks and learns from agent behavior
pub struct AgentProfilingSystem {
    profiles: Arc<RwLock<HashMap<String, AgentProfile>>>,
    behavior_patterns: Arc<RwLock<HashMap<String, BehaviorPattern>>>,
    anomaly_detector: AnomalyDetector,
}

/// Comprehensive profile of an agent's behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub agent_id: String,
    pub agent_name: String,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub declared_tools: HashSet<String>,
    pub actually_used_tools: HashSet<String>,
    pub tool_usage_frequency: HashMap<String, usize>,
    pub capabilities_demonstrated: HashSet<String>,
    pub execution_patterns: Vec<ExecutionPattern>,
    pub performance_metrics: PerformanceMetrics,
    pub trust_history: Vec<TrustEvent>,
    pub anomalies_detected: Vec<Anomaly>,
    pub adaptation_suggestions: Vec<String>,
}

/// Pattern of execution behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPattern {
    pub pattern_id: String,
    pub tool_sequence: Vec<String>,
    pub frequency: usize,
    pub avg_duration_ms: u64,
    pub success_rate: f64,
    pub typical_time_of_day: Option<String>,
    pub typical_context: Option<String>,
}

/// Performance metrics for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_tasks: usize,
    pub successful_tasks: usize,
    pub failed_tasks: usize,
    pub avg_task_duration_ms: u64,
    pub resource_efficiency: f64,  // 0.0 to 1.0
    pub context_violations: usize,
    pub permission_requests: usize,
    pub permission_grants: usize,
}

/// Trust event in agent history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: TrustEventType,
    pub trust_before: f64,
    pub trust_after: f64,
    pub reason: String,
}

/// Type of trust event
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustEventType {
    Increase,
    Decrease,
    Reset,
    Manual,
}

/// Detected anomaly in agent behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub timestamp: DateTime<Utc>,
    pub anomaly_type: AnomalyType,
    pub description: String,
    pub severity: AnomalySeverity,
    pub recommended_action: String,
}

/// Type of anomaly
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalyType {
    UnusualToolUsage,
    PermissionEscalation,
    ResourceSpike,
    PatternDeviation,
    SuspiciousSequence,
    TimeAnomaly,
}

/// Severity of anomaly
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Behavior pattern across multiple agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorPattern {
    pub pattern_name: String,
    pub agents_exhibiting: HashSet<String>,
    pub tool_combinations: Vec<Vec<String>>,
    pub typical_outcomes: HashMap<String, f64>,  // Outcome -> probability
    pub risk_indicators: Vec<String>,
}

/// Anomaly detector for identifying unusual behavior
pub struct AnomalyDetector {
    thresholds: AnomalyThresholds,
    baseline_patterns: HashMap<String, BaselinePattern>,
}

/// Thresholds for anomaly detection
#[derive(Debug, Clone)]
pub struct AnomalyThresholds {
    pub tool_usage_deviation: f64,  // Standard deviations
    pub resource_spike_factor: f64,
    pub pattern_confidence: f64,
    pub time_window_hours: i64,
}

/// Baseline pattern for normal behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselinePattern {
    pub agent_id: String,
    pub normal_tools: HashSet<String>,
    pub normal_sequences: Vec<Vec<String>>,
    pub avg_resource_usage: ResourceUsage,
    pub typical_hours: Vec<u32>,  // Hours of day (0-23)
}

/// Resource usage profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub avg_memory_mb: f64,
    pub avg_cpu_percent: f64,
    pub avg_network_kbps: f64,
    pub avg_disk_iops: f64,
}

impl AgentProfilingSystem {
    pub fn new() -> Self {
        Self {
            profiles: Arc::new(RwLock::new(HashMap::new())),
            behavior_patterns: Arc::new(RwLock::new(HashMap::new())),
            anomaly_detector: AnomalyDetector::new(),
        }
    }

    /// Create or update agent profile
    pub async fn profile_agent(&self, agent_def: &SubAgentDefinition) -> Result<AgentProfile> {
        let mut profiles = self.profiles.write().await;
        
        let profile = profiles.entry(agent_def.name.clone()).or_insert_with(|| {
            AgentProfile {
                agent_id: uuid::Uuid::new_v4().to_string(),
                agent_name: agent_def.name.clone(),
                created_at: Utc::now(),
                last_active: Utc::now(),
                declared_tools: agent_def.tools.iter().cloned().collect(),
                actually_used_tools: HashSet::new(),
                tool_usage_frequency: HashMap::new(),
                capabilities_demonstrated: HashSet::new(),
                execution_patterns: Vec::new(),
                performance_metrics: PerformanceMetrics {
                    total_tasks: 0,
                    successful_tasks: 0,
                    failed_tasks: 0,
                    avg_task_duration_ms: 0,
                    resource_efficiency: 1.0,
                    context_violations: 0,
                    permission_requests: 0,
                    permission_grants: 0,
                },
                trust_history: Vec::new(),
                anomalies_detected: Vec::new(),
                adaptation_suggestions: Vec::new(),
            }
        });

        Ok(profile.clone())
    }

    /// Record tool usage
    pub async fn record_tool_usage(
        &self,
        agent_id: &str,
        tool: &str,
        success: bool,
        duration_ms: u64,
    ) {
        let mut profiles = self.profiles.write().await;
        
        if let Some(profile) = profiles.get_mut(agent_id) {
            profile.last_active = Utc::now();
            profile.actually_used_tools.insert(tool.to_string());
            *profile.tool_usage_frequency.entry(tool.to_string()).or_insert(0) += 1;
            
            if success {
                profile.performance_metrics.successful_tasks += 1;
            } else {
                profile.performance_metrics.failed_tasks += 1;
            }
            profile.performance_metrics.total_tasks += 1;
            
            // Update average duration
            let total = profile.performance_metrics.total_tasks;
            profile.performance_metrics.avg_task_duration_ms = 
                ((profile.performance_metrics.avg_task_duration_ms * (total - 1) as u64) + duration_ms) / total as u64;
        }
    }

    /// Record execution pattern
    pub async fn record_execution_pattern(
        &self,
        agent_id: &str,
        tool_sequence: Vec<String>,
        duration_ms: u64,
        success: bool,
    ) {
        let mut profiles = self.profiles.write().await;
        
        if let Some(profile) = profiles.get_mut(agent_id) {
            let pattern_id = tool_sequence.join("->");
            
            // Find or create pattern
            let pattern = profile.execution_patterns.iter_mut()
                .find(|p| p.pattern_id == pattern_id);
            
            if let Some(pattern) = pattern {
                pattern.frequency += 1;
                pattern.avg_duration_ms = 
                    ((pattern.avg_duration_ms * (pattern.frequency - 1) as u64) + duration_ms) / pattern.frequency as u64;
                
                if success {
                    pattern.success_rate = 
                        (pattern.success_rate * (pattern.frequency - 1) as f64 + 1.0) / pattern.frequency as f64;
                } else {
                    pattern.success_rate = 
                        (pattern.success_rate * (pattern.frequency - 1) as f64) / pattern.frequency as f64;
                }
            } else {
                profile.execution_patterns.push(ExecutionPattern {
                    pattern_id,
                    tool_sequence,
                    frequency: 1,
                    avg_duration_ms: duration_ms,
                    success_rate: if success { 1.0 } else { 0.0 },
                    typical_time_of_day: Some(format!("{}", Utc::now().hour())),
                    typical_context: None,
                });
            }
        }
    }

    /// Detect anomalies in agent behavior
    pub async fn detect_anomalies(
        &self,
        agent_id: &str,
        current_tools: &[String],
        resource_usage: &ResourceUsage,
    ) -> Vec<Anomaly> {
        let profiles = self.profiles.read().await;
        let mut anomalies = Vec::new();
        
        if let Some(profile) = profiles.get(agent_id) {
            // Check for unusual tool usage
            for tool in current_tools {
                if !profile.declared_tools.contains(tool) && !profile.actually_used_tools.contains(tool) {
                    anomalies.push(Anomaly {
                        timestamp: Utc::now(),
                        anomaly_type: AnomalyType::UnusualToolUsage,
                        description: format!("Agent using undeclared tool: {}", tool),
                        severity: AnomalySeverity::Medium,
                        recommended_action: "Review tool permissions".to_string(),
                    });
                }
            }
            
            // Check for resource spikes
            if let Some(baseline) = self.anomaly_detector.baseline_patterns.get(agent_id) {
                if resource_usage.avg_cpu_percent > baseline.avg_resource_usage.avg_cpu_percent * 2.0 {
                    anomalies.push(Anomaly {
                        timestamp: Utc::now(),
                        anomaly_type: AnomalyType::ResourceSpike,
                        description: "CPU usage spike detected".to_string(),
                        severity: AnomalySeverity::Medium,
                        recommended_action: "Monitor resource consumption".to_string(),
                    });
                }
            }
            
            // Check for suspicious sequences
            let sequence = current_tools.join("->");
            if self.is_suspicious_sequence(&sequence) {
                anomalies.push(Anomaly {
                    timestamp: Utc::now(),
                    anomaly_type: AnomalyType::SuspiciousSequence,
                    description: format!("Suspicious tool sequence: {}", sequence),
                    severity: AnomalySeverity::High,
                    recommended_action: "Review execution logs".to_string(),
                });
            }
        }
        
        anomalies
    }

    /// Check if a tool sequence is suspicious
    fn is_suspicious_sequence(&self, sequence: &str) -> bool {
        let suspicious_patterns = vec![
            "read_file->run_command->delete",
            "web_search->run_command->upload",
            "debug_shell->sudo",
        ];
        
        suspicious_patterns.iter().any(|p| sequence.contains(p))
    }

    /// Get adaptation suggestions for an agent
    pub async fn get_adaptation_suggestions(&self, agent_id: &str) -> Vec<String> {
        let profiles = self.profiles.read().await;
        let mut suggestions = Vec::new();
        
        if let Some(profile) = profiles.get(agent_id) {
            // Suggest removing unused declared tools
            let unused_tools: Vec<_> = profile.declared_tools
                .difference(&profile.actually_used_tools)
                .cloned()
                .collect();
            
            if !unused_tools.is_empty() {
                suggestions.push(format!("Remove unused tools: {}", unused_tools.join(", ")));
            }
            
            // Suggest adding frequently requested tools
            if profile.performance_metrics.permission_requests > profile.performance_metrics.permission_grants * 2 {
                suggestions.push("Consider granting more permissions upfront".to_string());
            }
            
            // Suggest optimization based on patterns
            let inefficient_patterns: Vec<_> = profile.execution_patterns.iter()
                .filter(|p| p.success_rate < 0.5)
                .map(|p| p.pattern_id.clone())
                .collect();
            
            if !inefficient_patterns.is_empty() {
                suggestions.push(format!("Optimize inefficient patterns: {}", inefficient_patterns.join(", ")));
            }
            
            // Resource efficiency suggestions
            if profile.performance_metrics.resource_efficiency < 0.7 {
                suggestions.push("Improve resource efficiency through better tool selection".to_string());
            }
        }
        
        suggestions
    }

    /// Compare declared vs actual tool usage
    pub async fn analyze_tool_divergence(&self, agent_id: &str) -> ToolDivergenceAnalysis {
        let profiles = self.profiles.read().await;
        
        if let Some(profile) = profiles.get(agent_id) {
            let declared_only: HashSet<_> = profile.declared_tools
                .difference(&profile.actually_used_tools)
                .cloned()
                .collect();
            
            let used_not_declared: HashSet<_> = profile.actually_used_tools
                .difference(&profile.declared_tools)
                .cloned()
                .collect();
            
            let overlap: HashSet<_> = profile.declared_tools
                .intersection(&profile.actually_used_tools)
                .cloned()
                .collect();
            
            let divergence_score = if profile.declared_tools.is_empty() {
                0.0
            } else {
                declared_only.len() as f64 / profile.declared_tools.len() as f64
            };
            
            ToolDivergenceAnalysis {
                agent_id: agent_id.to_string(),
                declared_only,
                used_not_declared,
                overlap,
                divergence_score,
                recommendation: if divergence_score > 0.5 {
                    "High divergence - consider updating agent definition".to_string()
                } else if divergence_score > 0.2 {
                    "Moderate divergence - review tool requirements".to_string()
                } else {
                    "Low divergence - agent definition is accurate".to_string()
                },
            }
        } else {
            ToolDivergenceAnalysis {
                agent_id: agent_id.to_string(),
                declared_only: HashSet::new(),
                used_not_declared: HashSet::new(),
                overlap: HashSet::new(),
                divergence_score: 0.0,
                recommendation: "No profile found".to_string(),
            }
        }
    }

    /// Update trust score based on behavior
    pub async fn update_trust_score(
        &self,
        agent_id: &str,
        event_type: TrustEventType,
        current_trust: f64,
        reason: &str,
    ) -> f64 {
        let mut profiles = self.profiles.write().await;
        
        let new_trust = match event_type {
            TrustEventType::Increase => (current_trust + 0.05).min(1.0),
            TrustEventType::Decrease => (current_trust - 0.1).max(0.0),
            TrustEventType::Reset => 0.5,
            TrustEventType::Manual => current_trust,  // Set externally
        };
        
        if let Some(profile) = profiles.get_mut(agent_id) {
            profile.trust_history.push(TrustEvent {
                timestamp: Utc::now(),
                event_type,
                trust_before: current_trust,
                trust_after: new_trust,
                reason: reason.to_string(),
            });
        }
        
        new_trust
    }
}

impl AnomalyDetector {
    pub fn new() -> Self {
        Self {
            thresholds: AnomalyThresholds {
                tool_usage_deviation: 2.0,
                resource_spike_factor: 2.5,
                pattern_confidence: 0.7,
                time_window_hours: 24,
            },
            baseline_patterns: HashMap::new(),
        }
    }

    /// Establish baseline for an agent
    pub fn establish_baseline(&mut self, agent_id: &str, profile: &AgentProfile) {
        let baseline = BaselinePattern {
            agent_id: agent_id.to_string(),
            normal_tools: profile.actually_used_tools.clone(),
            normal_sequences: profile.execution_patterns.iter()
                .filter(|p| p.success_rate > 0.7)
                .map(|p| p.tool_sequence.clone())
                .collect(),
            avg_resource_usage: ResourceUsage {
                avg_memory_mb: 100.0,  // Default values
                avg_cpu_percent: 25.0,
                avg_network_kbps: 100.0,
                avg_disk_iops: 50.0,
            },
            typical_hours: vec![9, 10, 11, 14, 15, 16],  // Business hours
        };
        
        self.baseline_patterns.insert(agent_id.to_string(), baseline);
    }
}

/// Analysis of tool usage divergence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDivergenceAnalysis {
    pub agent_id: String,
    pub declared_only: HashSet<String>,
    pub used_not_declared: HashSet<String>,
    pub overlap: HashSet<String>,
    pub divergence_score: f64,
    pub recommendation: String,
}