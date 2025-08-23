//! View models for human-centric data presentation

mod task_view;
mod agent_view;
mod health_view;

pub use task_view::{TaskView, TaskStatus, TaskPhase};
pub use agent_view::{AgentView, AgentStatus, TrustLevel};
pub use health_view::{SystemHealthView, HealthStatus, ServiceHealth, ServiceStatus, ServiceType, SystemMetrics};

use serde::{Deserialize, Serialize};