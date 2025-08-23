//! Task-centric view of intent execution
//!
//! This module provides a human-readable view of task execution,
//! showing the journey from planning through completion.

use synapsed_intent::{IntentId, HierarchicalIntent};
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete view of a task's execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskView {
    /// Unique task identifier
    pub task_id: IntentId,
    
    /// Human-readable task name
    pub name: String,
    
    /// Task description/goal
    pub description: String,
    
    /// Current status
    pub status: TaskStatus,
    
    /// Current execution phase
    pub phase: TaskPhase,
    
    /// Progress percentage (0-100)
    pub progress: f32,
    
    /// Task hierarchy (parent and children)
    pub hierarchy: TaskHierarchy,
    
    /// Assigned agents and their roles
    pub agents: Vec<AgentAssignment>,
    
    /// Timeline of significant events
    pub timeline: Vec<TimelineEvent>,
    
    /// Resource consumption
    pub resources: ResourceUsage,
    
    /// Estimated completion time
    pub estimated_completion: Option<DateTime<Utc>>,
    
    /// Performance metrics
    pub metrics: TaskMetrics,
}

/// Task execution status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Not yet started
    Pending,
    /// Currently being planned
    Planning,
    /// Being verified before execution
    Verifying,
    /// Currently executing
    Executing,
    /// Paused or suspended
    Suspended,
    /// Completed successfully
    Completed,
    /// Failed to complete
    Failed,
    /// Cancelled by user or system
    Cancelled,
}

impl TaskStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Planning | Self::Verifying | Self::Executing)
    }
    
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
    
    pub fn to_color(&self) -> &str {
        match self {
            Self::Pending => "gray",
            Self::Planning | Self::Verifying => "blue",
            Self::Executing => "yellow",
            Self::Suspended => "orange",
            Self::Completed => "green",
            Self::Failed => "red",
            Self::Cancelled => "gray",
        }
    }
}

/// Phase of task execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskPhase {
    /// Initial setup
    Initialization,
    /// Breaking down into sub-tasks
    Decomposition,
    /// Assigning agents to sub-tasks
    AgentAssignment,
    /// Verifying preconditions
    PreconditionCheck,
    /// Main execution
    Execution,
    /// Verifying postconditions
    PostconditionCheck,
    /// Cleanup and finalization
    Cleanup,
}

/// Task hierarchy information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHierarchy {
    /// Parent task ID (if this is a sub-task)
    pub parent_id: Option<IntentId>,
    
    /// Child task IDs
    pub children_ids: Vec<IntentId>,
    
    /// Depth in hierarchy (0 for root)
    pub depth: usize,
    
    /// Total number of descendants
    pub total_descendants: usize,
}

/// Agent assignment to a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAssignment {
    /// Agent identifier
    pub agent_id: String,
    
    /// Agent name
    pub agent_name: String,
    
    /// Role in this task
    pub role: String,
    
    /// Assignment timestamp
    pub assigned_at: DateTime<Utc>,
    
    /// Current agent status
    pub status: String,
    
    /// Trust level for this agent
    pub trust_level: f32,
}

/// Event in the task timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Event type
    pub event_type: String,
    
    /// Human-readable description
    pub description: String,
    
    /// Associated agent (if any)
    pub agent_id: Option<String>,
    
    /// Event severity/importance
    pub severity: EventSeverity,
    
    /// Additional context data
    pub context: HashMap<String, serde_json::Value>,
}

/// Event severity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Resource usage for the task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage percentage
    pub cpu_percent: f32,
    
    /// Memory usage in MB
    pub memory_mb: f32,
    
    /// Network bandwidth in KB/s
    pub network_kbps: f32,
    
    /// Queue depth
    pub queue_depth: usize,
    
    /// Number of active threads/workers
    pub active_workers: usize,
}

/// Task performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetrics {
    /// Total execution time so far
    pub execution_time: Duration,
    
    /// Time spent in each phase
    pub phase_durations: HashMap<String, Duration>,
    
    /// Number of retries
    pub retry_count: usize,
    
    /// Number of errors encountered
    pub error_count: usize,
    
    /// Success rate of sub-tasks
    pub subtask_success_rate: f32,
    
    /// Average sub-task completion time
    pub avg_subtask_time: Duration,
}

impl TaskView {
    /// Create a new task view from an intent
    pub fn from_intent(intent: &HierarchicalIntent) -> Self {
        Self {
            task_id: intent.id(),
            name: intent.goal().to_string(),
            description: intent.description.clone().unwrap_or_default(),
            status: TaskStatus::Pending,
            phase: TaskPhase::Initialization,
            progress: 0.0,
            hierarchy: TaskHierarchy {
                parent_id: None,
                children_ids: Vec::new(),
                depth: 0,
                total_descendants: 0,
            },
            agents: Vec::new(),
            timeline: Vec::new(),
            resources: ResourceUsage {
                cpu_percent: 0.0,
                memory_mb: 0.0,
                network_kbps: 0.0,
                queue_depth: 0,
                active_workers: 0,
            },
            estimated_completion: None,
            metrics: TaskMetrics {
                execution_time: Duration::zero(),
                phase_durations: HashMap::new(),
                retry_count: 0,
                error_count: 0,
                subtask_success_rate: 0.0,
                avg_subtask_time: Duration::zero(),
            },
        }
    }
    
    /// Update progress based on completed sub-tasks
    pub fn update_progress(&mut self, completed: usize, total: usize) {
        if total > 0 {
            self.progress = (completed as f32 / total as f32) * 100.0;
        }
    }
    
    /// Add a timeline event
    pub fn add_event(&mut self, event: TimelineEvent) {
        self.timeline.push(event);
        
        // Keep timeline sorted by timestamp
        self.timeline.sort_by_key(|e| e.timestamp);
    }
    
    /// Get a human-readable status message
    pub fn status_message(&self) -> String {
        match self.status {
            TaskStatus::Pending => "Task is waiting to start".to_string(),
            TaskStatus::Planning => "Planning task execution strategy".to_string(),
            TaskStatus::Verifying => "Verifying preconditions and requirements".to_string(),
            TaskStatus::Executing => format!("Executing - {:.1}% complete", self.progress),
            TaskStatus::Suspended => "Task is temporarily suspended".to_string(),
            TaskStatus::Completed => format!("Task completed successfully in {:?}", self.metrics.execution_time),
            TaskStatus::Failed => format!("Task failed after {} errors", self.metrics.error_count),
            TaskStatus::Cancelled => "Task was cancelled".to_string(),
        }
    }
    
    /// Get estimated time remaining
    pub fn time_remaining(&self) -> Option<Duration> {
        if let Some(completion) = self.estimated_completion {
            let now = Utc::now();
            if completion > now {
                Some(completion.signed_duration_since(now))
            } else {
                None
            }
        } else {
            None
        }
    }
}