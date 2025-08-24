//! Common types for swarm coordination

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Unique identifier for an agent in the swarm
pub type AgentId = Uuid;

/// Unique identifier for a swarm
pub type SwarmId = Uuid;

/// Unique identifier for a task
pub type TaskId = Uuid;

/// Agent role in the swarm
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRole {
    /// Coordinator agent that manages the swarm
    Coordinator,
    /// Worker agent that executes tasks
    Worker,
    /// Verifier agent that validates execution
    Verifier,
    /// Observer agent that monitors without participating
    Observer,
    /// Specialized agent with custom role
    Specialized(String),
}

/// Current status of an agent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    /// Agent is initializing
    Initializing,
    /// Agent is ready to accept tasks
    Ready,
    /// Agent is currently executing a task
    Busy,
    /// Agent is cooperating with others
    Cooperating,
    /// Agent is in a degraded state
    Degraded,
    /// Agent has failed
    Failed,
    /// Agent is shutting down
    ShuttingDown,
}

/// Task assignment to an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    /// Unique task ID
    pub task_id: TaskId,
    /// Agent assigned to the task
    pub agent_id: AgentId,
    /// Intent to be executed
    pub intent: synapsed_intent::HierarchicalIntent,
    /// Promise made by the agent
    pub promise: Option<synapsed_promise::Promise>,
    /// Parent task if this is a sub-task
    pub parent_task: Option<TaskId>,
    /// Context for execution
    pub context: HashMap<String, serde_json::Value>,
    /// Verification requirements
    pub verification_required: bool,
    /// Deadline for completion
    pub deadline: Option<DateTime<Utc>>,
}

/// Result of task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Task that was executed
    pub task_id: TaskId,
    /// Agent that executed the task
    pub agent_id: AgentId,
    /// Whether the task succeeded
    pub success: bool,
    /// Output data from the task
    pub output: Option<serde_json::Value>,
    /// Error message if failed
    pub error: Option<String>,
    /// Verification proof if available
    pub verification_proof: Option<synapsed_verify::VerificationProof>,
    /// Execution duration
    pub duration_ms: u64,
    /// Timestamp of completion
    pub completed_at: DateTime<Utc>,
}

/// Coordination event in the swarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwarmEvent {
    /// Agent joined the swarm
    AgentJoined {
        agent_id: AgentId,
        role: AgentRole,
        timestamp: DateTime<Utc>,
    },
    /// Agent left the swarm
    AgentLeft {
        agent_id: AgentId,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    /// Task was assigned
    TaskAssigned {
        task_id: TaskId,
        agent_id: AgentId,
        timestamp: DateTime<Utc>,
    },
    /// Task was completed
    TaskCompleted {
        task_id: TaskId,
        agent_id: AgentId,
        success: bool,
        timestamp: DateTime<Utc>,
    },
    /// Promise was made
    PromiseMade {
        agent_id: AgentId,
        promise_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    /// Promise was fulfilled
    PromiseFulfilled {
        agent_id: AgentId,
        promise_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    /// Promise was broken
    PromiseBroken {
        agent_id: AgentId,
        promise_id: Uuid,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    /// Verification completed
    VerificationCompleted {
        task_id: TaskId,
        verified: bool,
        timestamp: DateTime<Utc>,
    },
    /// Trust score updated
    TrustUpdated {
        agent_id: AgentId,
        old_score: f64,
        new_score: f64,
        timestamp: DateTime<Utc>,
    },
    /// Consensus reached
    ConsensusReached {
        topic: String,
        participants: Vec<AgentId>,
        timestamp: DateTime<Utc>,
    },
}

/// Metrics for swarm performance
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SwarmMetrics {
    /// Total number of agents
    pub total_agents: usize,
    /// Number of active agents
    pub active_agents: usize,
    /// Total tasks assigned
    pub tasks_assigned: usize,
    /// Tasks completed successfully
    pub tasks_succeeded: usize,
    /// Tasks that failed
    pub tasks_failed: usize,
    /// Promises made
    pub promises_made: usize,
    /// Promises fulfilled
    pub promises_fulfilled: usize,
    /// Promises broken
    pub promises_broken: usize,
    /// Average task duration in milliseconds
    pub avg_task_duration_ms: f64,
    /// Average trust score across all agents
    pub avg_trust_score: f64,
    /// Verification success rate
    pub verification_success_rate: f64,
}