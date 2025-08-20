//! Unit tests for authorization and access control
//! 
//! These tests verify role-based access control, permissions, and policy enforcement.

#![cfg(test)]

use synapsed_identity::authorization::*;
use synapsed_identity::{Error, Result};
use crate::test_framework::{*, performance::*, security::*};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
mod role_tests {
    use super::*;

    #[test]
    fn test_role_creation() {
        let mut auth_service = AuthorizationService::new();
        
        let role = Role {
            id: "admin".to_string(),
            name: "Administrator".to_string(),
            description: "Full system access".to_string(),
            permissions: vec![
                "users:read".to_string(),
                "users:write".to_string(),
                "users:delete".to_string(),
                "system:manage".to_string(),
            ],
        };
        
        let result = auth_service.create_role(&role);
        assert!(result.is_ok(), "Failed to create role");
        
        // Verify role exists
        let retrieved = auth_service.get_role("admin");
        assert!(retrieved.is_ok(), "Failed to retrieve role");
        assert_eq!(retrieved.unwrap().name, "Administrator");
    }

    #[test]
    fn test_role_hierarchy() {
        let mut auth_service = AuthorizationService::new();
        
        // Create parent role
        let parent_role = Role {
            id: "manager".to_string(),
            name: "Manager".to_string(),
            description: "Department manager".to_string(),
            permissions: vec!["department:manage".to_string()],
        };
        auth_service.create_role(&parent_role).unwrap();
        
        // Create child role inheriting from parent
        let child_role = Role {
            id: "team_lead".to_string(),
            name: "Team Lead".to_string(),
            description: "Team leader".to_string(),
            permissions: vec!["team:manage".to_string()],
        };
        auth_service.create_role_with_inheritance(&child_role, &["manager"]).unwrap();
        
        // Child should have both its own and parent permissions
        let effective_perms = auth_service.get_effective_permissions("team_lead").unwrap();
        assert!(effective_perms.contains("team:manage"));
        assert!(effective_perms.contains("department:manage"));
    }

    #[test]
    fn test_role_assignment() {
        let mut auth_service = AuthorizationService::new();
        
        // Create roles
        let user_role = Role {
            id: "user".to_string(),
            name: "User".to_string(),
            description: "Standard user".to_string(),
            permissions: vec!["profile:read".to_string()],
        };
        let admin_role = Role {
            id: "admin".to_string(),
            name: "Admin".to_string(),
            description: "Administrator".to_string(),
            permissions: vec!["system:manage".to_string()],
        };
        
        auth_service.create_role(&user_role).unwrap();
        auth_service.create_role(&admin_role).unwrap();
        
        // Assign roles to user
        let user_id = "test-user-123";
        auth_service.assign_role(user_id, "user").unwrap();
        auth_service.assign_role(user_id, "admin").unwrap();
        
        // Verify user has both roles
        let user_roles = auth_service.get_user_roles(user_id).unwrap();
        assert_eq!(user_roles.len(), 2);
        assert!(user_roles.contains(&"user".to_string()));
        assert!(user_roles.contains(&"admin".to_string()));
    }

    #[test]
    fn test_role_removal() {
        let mut auth_service = AuthorizationService::new();
        
        // Setup role and assignment
        let role = Role {
            id: "temp_role".to_string(),
            name: "Temporary Role".to_string(),
            description: "Temporary access".to_string(),
            permissions: vec!["temp:access".to_string()],
        };
        auth_service.create_role(&role).unwrap();
        
        let user_id = "temp-user";
        auth_service.assign_role(user_id, "temp_role").unwrap();
        
        // Remove role from user
        auth_service.remove_role(user_id, "temp_role").unwrap();
        
        // Verify role is removed
        let user_roles = auth_service.get_user_roles(user_id).unwrap();
        assert!(!user_roles.contains(&"temp_role".to_string()));
    }

    #[test]
    fn test_role_constraints() {
        let mut auth_service = AuthorizationService::new();
        
        // Create mutually exclusive roles
        let role1 = Role {
            id: "approver".to_string(),
            name: "Approver".to_string(),
            description: "Can approve requests".to_string(),
            permissions: vec!["requests:approve".to_string()],
        };
        let role2 = Role {
            id: "requester".to_string(),
            name: "Requester".to_string(),
            description: "Can create requests".to_string(),
            permissions: vec!["requests:create".to_string()],
        };
        
        auth_service.create_role(&role1).unwrap();
        auth_service.create_role(&role2).unwrap();
        auth_service.add_role_constraint("approver", "requester", ConstraintType::MutuallyExclusive).unwrap();
        
        // Assign first role
        let user_id = "constrained-user";
        auth_service.assign_role(user_id, "approver").unwrap();
        
        // Attempt to assign conflicting role should fail
        let result = auth_service.assign_role(user_id, "requester");
        assert!(result.is_err(), "Should not allow mutually exclusive roles");
        assert!(matches!(result.unwrap_err(), Error::RoleConstraintViolation));
    }
}

#[cfg(test)]
mod permission_tests {
    use super::*;

    #[test]
    fn test_permission_check() {
        let mut auth_service = AuthorizationService::new();
        
        // Create role with permissions
        let role = Role {
            id: "editor".to_string(),
            name: "Editor".to_string(),
            description: "Can edit content".to_string(),
            permissions: vec![
                "content:read".to_string(),
                "content:write".to_string(),
                "content:publish".to_string(),
            ],
        };
        auth_service.create_role(&role).unwrap();
        
        // Assign role to user
        let user_id = "editor-user";
        auth_service.assign_role(user_id, "editor").unwrap();
        
        // Check permissions
        assert!(auth_service.has_permission(user_id, "content:read").unwrap());
        assert!(auth_service.has_permission(user_id, "content:write").unwrap());
        assert!(auth_service.has_permission(user_id, "content:publish").unwrap());
        assert!(!auth_service.has_permission(user_id, "content:delete").unwrap());
    }

    #[test]
    fn test_wildcard_permissions() {
        let mut auth_service = AuthorizationService::new();
        
        // Create role with wildcard permission
        let role = Role {
            id: "content_admin".to_string(),
            name: "Content Admin".to_string(),
            description: "Full content access".to_string(),
            permissions: vec!["content:*".to_string()],
        };
        auth_service.create_role(&role).unwrap();
        
        let user_id = "content-admin-user";
        auth_service.assign_role(user_id, "content_admin").unwrap();
        
        // Should match any content permission
        assert!(auth_service.has_permission(user_id, "content:read").unwrap());
        assert!(auth_service.has_permission(user_id, "content:write").unwrap());
        assert!(auth_service.has_permission(user_id, "content:delete").unwrap());
        assert!(auth_service.has_permission(user_id, "content:archive").unwrap());
        
        // Should not match non-content permissions
        assert!(!auth_service.has_permission(user_id, "users:read").unwrap());
    }

    #[test]
    fn test_hierarchical_permissions() {
        let mut auth_service = AuthorizationService::new();
        
        // Create role with hierarchical permissions
        let role = Role {
            id: "org_admin".to_string(),
            name: "Organization Admin".to_string(),
            description: "Organization administrator".to_string(),
            permissions: vec!["org:123:*".to_string()],
        };
        auth_service.create_role(&role).unwrap();
        
        let user_id = "org-admin-user";
        auth_service.assign_role(user_id, "org_admin").unwrap();
        
        // Should have access to all resources under org:123
        assert!(auth_service.has_permission(user_id, "org:123:users:read").unwrap());
        assert!(auth_service.has_permission(user_id, "org:123:projects:write").unwrap());
        assert!(auth_service.has_permission(user_id, "org:123:billing:view").unwrap());
        
        // Should not have access to other organizations
        assert!(!auth_service.has_permission(user_id, "org:456:users:read").unwrap());
    }

    #[test]
    fn test_permission_inheritance() {
        let mut auth_service = AuthorizationService::new();
        
        // Create base role
        let base_role = Role {
            id: "employee".to_string(),
            name: "Employee".to_string(),
            description: "Basic employee".to_string(),
            permissions: vec!["company:read".to_string()],
        };
        
        // Create specialized role
        let dev_role = Role {
            id: "developer".to_string(),
            name: "Developer".to_string(),
            description: "Software developer".to_string(),
            permissions: vec!["code:write".to_string()],
        };
        
        auth_service.create_role(&base_role).unwrap();
        auth_service.create_role_with_inheritance(&dev_role, &["employee"]).unwrap();
        
        let user_id = "dev-user";
        auth_service.assign_role(user_id, "developer").unwrap();
        
        // Should have both inherited and direct permissions
        assert!(auth_service.has_permission(user_id, "company:read").unwrap());
        assert!(auth_service.has_permission(user_id, "code:write").unwrap());
    }

    #[test]
    fn test_dynamic_permissions() {
        let mut auth_service = AuthorizationService::new();
        
        // Register dynamic permission evaluator
        auth_service.register_dynamic_permission("resource:own", |context| {
            // Check if user owns the resource
            if let (Some(user_id), Some(resource_owner)) = 
                (context.get("user_id"), context.get("resource_owner")) {
                user_id == resource_owner
            } else {
                false
            }
        });
        
        // Create role with dynamic permission
        let role = Role {
            id: "resource_owner".to_string(),
            name: "Resource Owner".to_string(),
            description: "Can manage own resources".to_string(),
            permissions: vec!["resource:own".to_string()],
        };
        auth_service.create_role(&role).unwrap();
        
        let user_id = "owner-user";
        auth_service.assign_role(user_id, "resource_owner").unwrap();
        
        // Test with owned resource
        let mut context = HashMap::new();
        context.insert("user_id".to_string(), user_id.to_string());
        context.insert("resource_owner".to_string(), user_id.to_string());
        assert!(auth_service.has_permission_with_context(user_id, "resource:own", &context).unwrap());
        
        // Test with non-owned resource
        context.insert("resource_owner".to_string(), "other-user".to_string());
        assert!(!auth_service.has_permission_with_context(user_id, "resource:own", &context).unwrap());
    }
}

#[cfg(test)]
mod policy_tests {
    use super::*;

    #[test]
    fn test_policy_creation() {
        let mut auth_service = AuthorizationService::new();
        
        let policy = Policy {
            id: "data_access_policy".to_string(),
            name: "Data Access Policy".to_string(),
            effect: PolicyEffect::Allow,
            principals: vec!["role:analyst".to_string()],
            actions: vec!["data:read".to_string(), "data:export".to_string()],
            resources: vec!["database:analytics/*".to_string()],
            conditions: HashMap::new(),
        };
        
        let result = auth_service.create_policy(&policy);
        assert!(result.is_ok(), "Failed to create policy");
    }

    #[test]
    fn test_policy_evaluation() {
        let mut auth_service = AuthorizationService::new();
        
        // Create allow policy
        let allow_policy = Policy {
            id: "allow_read".to_string(),
            name: "Allow Read".to_string(),
            effect: PolicyEffect::Allow,
            principals: vec!["user:test-user".to_string()],
            actions: vec!["document:read".to_string()],
            resources: vec!["document:*".to_string()],
            conditions: HashMap::new(),
        };
        
        // Create deny policy (should override allow)
        let deny_policy = Policy {
            id: "deny_sensitive".to_string(),
            name: "Deny Sensitive".to_string(),
            effect: PolicyEffect::Deny,
            principals: vec!["user:test-user".to_string()],
            actions: vec!["document:read".to_string()],
            resources: vec!["document:sensitive/*".to_string()],
            conditions: HashMap::new(),
        };
        
        auth_service.create_policy(&allow_policy).unwrap();
        auth_service.create_policy(&deny_policy).unwrap();
        
        // Should allow regular documents
        let request1 = AccessRequest {
            principal: "user:test-user".to_string(),
            action: "document:read".to_string(),
            resource: "document:regular/file1".to_string(),
            context: HashMap::new(),
        };
        assert!(auth_service.evaluate_policies(&request1).unwrap());
        
        // Should deny sensitive documents
        let request2 = AccessRequest {
            principal: "user:test-user".to_string(),
            action: "document:read".to_string(),
            resource: "document:sensitive/file2".to_string(),
            context: HashMap::new(),
        };
        assert!(!auth_service.evaluate_policies(&request2).unwrap());
    }

    #[test]
    fn test_policy_conditions() {
        let mut auth_service = AuthorizationService::new();
        
        // Create policy with time-based condition
        let mut conditions = HashMap::new();
        conditions.insert("time_range".to_string(), PolicyCondition::TimeRange {
            start: "09:00".to_string(),
            end: "17:00".to_string(),
        });
        
        let policy = Policy {
            id: "business_hours_policy".to_string(),
            name: "Business Hours Only".to_string(),
            effect: PolicyEffect::Allow,
            principals: vec!["role:employee".to_string()],
            actions: vec!["system:access".to_string()],
            resources: vec!["system:production".to_string()],
            conditions,
        };
        
        auth_service.create_policy(&policy).unwrap();
        
        // Test during business hours
        let mut context = HashMap::new();
        context.insert("current_time".to_string(), "14:30".to_string());
        
        let request = AccessRequest {
            principal: "role:employee".to_string(),
            action: "system:access".to_string(),
            resource: "system:production".to_string(),
            context,
        };
        
        assert!(auth_service.evaluate_policies(&request).unwrap());
        
        // Test outside business hours
        let mut after_hours_context = HashMap::new();
        after_hours_context.insert("current_time".to_string(), "22:00".to_string());
        
        let after_hours_request = AccessRequest {
            principal: "role:employee".to_string(),
            action: "system:access".to_string(),
            resource: "system:production".to_string(),
            context: after_hours_context,
        };
        
        assert!(!auth_service.evaluate_policies(&after_hours_request).unwrap());
    }

    #[test]
    fn test_policy_priority() {
        let mut auth_service = AuthorizationService::new();
        
        // Create conflicting policies with different priorities
        let low_priority_allow = Policy {
            id: "low_allow".to_string(),
            name: "Low Priority Allow".to_string(),
            effect: PolicyEffect::Allow,
            principals: vec!["user:test".to_string()],
            actions: vec!["resource:access".to_string()],
            resources: vec!["resource:test".to_string()],
            conditions: HashMap::new(),
            priority: Some(10),
        };
        
        let high_priority_deny = Policy {
            id: "high_deny".to_string(),
            name: "High Priority Deny".to_string(),
            effect: PolicyEffect::Deny,
            principals: vec!["user:test".to_string()],
            actions: vec!["resource:access".to_string()],
            resources: vec!["resource:test".to_string()],
            conditions: HashMap::new(),
            priority: Some(100),
        };
        
        auth_service.create_policy(&low_priority_allow).unwrap();
        auth_service.create_policy(&high_priority_deny).unwrap();
        
        // Higher priority deny should win
        let request = AccessRequest {
            principal: "user:test".to_string(),
            action: "resource:access".to_string(),
            resource: "resource:test".to_string(),
            context: HashMap::new(),
        };
        
        assert!(!auth_service.evaluate_policies(&request).unwrap());
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;

    #[test]
    fn test_permission_check_performance() {
        let mut auth_service = AuthorizationService::new();
        
        // Create complex role hierarchy
        for i in 0..100 {
            let role = Role {
                id: format!("role_{}", i),
                name: format!("Role {}", i),
                description: format!("Test role {}", i),
                permissions: (0..10).map(|j| format!("perm_{}_{}", i, j)).collect(),
            };
            auth_service.create_role(&role).unwrap();
        }
        
        // Assign multiple roles to user
        let user_id = "perf-test-user";
        for i in 0..10 {
            auth_service.assign_role(user_id, &format!("role_{}", i)).unwrap();
        }
        
        // Test permission check performance
        assert_performance!(
            || {
                auth_service.has_permission(user_id, "perm_5_5").unwrap();
            },
            5 // 5ms threshold
        );
    }

    #[test]
    fn test_policy_evaluation_performance() {
        let mut auth_service = AuthorizationService::new();
        
        // Create many policies
        for i in 0..1000 {
            let policy = Policy {
                id: format!("policy_{}", i),
                name: format!("Policy {}", i),
                effect: if i % 2 == 0 { PolicyEffect::Allow } else { PolicyEffect::Deny },
                principals: vec![format!("user:user_{}", i % 100)],
                actions: vec![format!("action:{}", i % 50)],
                resources: vec![format!("resource:{}", i % 200)],
                conditions: HashMap::new(),
            };
            auth_service.create_policy(&policy).unwrap();
        }
        
        // Test policy evaluation performance
        let request = AccessRequest {
            principal: "user:user_50".to_string(),
            action: "action:25".to_string(),
            resource: "resource:100".to_string(),
            context: HashMap::new(),
        };
        
        assert_performance!(
            || {
                auth_service.evaluate_policies(&request).unwrap();
            },
            10 // 10ms threshold
        );
    }

    #[test]
    fn test_bulk_authorization_operations() {
        let mut auth_service = AuthorizationService::new();
        
        // Setup roles and users
        let num_users = 1000;
        let num_roles = 50;
        
        for i in 0..num_roles {
            let role = Role {
                id: format!("bulk_role_{}", i),
                name: format!("Bulk Role {}", i),
                description: format!("Bulk test role {}", i),
                permissions: vec![format!("bulk:perm:{}", i)],
            };
            auth_service.create_role(&role).unwrap();
        }
        
        // Test bulk role assignment
        let (_, assign_time) = measure_time(|| {
            for user_idx in 0..num_users {
                let user_id = format!("bulk_user_{}", user_idx);
                let role_id = format!("bulk_role_{}", user_idx % num_roles);
                auth_service.assign_role(&user_id, &role_id).unwrap();
            }
        });
        
        let avg_assign = assign_time as f64 / num_users as f64;
        assert!(avg_assign < 1.0, "Average role assignment time too high: {:.2} ms", avg_assign);
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;

    #[test]
    fn test_privilege_escalation_prevention() {
        let mut auth_service = AuthorizationService::new();
        
        // Create admin role
        let admin_role = Role {
            id: "admin".to_string(),
            name: "Administrator".to_string(),
            description: "System administrator".to_string(),
            permissions: vec!["system:*".to_string()],
        };
        auth_service.create_role(&admin_role).unwrap();
        
        // Create user role with limited permissions
        let user_role = Role {
            id: "user".to_string(),
            name: "User".to_string(),
            description: "Regular user".to_string(),
            permissions: vec!["profile:read".to_string(), "profile:write".to_string()],
        };
        auth_service.create_role(&user_role).unwrap();
        
        // Regular user tries to grant themselves admin role
        let user_id = "regular-user";
        auth_service.assign_role(user_id, "user").unwrap();
        
        // Attempt to self-assign admin role should fail
        let result = auth_service.assign_role_with_authorization(
            user_id,  // actor
            user_id,  // target (self)
            "admin"   // role to assign
        );
        
        assert!(result.is_err(), "User should not be able to self-assign admin role");
        assert!(matches!(result.unwrap_err(), Error::InsufficientPermissions));
    }

    #[test]
    fn test_permission_bypass_prevention() {
        let mut auth_service = AuthorizationService::new();
        
        // Create role with specific permissions
        let role = Role {
            id: "limited".to_string(),
            name: "Limited Access".to_string(),
            description: "Limited permissions".to_string(),
            permissions: vec!["data:read:public".to_string()],
        };
        auth_service.create_role(&role).unwrap();
        
        let user_id = "limited-user";
        auth_service.assign_role(user_id, "limited").unwrap();
        
        // Try various permission bypass attempts
        assert!(!auth_service.has_permission(user_id, "data:read:*").unwrap());
        assert!(!auth_service.has_permission(user_id, "data:*:public").unwrap());
        assert!(!auth_service.has_permission(user_id, "data:read:private").unwrap());
        assert!(!auth_service.has_permission(user_id, "../data:read:public").unwrap());
    }

    #[test]
    fn test_role_injection_prevention() {
        let mut auth_service = AuthorizationService::new();
        
        // Attempt to create role with malicious ID
        let malicious_role = Role {
            id: "admin' OR '1'='1".to_string(),
            name: "Malicious".to_string(),
            description: "Injection attempt".to_string(),
            permissions: vec!["*".to_string()],
        };
        
        let result = auth_service.create_role(&malicious_role);
        assert!(result.is_err(), "Should reject role with potential injection");
    }
}