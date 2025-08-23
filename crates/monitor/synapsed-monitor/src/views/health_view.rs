//! System health view showing overall system status
//!
//! This module provides a comprehensive view of system health,
//! combining data from multiple services and components.

use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete system health overview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealthView {
    /// Overall health status
    pub status: HealthStatus,
    
    /// Health score (0-100)
    pub health_score: f32,
    
    /// Individual service health
    pub services: Vec<ServiceHealth>,
    
    /// System metrics
    pub metrics: SystemMetrics,
    
    /// Active alerts
    pub alerts: Vec<HealthAlert>,
    
    /// Recent incidents
    pub incidents: Vec<Incident>,
    
    /// Recommendations for improvement
    pub recommendations: Vec<Recommendation>,
    
    /// Last update timestamp
    pub last_updated: DateTime<Utc>,
}

/// Overall health status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// All systems operational
    Healthy,
    /// Minor issues present
    Degraded,
    /// Major issues affecting performance
    Impaired,
    /// Critical issues requiring attention
    Critical,
    /// System is down
    Down,
}

impl HealthStatus {
    pub fn from_score(score: f32) -> Self {
        match score {
            s if s >= 90.0 => Self::Healthy,
            s if s >= 70.0 => Self::Degraded,
            s if s >= 50.0 => Self::Impaired,
            s if s >= 25.0 => Self::Critical,
            _ => Self::Down,
        }
    }
    
    pub fn to_color(&self) -> &str {
        match self {
            Self::Healthy => "green",
            Self::Degraded => "yellow",
            Self::Impaired => "orange",
            Self::Critical => "red",
            Self::Down => "dark-red",
        }
    }
    
    pub fn to_icon(&self) -> &str {
        match self {
            Self::Healthy => "✅",
            Self::Degraded => "⚠️",
            Self::Impaired => "🔶",
            Self::Critical => "🔴",
            Self::Down => "💀",
        }
    }
}

/// Health of an individual service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    /// Service identifier
    pub service_id: String,
    
    /// Service name
    pub name: String,
    
    /// Service type
    pub service_type: ServiceType,
    
    /// Current status
    pub status: ServiceStatus,
    
    /// Response time (milliseconds)
    pub response_time_ms: Option<f32>,
    
    /// Error rate (percentage)
    pub error_rate: f32,
    
    /// Throughput (requests per second)
    pub throughput: f32,
    
    /// Queue depth
    pub queue_depth: usize,
    
    /// Last successful check
    pub last_success: DateTime<Utc>,
}

/// Type of service
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceType {
    /// Intent execution service
    IntentExecutor,
    /// Agent manager
    AgentManager,
    /// Verification service
    Verifier,
    /// Monitoring service
    Monitor,
    /// Queue manager
    QueueManager,
    /// Data store
    Storage,
}

/// Service operational status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceStatus {
    /// Operating normally
    Online,
    /// Slow but responding
    Slow,
    /// Not responding
    Unresponsive,
    /// Service is offline
    Offline,
}

impl ServiceStatus {
    pub fn to_color(&self) -> &str {
        match self {
            Self::Online => "green",
            Self::Slow => "yellow",
            Self::Unresponsive => "orange",
            Self::Offline => "red",
        }
    }
}

/// System-wide metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// CPU usage percentage
    pub cpu_usage: f64,
    
    /// Memory usage percentage
    pub memory_usage: f64,
    
    /// Disk usage percentage
    pub disk_usage: f64,
    
    /// Network bandwidth usage (MB/s)
    pub network_bandwidth: f64,
    
    /// Active tasks
    pub active_tasks: usize,
    
    /// Queued tasks
    pub queued_tasks: usize,
    
    /// Active agents
    pub active_agents: usize,
    
    /// Total agents
    pub total_agents: usize,
    
    /// Uptime
    pub uptime: Duration,
    
    /// Request rate (per second)
    pub request_rate: f64,
    
    /// Average response time (ms)
    pub avg_response_time: f64,
}

/// Health alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAlert {
    /// Alert identifier
    pub alert_id: String,
    
    /// Alert severity
    pub severity: AlertSeverity,
    
    /// Alert category
    pub category: AlertCategory,
    
    /// Alert message
    pub message: String,
    
    /// Affected component
    pub component: String,
    
    /// When the alert was triggered
    pub triggered_at: DateTime<Utc>,
    
    /// Suggested action
    pub action: Option<String>,
}

/// Alert severity levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl AlertSeverity {
    pub fn to_color(&self) -> &str {
        match self {
            Self::Info => "blue",
            Self::Warning => "yellow",
            Self::Error => "orange",
            Self::Critical => "red",
        }
    }
}

/// Alert categories
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertCategory {
    Performance,
    Resource,
    Security,
    Availability,
    Configuration,
}

/// System incident
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    /// Incident ID
    pub incident_id: String,
    
    /// Incident title
    pub title: String,
    
    /// Description
    pub description: String,
    
    /// Severity
    pub severity: AlertSeverity,
    
    /// Start time
    pub started_at: DateTime<Utc>,
    
    /// Resolution time (if resolved)
    pub resolved_at: Option<DateTime<Utc>>,
    
    /// Impact description
    pub impact: String,
    
    /// Resolution steps taken
    pub resolution: Option<String>,
}

/// System improvement recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// Recommendation category
    pub category: RecommendationCategory,
    
    /// Priority
    pub priority: Priority,
    
    /// Title
    pub title: String,
    
    /// Description
    pub description: String,
    
    /// Expected impact
    pub impact: String,
    
    /// Implementation steps
    pub steps: Vec<String>,
}

/// Recommendation categories
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecommendationCategory {
    Performance,
    Scaling,
    Configuration,
    Security,
    Monitoring,
}

/// Priority levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Urgent,
}

impl SystemHealthView {
    /// Create a health summary message
    pub fn summary(&self) -> String {
        let active_alerts = self.alerts.len();
        let critical_alerts = self.alerts.iter()
            .filter(|a| a.severity == AlertSeverity::Critical)
            .count();
        
        match self.status {
            HealthStatus::Healthy => {
                format!("System is healthy. All {} services operational.", self.services.len())
            },
            HealthStatus::Degraded => {
                format!("System is degraded with {} active alerts ({} critical).", 
                    active_alerts, critical_alerts)
            },
            HealthStatus::Impaired => {
                format!("System is impaired. {} services affected, {} alerts active.", 
                    self.services.iter().filter(|s| s.status != ServiceStatus::Online).count(),
                    active_alerts)
            },
            HealthStatus::Critical => {
                format!("CRITICAL: System experiencing major issues. {} critical alerts active.", 
                    critical_alerts)
            },
            HealthStatus::Down => {
                "System is DOWN. Immediate attention required.".to_string()
            }
        }
    }
    
    /// Calculate overall health score from components
    pub fn calculate_health_score(&self) -> f32 {
        let service_score = self.services.iter()
            .map(|s| match s.status {
                ServiceStatus::Online => 100.0,
                ServiceStatus::Slow => 70.0,
                ServiceStatus::Unresponsive => 30.0,
                ServiceStatus::Offline => 0.0,
            })
            .sum::<f32>() / self.services.len().max(1) as f32;
        
        let alert_penalty = self.alerts.iter()
            .map(|a| match a.severity {
                AlertSeverity::Info => 2.0,
                AlertSeverity::Warning => 5.0,
                AlertSeverity::Error => 10.0,
                AlertSeverity::Critical => 25.0,
            })
            .sum::<f32>();
        
        let resource_score = 100.0 
            - (self.metrics.cpu_usage.max(self.metrics.memory_usage)
                .max(self.metrics.disk_usage)) as f32;
        
        ((service_score + resource_score) / 2.0 - alert_penalty).max(0.0).min(100.0)
    }
    
    /// Get the most critical issue
    pub fn critical_issue(&self) -> Option<String> {
        // Check for critical alerts first
        if let Some(alert) = self.alerts.iter()
            .find(|a| a.severity == AlertSeverity::Critical) {
            return Some(alert.message.clone());
        }
        
        // Check for offline services
        if let Some(service) = self.services.iter()
            .find(|s| s.status == ServiceStatus::Offline) {
            return Some(format!("{} service is offline", service.name));
        }
        
        // Check resource usage
        if self.metrics.cpu_usage > 90.0 {
            return Some(format!("CPU usage critical: {:.1}%", self.metrics.cpu_usage));
        }
        if self.metrics.memory_usage > 90.0 {
            return Some(format!("Memory usage critical: {:.1}%", self.metrics.memory_usage));
        }
        
        None
    }
}