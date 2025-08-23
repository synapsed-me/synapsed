//! Permission negotiation system for dynamic agent capabilities
//! 
//! This module implements a negotiation protocol where agents can request
//! additional permissions with justification, and the system can grant or
//! deny based on context, trust, and risk assessment.

use crate::{
    context::IntentContext,
    dynamic_agents::{RiskLevel, SecurityLevel},
    tool_registry::RequiredPermissions,
    Result, IntentError,
};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, mpsc};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};

/// Permission negotiation request from an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub request_id: Uuid,
    pub agent_id: String,
    pub requested_permissions: RequestedPermissions,
    pub justification: String,
    pub context: HashMap<String, serde_json::Value>,
    pub duration: Option<Duration>,
    pub priority: Priority,
    pub timestamp: DateTime<Utc>,
}

/// Specific permissions being requested
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestedPermissions {
    pub additional_commands: Vec<String>,
    pub additional_paths: Vec<String>,
    pub additional_endpoints: Vec<String>,
    pub increased_memory_mb: Option<usize>,
    pub increased_cpu_seconds: Option<u64>,
    pub network_access: bool,
    pub spawn_processes: bool,
}

/// Priority of the request
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

/// Response to a permission request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionResponse {
    pub request_id: Uuid,
    pub decision: Decision,
    pub granted_permissions: Option<GrantedPermissions>,
    pub reason: String,
    pub alternatives: Vec<Alternative>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Decision on the permission request
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Decision {
    Approved,
    PartiallyApproved,
    Denied,
    RequiresEscalation,
    Deferred,
}

/// Permissions that were actually granted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantedPermissions {
    pub commands: Vec<String>,
    pub paths: Vec<String>,
    pub endpoints: Vec<String>,
    pub memory_mb: Option<usize>,
    pub cpu_seconds: Option<u64>,
    pub valid_until: DateTime<Utc>,
    pub revocable: bool,
}

/// Alternative suggestions when request is denied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    pub description: String,
    pub modified_permissions: RequestedPermissions,
    pub likelihood_of_approval: f64,
}

/// Permission negotiator that handles requests and decisions
pub struct PermissionNegotiator {
    pending_requests: Arc<RwLock<HashMap<Uuid, PermissionRequest>>>,
    approved_requests: Arc<RwLock<HashMap<Uuid, GrantedPermissions>>>,
    escalation_queue: Arc<RwLock<Vec<PermissionRequest>>>,
    policy_engine: PolicyEngine,
    audit_log: Arc<RwLock<Vec<NegotiationAuditEntry>>>,
    notification_channel: mpsc::Sender<PermissionNotification>,
}

/// Policy engine that evaluates permission requests
pub struct PolicyEngine {
    policies: Vec<Box<dyn Policy + Send + Sync>>,
    risk_thresholds: HashMap<RiskLevel, f64>,
    auto_approve_patterns: Vec<AutoApprovePattern>,
    auto_deny_patterns: Vec<AutoDenyPattern>,
}

/// Trait for permission policies
#[async_trait::async_trait]
pub trait Policy: Send + Sync {
    async fn evaluate(&self, request: &PermissionRequest, context: &EvaluationContext) -> PolicyDecision;
    fn name(&self) -> &str;
    fn priority(&self) -> i32;
}

/// Context for policy evaluation
#[derive(Debug, Clone)]
pub struct EvaluationContext {
    pub agent_trust_score: f64,
    pub current_risk_level: RiskLevel,
    pub security_zone: SecurityLevel,
    pub recent_violations: Vec<String>,
    pub resource_usage: ResourceUsage,
    pub parent_context: Option<Box<IntentContext>>,
}

/// Current resource usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub memory_used_mb: usize,
    pub cpu_percent: f32,
    pub disk_io_kbps: usize,
    pub network_io_kbps: usize,
}

/// Decision from a policy
#[derive(Debug, Clone)]
pub struct PolicyDecision {
    pub recommendation: Decision,
    pub confidence: f64,
    pub reasoning: String,
    pub conditions: Vec<String>,
}

/// Auto-approve pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoApprovePattern {
    pub name: String,
    pub command_pattern: Option<String>,
    pub path_pattern: Option<String>,
    pub max_risk_level: RiskLevel,
    pub min_trust_score: f64,
}

/// Auto-deny pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoDenyPattern {
    pub name: String,
    pub command_pattern: Option<String>,
    pub path_pattern: Option<String>,
    pub reason: String,
}

/// Audit entry for negotiation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiationAuditEntry {
    pub timestamp: DateTime<Utc>,
    pub request_id: Uuid,
    pub agent_id: String,
    pub decision: Decision,
    pub policy_decisions: Vec<String>,
    pub final_reason: String,
}

/// Notification about permission changes
#[derive(Debug, Clone)]
pub enum PermissionNotification {
    RequestReceived(PermissionRequest),
    RequestApproved(PermissionResponse),
    RequestDenied(PermissionResponse),
    PermissionRevoked(Uuid),
    EscalationRequired(PermissionRequest),
}

impl PermissionNegotiator {
    pub fn new(notification_channel: mpsc::Sender<PermissionNotification>) -> Self {
        Self {
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            approved_requests: Arc::new(RwLock::new(HashMap::new())),
            escalation_queue: Arc::new(RwLock::new(Vec::new())),
            policy_engine: PolicyEngine::new(),
            audit_log: Arc::new(RwLock::new(Vec::new())),
            notification_channel,
        }
    }

    /// Request additional permissions
    pub async fn request_permissions(
        &self,
        request: PermissionRequest,
        context: &EvaluationContext,
    ) -> Result<PermissionResponse> {
        // Store the request
        let mut pending = self.pending_requests.write().await;
        pending.insert(request.request_id, request.clone());
        drop(pending);

        // Notify about the request
        let _ = self.notification_channel.send(
            PermissionNotification::RequestReceived(request.clone())
        ).await;

        // Evaluate the request
        let decision = self.policy_engine.evaluate(&request, context).await;

        // Create response based on decision
        let response = match decision.recommendation.clone() {
            Decision::Approved => {
                let granted = self.grant_permissions(&request, context).await?;
                PermissionResponse {
                    request_id: request.request_id,
                    decision: Decision::Approved,
                    granted_permissions: Some(granted),
                    reason: decision.reasoning.clone(),
                    alternatives: vec![],
                    expires_at: request.duration.map(|d| Utc::now() + d),
                }
            },
            Decision::PartiallyApproved => {
                let granted = self.grant_partial_permissions(&request, context).await?;
                PermissionResponse {
                    request_id: request.request_id,
                    decision: Decision::PartiallyApproved,
                    granted_permissions: Some(granted),
                    reason: decision.reasoning.clone(),
                    alternatives: self.suggest_alternatives(&request, context).await,
                    expires_at: request.duration.map(|d| Utc::now() + d),
                }
            },
            Decision::Denied => {
                PermissionResponse {
                    request_id: request.request_id,
                    decision: Decision::Denied,
                    granted_permissions: None,
                    reason: decision.reasoning.clone(),
                    alternatives: self.suggest_alternatives(&request, context).await,
                    expires_at: None,
                }
            },
            Decision::RequiresEscalation => {
                // Add to escalation queue
                let mut queue = self.escalation_queue.write().await;
                queue.push(request.clone());
                drop(queue);

                let _ = self.notification_channel.send(
                    PermissionNotification::EscalationRequired(request.clone())
                ).await;

                PermissionResponse {
                    request_id: request.request_id,
                    decision: Decision::RequiresEscalation,
                    granted_permissions: None,
                    reason: "Request requires human approval".to_string(),
                    alternatives: vec![],
                    expires_at: None,
                }
            },
            Decision::Deferred => {
                PermissionResponse {
                    request_id: request.request_id,
                    decision: Decision::Deferred,
                    granted_permissions: None,
                    reason: "Request deferred for later evaluation".to_string(),
                    alternatives: vec![],
                    expires_at: None,
                }
            },
        };

        // Log the negotiation
        self.log_negotiation(&request, &response, decision).await;

        // Send notification
        match response.decision {
            Decision::Approved | Decision::PartiallyApproved => {
                let _ = self.notification_channel.send(
                    PermissionNotification::RequestApproved(response.clone())
                ).await;
            },
            Decision::Denied => {
                let _ = self.notification_channel.send(
                    PermissionNotification::RequestDenied(response.clone())
                ).await;
            },
            _ => {},
        }

        Ok(response)
    }

    /// Grant full permissions
    async fn grant_permissions(
        &self,
        request: &PermissionRequest,
        _context: &EvaluationContext,
    ) -> Result<GrantedPermissions> {
        let granted = GrantedPermissions {
            commands: request.requested_permissions.additional_commands.clone(),
            paths: request.requested_permissions.additional_paths.clone(),
            endpoints: request.requested_permissions.additional_endpoints.clone(),
            memory_mb: request.requested_permissions.increased_memory_mb,
            cpu_seconds: request.requested_permissions.increased_cpu_seconds,
            valid_until: request.duration
                .map(|d| Utc::now() + d)
                .unwrap_or_else(|| Utc::now() + Duration::hours(1)),
            revocable: true,
        };

        // Store approved request
        let mut approved = self.approved_requests.write().await;
        approved.insert(request.request_id, granted.clone());

        Ok(granted)
    }

    /// Grant partial permissions (reduced from request)
    async fn grant_partial_permissions(
        &self,
        request: &PermissionRequest,
        context: &EvaluationContext,
    ) -> Result<GrantedPermissions> {
        // Filter based on risk and trust
        let safe_commands: Vec<String> = request.requested_permissions.additional_commands
            .iter()
            .filter(|cmd| !self.is_dangerous_command(cmd))
            .cloned()
            .collect();

        let safe_paths: Vec<String> = request.requested_permissions.additional_paths
            .iter()
            .filter(|path| self.is_safe_path(path, context))
            .cloned()
            .collect();

        let granted = GrantedPermissions {
            commands: safe_commands,
            paths: safe_paths,
            endpoints: vec![], // No network in partial approval
            memory_mb: request.requested_permissions.increased_memory_mb.map(|m| m / 2), // Half requested
            cpu_seconds: request.requested_permissions.increased_cpu_seconds.map(|c| c / 2),
            valid_until: Utc::now() + Duration::minutes(30), // Shorter duration
            revocable: true,
        };

        let mut approved = self.approved_requests.write().await;
        approved.insert(request.request_id, granted.clone());

        Ok(granted)
    }

    /// Suggest alternatives when request is denied
    async fn suggest_alternatives(
        &self,
        request: &PermissionRequest,
        context: &EvaluationContext,
    ) -> Vec<Alternative> {
        let mut alternatives = Vec::new();

        // Suggest read-only version
        if request.requested_permissions.additional_commands.iter().any(|c| c.contains("write")) {
            alternatives.push(Alternative {
                description: "Read-only access to requested resources".to_string(),
                modified_permissions: RequestedPermissions {
                    additional_commands: vec!["cat".to_string(), "ls".to_string()],
                    additional_paths: request.requested_permissions.additional_paths.clone(),
                    additional_endpoints: vec![],
                    increased_memory_mb: None,
                    increased_cpu_seconds: None,
                    network_access: false,
                    spawn_processes: false,
                },
                likelihood_of_approval: 0.8,
            });
        }

        // Suggest sandboxed version
        alternatives.push(Alternative {
            description: "Execute in isolated sandbox environment".to_string(),
            modified_permissions: RequestedPermissions {
                additional_commands: request.requested_permissions.additional_commands.clone(),
                additional_paths: vec!["/tmp/sandbox".to_string()],
                additional_endpoints: vec![],
                increased_memory_mb: Some(50),
                increased_cpu_seconds: Some(30),
                network_access: false,
                spawn_processes: false,
            },
            likelihood_of_approval: 0.9,
        });

        // Suggest delegated version
        if context.agent_trust_score < 0.7 {
            alternatives.push(Alternative {
                description: "Delegate to higher-trust agent".to_string(),
                modified_permissions: RequestedPermissions {
                    additional_commands: vec![],
                    additional_paths: vec![],
                    additional_endpoints: vec![],
                    increased_memory_mb: None,
                    increased_cpu_seconds: None,
                    network_access: false,
                    spawn_processes: false,
                },
                likelihood_of_approval: 1.0,
            });
        }

        alternatives
    }

    /// Check if command is dangerous
    fn is_dangerous_command(&self, command: &str) -> bool {
        let dangerous = vec!["rm", "sudo", "kill", "eval", "exec"];
        dangerous.iter().any(|d| command.contains(d))
    }

    /// Check if path is safe
    fn is_safe_path(&self, path: &str, context: &EvaluationContext) -> bool {
        let forbidden = vec!["/etc", "/sys", "/proc", "/root"];
        
        // Check against forbidden paths
        if forbidden.iter().any(|f| path.starts_with(f)) {
            return false;
        }

        // Check against security zone
        match context.security_zone {
            SecurityLevel::Sandbox => path.starts_with("/tmp/sandbox"),
            SecurityLevel::Development => path.starts_with("/workspace") || path.starts_with("/tmp"),
            SecurityLevel::Staging => !path.starts_with("/production"),
            SecurityLevel::Production => false, // No new paths in production
        }
    }

    /// Log negotiation for audit
    async fn log_negotiation(
        &self,
        request: &PermissionRequest,
        response: &PermissionResponse,
        decision: PolicyDecision,
    ) {
        let entry = NegotiationAuditEntry {
            timestamp: Utc::now(),
            request_id: request.request_id,
            agent_id: request.agent_id.clone(),
            decision: response.decision.clone(),
            policy_decisions: vec![decision.reasoning.clone()],
            final_reason: response.reason.clone(),
        };

        let mut audit = self.audit_log.write().await;
        audit.push(entry);
    }

    /// Revoke previously granted permissions
    pub async fn revoke_permissions(&self, request_id: Uuid) -> Result<()> {
        let mut approved = self.approved_requests.write().await;
        if approved.remove(&request_id).is_some() {
            let _ = self.notification_channel.send(
                PermissionNotification::PermissionRevoked(request_id)
            ).await;
            Ok(())
        } else {
            Err(IntentError::NotFound(format!("Permission grant {} not found", request_id)))
        }
    }

    /// Check if permissions are still valid
    pub async fn check_permissions(&self, request_id: Uuid) -> Option<GrantedPermissions> {
        let approved = self.approved_requests.read().await;
        if let Some(granted) = approved.get(&request_id) {
            if granted.valid_until > Utc::now() {
                return Some(granted.clone());
            }
        }
        None
    }
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self {
            policies: Self::create_default_policies(),
            risk_thresholds: Self::create_risk_thresholds(),
            auto_approve_patterns: Self::create_auto_approve_patterns(),
            auto_deny_patterns: Self::create_auto_deny_patterns(),
        }
    }

    /// Evaluate a permission request against all policies
    pub async fn evaluate(
        &self,
        request: &PermissionRequest,
        context: &EvaluationContext,
    ) -> PolicyDecision {
        // Check auto-deny patterns first
        for pattern in &self.auto_deny_patterns {
            if self.matches_deny_pattern(request, pattern) {
                return PolicyDecision {
                    recommendation: Decision::Denied,
                    confidence: 1.0,
                    reasoning: pattern.reason.clone(),
                    conditions: vec![],
                };
            }
        }

        // Check auto-approve patterns
        for pattern in &self.auto_approve_patterns {
            if self.matches_approve_pattern(request, pattern, context) {
                return PolicyDecision {
                    recommendation: Decision::Approved,
                    confidence: 0.9,
                    reasoning: format!("Auto-approved by pattern: {}", pattern.name),
                    conditions: vec![],
                };
            }
        }

        // Evaluate all policies
        let mut decisions = Vec::new();
        for policy in &self.policies {
            let decision = policy.evaluate(request, context).await;
            decisions.push(decision);
        }

        // Aggregate decisions (weighted by confidence)
        self.aggregate_decisions(decisions)
    }

    /// Check if request matches deny pattern
    fn matches_deny_pattern(&self, request: &PermissionRequest, pattern: &AutoDenyPattern) -> bool {
        if let Some(cmd_pattern) = &pattern.command_pattern {
            for cmd in &request.requested_permissions.additional_commands {
                if cmd.contains(cmd_pattern) {
                    return true;
                }
            }
        }

        if let Some(path_pattern) = &pattern.path_pattern {
            for path in &request.requested_permissions.additional_paths {
                if path.contains(path_pattern) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if request matches approve pattern
    fn matches_approve_pattern(
        &self,
        request: &PermissionRequest,
        pattern: &AutoApprovePattern,
        context: &EvaluationContext,
    ) -> bool {
        // Check trust score
        if context.agent_trust_score < pattern.min_trust_score {
            return false;
        }

        // Check risk level
        if context.current_risk_level > pattern.max_risk_level {
            return false;
        }

        // Check patterns
        let mut matches = true;
        if let Some(cmd_pattern) = &pattern.command_pattern {
            matches &= request.requested_permissions.additional_commands
                .iter()
                .all(|cmd| cmd.contains(cmd_pattern));
        }

        if let Some(path_pattern) = &pattern.path_pattern {
            matches &= request.requested_permissions.additional_paths
                .iter()
                .all(|path| path.contains(path_pattern));
        }

        matches
    }

    /// Aggregate multiple policy decisions
    fn aggregate_decisions(&self, decisions: Vec<PolicyDecision>) -> PolicyDecision {
        if decisions.is_empty() {
            return PolicyDecision {
                recommendation: Decision::Deferred,
                confidence: 0.0,
                reasoning: "No policies evaluated".to_string(),
                conditions: vec![],
            };
        }

        // Count recommendations
        let mut counts = HashMap::new();
        let mut total_confidence = 0.0;
        
        for decision in &decisions {
            let count = counts.entry(decision.recommendation.clone()).or_insert(0.0);
            *count += decision.confidence;
            total_confidence += decision.confidence;
        }

        // Find the recommendation with highest weighted count
        let (recommendation, confidence) = counts
            .into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(rec, conf)| (rec, conf / total_confidence))
            .unwrap();

        // Collect all conditions
        let conditions: Vec<String> = decisions
            .iter()
            .flat_map(|d| d.conditions.clone())
            .collect();

        // Build reasoning
        let reasoning = decisions
            .iter()
            .map(|d| d.reasoning.clone())
            .collect::<Vec<_>>()
            .join("; ");

        PolicyDecision {
            recommendation,
            confidence,
            reasoning,
            conditions,
        }
    }

    fn create_default_policies() -> Vec<Box<dyn Policy + Send + Sync>> {
        vec![
            Box::new(TrustBasedPolicy),
            Box::new(RiskBasedPolicy),
            Box::new(ResourceBasedPolicy),
        ]
    }

    fn create_risk_thresholds() -> HashMap<RiskLevel, f64> {
        let mut thresholds = HashMap::new();
        thresholds.insert(RiskLevel::Minimal, 0.1);
        thresholds.insert(RiskLevel::Low, 0.3);
        thresholds.insert(RiskLevel::Medium, 0.5);
        thresholds.insert(RiskLevel::High, 0.7);
        thresholds.insert(RiskLevel::Critical, 0.9);
        thresholds
    }

    fn create_auto_approve_patterns() -> Vec<AutoApprovePattern> {
        vec![
            AutoApprovePattern {
                name: "read_only_workspace".to_string(),
                command_pattern: Some("cat".to_string()),
                path_pattern: Some("/workspace".to_string()),
                max_risk_level: RiskLevel::Low,
                min_trust_score: 0.5,
            },
            AutoApprovePattern {
                name: "temp_files".to_string(),
                command_pattern: None,
                path_pattern: Some("/tmp".to_string()),
                max_risk_level: RiskLevel::Medium,
                min_trust_score: 0.3,
            },
        ]
    }

    fn create_auto_deny_patterns() -> Vec<AutoDenyPattern> {
        vec![
            AutoDenyPattern {
                name: "system_modification".to_string(),
                command_pattern: Some("sudo".to_string()),
                path_pattern: None,
                reason: "System modification commands are never auto-approved".to_string(),
            },
            AutoDenyPattern {
                name: "sensitive_paths".to_string(),
                command_pattern: None,
                path_pattern: Some("/etc/passwd".to_string()),
                reason: "Access to sensitive system files is forbidden".to_string(),
            },
        ]
    }
}

/// Trust-based policy implementation
struct TrustBasedPolicy;

#[async_trait::async_trait]
impl Policy for TrustBasedPolicy {
    async fn evaluate(&self, _request: &PermissionRequest, context: &EvaluationContext) -> PolicyDecision {
        let recommendation = if context.agent_trust_score > 0.8 {
            Decision::Approved
        } else if context.agent_trust_score > 0.5 {
            Decision::PartiallyApproved
        } else if context.agent_trust_score > 0.3 {
            Decision::RequiresEscalation
        } else {
            Decision::Denied
        };

        PolicyDecision {
            recommendation,
            confidence: context.agent_trust_score,
            reasoning: format!("Trust score: {:.2}", context.agent_trust_score),
            conditions: vec![],
        }
    }

    fn name(&self) -> &str {
        "TrustBasedPolicy"
    }

    fn priority(&self) -> i32 {
        100
    }
}

/// Risk-based policy implementation
struct RiskBasedPolicy;

#[async_trait::async_trait]
impl Policy for RiskBasedPolicy {
    async fn evaluate(&self, _request: &PermissionRequest, context: &EvaluationContext) -> PolicyDecision {
        let recommendation = match context.current_risk_level {
            RiskLevel::Minimal | RiskLevel::Low => Decision::Approved,
            RiskLevel::Medium => Decision::PartiallyApproved,
            RiskLevel::High => Decision::RequiresEscalation,
            RiskLevel::Critical => Decision::Denied,
        };

        PolicyDecision {
            recommendation,
            confidence: 0.7,
            reasoning: format!("Risk level: {:?}", context.current_risk_level),
            conditions: vec![],
        }
    }

    fn name(&self) -> &str {
        "RiskBasedPolicy"
    }

    fn priority(&self) -> i32 {
        90
    }
}

/// Resource-based policy implementation
struct ResourceBasedPolicy;

#[async_trait::async_trait]
impl Policy for ResourceBasedPolicy {
    async fn evaluate(&self, request: &PermissionRequest, context: &EvaluationContext) -> PolicyDecision {
        // Check if requested resources exceed available
        let mut issues = Vec::new();
        
        if let Some(mem) = request.requested_permissions.increased_memory_mb {
            if context.resource_usage.memory_used_mb + mem > 1024 {
                issues.push("Memory limit would be exceeded".to_string());
            }
        }

        if context.resource_usage.cpu_percent > 80.0 {
            issues.push("CPU usage already high".to_string());
        }

        let recommendation = if issues.is_empty() {
            Decision::Approved
        } else if issues.len() == 1 {
            Decision::PartiallyApproved
        } else {
            Decision::Denied
        };

        PolicyDecision {
            recommendation,
            confidence: 0.6,
            reasoning: if issues.is_empty() {
                "Resource availability OK".to_string()
            } else {
                issues.join("; ")
            },
            conditions: issues,
        }
    }

    fn name(&self) -> &str {
        "ResourceBasedPolicy"
    }

    fn priority(&self) -> i32 {
        80
    }
}