//! # Synapsed Swarm
//! 
//! Unified swarm coordination system that integrates Intent, Promise Theory, and Verification
//! to enable reliable multi-agent collaboration with verifiable execution.
//!
//! ## Key Features
//! 
//! - **Swarm Coordination**: Manage multiple autonomous agents working together
//! - **Promise-Based Cooperation**: Agents make voluntary promises about their behavior
//! - **Intent Delegation**: Hierarchical task breakdown with sub-agent delegation
//! - **Execution Verification**: All agent claims are verified against reality
//! - **Trust Management**: Reputation-based trust scores for agents
//! - **Context Propagation**: Parent context passed to sub-agents
//!
//! ## Architecture
//! 
//! ```text
//! ┌─────────────────────────────────────┐
//! │         Swarm Coordinator           │
//! ├─────────────────────────────────────┤
//! │  Intent  │  Promise  │  Verification │
//! │  System  │  Theory   │  Framework    │
//! ├─────────────────────────────────────┤
//! │         Agent Protocol              │
//! ├─────────────────────────────────────┤
//! │    Agent₁    Agent₂    Agent₃       │
//! └─────────────────────────────────────┘
//! ```

pub mod coordinator;
pub mod protocol;
pub mod claude_agent;
pub mod verification;
pub mod trust;
pub mod persistence;
pub mod types;
pub mod error;
pub mod execution;
pub mod monitoring;
pub mod fault_tolerance;
pub mod consensus;
pub mod recovery;

pub use coordinator::{SwarmCoordinator, SwarmConfig, SwarmState};
pub use protocol::{AgentMessage, AgentProtocol, ProtocolVersion, MessageType};
pub use claude_agent::{ClaudeAgent, ClaudeAgentConfig, ClaudeContext};
pub use verification::{SwarmVerifier, VerificationPolicy, VerificationReport};
pub use trust::{TrustManager, TrustScore, TrustUpdate, BackupConfig};
pub use persistence::{TrustStore, SqliteTrustStore, FileTrustStore, InMemoryTrustStore, StorageHealth};
pub use execution::{ExecutionEngine, ExecutionConfig, ExecutionResult};
pub use monitoring::{
    MetricsCollector, PrometheusExporter, DashboardProvider, MonitoringConfig,
    AlertThresholds, Alert, AlertSeverity, DashboardMetrics, AgentMetrics,
    PerformanceTrends, HealthStatus, HealthLevel, ComponentHealth,
};
pub use fault_tolerance::{
    FaultToleranceManager, FaultToleranceConfig, AgentHealthStatus, AgentHeartbeat,
    CircuitBreakerState, CircuitBreakerStatus, TaskCheckpoint, RecoveryAction,
    RecoveryStatistics,
};
pub use consensus::{
    ConsensusProtocol, PBFTConsensus, VotingRound, QuorumCertificate,
    ConsensusMessage, ConsensusProposal, ConsensusResult, ConsensusStatistics,
};
pub use recovery::{
    RecoveryStrategy, RecoveryManager, RecoveryContext, RecoveryResult,
    ExponentialBackoffStrategy, CheckpointRecoveryStrategy, GracefulDegradationStrategy,
    SelfHealingStrategy, RecoveryError,
};
pub use types::*;
pub use error::{SwarmError, SwarmResult};

// Re-export commonly used types from dependencies
pub use synapsed_intent::{HierarchicalIntent, IntentBuilder, IntentContext};
pub use synapsed_promise::{Promise, PromiseContract, AutonomousAgent, Willingness};
pub use synapsed_verify::{VerificationResult, VerificationProof};

/// Version of the swarm coordination protocol
pub const PROTOCOL_VERSION: &str = "1.0.0";

/// Maximum number of agents in a swarm
pub const MAX_SWARM_SIZE: usize = 100;

/// Default trust score for new agents
pub const DEFAULT_TRUST_SCORE: f64 = 0.5;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::{
        SwarmCoordinator, SwarmConfig, SwarmState,
        AgentMessage, AgentProtocol,
        ClaudeAgent, ClaudeAgentConfig,
        SwarmVerifier, VerificationPolicy,
        TrustManager, TrustScore, BackupConfig,
        TrustStore, SqliteTrustStore, FileTrustStore, InMemoryTrustStore,
        ExecutionEngine, ExecutionConfig, ExecutionResult,
        MetricsCollector, PrometheusExporter, DashboardProvider, MonitoringConfig,
        FaultToleranceManager, FaultToleranceConfig, AgentHealthStatus,
        CircuitBreakerState, TaskCheckpoint, RecoveryStatistics,
        SwarmError, SwarmResult,
    };
    
    pub use synapsed_intent::prelude::*;
    pub use synapsed_promise::prelude::*;
    pub use synapsed_verify::prelude::*;
}