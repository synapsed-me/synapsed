//! Resource-based authorization
//! 
//! Provides fine-grained access control for individual resources with:
//! - Resource hierarchies
//! - Access control lists (ACLs)
//! - Resource ownership
//! - Delegation support

use crate::{Error, Result};
use super::{AuthorizationProvider, AuthzRequest, AuthzDecision, Permission};

#[cfg(not(feature = "std"))]
use std::collections::BTreeMap;
#[cfg(feature = "std")]
use std::collections::BTreeMap;

/// Resource authorization system
pub struct ResourceAuthz {
    /// Resource definitions
    resources: BTreeMap<String, Resource>,
    /// Access control lists
    acls: BTreeMap<String, Acl>,
    /// Resource ownership
    ownership: BTreeMap<String, Ownership>,
    /// Delegation rules
    delegations: BTreeMap<String, Vec<Delegation>>,
}

/// Resource definition
#[derive(Debug, Clone)]
pub struct Resource {
    /// Resource ID
    pub id: String,
    /// Resource type
    pub resource_type: String,
    /// Parent resource (for hierarchies)
    pub parent: Option<String>,
    /// Resource metadata
    pub metadata: BTreeMap<String, serde_json::Value>,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Modified timestamp
    pub modified_at: chrono::DateTime<chrono::Utc>,
}

/// Access Control List
#[derive(Debug, Clone)]
pub struct Acl {
    /// Resource ID this ACL applies to
    pub resource_id: String,
    /// Access control entries
    pub entries: Vec<AclEntry>,
    /// Inherit from parent
    pub inherit_parent: bool,
}

/// ACL entry
#[derive(Debug, Clone)]
pub struct AclEntry {
    /// Principal (user or group)
    pub principal: Principal,
    /// Allowed actions
    pub permissions: Vec<String>,
    /// Grant or deny
    pub effect: AclEffect,
    /// Can delegate these permissions
    pub delegatable: bool,
}

/// ACL effect
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AclEffect {
    /// Grant permissions
    Allow,
    /// Deny permissions (takes precedence)
    Deny,
}

/// Principal type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Principal {
    /// Individual user
    User(String),
    /// Group of users
    Group(String),
    /// Everyone
    Everyone,
    /// Authenticated users
    Authenticated,
    /// Anonymous users
    Anonymous,
}

/// Resource ownership
#[derive(Debug, Clone)]
pub struct Ownership {
    /// Resource ID
    pub resource_id: String,
    /// Owner principal
    pub owner: Principal,
    /// Co-owners
    pub co_owners: Vec<Principal>,
    /// Ownership transferred from
    pub transferred_from: Option<Principal>,
    /// Transfer timestamp
    pub transferred_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Permission delegation
#[derive(Debug, Clone)]
pub struct Delegation {
    /// Who delegated the permission
    pub delegator: String,
    /// Who received the delegation
    pub delegate: String,
    /// What permissions were delegated
    pub permissions: Vec<String>,
    /// When the delegation expires
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Can the delegate further delegate
    pub can_delegate: bool,
}

impl ResourceAuthz {
    /// Create new resource authorization system
    pub fn new() -> Self {
        Self {
            resources: BTreeMap::new(),
            acls: BTreeMap::new(),
            ownership: BTreeMap::new(),
            delegations: BTreeMap::new(),
        }
    }
    
    /// Add a resource
    pub fn add_resource(&mut self, resource: Resource) -> Result<()> {
        self.resources.insert(resource.id.clone(), resource);
        Ok(())
    }
    
    /// Set resource ACL
    pub fn set_acl(&mut self, acl: Acl) -> Result<()> {
        self.acls.insert(acl.resource_id.clone(), acl);
        Ok(())
    }
    
    /// Set resource ownership
    pub fn set_ownership(&mut self, ownership: Ownership) -> Result<()> {
        self.ownership.insert(ownership.resource_id.clone(), ownership);
        Ok(())
    }
    
    /// Delegate permissions
    pub fn delegate_permission(
        &mut self,
        resource_id: &str,
        delegation: Delegation
    ) -> Result<()> {
        // Verify delegator has permission to delegate
        let request = AuthzRequest {
            subject: delegation.delegator.clone(),
            action: "delegate".to_string(),
            resource: resource_id.to_string(),
            context: None,
        };
        
        if self.authorize(&request)? != AuthzDecision::Allow {
            return Err(Error::AuthorizationFailed(
                "Delegator lacks permission to delegate".into()
            ));
        }
        
        self.delegations
            .entry(resource_id.to_string())
            .or_insert_with(Vec::new)
            .push(delegation);
        
        Ok(())
    }
    
    /// Check if principal matches subject
    fn principal_matches(&self, principal: &Principal, subject: &str) -> bool {
        match principal {
            Principal::User(user) => user == subject,
            Principal::Group(group) => {
                // In a real implementation, check group membership
                false
            }
            Principal::Everyone => true,
            Principal::Authenticated => !subject.is_empty(),
            Principal::Anonymous => subject.is_empty(),
        }
    }
    
    /// Get effective permissions for a subject on a resource
    fn get_effective_permissions(
        &self,
        subject: &str,
        resource_id: &str
    ) -> (Vec<String>, bool) {
        let mut allowed = Vec::new();
        let mut denied = Vec::new();
        let mut is_owner = false;
        
        // Check ownership
        if let Some(ownership) = self.ownership.get(resource_id) {
            if self.principal_matches(&ownership.owner, subject) {
                is_owner = true;
                // Owners typically have full access
                allowed.push("*".to_string());
            }
            
            for co_owner in &ownership.co_owners {
                if self.principal_matches(co_owner, subject) {
                    is_owner = true;
                    allowed.push("*".to_string());
                }
            }
        }
        
        // Check ACLs
        if let Some(acl) = self.acls.get(resource_id) {
            for entry in &acl.entries {
                if self.principal_matches(&entry.principal, subject) {
                    match entry.effect {
                        AclEffect::Allow => allowed.extend(entry.permissions.clone()),
                        AclEffect::Deny => denied.extend(entry.permissions.clone()),
                    }
                }
            }
            
            // Check parent ACLs if inheritance is enabled
            if acl.inherit_parent {
                if let Some(resource) = self.resources.get(resource_id) {
                    if let Some(parent_id) = &resource.parent {
                        let (parent_allowed, _) = self.get_effective_permissions(subject, parent_id);
                        allowed.extend(parent_allowed);
                    }
                }
            }
        }
        
        // Check delegations
        if let Some(delegations) = self.delegations.get(resource_id) {
            for delegation in delegations {
                if delegation.delegate == subject {
                    // Check if delegation is still valid
                    if let Some(expires_at) = delegation.expires_at {
                        if expires_at < chrono::Utc::now() {
                            continue;
                        }
                    }
                    allowed.extend(delegation.permissions.clone());
                }
            }
        }
        
        // Remove denied permissions
        allowed.retain(|perm| !denied.contains(perm) && !denied.contains(&"*".to_string()));
        
        (allowed, is_owner)
    }
}

impl AuthorizationProvider for ResourceAuthz {
    fn authorize(&self, request: &AuthzRequest) -> Result<AuthzDecision> {
        let (permissions, is_owner) = self.get_effective_permissions(
            &request.subject,
            &request.resource
        );
        
        // Check if action is allowed
        if permissions.contains(&request.action) || permissions.contains(&"*".to_string()) {
            return Ok(AuthzDecision::Allow);
        }
        
        // Owners can perform administrative actions
        if is_owner && (request.action == "delete" || 
                       request.action == "share" || 
                       request.action == "delegate") {
            return Ok(AuthzDecision::Allow);
        }
        
        Ok(AuthzDecision::Deny)
    }
    
    fn get_permissions(&self, subject: &str) -> Result<Vec<Permission>> {
        // Resource-based auth doesn't use traditional permissions
        Ok(Vec::new())
    }
    
    fn get_roles(&self, subject: &str) -> Result<Vec<super::Role>> {
        // Resource-based auth doesn't use roles
        Ok(Vec::new())
    }
}

/// Helper to create common ACL entries
pub struct AclBuilder {
    entries: Vec<AclEntry>,
}

impl AclBuilder {
    /// Create new ACL builder
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
    
    /// Allow user permissions
    pub fn allow_user(mut self, user: &str, permissions: Vec<String>) -> Self {
        self.entries.push(AclEntry {
            principal: Principal::User(user.to_string()),
            permissions,
            effect: AclEffect::Allow,
            delegatable: false,
        });
        self
    }
    
    /// Deny user permissions
    pub fn deny_user(mut self, user: &str, permissions: Vec<String>) -> Self {
        self.entries.push(AclEntry {
            principal: Principal::User(user.to_string()),
            permissions,
            effect: AclEffect::Deny,
            delegatable: false,
        });
        self
    }
    
    /// Allow group permissions
    pub fn allow_group(mut self, group: &str, permissions: Vec<String>) -> Self {
        self.entries.push(AclEntry {
            principal: Principal::Group(group.to_string()),
            permissions,
            effect: AclEffect::Allow,
            delegatable: false,
        });
        self
    }
    
    /// Allow everyone read access
    pub fn allow_public_read(mut self) -> Self {
        self.entries.push(AclEntry {
            principal: Principal::Everyone,
            permissions: vec!["read".to_string()],
            effect: AclEffect::Allow,
            delegatable: false,
        });
        self
    }
    
    /// Build ACL for resource
    pub fn build(self, resource_id: String, inherit_parent: bool) -> Acl {
        Acl {
            resource_id,
            entries: self.entries,
            inherit_parent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resource_ownership() {
        let mut authz = ResourceAuthz::new();
        
        // Add a resource
        let resource = Resource {
            id: "doc123".to_string(),
            resource_type: "document".to_string(),
            parent: None,
            metadata: BTreeMap::new(),
            created_at: chrono::Utc::now(),
            modified_at: chrono::Utc::now(),
        };
        
        authz.add_resource(resource).unwrap();
        
        // Set ownership
        let ownership = Ownership {
            resource_id: "doc123".to_string(),
            owner: Principal::User("alice".to_string()),
            co_owners: vec![],
            transferred_from: None,
            transferred_at: None,
        };
        
        authz.set_ownership(ownership).unwrap();
        
        // Owner should have access
        let request = AuthzRequest {
            subject: "alice".to_string(),
            action: "delete".to_string(),
            resource: "doc123".to_string(),
            context: None,
        };
        
        assert_eq!(authz.authorize(&request).unwrap(), AuthzDecision::Allow);
        
        // Non-owner should not have access
        let request2 = AuthzRequest {
            subject: "bob".to_string(),
            action: "delete".to_string(),
            resource: "doc123".to_string(),
            context: None,
        };
        
        assert_eq!(authz.authorize(&request2).unwrap(), AuthzDecision::Deny);
    }
    
    #[test]
    fn test_acl() {
        let mut authz = ResourceAuthz::new();
        
        // Add resource
        let resource = Resource {
            id: "folder/file.txt".to_string(),
            resource_type: "file".to_string(),
            parent: Some("folder".to_string()),
            metadata: BTreeMap::new(),
            created_at: chrono::Utc::now(),
            modified_at: chrono::Utc::now(),
        };
        
        authz.add_resource(resource).unwrap();
        
        // Create ACL
        let acl = AclBuilder::new()
            .allow_user("alice", vec!["read".to_string(), "write".to_string()])
            .allow_group("editors", vec!["read".to_string(), "write".to_string()])
            .deny_user("bob", vec!["write".to_string()])
            .allow_public_read()
            .build("folder/file.txt".to_string(), false);
        
        authz.set_acl(acl).unwrap();
        
        // Test alice can write
        let request = AuthzRequest {
            subject: "alice".to_string(),
            action: "write".to_string(),
            resource: "folder/file.txt".to_string(),
            context: None,
        };
        
        assert_eq!(authz.authorize(&request).unwrap(), AuthzDecision::Allow);
        
        // Test bob cannot write (explicit deny)
        let request2 = AuthzRequest {
            subject: "bob".to_string(),
            action: "write".to_string(),
            resource: "folder/file.txt".to_string(),
            context: None,
        };
        
        assert_eq!(authz.authorize(&request2).unwrap(), AuthzDecision::Deny);
        
        // Test everyone can read
        let request3 = AuthzRequest {
            subject: "anyone".to_string(),
            action: "read".to_string(),
            resource: "folder/file.txt".to_string(),
            context: None,
        };
        
        assert_eq!(authz.authorize(&request3).unwrap(), AuthzDecision::Allow);
    }
}