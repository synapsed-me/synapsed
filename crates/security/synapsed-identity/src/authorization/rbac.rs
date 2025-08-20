//! Role-Based Access Control (RBAC) implementation
//! 
//! Provides advanced RBAC features including:
//! - Hierarchical roles
//! - Dynamic permissions
//! - Separation of duties
//! - Role activation/deactivation

use crate::{Error, Result};
use super::{Role, Permission, AuthorizationProvider, AuthzRequest, AuthzDecision};

#[cfg(not(feature = "std"))]
use std::collections::{BTreeMap, BTreeSet};
#[cfg(feature = "std")]
use std::collections::{BTreeMap, BTreeSet};

/// Advanced RBAC system with hierarchical roles
pub struct HierarchicalRbac {
    /// Role definitions
    roles: BTreeMap<String, Role>,
    /// User-role assignments
    user_roles: BTreeMap<String, BTreeSet<RoleAssignment>>,
    /// Role hierarchy (child -> parents)
    role_hierarchy: BTreeMap<String, Vec<String>>,
    /// Separation of duties constraints
    sod_constraints: Vec<SodConstraint>,
    /// Dynamic permissions
    dynamic_permissions: BTreeMap<String, Box<dyn Fn(&AuthzRequest) -> bool + Send + Sync>>,
}

/// Role assignment with additional metadata
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RoleAssignment {
    /// Role ID
    pub role_id: String,
    /// Is the role currently active
    pub active: bool,
    /// When the role was assigned
    pub assigned_at: chrono::DateTime<chrono::Utc>,
    /// When the role expires (if applicable)
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Who assigned the role
    pub assigned_by: String,
}

/// Separation of Duties constraint
#[derive(Debug, Clone)]
pub struct SodConstraint {
    /// Constraint ID
    pub id: String,
    /// Constraint name
    pub name: String,
    /// Conflicting roles (user cannot have both)
    pub conflicting_roles: Vec<String>,
    /// Constraint type
    pub constraint_type: SodType,
}

/// Types of separation of duties
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SodType {
    /// Static: Roles cannot be assigned together
    Static,
    /// Dynamic: Roles cannot be activated together
    Dynamic,
}

impl HierarchicalRbac {
    /// Create a new hierarchical RBAC system
    pub fn new() -> Self {
        Self {
            roles: BTreeMap::new(),
            user_roles: BTreeMap::new(),
            role_hierarchy: BTreeMap::new(),
            sod_constraints: Vec::new(),
            dynamic_permissions: BTreeMap::new(),
        }
    }
    
    /// Add a role with hierarchy
    pub fn add_role(&mut self, role: Role) -> Result<()> {
        // Update hierarchy
        if !role.parents.is_empty() {
            self.role_hierarchy.insert(role.id.clone(), role.parents.clone());
        }
        
        self.roles.insert(role.id.clone(), role);
        Ok(())
    }
    
    /// Assign a role to a user with validation
    pub fn assign_role(&mut self, user: &str, role_id: &str, assigned_by: &str) -> Result<()> {
        // Check if role exists
        if !self.roles.contains_key(role_id) {
            return Err(Error::NotFound(format!("Role {} not found", role_id)));
        }
        
        // Check separation of duties
        self.check_sod_constraints(user, role_id, SodType::Static)?;
        
        let assignment = RoleAssignment {
            role_id: role_id.to_string(),
            active: true,
            assigned_at: chrono::Utc::now(),
            expires_at: None,
            assigned_by: assigned_by.to_string(),
        };
        
        self.user_roles
            .entry(user.to_string())
            .or_insert_with(BTreeSet::new)
            .insert(assignment);
        
        Ok(())
    }
    
    /// Activate/deactivate a role for a user
    pub fn set_role_active(&mut self, user: &str, role_id: &str, active: bool) -> Result<()> {
        if active {
            // Check dynamic separation of duties
            self.check_sod_constraints(user, role_id, SodType::Dynamic)?;
        }
        
        if let Some(assignments) = self.user_roles.get_mut(user) {
            // Find and remove the old assignment
            let mut found_assignment = None;
            for assignment in assignments.iter() {
                if assignment.role_id == role_id {
                    found_assignment = Some(assignment.clone());
                    break;
                }
            }
            
            if let Some(mut assignment) = found_assignment {
                // Remove old assignment
                assignments.retain(|a| a.role_id != role_id);
                // Update and re-insert
                assignment.active = active;
                assignments.insert(assignment);
                return Ok(());
            }
        }
        
        Err(Error::NotFound(format!("Role assignment not found for user {}", user)))
    }
    
    /// Add a separation of duties constraint
    pub fn add_sod_constraint(&mut self, constraint: SodConstraint) {
        self.sod_constraints.push(constraint);
    }
    
    /// Check separation of duties constraints
    fn check_sod_constraints(&self, user: &str, new_role: &str, sod_type: SodType) -> Result<()> {
        if let Some(assignments) = self.user_roles.get(user) {
            for constraint in &self.sod_constraints {
                if constraint.constraint_type != sod_type {
                    continue;
                }
                
                if constraint.conflicting_roles.contains(&new_role.to_string()) {
                    for assignment in assignments {
                        let check_active = sod_type == SodType::Dynamic && assignment.active;
                        let check_assigned = sod_type == SodType::Static;
                        
                        if (check_active || check_assigned) && 
                           constraint.conflicting_roles.contains(&assignment.role_id) {
                            return Err(Error::AuthorizationFailed(
                                format!("Separation of duties violation: {} conflicts with {}", 
                                        new_role, assignment.role_id)
                            ));
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Get all permissions for a user including inherited ones
    fn get_all_permissions(&self, user: &str) -> Vec<Permission> {
        let mut permissions = Vec::new();
        let mut processed_roles = BTreeSet::new();
        
        if let Some(assignments) = self.user_roles.get(user) {
            for assignment in assignments {
                if !assignment.active {
                    continue;
                }
                
                // Check expiration
                if let Some(expires_at) = assignment.expires_at {
                    if expires_at < chrono::Utc::now() {
                        continue;
                    }
                }
                
                self.collect_role_permissions(
                    &assignment.role_id,
                    &mut permissions,
                    &mut processed_roles
                );
            }
        }
        
        permissions
    }
    
    /// Recursively collect permissions from a role and its parents
    fn collect_role_permissions(
        &self,
        role_id: &str,
        permissions: &mut Vec<Permission>,
        processed: &mut BTreeSet<String>
    ) {
        if processed.contains(role_id) {
            return;
        }
        
        processed.insert(role_id.to_string());
        
        if let Some(role) = self.roles.get(role_id) {
            permissions.extend(role.permissions.clone());
            
            // Process parent roles
            if let Some(parents) = self.role_hierarchy.get(role_id) {
                for parent_id in parents {
                    self.collect_role_permissions(parent_id, permissions, processed);
                }
            }
        }
    }
    
    /// Add a dynamic permission evaluator
    pub fn add_dynamic_permission(
        &mut self,
        id: String,
        evaluator: Box<dyn Fn(&AuthzRequest) -> bool + Send + Sync>
    ) {
        self.dynamic_permissions.insert(id, evaluator);
    }
}

impl AuthorizationProvider for HierarchicalRbac {
    fn authorize(&self, request: &AuthzRequest) -> Result<AuthzDecision> {
        // Check static permissions
        let permissions = self.get_all_permissions(&request.subject);
        
        for permission in permissions {
            if permission.allows(&request.action, &request.resource) {
                return Ok(AuthzDecision::Allow);
            }
        }
        
        // Check dynamic permissions
        for (_, evaluator) in &self.dynamic_permissions {
            if evaluator(request) {
                return Ok(AuthzDecision::Allow);
            }
        }
        
        Ok(AuthzDecision::Deny)
    }
    
    fn get_permissions(&self, subject: &str) -> Result<Vec<Permission>> {
        Ok(self.get_all_permissions(subject))
    }
    
    fn get_roles(&self, subject: &str) -> Result<Vec<Role>> {
        let mut roles = Vec::new();
        let mut processed = BTreeSet::new();
        
        if let Some(assignments) = self.user_roles.get(subject) {
            for assignment in assignments {
                if assignment.active {
                    self.collect_roles(&assignment.role_id, &mut roles, &mut processed);
                }
            }
        }
        
        Ok(roles)
    }
}

impl HierarchicalRbac {
    /// Recursively collect roles including inherited ones
    fn collect_roles(
        &self,
        role_id: &str,
        roles: &mut Vec<Role>,
        processed: &mut BTreeSet<String>
    ) {
        if processed.contains(role_id) {
            return;
        }
        
        processed.insert(role_id.to_string());
        
        if let Some(role) = self.roles.get(role_id) {
            roles.push(role.clone());
            
            if let Some(parents) = self.role_hierarchy.get(role_id) {
                for parent_id in parents {
                    self.collect_roles(parent_id, roles, processed);
                }
            }
        }
    }
}

/// Pre-defined system roles
pub fn create_default_roles() -> Vec<Role> {
    vec![
        Role {
            id: "super_admin".to_string(),
            name: "Super Administrator".to_string(),
            description: Some("Full system access".to_string()),
            permissions: vec![
                Permission {
                    id: "all_access".to_string(),
                    resource: "*".to_string(),
                    actions: {
                        let mut set = BTreeSet::new();
                        set.insert("*".to_string());
                        set
                    },
                    conditions: None,
                }
            ],
            parents: vec![],
        },
        Role {
            id: "user_admin".to_string(),
            name: "User Administrator".to_string(),
            description: Some("Manage users and roles".to_string()),
            permissions: vec![
                Permission {
                    id: "user_mgmt".to_string(),
                    resource: "/users/*".to_string(),
                    actions: {
                        let mut set = BTreeSet::new();
                        set.insert("create".to_string());
                        set.insert("read".to_string());
                        set.insert("update".to_string());
                        set.insert("delete".to_string());
                        set
                    },
                    conditions: None,
                },
                Permission {
                    id: "role_mgmt".to_string(),
                    resource: "/roles/*".to_string(),
                    actions: {
                        let mut set = BTreeSet::new();
                        set.insert("read".to_string());
                        set.insert("assign".to_string());
                        set
                    },
                    conditions: None,
                }
            ],
            parents: vec![],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hierarchical_roles() {
        let mut rbac = HierarchicalRbac::new();
        
        // Create parent role
        let mut parent_perms = vec![
            Permission {
                id: "base_read".to_string(),
                resource: "/data/*".to_string(),
                actions: {
                    let mut set = BTreeSet::new();
                    set.insert("read".to_string());
                    set
                },
                conditions: None,
            }
        ];
        
        let parent_role = Role {
            id: "reader".to_string(),
            name: "Reader".to_string(),
            description: None,
            permissions: parent_perms,
            parents: vec![],
        };
        
        // Create child role
        let child_perms = vec![
            Permission {
                id: "write_perm".to_string(),
                resource: "/data/*".to_string(),
                actions: {
                    let mut set = BTreeSet::new();
                    set.insert("write".to_string());
                    set
                },
                conditions: None,
            }
        ];
        
        let child_role = Role {
            id: "writer".to_string(),
            name: "Writer".to_string(),
            description: None,
            permissions: child_perms,
            parents: vec!["reader".to_string()],
        };
        
        rbac.add_role(parent_role).unwrap();
        rbac.add_role(child_role).unwrap();
        rbac.assign_role("alice", "writer", "admin").unwrap();
        
        // Test that alice has both read and write permissions
        let request_read = AuthzRequest {
            subject: "alice".to_string(),
            action: "read".to_string(),
            resource: "/data/file.txt".to_string(),
            context: None,
        };
        
        let request_write = AuthzRequest {
            subject: "alice".to_string(),
            action: "write".to_string(),
            resource: "/data/file.txt".to_string(),
            context: None,
        };
        
        assert_eq!(rbac.authorize(&request_read).unwrap(), AuthzDecision::Allow);
        assert_eq!(rbac.authorize(&request_write).unwrap(), AuthzDecision::Allow);
    }
    
    #[test]
    fn test_separation_of_duties() {
        let mut rbac = HierarchicalRbac::new();
        
        // Create conflicting roles
        let role1 = Role {
            id: "approver".to_string(),
            name: "Approver".to_string(),
            description: None,
            permissions: vec![],
            parents: vec![],
        };
        
        let role2 = Role {
            id: "requester".to_string(),
            name: "Requester".to_string(),
            description: None,
            permissions: vec![],
            parents: vec![],
        };
        
        rbac.add_role(role1).unwrap();
        rbac.add_role(role2).unwrap();
        
        // Add SoD constraint
        let constraint = SodConstraint {
            id: "approval_sod".to_string(),
            name: "Approval Separation".to_string(),
            conflicting_roles: vec!["approver".to_string(), "requester".to_string()],
            constraint_type: SodType::Static,
        };
        
        rbac.add_sod_constraint(constraint);
        
        // Assign first role
        rbac.assign_role("bob", "approver", "admin").unwrap();
        
        // Try to assign conflicting role
        let result = rbac.assign_role("bob", "requester", "admin");
        assert!(result.is_err());
    }
}