//! # Synapsed Intent
//! 
//! Hierarchical intent trees with verification for AI agent systems.
//! Implements HTN (Hierarchical Task Network) planning with observable execution.

pub mod intent;
pub mod tree;
pub mod context;
pub mod checkpoint;
pub mod types;
pub mod observability;
pub mod verification;
pub mod execution;
pub mod enhanced_intent;
pub mod dynamic_agents;
pub mod tool_registry;
pub mod permission_negotiation;
pub mod agent_parser;
pub mod capability_inference;
pub mod tool_discovery;
pub mod agent_profiling;

pub use intent::{HierarchicalIntent, IntentBuilder};
pub use tree::{IntentTree, IntentForest, IntentRelation};
pub use context::{IntentContext, ContextBuilder};
pub use checkpoint::{IntentCheckpoint, CheckpointManager};
pub use types::*;
pub use observability::{ObservableIntent, ObservableIntentBuilder, IntentMonitor};
pub use execution::{VerifiedExecutor, BoundsEnforcer, ContextMonitor, ContextViolation};
pub use enhanced_intent::{VerifiedIntent, RecoveryStrategy, RecoveryAction, ExecutionMetrics};
pub use dynamic_agents::{
    SubAgentDefinition, ToolSecurityProfile, ResourceRequirements, RiskLevel,
    SecurityLevel, DynamicContextGenerator, RiskAnalyzer, WorkspaceZones, Zone, Operation
};
pub use tool_registry::{
    ToolRegistry, CustomTool, ToolImplementation, ToolCategory, PermissionMatrix,
    RequiredPermissions, FilesystemPermissions, NetworkPermissions, ProcessPermissions,
    SystemPermissions, AllowedPermissions
};
pub use permission_negotiation::{
    PermissionNegotiator, PermissionRequest, RequestedPermissions, Priority,
    PermissionResponse, Decision, GrantedPermissions, Alternative, PolicyEngine,
    Policy, EvaluationContext, ResourceUsage, PolicyDecision, NegotiationAuditEntry,
    PermissionNotification
};
pub use agent_parser::{
    AgentMarkdownParser, ParsedAgentDefinition, Example, CapabilityPattern
};
pub use capability_inference::{
    CapabilityInferenceEngine, ToolRelationshipGraph, ToolNode, ToolEdge,
    ToolRelationship, ToolCategory as InferenceToolCategory, InferenceRule,
    RuleCondition, LearnedPattern, InferredCapability
};
pub use tool_discovery::{
    ToolDiscoverySystem, DiscoveredTool, UsageContext, RiskAssessment,
    ApprovalStatus, ToolUsageStats, DiscoveryPolicy, PolicyCondition,
    PolicyAction, DiscoveryEvent, ToolAccessDecision, SuggestedTool
};
pub use agent_profiling::{
    AgentProfilingSystem, AgentProfile, ExecutionPattern, PerformanceMetrics,
    TrustEvent, TrustEventType, Anomaly, AnomalyType, AnomalySeverity,
    BehaviorPattern, AnomalyDetector, AnomalyThresholds, BaselinePattern,
    ResourceUsage as ProfileResourceUsage, ToolDivergenceAnalysis
};

// Re-export commonly used types
pub use crate::types::Step;

/// Result type for intent operations
pub type Result<T> = std::result::Result<T, IntentError>;

/// Intent-specific errors
#[derive(Debug, thiserror::Error)]
pub enum IntentError {
    #[error("Intent validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Intent execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Intent not found: {0}")]
    NotFound(String),
    
    #[error("Context violation: {0}")]
    ContextViolation(String),
    
    #[error("Dependency failed: {0}")]
    DependencyFailed(String),
    
    #[error("Observable error: {0}")]
    ObservableError(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}