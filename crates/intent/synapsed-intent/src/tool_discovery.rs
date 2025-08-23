//! Dynamic tool discovery mechanism
//! 
//! This module monitors agent execution attempts and discovers new tools
//! that agents try to use, evaluating whether they should be granted access.

use crate::{
    dynamic_agents::{RiskLevel, ToolSecurityProfile, ResourceRequirements},
    tool_registry::ToolRegistry,
    Result, IntentError,
};
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;
use chrono::{DateTime, Utc};

/// Dynamic tool discovery system
pub struct ToolDiscoverySystem {
    discovered_tools: Arc<RwLock<HashMap<String, DiscoveredTool>>>,
    tool_usage_stats: Arc<RwLock<HashMap<String, ToolUsageStats>>>,
    discovery_policies: Vec<DiscoveryPolicy>,
    tool_registry: Arc<ToolRegistry>,
}

/// A tool discovered during agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredTool {
    pub name: String,
    pub first_seen: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    pub discovered_by: String,  // Agent ID
    pub usage_context: Vec<UsageContext>,
    pub inferred_purpose: Option<String>,
    pub risk_assessment: RiskAssessment,
    pub approval_status: ApprovalStatus,
}

/// Context in which a tool was used
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageContext {
    pub agent_id: String,
    pub task_description: String,
    pub command_line: String,
    pub arguments: Vec<String>,
    pub environment: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Risk assessment for a discovered tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub risk_level: RiskLevel,
    pub risk_factors: Vec<String>,
    pub mitigation_suggestions: Vec<String>,
    pub confidence: f64,
}

/// Approval status for a discovered tool
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    AutoApproved,
    ManuallyApproved,
    Denied,
    Restricted,  // Approved with limitations
}

/// Statistics about tool usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageStats {
    pub tool_name: String,
    pub total_uses: usize,
    pub successful_uses: usize,
    pub failed_uses: usize,
    pub unique_agents: HashSet<String>,
    pub avg_execution_time_ms: u64,
    pub common_arguments: HashMap<String, usize>,
    pub common_errors: HashMap<String, usize>,
}

/// Policy for tool discovery
#[derive(Debug, Clone)]
pub struct DiscoveryPolicy {
    pub name: String,
    pub condition: PolicyCondition,
    pub action: PolicyAction,
}

/// Condition for applying a discovery policy
#[derive(Debug, Clone)]
pub enum PolicyCondition {
    ToolNamePattern(String),  // Regex pattern
    RiskLevelBelow(RiskLevel),
    TrustedAgent(f64),  // Minimum trust score
    CommonlyUsed(usize),  // Minimum usage count
    SuccessRateAbove(f64),
}

/// Action to take when policy matches
#[derive(Debug, Clone)]
pub enum PolicyAction {
    AutoApprove,
    RequestApproval,
    Deny,
    RestrictToAgents(Vec<String>),
    RequireSupervision,
}

/// Tool discovery event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryEvent {
    pub timestamp: DateTime<Utc>,
    pub agent_id: String,
    pub tool_name: String,
    pub command: String,
    pub was_allowed: bool,
    pub reason: String,
}

impl ToolDiscoverySystem {
    pub fn new(tool_registry: Arc<ToolRegistry>) -> Self {
        Self {
            discovered_tools: Arc::new(RwLock::new(HashMap::new())),
            tool_usage_stats: Arc::new(RwLock::new(HashMap::new())),
            discovery_policies: Self::create_default_policies(),
            tool_registry,
        }
    }

    /// Handle an attempt to use an unknown tool
    pub async fn handle_tool_attempt(
        &self,
        agent_id: &str,
        tool_name: &str,
        command: &str,
        args: &[String],
        task_context: &str,
        agent_trust: f64,
    ) -> Result<ToolAccessDecision> {
        // Check if tool is already registered
        if let Some(profile) = self.tool_registry.get_tool_profile(tool_name).await {
            return Ok(ToolAccessDecision::Allowed {
                profile,
                reason: "Tool is registered".to_string(),
            });
        }

        // Check if tool has been discovered before
        let mut discovered = self.discovered_tools.write().await;
        let tool = discovered.entry(tool_name.to_string()).or_insert_with(|| {
            DiscoveredTool {
                name: tool_name.to_string(),
                first_seen: Utc::now(),
                last_used: Utc::now(),
                discovered_by: agent_id.to_string(),
                usage_context: Vec::new(),
                inferred_purpose: None,
                risk_assessment: self.assess_risk(tool_name, command, args),
                approval_status: ApprovalStatus::Pending,
            }
        });

        // Update usage context
        tool.last_used = Utc::now();
        tool.usage_context.push(UsageContext {
            agent_id: agent_id.to_string(),
            task_description: task_context.to_string(),
            command_line: command.to_string(),
            arguments: args.to_vec(),
            environment: HashMap::new(),
            timestamp: Utc::now(),
            success: false,  // Will be updated later
            error_message: None,
        });

        // Apply discovery policies
        let decision = self.apply_policies(tool, agent_trust).await;

        // Update tool status based on decision
        match &decision {
            ToolAccessDecision::Allowed { .. } => {
                if tool.approval_status == ApprovalStatus::Pending {
                    tool.approval_status = ApprovalStatus::AutoApproved;
                }
            },
            ToolAccessDecision::Denied { .. } => {
                if tool.approval_status == ApprovalStatus::Pending {
                    tool.approval_status = ApprovalStatus::Denied;
                }
            },
            _ => {}
        }

        // Update usage statistics
        self.update_usage_stats(tool_name, agent_id, args).await;

        Ok(decision)
    }

    /// Assess risk of a discovered tool
    fn assess_risk(&self, tool_name: &str, command: &str, args: &[String]) -> RiskAssessment {
        let mut risk_factors = Vec::new();
        let mut risk_level = RiskLevel::Low;
        let mut mitigation_suggestions = Vec::new();

        // Check for dangerous patterns
        let dangerous_commands = vec!["rm", "sudo", "eval", "exec", "kill", "shutdown"];
        if dangerous_commands.iter().any(|d| command.contains(d)) {
            risk_factors.push(format!("Command contains dangerous keyword: {}", command));
            risk_level = RiskLevel::High;
            mitigation_suggestions.push("Run in sandboxed environment".to_string());
        }

        // Check for file system operations
        if command.contains("write") || command.contains("delete") || command.contains("modify") {
            risk_factors.push("Tool performs file system modifications".to_string());
            if risk_level < RiskLevel::Medium {
                risk_level = RiskLevel::Medium;
            }
            mitigation_suggestions.push("Restrict to specific paths".to_string());
        }

        // Check for network operations
        if command.contains("http") || command.contains("curl") || command.contains("wget") {
            risk_factors.push("Tool performs network operations".to_string());
            if risk_level < RiskLevel::Medium {
                risk_level = RiskLevel::Medium;
            }
            mitigation_suggestions.push("Whitelist allowed endpoints".to_string());
        }

        // Check for script execution
        if tool_name.ends_with(".sh") || tool_name.ends_with(".py") || tool_name.ends_with(".js") {
            risk_factors.push("Tool is a script".to_string());
            if risk_level < RiskLevel::Medium {
                risk_level = RiskLevel::Medium;
            }
            mitigation_suggestions.push("Review script contents before execution".to_string());
        }

        // Check arguments for suspicious patterns
        for arg in args {
            if arg.contains("..") || arg.starts_with("/etc") || arg.starts_with("/sys") {
                risk_factors.push(format!("Suspicious argument: {}", arg));
                risk_level = RiskLevel::High;
                mitigation_suggestions.push("Validate all arguments".to_string());
            }
        }

        RiskAssessment {
            risk_level,
            risk_factors,
            mitigation_suggestions,
            confidence: 0.75,  // Medium confidence for heuristic assessment
        }
    }

    /// Apply discovery policies to determine access
    async fn apply_policies(&self, tool: &DiscoveredTool, agent_trust: f64) -> ToolAccessDecision {
        for policy in &self.discovery_policies {
            if self.evaluate_policy_condition(&policy.condition, tool, agent_trust).await {
                return match &policy.action {
                    PolicyAction::AutoApprove => {
                        ToolAccessDecision::Allowed {
                            profile: self.create_provisional_profile(tool),
                            reason: format!("Auto-approved by policy: {}", policy.name),
                        }
                    },
                    PolicyAction::Deny => {
                        ToolAccessDecision::Denied {
                            reason: format!("Denied by policy: {}", policy.name),
                            alternatives: vec![],
                        }
                    },
                    PolicyAction::RequestApproval => {
                        ToolAccessDecision::RequiresApproval {
                            tool_name: tool.name.clone(),
                            risk_assessment: tool.risk_assessment.clone(),
                            request_id: uuid::Uuid::new_v4(),
                        }
                    },
                    PolicyAction::RestrictToAgents(agents) => {
                        if agents.iter().any(|a| a == &tool.discovered_by) {
                            ToolAccessDecision::Allowed {
                                profile: self.create_provisional_profile(tool),
                                reason: "Restricted access granted".to_string(),
                            }
                        } else {
                            ToolAccessDecision::Denied {
                                reason: "Agent not authorized for this tool".to_string(),
                                alternatives: vec![],
                            }
                        }
                    },
                    PolicyAction::RequireSupervision => {
                        ToolAccessDecision::AllowedWithSupervision {
                            profile: self.create_provisional_profile(tool),
                            supervision_requirements: vec![
                                "Log all executions".to_string(),
                                "Notify administrator".to_string(),
                            ],
                        }
                    },
                };
            }
        }

        // Default: require approval for unknown tools
        ToolAccessDecision::RequiresApproval {
            tool_name: tool.name.clone(),
            risk_assessment: tool.risk_assessment.clone(),
            request_id: uuid::Uuid::new_v4(),
        }
    }

    /// Evaluate a policy condition
    async fn evaluate_policy_condition(
        &self,
        condition: &PolicyCondition,
        tool: &DiscoveredTool,
        agent_trust: f64,
    ) -> bool {
        match condition {
            PolicyCondition::ToolNamePattern(pattern) => {
                tool.name.contains(pattern)
            },
            PolicyCondition::RiskLevelBelow(max_risk) => {
                tool.risk_assessment.risk_level <= *max_risk
            },
            PolicyCondition::TrustedAgent(min_trust) => {
                agent_trust >= *min_trust
            },
            PolicyCondition::CommonlyUsed(min_uses) => {
                let stats = self.tool_usage_stats.read().await;
                stats.get(&tool.name)
                    .map(|s| s.total_uses >= *min_uses)
                    .unwrap_or(false)
            },
            PolicyCondition::SuccessRateAbove(min_rate) => {
                let stats = self.tool_usage_stats.read().await;
                stats.get(&tool.name)
                    .map(|s| {
                        if s.total_uses > 0 {
                            s.successful_uses as f64 / s.total_uses as f64 >= *min_rate
                        } else {
                            false
                        }
                    })
                    .unwrap_or(false)
            },
        }
    }

    /// Create a provisional tool profile for a discovered tool
    fn create_provisional_profile(&self, tool: &DiscoveredTool) -> ToolSecurityProfile {
        ToolSecurityProfile {
            tool_name: tool.name.clone(),
            required_commands: vec![tool.name.clone()],
            required_paths: vec!["${workspace}/**".to_string()],
            required_endpoints: vec![],
            risk_level: tool.risk_assessment.risk_level,
            verification_requirements: vec![],
            resource_requirements: ResourceRequirements::default(),
        }
    }

    /// Update usage statistics
    async fn update_usage_stats(&self, tool_name: &str, agent_id: &str, args: &[String]) {
        let mut stats = self.tool_usage_stats.write().await;
        let tool_stats = stats.entry(tool_name.to_string()).or_insert_with(|| {
            ToolUsageStats {
                tool_name: tool_name.to_string(),
                total_uses: 0,
                successful_uses: 0,
                failed_uses: 0,
                unique_agents: HashSet::new(),
                avg_execution_time_ms: 0,
                common_arguments: HashMap::new(),
                common_errors: HashMap::new(),
            }
        });

        tool_stats.total_uses += 1;
        tool_stats.unique_agents.insert(agent_id.to_string());
        
        // Track common arguments
        for arg in args {
            *tool_stats.common_arguments.entry(arg.clone()).or_insert(0) += 1;
        }
    }

    /// Report tool execution result
    pub async fn report_execution_result(
        &self,
        tool_name: &str,
        success: bool,
        execution_time_ms: u64,
        error: Option<String>,
    ) {
        let mut stats = self.tool_usage_stats.write().await;
        if let Some(tool_stats) = stats.get_mut(tool_name) {
            if success {
                tool_stats.successful_uses += 1;
            } else {
                tool_stats.failed_uses += 1;
                if let Some(err) = error {
                    *tool_stats.common_errors.entry(err).or_insert(0) += 1;
                }
            }
            
            // Update average execution time
            tool_stats.avg_execution_time_ms = 
                ((tool_stats.avg_execution_time_ms * (tool_stats.total_uses - 1) as u64) + execution_time_ms) 
                / tool_stats.total_uses as u64;
        }
    }

    /// Get discovered tools that might be useful for a capability
    pub async fn suggest_tools_for_capability(&self, capability: &str) -> Vec<SuggestedTool> {
        let discovered = self.discovered_tools.read().await;
        let stats = self.tool_usage_stats.read().await;
        let mut suggestions = Vec::new();

        for (tool_name, tool) in discovered.iter() {
            // Only suggest approved or auto-approved tools
            if tool.approval_status != ApprovalStatus::AutoApproved 
                && tool.approval_status != ApprovalStatus::ManuallyApproved {
                continue;
            }

            // Check if tool might be relevant based on usage context
            let relevance = self.calculate_relevance(tool, capability);
            if relevance > 0.3 {
                let success_rate = stats.get(tool_name)
                    .map(|s| {
                        if s.total_uses > 0 {
                            s.successful_uses as f64 / s.total_uses as f64
                        } else {
                            0.0
                        }
                    })
                    .unwrap_or(0.0);

                suggestions.push(SuggestedTool {
                    name: tool_name.clone(),
                    relevance,
                    success_rate,
                    risk_level: tool.risk_assessment.risk_level,
                    usage_count: stats.get(tool_name).map(|s| s.total_uses).unwrap_or(0),
                });
            }
        }

        // Sort by relevance and success rate
        suggestions.sort_by(|a, b| {
            let score_a = a.relevance * 0.6 + a.success_rate * 0.4;
            let score_b = b.relevance * 0.6 + b.success_rate * 0.4;
            score_b.partial_cmp(&score_a).unwrap()
        });

        suggestions
    }

    /// Calculate relevance of a tool for a capability
    fn calculate_relevance(&self, tool: &DiscoveredTool, capability: &str) -> f64 {
        // Simple keyword matching for now
        let capability_lower = capability.to_lowercase();
        let mut relevance = 0.0;

        // Check tool name
        if tool.name.to_lowercase().contains(&capability_lower) {
            relevance += 0.5;
        }

        // Check inferred purpose
        if let Some(purpose) = &tool.inferred_purpose {
            if purpose.to_lowercase().contains(&capability_lower) {
                relevance += 0.4;
            }
        }

        // Check usage contexts
        for context in &tool.usage_context {
            if context.task_description.to_lowercase().contains(&capability_lower) {
                relevance += 0.1;
                if relevance >= 1.0 {
                    return 1.0;
                }
            }
        }

        relevance
    }

    /// Create default discovery policies
    fn create_default_policies() -> Vec<DiscoveryPolicy> {
        vec![
            DiscoveryPolicy {
                name: "Deny dangerous tools".to_string(),
                condition: PolicyCondition::ToolNamePattern("sudo".to_string()),
                action: PolicyAction::Deny,
            },
            DiscoveryPolicy {
                name: "Auto-approve common safe tools".to_string(),
                condition: PolicyCondition::RiskLevelBelow(RiskLevel::Low),
                action: PolicyAction::AutoApprove,
            },
            DiscoveryPolicy {
                name: "Trusted agents can discover medium-risk tools".to_string(),
                condition: PolicyCondition::TrustedAgent(0.8),
                action: PolicyAction::AutoApprove,
            },
            DiscoveryPolicy {
                name: "Commonly successful tools".to_string(),
                condition: PolicyCondition::SuccessRateAbove(0.9),
                action: PolicyAction::AutoApprove,
            },
            DiscoveryPolicy {
                name: "High-risk tools require approval".to_string(),
                condition: PolicyCondition::RiskLevelBelow(RiskLevel::Critical),
                action: PolicyAction::RequestApproval,
            },
        ]
    }
}

/// Decision about tool access
#[derive(Debug, Clone)]
pub enum ToolAccessDecision {
    Allowed {
        profile: ToolSecurityProfile,
        reason: String,
    },
    Denied {
        reason: String,
        alternatives: Vec<String>,
    },
    RequiresApproval {
        tool_name: String,
        risk_assessment: RiskAssessment,
        request_id: uuid::Uuid,
    },
    AllowedWithSupervision {
        profile: ToolSecurityProfile,
        supervision_requirements: Vec<String>,
    },
}

/// Suggested tool for a capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedTool {
    pub name: String,
    pub relevance: f64,
    pub success_rate: f64,
    pub risk_level: RiskLevel,
    pub usage_count: usize,
}