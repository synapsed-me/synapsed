//! Policy-based authorization
//! 
//! Implements flexible policy evaluation with:
//! - Policy language (simplified)
//! - Condition evaluation
//! - Policy combination strategies
//! - Attribute-based access control (ABAC)

use crate::{Error, Result};
use super::{AuthorizationProvider, AuthzRequest, AuthzDecision, Policy, PolicyEffect};

#[cfg(not(feature = "std"))]
use std::collections::BTreeMap;
#[cfg(feature = "std")]
use std::collections::BTreeMap;

/// Policy engine for evaluating authorization policies
pub struct PolicyEngine {
    /// Policies indexed by ID
    policies: BTreeMap<String, Policy>,
    /// Policy sets for grouping
    policy_sets: BTreeMap<String, PolicySet>,
    /// Attribute providers
    attribute_providers: Vec<Box<dyn AttributeProvider>>,
}

/// Policy set for grouping related policies
#[derive(Debug, Clone)]
pub struct PolicySet {
    /// Set ID
    pub id: String,
    /// Set name
    pub name: String,
    /// Policy IDs in this set
    pub policies: Vec<String>,
    /// Combination algorithm
    pub algorithm: CombiningAlgorithm,
}

/// Policy combining algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombiningAlgorithm {
    /// Deny overrides - any deny results in deny
    DenyOverrides,
    /// Permit overrides - any permit results in permit
    PermitOverrides,
    /// First applicable - use first matching policy
    FirstApplicable,
    /// Deny unless permit - default deny
    DenyUnlessPermit,
    /// Permit unless deny - default permit
    PermitUnlessDeny,
}

/// Attribute provider for dynamic attributes
pub trait AttributeProvider: Send + Sync {
    /// Get attribute value
    fn get_attribute(&self, subject: &str, attribute: &str) -> Option<serde_json::Value>;
}

/// Policy condition evaluator
pub struct ConditionEvaluator;

impl ConditionEvaluator {
    /// Evaluate a condition against a context
    pub fn evaluate(condition: &serde_json::Value, context: &EvaluationContext) -> Result<bool> {
        match condition {
            serde_json::Value::Object(map) => {
                // Handle logical operators
                if let Some(and_conditions) = map.get("and") {
                    if let serde_json::Value::Array(conditions) = and_conditions {
                        return Ok(conditions.iter().all(|c| {
                            Self::evaluate(c, context).unwrap_or(false)
                        }));
                    }
                }
                
                if let Some(or_conditions) = map.get("or") {
                    if let serde_json::Value::Array(conditions) = or_conditions {
                        return Ok(conditions.iter().any(|c| {
                            Self::evaluate(c, context).unwrap_or(false)
                        }));
                    }
                }
                
                if let Some(not_condition) = map.get("not") {
                    return Ok(!Self::evaluate(not_condition, context)?);
                }
                
                // Handle comparison operators
                if let Some(equals) = map.get("equals") {
                    return Self::evaluate_comparison(equals, context, |a, b| a == b);
                }
                
                if let Some(greater) = map.get("greater_than") {
                    return Self::evaluate_comparison(greater, context, |a, b| {
                        match (a, b) {
                            (serde_json::Value::Number(n1), serde_json::Value::Number(n2)) => {
                                n1.as_f64().unwrap_or(0.0) > n2.as_f64().unwrap_or(0.0)
                            }
                            _ => false,
                        }
                    });
                }
                
                if let Some(contains) = map.get("contains") {
                    return Self::evaluate_contains(contains, context);
                }
                
                Ok(true)
            }
            serde_json::Value::Bool(b) => Ok(*b),
            _ => Ok(true),
        }
    }
    
    /// Evaluate a comparison
    fn evaluate_comparison<F>(
        comparison: &serde_json::Value,
        context: &EvaluationContext,
        op: F
    ) -> Result<bool>
    where
        F: Fn(&serde_json::Value, &serde_json::Value) -> bool,
    {
        if let serde_json::Value::Object(map) = comparison {
            if let (Some(left), Some(right)) = (map.get("left"), map.get("right")) {
                let left_val = Self::resolve_value(left, context)?;
                let right_val = Self::resolve_value(right, context)?;
                return Ok(op(&left_val, &right_val));
            }
        }
        Ok(false)
    }
    
    /// Evaluate contains operation
    fn evaluate_contains(contains: &serde_json::Value, context: &EvaluationContext) -> Result<bool> {
        if let serde_json::Value::Object(map) = contains {
            if let (Some(haystack), Some(needle)) = (map.get("in"), map.get("value")) {
                let haystack_val = Self::resolve_value(haystack, context)?;
                let needle_val = Self::resolve_value(needle, context)?;
                
                if let serde_json::Value::Array(arr) = haystack_val {
                    return Ok(arr.contains(&needle_val));
                }
                
                if let (serde_json::Value::String(s), serde_json::Value::String(n)) = (&haystack_val, &needle_val) {
                    return Ok(s.contains(n));
                }
            }
        }
        Ok(false)
    }
    
    /// Resolve a value (could be literal or attribute reference)
    fn resolve_value(value: &serde_json::Value, context: &EvaluationContext) -> Result<serde_json::Value> {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(attr) = map.get("attribute") {
                    if let serde_json::Value::String(attr_name) = attr {
                        return Ok(context.get_attribute(attr_name).unwrap_or(serde_json::Value::Null));
                    }
                }
                Ok(value.clone())
            }
            _ => Ok(value.clone()),
        }
    }
}

/// Evaluation context for policy conditions
pub struct EvaluationContext<'a> {
    /// Request being evaluated
    pub request: &'a AuthzRequest,
    /// Additional attributes
    pub attributes: BTreeMap<String, serde_json::Value>,
}

impl<'a> EvaluationContext<'a> {
    /// Create new evaluation context
    pub fn new(request: &'a AuthzRequest) -> Self {
        Self {
            request,
            attributes: BTreeMap::new(),
        }
    }
    
    /// Get attribute value
    pub fn get_attribute(&self, name: &str) -> Option<serde_json::Value> {
        // Check built-in attributes
        match name {
            "subject" => Some(serde_json::Value::String(self.request.subject.clone())),
            "action" => Some(serde_json::Value::String(self.request.action.clone())),
            "resource" => Some(serde_json::Value::String(self.request.resource.clone())),
            _ => self.attributes.get(name).cloned(),
        }
    }
    
    /// Add attribute
    pub fn add_attribute(&mut self, name: String, value: serde_json::Value) {
        self.attributes.insert(name, value);
    }
}

impl PolicyEngine {
    /// Create new policy engine
    pub fn new() -> Self {
        Self {
            policies: BTreeMap::new(),
            policy_sets: BTreeMap::new(),
            attribute_providers: Vec::new(),
        }
    }
    
    /// Add a policy
    pub fn add_policy(&mut self, policy: Policy) {
        self.policies.insert(policy.id.clone(), policy);
    }
    
    /// Add a policy set
    pub fn add_policy_set(&mut self, policy_set: PolicySet) {
        self.policy_sets.insert(policy_set.id.clone(), policy_set);
    }
    
    /// Add an attribute provider
    pub fn add_attribute_provider(&mut self, provider: Box<dyn AttributeProvider>) {
        self.attribute_providers.push(provider);
    }
    
    /// Evaluate a single policy
    fn evaluate_policy(&self, policy: &Policy, context: &EvaluationContext) -> Result<AuthzDecision> {
        // Check if policy applies to this request
        if !self.policy_applies(policy, context.request)? {
            return Ok(AuthzDecision::Indeterminate);
        }
        
        // Evaluate conditions if present
        if let Some(conditions) = &policy.conditions {
            if !ConditionEvaluator::evaluate(conditions, context)? {
                return Ok(AuthzDecision::Indeterminate);
            }
        }
        
        // Return decision based on policy effect
        match policy.effect {
            PolicyEffect::Allow => Ok(AuthzDecision::Allow),
            PolicyEffect::Deny => Ok(AuthzDecision::Deny),
        }
    }
    
    /// Check if policy applies to request
    fn policy_applies(&self, policy: &Policy, request: &AuthzRequest) -> Result<bool> {
        // Check subjects
        if !policy.subjects.is_empty() && !policy.subjects.contains(&request.subject) {
            return Ok(false);
        }
        
        // Check resources
        if !policy.resources.is_empty() {
            let mut matches = false;
            for resource_pattern in &policy.resources {
                if resource_matches(resource_pattern, &request.resource) {
                    matches = true;
                    break;
                }
            }
            if !matches {
                return Ok(false);
            }
        }
        
        // Check actions
        if !policy.actions.is_empty() && !policy.actions.contains(&request.action) {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    /// Combine policy decisions
    fn combine_decisions(
        &self,
        decisions: Vec<AuthzDecision>,
        algorithm: CombiningAlgorithm
    ) -> AuthzDecision {
        match algorithm {
            CombiningAlgorithm::DenyOverrides => {
                if decisions.iter().any(|d| *d == AuthzDecision::Deny) {
                    AuthzDecision::Deny
                } else if decisions.iter().any(|d| *d == AuthzDecision::Allow) {
                    AuthzDecision::Allow
                } else {
                    AuthzDecision::Indeterminate
                }
            }
            CombiningAlgorithm::PermitOverrides => {
                if decisions.iter().any(|d| *d == AuthzDecision::Allow) {
                    AuthzDecision::Allow
                } else if decisions.iter().any(|d| *d == AuthzDecision::Deny) {
                    AuthzDecision::Deny
                } else {
                    AuthzDecision::Indeterminate
                }
            }
            CombiningAlgorithm::FirstApplicable => {
                decisions.into_iter()
                    .find(|d| *d != AuthzDecision::Indeterminate)
                    .unwrap_or(AuthzDecision::Indeterminate)
            }
            CombiningAlgorithm::DenyUnlessPermit => {
                if decisions.iter().any(|d| *d == AuthzDecision::Allow) {
                    AuthzDecision::Allow
                } else {
                    AuthzDecision::Deny
                }
            }
            CombiningAlgorithm::PermitUnlessDeny => {
                if decisions.iter().any(|d| *d == AuthzDecision::Deny) {
                    AuthzDecision::Deny
                } else {
                    AuthzDecision::Allow
                }
            }
        }
    }
}

impl AuthorizationProvider for PolicyEngine {
    fn authorize(&self, request: &AuthzRequest) -> Result<AuthzDecision> {
        let mut context = EvaluationContext::new(request);
        
        // Gather attributes from providers
        for provider in &self.attribute_providers {
            // Add common attributes
            if let Some(val) = provider.get_attribute(&request.subject, "department") {
                context.add_attribute("department".to_string(), val);
            }
            if let Some(val) = provider.get_attribute(&request.subject, "clearance_level") {
                context.add_attribute("clearance_level".to_string(), val);
            }
        }
        
        // Evaluate all policies
        let mut decisions = Vec::new();
        for (_, policy) in &self.policies {
            decisions.push(self.evaluate_policy(policy, &context)?);
        }
        
        // Use deny-overrides by default
        Ok(self.combine_decisions(decisions, CombiningAlgorithm::DenyOverrides))
    }
    
    fn get_permissions(&self, _subject: &str) -> Result<Vec<super::Permission>> {
        // Policy engine doesn't use traditional permissions
        Ok(Vec::new())
    }
    
    fn get_roles(&self, _subject: &str) -> Result<Vec<super::Role>> {
        // Policy engine doesn't use roles
        Ok(Vec::new())
    }
}

/// Helper function for resource pattern matching
fn resource_matches(pattern: &str, resource: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    
    if pattern == resource {
        return true;
    }
    
    if pattern.ends_with("/*") {
        let prefix = &pattern[..pattern.len() - 2];
        return resource.starts_with(prefix);
    }
    
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_condition_evaluation() {
        let request = AuthzRequest {
            subject: "alice".to_string(),
            action: "read".to_string(),
            resource: "/data/file.txt".to_string(),
            context: None,
        };
        
        let mut context = EvaluationContext::new(&request);
        context.add_attribute("department".to_string(), serde_json::json!("engineering"));
        
        // Test equals condition
        let condition = serde_json::json!({
            "equals": {
                "left": { "attribute": "department" },
                "right": "engineering"
            }
        });
        
        assert!(ConditionEvaluator::evaluate(&condition, &context).unwrap());
        
        // Test AND condition
        let and_condition = serde_json::json!({
            "and": [
                {
                    "equals": {
                        "left": { "attribute": "subject" },
                        "right": "alice"
                    }
                },
                {
                    "equals": {
                        "left": { "attribute": "action" },
                        "right": "read"
                    }
                }
            ]
        });
        
        assert!(ConditionEvaluator::evaluate(&and_condition, &context).unwrap());
    }
    
    #[test]
    fn test_policy_evaluation() {
        let mut engine = PolicyEngine::new();
        
        // Add a policy
        let policy = Policy {
            id: "allow_read".to_string(),
            name: "Allow Read Access".to_string(),
            effect: PolicyEffect::Allow,
            subjects: vec!["alice".to_string()],
            resources: vec!["/data/*".to_string()],
            actions: vec!["read".to_string()],
            conditions: Some(serde_json::json!({
                "equals": {
                    "left": { "attribute": "action" },
                    "right": "read"
                }
            })),
        };
        
        engine.add_policy(policy);
        
        // Test allowed request
        let request = AuthzRequest {
            subject: "alice".to_string(),
            action: "read".to_string(),
            resource: "/data/file.txt".to_string(),
            context: None,
        };
        
        assert_eq!(engine.authorize(&request).unwrap(), AuthzDecision::Allow);
        
        // Test denied request
        let request2 = AuthzRequest {
            subject: "alice".to_string(),
            action: "write".to_string(),
            resource: "/data/file.txt".to_string(),
            context: None,
        };
        
        assert_eq!(engine.authorize(&request2).unwrap(), AuthzDecision::Deny);
    }
}