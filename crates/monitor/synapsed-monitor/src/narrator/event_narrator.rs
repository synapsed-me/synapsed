//! Event narrator that converts technical events into human-readable stories
//!
//! This module takes technical events and transforms them into natural language
//! narratives that are easy for humans to understand.

use crate::{
    aggregator::{CorrelatedEvent, EventPattern},
    collector::CollectedEvent,
    views::{TaskView, AgentView, SystemHealthView},
    Result,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Main event narrator
pub struct EventNarrator {
    /// Narrative style preference
    style: NarrativeStyle,
    
    /// Template engine for generating narratives
    template_engine: super::TemplateEngine,
}

/// Style of narrative generation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NarrativeStyle {
    /// Brief, concise narratives
    Concise,
    /// Detailed technical narratives
    Technical,
    /// Conversational, friendly narratives
    Conversational,
    /// Executive summary style
    Executive,
}

/// A generated narrative
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Narrative {
    /// Type of narrative
    pub narrative_type: NarrativeType,
    
    /// Main narrative text
    pub text: String,
    
    /// Key points extracted
    pub key_points: Vec<String>,
    
    /// Severity/importance
    pub importance: Importance,
    
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Related entity IDs
    pub related_entities: Vec<String>,
    
    /// Suggested actions
    pub actions: Vec<SuggestedAction>,
}

/// Type of narrative
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NarrativeType {
    TaskUpdate,
    AgentBehavior,
    SystemHealth,
    Incident,
    Performance,
    Anomaly,
}

/// Importance level
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Importance {
    Low,
    Medium,
    High,
    Critical,
}

/// Suggested action for the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAction {
    pub action: String,
    pub reason: String,
    pub urgency: Importance,
}

impl EventNarrator {
    pub fn new(style: NarrativeStyle) -> Self {
        Self {
            style,
            template_engine: super::TemplateEngine::new(),
        }
    }
    
    /// Narrate a correlated event
    pub fn narrate_event(&self, event: &CorrelatedEvent) -> Narrative {
        let text = match &event.pattern {
            Some(EventPattern::NormalExecution) => {
                self.narrate_normal_execution(event)
            },
            Some(EventPattern::FailedExecution) => {
                self.narrate_failed_execution(event)
            },
            Some(EventPattern::RetrySuccess) => {
                self.narrate_retry_success(event)
            },
            Some(EventPattern::RepeatedFailure) => {
                self.narrate_repeated_failure(event)
            },
            Some(EventPattern::ResourceContention) => {
                self.narrate_resource_contention(event)
            },
            Some(EventPattern::AnomalousBehavior) => {
                self.narrate_anomaly(event)
            },
            Some(EventPattern::PerformanceDegradation) => {
                self.narrate_performance_issue(event)
            },
            None => self.narrate_generic_event(event),
        };
        
        let key_points = self.extract_key_points(event);
        let importance = self.determine_importance(event);
        let actions = self.suggest_actions(event);
        
        Narrative {
            narrative_type: self.determine_narrative_type(event),
            text,
            key_points,
            importance,
            timestamp: Utc::now(),
            related_entities: vec![event.intent_id.to_string()],
            actions,
        }
    }
    
    /// Narrate a task view update
    pub fn narrate_task(&self, task: &TaskView) -> Narrative {
        let text = match self.style {
            NarrativeStyle::Concise => {
                format!("Task '{}' is {} ({:.1}% complete).",
                    task.name, task.status_message(), task.progress)
            },
            NarrativeStyle::Technical => {
                format!("Task {} (ID: {}) in phase {:?} with status {:?}. Progress: {:.1}%, {} agents assigned, {} events recorded.",
                    task.name, task.task_id, task.phase, task.status, task.progress,
                    task.agents.len(), task.timeline.len())
            },
            NarrativeStyle::Conversational => {
                format!("I'm working on '{}'. {}\nI've completed {:.1}% of the work so far.",
                    task.name, task.status_message(), task.progress)
            },
            NarrativeStyle::Executive => {
                format!("'{}': {} - {:.1}% complete.",
                    task.name,
                    match task.status {
                        crate::views::TaskStatus::Executing => "In Progress",
                        crate::views::TaskStatus::Completed => "Complete",
                        crate::views::TaskStatus::Failed => "Failed",
                        _ => "Pending",
                    },
                    task.progress)
            },
        };
        
        let mut key_points = vec![
            format!("Status: {:?}", task.status),
            format!("Progress: {:.1}%", task.progress),
        ];
        
        if let Some(remaining) = task.time_remaining() {
            key_points.push(format!("Time remaining: {} minutes", 
                remaining.num_minutes()));
        }
        
        Narrative {
            narrative_type: NarrativeType::TaskUpdate,
            text,
            key_points,
            importance: if task.status.is_terminal() { 
                Importance::High 
            } else { 
                Importance::Medium 
            },
            timestamp: Utc::now(),
            related_entities: vec![task.task_id.to_string()],
            actions: Vec::new(),
        }
    }
    
    /// Narrate an agent view update
    pub fn narrate_agent(&self, agent: &AgentView) -> Narrative {
        let text = match self.style {
            NarrativeStyle::Concise => {
                agent.status_message()
            },
            NarrativeStyle::Technical => {
                format!("Agent {} ({}): Status={:?}, Trust={:.2}, Performance={:.1}%, Anomalies={}",
                    agent.name, agent.agent_id, agent.status, agent.trust.score,
                    agent.performance.success_rate * 100.0, agent.anomalies.len())
            },
            NarrativeStyle::Conversational => {
                format!("{} is {}. Trust level: {} ({:.0}%). Recent performance: {:.0}% success rate.",
                    agent.name,
                    match agent.status {
                        crate::views::AgentStatus::Active => "busy working",
                        crate::views::AgentStatus::Idle => "ready for tasks",
                        crate::views::AgentStatus::Error => "having issues",
                        _ => "in an unusual state",
                    },
                    agent.trust_stars(),
                    agent.trust.score * 100.0,
                    agent.performance.success_rate * 100.0)
            },
            NarrativeStyle::Executive => {
                format!("{}: {} (Trust: {:.0}%)",
                    agent.name,
                    agent.health_indicator().to_status(),
                    agent.trust.score * 100.0)
            },
        };
        
        let mut key_points = vec![
            format!("Trust: {:?}", agent.trust.category),
            format!("Performance: {:.1}%", agent.performance.success_rate * 100.0),
        ];
        
        if !agent.anomalies.is_empty() {
            key_points.push(format!("{} recent anomalies", agent.anomalies.len()));
        }
        
        let importance = if !agent.anomalies.is_empty() {
            Importance::High
        } else if agent.trust.score < 0.5 {
            Importance::Medium
        } else {
            Importance::Low
        };
        
        Narrative {
            narrative_type: NarrativeType::AgentBehavior,
            text,
            key_points,
            importance,
            timestamp: Utc::now(),
            related_entities: vec![agent.agent_id.clone()],
            actions: self.suggest_agent_actions(agent),
        }
    }
    
    /// Narrate system health
    pub fn narrate_health(&self, health: &SystemHealthView) -> Narrative {
        let text = match self.style {
            NarrativeStyle::Concise => {
                health.summary()
            },
            NarrativeStyle::Technical => {
                format!("System Health: {:?} (Score: {:.1}/100). Services: {}/{} online. CPU: {:.1}%, Memory: {:.1}%, {} active tasks, {} alerts.",
                    health.status, health.health_score, 
                    health.services.iter().filter(|s| s.status == crate::views::ServiceStatus::Online).count(),
                    health.services.len(),
                    health.metrics.cpu_usage, health.metrics.memory_usage,
                    health.metrics.active_tasks, health.alerts.len())
            },
            NarrativeStyle::Conversational => {
                match health.status {
                    crate::views::HealthStatus::Healthy => {
                        format!("Everything is running smoothly! All {} services are operational and system resources are at comfortable levels.", 
                            health.services.len())
                    },
                    crate::views::HealthStatus::Degraded => {
                        format!("The system is experiencing some minor issues. {} alerts are active but nothing critical. Performance may be slightly affected.",
                            health.alerts.len())
                    },
                    _ => {
                        format!("We're experiencing system issues. {} I'm monitoring the situation closely.",
                            health.critical_issue().unwrap_or_else(|| "Multiple components are affected.".to_string()))
                    }
                }
            },
            NarrativeStyle::Executive => {
                if let Some(issue) = health.critical_issue() {
                    format!("System Status: {:?} - {}", health.status, issue)
                } else {
                    format!("System Status: {:?} - Score: {:.0}/100", 
                        health.status, health.health_score)
                }
            },
        };
        
        let mut key_points = vec![
            format!("Health Score: {:.1}/100", health.health_score),
            format!("Active Alerts: {}", health.alerts.len()),
        ];
        
        if let Some(issue) = health.critical_issue() {
            key_points.insert(0, issue);
        }
        
        let importance = match health.status {
            crate::views::HealthStatus::Critical | crate::views::HealthStatus::Down => Importance::Critical,
            crate::views::HealthStatus::Impaired => Importance::High,
            crate::views::HealthStatus::Degraded => Importance::Medium,
            _ => Importance::Low,
        };
        
        Narrative {
            narrative_type: NarrativeType::SystemHealth,
            text,
            key_points,
            importance,
            timestamp: Utc::now(),
            related_entities: Vec::new(),
            actions: self.suggest_health_actions(health),
        }
    }
    
    // Private helper methods
    
    fn narrate_normal_execution(&self, event: &CorrelatedEvent) -> String {
        match self.style {
            NarrativeStyle::Concise => {
                format!("Task executed successfully in {}ms.",
                    event.time_window.end.signed_duration_since(event.time_window.start).num_milliseconds())
            },
            _ => {
                format!("The task completed successfully. It started at {} and finished at {}, processing {} events without any issues.",
                    event.time_window.start.format("%H:%M:%S"),
                    event.time_window.end.format("%H:%M:%S"),
                    event.substrates_events.len() + event.serventis_events.len())
            }
        }
    }
    
    fn narrate_failed_execution(&self, event: &CorrelatedEvent) -> String {
        "The task failed during execution. Review the error details to understand what went wrong.".to_string()
    }
    
    fn narrate_retry_success(&self, event: &CorrelatedEvent) -> String {
        "The task succeeded after retrying. The initial attempt failed but the retry was successful.".to_string()
    }
    
    fn narrate_repeated_failure(&self, event: &CorrelatedEvent) -> String {
        "The task failed multiple times despite retries. This indicates a persistent issue that needs investigation.".to_string()
    }
    
    fn narrate_resource_contention(&self, event: &CorrelatedEvent) -> String {
        "High resource contention detected. Multiple tasks are competing for resources, causing delays.".to_string()
    }
    
    fn narrate_anomaly(&self, event: &CorrelatedEvent) -> String {
        "Anomalous behavior detected. An agent is behaving unexpectedly - this requires review.".to_string()
    }
    
    fn narrate_performance_issue(&self, event: &CorrelatedEvent) -> String {
        "Performance degradation observed. Response times are increasing and may affect user experience.".to_string()
    }
    
    fn narrate_generic_event(&self, event: &CorrelatedEvent) -> String {
        format!("{} events occurred in this time window.", 
            event.substrates_events.len() + event.serventis_events.len())
    }
    
    fn extract_key_points(&self, event: &CorrelatedEvent) -> Vec<String> {
        let mut points = Vec::new();
        
        if let Some(pattern) = &event.pattern {
            points.push(format!("Pattern: {:?}", pattern));
        }
        
        points.push(format!("Duration: {}ms", 
            event.time_window.end.signed_duration_since(event.time_window.start).num_milliseconds()));
        
        points.push(format!("Events: {} Substrates, {} Serventis",
            event.substrates_events.len(), event.serventis_events.len()));
        
        points
    }
    
    fn determine_importance(&self, event: &CorrelatedEvent) -> Importance {
        match &event.pattern {
            Some(EventPattern::AnomalousBehavior) => Importance::Critical,
            Some(EventPattern::RepeatedFailure) => Importance::High,
            Some(EventPattern::FailedExecution) | Some(EventPattern::ResourceContention) => Importance::Medium,
            _ => Importance::Low,
        }
    }
    
    fn determine_narrative_type(&self, event: &CorrelatedEvent) -> NarrativeType {
        match &event.pattern {
            Some(EventPattern::AnomalousBehavior) => NarrativeType::Anomaly,
            Some(EventPattern::PerformanceDegradation) => NarrativeType::Performance,
            _ => NarrativeType::TaskUpdate,
        }
    }
    
    fn suggest_actions(&self, event: &CorrelatedEvent) -> Vec<SuggestedAction> {
        let mut actions = Vec::new();
        
        match &event.pattern {
            Some(EventPattern::RepeatedFailure) => {
                actions.push(SuggestedAction {
                    action: "Review error logs".to_string(),
                    reason: "Multiple failures indicate a persistent issue".to_string(),
                    urgency: Importance::High,
                });
            },
            Some(EventPattern::AnomalousBehavior) => {
                actions.push(SuggestedAction {
                    action: "Review agent permissions".to_string(),
                    reason: "Anomalous behavior may indicate compromised agent".to_string(),
                    urgency: Importance::Critical,
                });
            },
            Some(EventPattern::ResourceContention) => {
                actions.push(SuggestedAction {
                    action: "Consider scaling resources".to_string(),
                    reason: "High contention is causing delays".to_string(),
                    urgency: Importance::Medium,
                });
            },
            _ => {}
        }
        
        actions
    }
    
    fn suggest_agent_actions(&self, agent: &AgentView) -> Vec<SuggestedAction> {
        let mut actions = Vec::new();
        
        if agent.trust.score < 0.3 {
            actions.push(SuggestedAction {
                action: "Review agent configuration".to_string(),
                reason: "Trust level is critically low".to_string(),
                urgency: Importance::High,
            });
        }
        
        if agent.capability_divergence() > 0.5 {
            actions.push(SuggestedAction {
                action: "Update agent capabilities".to_string(),
                reason: "Large divergence between declared and used tools".to_string(),
                urgency: Importance::Medium,
            });
        }
        
        actions
    }
    
    fn suggest_health_actions(&self, health: &SystemHealthView) -> Vec<SuggestedAction> {
        let mut actions = Vec::new();
        
        if health.metrics.cpu_usage > 80.0 {
            actions.push(SuggestedAction {
                action: "Scale compute resources".to_string(),
                reason: format!("CPU usage at {:.1}%", health.metrics.cpu_usage),
                urgency: Importance::High,
            });
        }
        
        if health.metrics.memory_usage > 80.0 {
            actions.push(SuggestedAction {
                action: "Increase memory allocation".to_string(),
                reason: format!("Memory usage at {:.1}%", health.metrics.memory_usage),
                urgency: Importance::High,
            });
        }
        
        actions
    }
}