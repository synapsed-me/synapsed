//! Agent-centric view showing behavior and trust metrics
//!
//! This module provides human-readable views of agent status,
//! behavior patterns, and trust levels.

use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Complete view of an agent's current state and history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentView {
    /// Unique agent identifier
    pub agent_id: String,
    
    /// Human-readable agent name
    pub name: String,
    
    /// Agent type/role
    pub agent_type: String,
    
    /// Current status
    pub status: AgentStatus,
    
    /// Current trust level
    pub trust: TrustLevel,
    
    /// Current activity description
    pub current_activity: Option<String>,
    
    /// Tools the agent can use
    pub capabilities: AgentCapabilities,
    
    /// Behavior patterns
    pub behavior: BehaviorProfile,
    
    /// Performance metrics
    pub performance: PerformanceStats,
    
    /// Recent anomalies
    pub anomalies: Vec<AnomalyEvent>,
    
    /// Trust history
    pub trust_history: Vec<TrustChange>,
}

/// Agent operational status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    /// Agent is idle
    Idle,
    /// Agent is working on a task
    Active,
    /// Agent is waiting for resources
    Waiting,
    /// Agent encountered an error
    Error,
    /// Agent is offline/unavailable
    Offline,
}

impl AgentStatus {
    pub fn to_color(&self) -> &str {
        match self {
            Self::Idle => "gray",
            Self::Active => "green",
            Self::Waiting => "yellow",
            Self::Error => "red",
            Self::Offline => "dark-gray",
        }
    }
    
    pub fn to_icon(&self) -> &str {
        match self {
            Self::Idle => "⏸",
            Self::Active => "▶",
            Self::Waiting => "⏳",
            Self::Error => "⚠",
            Self::Offline => "⭕",
        }
    }
}

/// Trust level with visual representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustLevel {
    /// Numeric trust score (0.0 to 1.0)
    pub score: f32,
    
    /// Trust category
    pub category: TrustCategory,
    
    /// Factors affecting trust
    pub factors: Vec<TrustFactor>,
    
    /// Visual representation (e.g., number of stars)
    pub visual: String,
}

/// Trust categories
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustCategory {
    /// Fully trusted (0.9-1.0)
    FullyTrusted,
    /// Highly trusted (0.7-0.9)
    HighlyTrusted,
    /// Moderately trusted (0.5-0.7)
    ModeratelyTrusted,
    /// Limited trust (0.3-0.5)
    LimitedTrust,
    /// Minimal trust (0.1-0.3)
    MinimalTrust,
    /// Untrusted (0.0-0.1)
    Untrusted,
}

impl TrustCategory {
    pub fn from_score(score: f32) -> Self {
        match score {
            s if s >= 0.9 => Self::FullyTrusted,
            s if s >= 0.7 => Self::HighlyTrusted,
            s if s >= 0.5 => Self::ModeratelyTrusted,
            s if s >= 0.3 => Self::LimitedTrust,
            s if s >= 0.1 => Self::MinimalTrust,
            _ => Self::Untrusted,
        }
    }
    
    pub fn to_color(&self) -> &str {
        match self {
            Self::FullyTrusted => "bright-green",
            Self::HighlyTrusted => "green",
            Self::ModeratelyTrusted => "yellow",
            Self::LimitedTrust => "orange",
            Self::MinimalTrust => "red",
            Self::Untrusted => "dark-red",
        }
    }
}

/// Factor affecting trust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustFactor {
    pub name: String,
    pub impact: f32, // Positive or negative impact on trust
    pub description: String,
}

/// Agent capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// Tools the agent declared it can use
    pub declared_tools: HashSet<String>,
    
    /// Tools the agent actually uses
    pub used_tools: HashSet<String>,
    
    /// Inferred capabilities from behavior
    pub inferred_capabilities: Vec<String>,
    
    /// Tool usage frequency
    pub tool_frequency: HashMap<String, usize>,
    
    /// Permission level
    pub permission_level: PermissionLevel,
}

/// Permission levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionLevel {
    /// Can only read data
    ReadOnly,
    /// Can read and write local files
    ReadWrite,
    /// Can execute commands
    Execute,
    /// Can access network
    Network,
    /// Full system access
    Full,
}

/// Behavior profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorProfile {
    /// Common execution patterns
    pub patterns: Vec<ExecutionPattern>,
    
    /// Typical task duration
    pub typical_duration: Duration,
    
    /// Preferred working hours
    pub active_hours: Vec<u32>,
    
    /// Resource usage profile
    pub resource_usage: ResourceProfile,
}

/// Execution pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPattern {
    pub name: String,
    pub tool_sequence: Vec<String>,
    pub frequency: usize,
    pub success_rate: f32,
}

/// Resource usage profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceProfile {
    pub avg_cpu: f32,
    pub avg_memory: f32,
    pub avg_network: f32,
}

/// Performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    /// Total tasks completed
    pub tasks_completed: usize,
    
    /// Success rate
    pub success_rate: f32,
    
    /// Average task duration
    pub avg_duration: Duration,
    
    /// Error rate
    pub error_rate: f32,
    
    /// Retry rate
    pub retry_rate: f32,
}

/// Anomaly event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyEvent {
    pub timestamp: DateTime<Utc>,
    pub anomaly_type: String,
    pub description: String,
    pub severity: AnomalySeverity,
    pub resolution: Option<String>,
}

/// Anomaly severity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl AnomalySeverity {
    pub fn to_color(&self) -> &str {
        match self {
            Self::Low => "yellow",
            Self::Medium => "orange",
            Self::High => "red",
            Self::Critical => "dark-red",
        }
    }
}

/// Trust change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustChange {
    pub timestamp: DateTime<Utc>,
    pub old_score: f32,
    pub new_score: f32,
    pub reason: String,
}

impl AgentView {
    /// Create a human-readable status message
    pub fn status_message(&self) -> String {
        match &self.status {
            AgentStatus::Idle => format!("{} is idle and ready for tasks", self.name),
            AgentStatus::Active => {
                if let Some(activity) = &self.current_activity {
                    format!("{} is {}", self.name, activity)
                } else {
                    format!("{} is actively working", self.name)
                }
            },
            AgentStatus::Waiting => format!("{} is waiting for resources", self.name),
            AgentStatus::Error => format!("{} encountered an error", self.name),
            AgentStatus::Offline => format!("{} is offline", self.name),
        }
    }
    
    /// Get trust level as stars (visual representation)
    pub fn trust_stars(&self) -> String {
        let stars = (self.trust.score * 5.0).round() as usize;
        "⭐".repeat(stars.min(5))
    }
    
    /// Get capability divergence (difference between declared and used tools)
    pub fn capability_divergence(&self) -> f32 {
        let declared = &self.capabilities.declared_tools;
        let used = &self.capabilities.used_tools;
        
        if declared.is_empty() {
            return 0.0;
        }
        
        let intersection = declared.intersection(used).count();
        let union = declared.union(used).count();
        
        if union == 0 {
            0.0
        } else {
            1.0 - (intersection as f32 / union as f32)
        }
    }
    
    /// Get health indicator combining multiple factors
    pub fn health_indicator(&self) -> HealthIndicator {
        let trust_health = self.trust.score;
        let performance_health = self.performance.success_rate;
        let anomaly_health = 1.0 - (self.anomalies.len() as f32 / 10.0).min(1.0);
        
        let overall = (trust_health + performance_health + anomaly_health) / 3.0;
        
        HealthIndicator {
            overall,
            trust: trust_health,
            performance: performance_health,
            anomaly: anomaly_health,
        }
    }
}

/// Health indicator combining multiple factors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthIndicator {
    pub overall: f32,
    pub trust: f32,
    pub performance: f32,
    pub anomaly: f32,
}

impl HealthIndicator {
    pub fn to_status(&self) -> &str {
        match self.overall {
            s if s >= 0.8 => "Healthy",
            s if s >= 0.6 => "Good",
            s if s >= 0.4 => "Fair",
            s if s >= 0.2 => "Poor",
            _ => "Critical",
        }
    }
}