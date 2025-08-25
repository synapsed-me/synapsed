//! Pre-built application templates

use crate::{
    builder::{SynapsedBuilder, StorageBackend, ObservabilityLevel, NetworkType},
    Result,
};

/// Collection of pre-built templates
pub struct Templates;

impl Templates {
    /// Verified AI Agent with full observability
    pub fn verified_ai_agent() -> SynapsedBuilder {
        SynapsedBuilder::new("verified-ai-agent")
            .description("AI agent with intent verification and full observability")
            .add_intent_verification()
            .add_storage(StorageBackend::Sqlite)
            .add_observability(ObservabilityLevel::Full)
            .configure("synapsed-intent", serde_json::json!({
                "max_depth": 5,
                "timeout_ms": 30000,
                "strict_mode": true
            }))
            .env("RUST_LOG", "info")
    }
    
    /// Distributed payment system
    pub fn payment_system() -> SynapsedBuilder {
        SynapsedBuilder::new("payment-system")
            .description("Secure payment processing with identity and crypto")
            .add_payments()
            .add_storage(StorageBackend::RocksDb)
            .add_observability(ObservabilityLevel::Full)
            .configure("synapsed-payments", serde_json::json!({
                "supported_currencies": ["USD", "EUR", "BTC"],
                "risk_threshold": 70
            }))
            .configure("synapsed-identity", serde_json::json!({
                "auth_providers": ["oauth2", "webauthn"],
                "session_timeout": 3600
            }))
    }
    
    /// P2P swarm coordinator
    pub fn swarm_coordinator() -> SynapsedBuilder {
        SynapsedBuilder::new("swarm-coordinator")
            .description("Distributed swarm coordination with consensus")
            .add_component("synapsed-swarm")
            .add_component("synapsed-consensus")
            .add_component("synapsed-crdt")
            .add_network(NetworkType::Consensus)
            .add_observability(ObservabilityLevel::Basic)
            .connect("synapsed-swarm", "task_created", "synapsed-consensus", "propose")
            .connect("synapsed-consensus", "committed", "synapsed-crdt", "merge")
            .configure("synapsed-consensus", serde_json::json!({
                "consensus_type": "hotstuff",
                "block_time_ms": 1000,
                "committee_size": 7
            }))
    }
    
    /// Observable microservice
    pub fn observable_service() -> SynapsedBuilder {
        SynapsedBuilder::new("observable-service")
            .description("Microservice with comprehensive observability")
            .add_component("synapsed-core")
            .add_storage(StorageBackend::Memory)
            .add_observability(ObservabilityLevel::Full)
            .add_component("synapsed-serventis")
            .configure("synapsed-substrates", serde_json::json!({
                "emit_to_stdout": true,
                "buffer_size": 1000,
                "sampling_rate": 1.0
            }))
            .configure("synapsed-monitor", serde_json::json!({
                "dashboard_port": 8080,
                "metrics_port": 9090
            }))
    }
    
    /// Edge compute node
    pub fn edge_compute() -> SynapsedBuilder {
        SynapsedBuilder::new("edge-compute")
            .description("Edge computing node with WASM and GPU support")
            .add_component("synapsed-wasm")
            .add_component("synapsed-gpu")
            .add_storage(StorageBackend::Memory)
            .add_network(NetworkType::Simple)
            .configure("synapsed-wasm", serde_json::json!({
                "max_memory_pages": 1024,
                "enable_simd": true,
                "enable_threads": true
            }))
            .configure("synapsed-gpu", serde_json::json!({
                "device_index": 0,
                "memory_pool_size": 2048
            }))
    }
    
    /// Secure vault
    pub fn secure_vault() -> SynapsedBuilder {
        SynapsedBuilder::new("secure-vault")
            .description("Secure storage with encryption and identity management")
            .add_component("synapsed-storage")
            .add_component("synapsed-crypto")
            .add_component("synapsed-identity")
            .add_component("synapsed-safety")
            .configure("synapsed-crypto", serde_json::json!({
                "algorithm": "kyber1024",
                "key_derivation": "argon2id"
            }))
            .configure("synapsed-storage", serde_json::json!({
                "backend": "sqlite",
                "encryption": true,
                "path": "./vault.db"
            }))
    }
    
    /// MCP server for Claude
    pub fn mcp_server() -> SynapsedBuilder {
        SynapsedBuilder::new("mcp-server")
            .description("Model Context Protocol server for Claude integration")
            .add_component("synapsed-mcp")
            .add_intent_verification()
            .add_storage(StorageBackend::Sqlite)
            .add_observability(ObservabilityLevel::Basic)
            .configure("synapsed-mcp", serde_json::json!({
                "transport": "stdio",
                "protocol_version": "2024-11-05"
            }))
            .env("SYNAPSED_MCP_PORT", "0")  // stdio mode
    }
    
    /// Neural processing pipeline
    pub fn neural_pipeline() -> SynapsedBuilder {
        SynapsedBuilder::new("neural-pipeline")
            .description("Neural network processing pipeline")
            .add_component("synapsed-neural-core")
            .add_component("synapsed-gpu")
            .add_storage(StorageBackend::Memory)
            .add_observability(ObservabilityLevel::Basic)
            .configure("synapsed-neural-core", serde_json::json!({
                "model_path": "./models",
                "batch_size": 32,
                "precision": "fp16"
            }))
    }
    
    /// Development sandbox
    pub fn dev_sandbox() -> SynapsedBuilder {
        SynapsedBuilder::new("dev-sandbox")
            .description("Development environment with all core components")
            .add_component("synapsed-core")
            .add_intent_verification()
            .add_storage(StorageBackend::Memory)
            .add_observability(ObservabilityLevel::Full)
            .add_network(NetworkType::Simple)
            .skip_validations()  // Allow experimental configs
            .env("RUST_LOG", "debug")
            .env("RUST_BACKTRACE", "1")
    }
    
    /// Minimal application
    pub fn minimal() -> SynapsedBuilder {
        SynapsedBuilder::new("minimal-app")
            .description("Minimal Synapsed application")
            .add_component("synapsed-core")
            .add_storage(StorageBackend::Memory)
    }
    
    /// List all available templates
    pub fn list() -> Vec<TemplateInfo> {
        vec![
            TemplateInfo {
                name: "verified-ai-agent".to_string(),
                description: "AI agent with intent verification and full observability".to_string(),
                components: vec![
                    "synapsed-intent".to_string(),
                    "synapsed-verify".to_string(),
                    "synapsed-storage".to_string(),
                    "synapsed-substrates".to_string(),
                    "synapsed-monitor".to_string(),
                ],
                tags: vec!["ai".to_string(), "verification".to_string(), "observable".to_string()],
            },
            TemplateInfo {
                name: "payment-system".to_string(),
                description: "Secure payment processing with identity and crypto".to_string(),
                components: vec![
                    "synapsed-payments".to_string(),
                    "synapsed-identity".to_string(),
                    "synapsed-crypto".to_string(),
                    "synapsed-storage".to_string(),
                ],
                tags: vec!["payments".to_string(), "security".to_string()],
            },
            TemplateInfo {
                name: "swarm-coordinator".to_string(),
                description: "Distributed swarm coordination with consensus".to_string(),
                components: vec![
                    "synapsed-swarm".to_string(),
                    "synapsed-consensus".to_string(),
                    "synapsed-crdt".to_string(),
                    "synapsed-net".to_string(),
                ],
                tags: vec!["distributed".to_string(), "consensus".to_string(), "p2p".to_string()],
            },
            TemplateInfo {
                name: "observable-service".to_string(),
                description: "Microservice with comprehensive observability".to_string(),
                components: vec![
                    "synapsed-core".to_string(),
                    "synapsed-substrates".to_string(),
                    "synapsed-serventis".to_string(),
                    "synapsed-monitor".to_string(),
                ],
                tags: vec!["observability".to_string(), "monitoring".to_string()],
            },
            TemplateInfo {
                name: "edge-compute".to_string(),
                description: "Edge computing node with WASM and GPU support".to_string(),
                components: vec![
                    "synapsed-wasm".to_string(),
                    "synapsed-gpu".to_string(),
                ],
                tags: vec!["compute".to_string(), "edge".to_string(), "wasm".to_string()],
            },
            TemplateInfo {
                name: "secure-vault".to_string(),
                description: "Secure storage with encryption and identity management".to_string(),
                components: vec![
                    "synapsed-storage".to_string(),
                    "synapsed-crypto".to_string(),
                    "synapsed-identity".to_string(),
                    "synapsed-safety".to_string(),
                ],
                tags: vec!["security".to_string(), "encryption".to_string(), "storage".to_string()],
            },
            TemplateInfo {
                name: "mcp-server".to_string(),
                description: "Model Context Protocol server for Claude integration".to_string(),
                components: vec![
                    "synapsed-mcp".to_string(),
                    "synapsed-intent".to_string(),
                    "synapsed-verify".to_string(),
                ],
                tags: vec!["mcp".to_string(), "claude".to_string(), "ai".to_string()],
            },
            TemplateInfo {
                name: "neural-pipeline".to_string(),
                description: "Neural network processing pipeline".to_string(),
                components: vec![
                    "synapsed-neural-core".to_string(),
                    "synapsed-gpu".to_string(),
                ],
                tags: vec!["neural".to_string(), "ai".to_string(), "gpu".to_string()],
            },
            TemplateInfo {
                name: "dev-sandbox".to_string(),
                description: "Development environment with all core components".to_string(),
                components: vec![
                    "synapsed-core".to_string(),
                    "synapsed-intent".to_string(),
                    "synapsed-verify".to_string(),
                ],
                tags: vec!["development".to_string(), "testing".to_string()],
            },
            TemplateInfo {
                name: "minimal".to_string(),
                description: "Minimal Synapsed application".to_string(),
                components: vec![
                    "synapsed-core".to_string(),
                    "synapsed-storage".to_string(),
                ],
                tags: vec!["minimal".to_string(), "starter".to_string()],
            },
        ]
    }
    
    /// Get template by name
    pub fn get(name: &str) -> Option<SynapsedBuilder> {
        match name {
            "verified-ai-agent" => Some(Self::verified_ai_agent()),
            "payment-system" => Some(Self::payment_system()),
            "swarm-coordinator" => Some(Self::swarm_coordinator()),
            "observable-service" => Some(Self::observable_service()),
            "edge-compute" => Some(Self::edge_compute()),
            "secure-vault" => Some(Self::secure_vault()),
            "mcp-server" => Some(Self::mcp_server()),
            "neural-pipeline" => Some(Self::neural_pipeline()),
            "dev-sandbox" => Some(Self::dev_sandbox()),
            "minimal" => Some(Self::minimal()),
            _ => None,
        }
    }
}

/// Information about a template
#[derive(Debug, Clone)]
pub struct TemplateInfo {
    pub name: String,
    pub description: String,
    pub components: Vec<String>,
    pub tags: Vec<String>,
}

impl TemplateInfo {
    /// Check if template matches tags
    pub fn matches_tags(&self, tags: &[String]) -> bool {
        tags.iter().any(|tag| self.tags.contains(tag))
    }
}