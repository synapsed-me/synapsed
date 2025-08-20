//! Authorization module for access control
//! 
//! Provides:
//! - Role-based access control (RBAC)
//! - Permission management
//! - Policy enforcement
//! - Resource-based access control

use crate::{Error, Result};

use std::collections::BTreeSet;

pub mod rbac;
pub mod policy;
pub mod resource;

use async_trait::async_trait;

/// Simple authorizer trait for the IdentityManager
#[async_trait]
pub trait Authorizer: Send + Sync {
    /// Check if an identity is authorized for a resource/action
    async fn authorize(
        &self,
        identity: &crate::Identity,
        resource: &str,
        action: &str,
    ) -> Result<bool>;
}

/// Authorization provider trait
pub trait AuthorizationProvider: Send + Sync {
    /// Check if a subject has permission for an action on a resource
    fn authorize(&self, request: &AuthzRequest) -> Result<AuthzDecision>;
    
    /// Get all permissions for a subject
    fn get_permissions(&self, subject: &str) -> Result<Vec<Permission>>;
    
    /// Get all roles for a subject
    fn get_roles(&self, subject: &str) -> Result<Vec<Role>>;
}

/// Authorization request
#[derive(Debug, Clone)]
pub struct AuthzRequest {
    /// Subject (user or service) making the request
    pub subject: String,
    /// Action being requested
    pub action: String,
    /// Resource being accessed
    pub resource: String,
    /// Additional context
    pub context: Option<serde_json::Value>,
}

/// Authorization decision
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthzDecision {
    /// Access allowed
    Allow,
    /// Access denied
    Deny,
    /// Decision cannot be made (need more info)
    Indeterminate,
}

/// Permission definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Permission {
    /// Permission ID
    pub id: String,
    /// Resource pattern (supports wildcards)
    pub resource: String,
    /// Allowed actions
    pub actions: BTreeSet<String>,
    /// Optional conditions
    pub conditions: Option<serde_json::Value>,
}

impl Permission {
    /// Check if this permission allows an action on a resource
    pub fn allows(&self, action: &str, resource: &str) -> bool {
        // Check if action is allowed
        if !self.actions.contains(action) && !self.actions.contains("*") {
            return false;
        }
        
        // Check if resource matches pattern
        resource_matches(&self.resource, resource)
    }
}

/// Role definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    /// Role ID
    pub id: String,
    /// Role name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Permissions assigned to this role
    pub permissions: Vec<Permission>,
    /// Parent roles (for inheritance)
    pub parents: Vec<String>,
}

/// Policy definition
#[derive(Debug, Clone)]
pub struct Policy {
    /// Policy ID
    pub id: String,
    /// Policy name
    pub name: String,
    /// Policy effect (allow or deny)
    pub effect: PolicyEffect,
    /// Subjects this policy applies to
    pub subjects: Vec<String>,
    /// Resources this policy applies to
    pub resources: Vec<String>,
    /// Actions this policy covers
    pub actions: Vec<String>,
    /// Optional conditions
    pub conditions: Option<serde_json::Value>,
}

/// Policy effect
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyEffect {
    /// Allow the action
    Allow,
    /// Deny the action
    Deny,
}

/// Simple RBAC implementation
pub struct SimpleRbac {
    /// User to roles mapping
    user_roles: BTreeMap<String, BTreeSet<String>>,
    /// Role definitions
    roles: BTreeMap<String, Role>,
    /// Direct user permissions
    user_permissions: BTreeMap<String, Vec<Permission>>,
}

impl SimpleRbac {
    /// Create a new RBAC system
    pub fn new() -> Self {
        Self {
            user_roles: BTreeMap::new(),
            roles: BTreeMap::new(),
            user_permissions: BTreeMap::new(),
        }
    }
    
    /// Add a role definition
    pub fn add_role(&mut self, role: Role) {
        self.roles.insert(role.id.clone(), role);
    }
    
    /// Assign a role to a user
    pub fn assign_role(&mut self, user: &str, role_id: &str) -> Result<()> {
        if !self.roles.contains_key(role_id) {
            return Err(Error::NotFound(format!("Role {} not found", role_id)));
        }
        
        self.user_roles
            .entry(user.to_string())
            .or_insert_with(BTreeSet::new)
            .insert(role_id.to_string());
        
        Ok(())
    }
    
    /// Remove a role from a user
    pub fn revoke_role(&mut self, user: &str, role_id: &str) -> Result<()> {
        if let Some(roles) = self.user_roles.get_mut(user) {
            roles.remove(role_id);
        }
        Ok(())
    }
    
    /// Grant a direct permission to a user
    pub fn grant_permission(&mut self, user: &str, permission: Permission) {
        self.user_permissions
            .entry(user.to_string())
            .or_insert_with(Vec::new)
            .push(permission);
    }
    
    /// Get all permissions for a user (including from roles)
    fn get_all_permissions(&self, user: &str) -> Vec<Permission> {
        let mut permissions = Vec::new();
        
        // Direct permissions
        if let Some(direct) = self.user_permissions.get(user) {
            permissions.extend(direct.clone());
        }
        
        // Role permissions
        if let Some(role_ids) = self.user_roles.get(user) {
            for role_id in role_ids {
                if let Some(role) = self.roles.get(role_id) {
                    permissions.extend(role.permissions.clone());
                    
                    // Handle role inheritance
                    for parent_id in &role.parents {
                        if let Some(parent) = self.roles.get(parent_id) {
                            permissions.extend(parent.permissions.clone());
                        }
                    }
                }
            }
        }
        
        permissions
    }
}

impl AuthorizationProvider for SimpleRbac {
    fn authorize(&self, request: &AuthzRequest) -> Result<AuthzDecision> {
        let permissions = self.get_all_permissions(&request.subject);
        
        // Check if any permission allows this action
        for permission in permissions {
            if permission.allows(&request.action, &request.resource) {
                // TODO: Check conditions if present
                return Ok(AuthzDecision::Allow);
            }
        }
        
        // No permission found, deny by default
        Ok(AuthzDecision::Deny)
    }
    
    fn get_permissions(&self, subject: &str) -> Result<Vec<Permission>> {
        Ok(self.get_all_permissions(subject))
    }
    
    fn get_roles(&self, subject: &str) -> Result<Vec<Role>> {
        let mut roles = Vec::new();
        
        if let Some(role_ids) = self.user_roles.get(subject) {
            for role_id in role_ids {
                if let Some(role) = self.roles.get(role_id) {
                    roles.push(role.clone());
                }
            }
        }
        
        Ok(roles)
    }
}

/// Check if a resource matches a pattern (with wildcards)
fn resource_matches(pattern: &str, resource: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    
    if pattern == resource {
        return true;
    }
    
    // Simple wildcard matching
    if pattern.ends_with("/*") {
        let prefix = &pattern[..pattern.len() - 2];
        return resource.starts_with(prefix);
    }
    
    // Glob-style matching (simplified)
    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    let resource_parts: Vec<&str> = resource.split('/').collect();
    
    if pattern_parts.len() != resource_parts.len() {
        return false;
    }
    
    for (p, r) in pattern_parts.iter().zip(resource_parts.iter()) {
        if *p != "*" && *p != *r {
            return false;
        }
    }
    
    true
}

// Import BTreeMap for no_std environments
#[cfg(not(feature = "std"))]
use std::collections::BTreeMap;
#[cfg(feature = "std")]
use std::collections::BTreeMap;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resource_matching() {
        assert!(resource_matches("*", "anything"));
        assert!(resource_matches("/users/*", "/users/123"));
        assert!(resource_matches("/users/*/profile", "/users/123/profile"));
        assert!(!resource_matches("/users/*", "/posts/123"));
        assert!(!resource_matches("/users/*/profile", "/users/123/settings"));
    }
    
    #[test]
    fn test_permission_allows() {
        let mut actions = BTreeSet::new();
        actions.insert("read".to_string());
        actions.insert("write".to_string());
        
        let perm = Permission {
            id: "perm1".to_string(),
            resource: "/users/*".to_string(),
            actions,
            conditions: None,
        };
        
        assert!(perm.allows("read", "/users/123"));
        assert!(perm.allows("write", "/users/456"));
        assert!(!perm.allows("delete", "/users/123"));
        assert!(!perm.allows("read", "/posts/123"));
    }
    
    #[test]
    fn test_rbac_authorization() {
        let mut rbac = SimpleRbac::new();
        
        // Create a role
        let mut permissions = Vec::new();
        let mut actions = BTreeSet::new();
        actions.insert("read".to_string());
        actions.insert("write".to_string());
        
        permissions.push(Permission {
            id: "perm1".to_string(),
            resource: "/users/*".to_string(),
            actions,
            conditions: None,
        });
        
        let role = Role {
            id: "user_admin".to_string(),
            name: "User Administrator".to_string(),
            description: Some("Can manage users".to_string()),
            permissions,
            parents: Vec::new(),
        };
        
        rbac.add_role(role);
        rbac.assign_role("alice", "user_admin").unwrap();
        
        // Test authorization
        let request = AuthzRequest {
            subject: "alice".to_string(),
            action: "read".to_string(),
            resource: "/users/123".to_string(),
            context: None,
        };
        
        assert_eq!(rbac.authorize(&request).unwrap(), AuthzDecision::Allow);
        
        // Test unauthorized action
        let request2 = AuthzRequest {
            subject: "alice".to_string(),
            action: "delete".to_string(),
            resource: "/users/123".to_string(),
            context: None,
        };
        
        assert_eq!(rbac.authorize(&request2).unwrap(), AuthzDecision::Deny);
    }
}