//! Unit tests for user management and CRUD operations
//! 
//! These tests verify user creation, retrieval, update, and deletion functionality.

#![cfg(test)]

use synapsed_identity::storage::*;
use synapsed_identity::{Identity, Error, Result};
use synapsed_crypto::{ml_kem::*, ml_dsa::*};
use crate::test_framework::{*, performance::*, security::*};
use std::collections::HashMap;

#[cfg(test)]
mod user_crud_tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let mut user_store = UserStore::new();
        
        let user_data = UserData {
            id: None, // Let system generate ID
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            full_name: "Test User".to_string(),
            metadata: HashMap::new(),
        };
        
        let result = user_store.create_user(&user_data);
        assert!(result.is_ok(), "Failed to create user");
        
        let user = result.unwrap();
        assert!(!user.id.is_empty(), "User ID should be generated");
        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
    }

    #[test]
    fn test_user_creation_with_validation() {
        let mut user_store = UserStore::new();
        
        // Test invalid email
        let invalid_email_user = UserData {
            id: None,
            username: "validuser".to_string(),
            email: "invalid-email".to_string(),
            full_name: "Valid User".to_string(),
            metadata: HashMap::new(),
        };
        
        let result = user_store.create_user(&invalid_email_user);
        assert!(result.is_err(), "Should reject invalid email");
        assert!(matches!(result.unwrap_err(), Error::ValidationError(_)));
        
        // Test invalid username
        let invalid_username_user = UserData {
            id: None,
            username: "a".to_string(), // Too short
            email: "valid@example.com".to_string(),
            full_name: "Valid User".to_string(),
            metadata: HashMap::new(),
        };
        
        let result = user_store.create_user(&invalid_username_user);
        assert!(result.is_err(), "Should reject short username");
    }

    #[test]
    fn test_user_retrieval() {
        let mut user_store = UserStore::new();
        
        // Create user
        let user_data = UserData {
            id: None,
            username: "retrieveuser".to_string(),
            email: "retrieve@example.com".to_string(),
            full_name: "Retrieve User".to_string(),
            metadata: HashMap::new(),
        };
        
        let created_user = user_store.create_user(&user_data).unwrap();
        let user_id = &created_user.id;
        
        // Retrieve by ID
        let retrieved = user_store.get_user_by_id(user_id);
        assert!(retrieved.is_ok(), "Failed to retrieve user by ID");
        assert_eq!(retrieved.unwrap().username, "retrieveuser");
        
        // Retrieve by username
        let by_username = user_store.get_user_by_username("retrieveuser");
        assert!(by_username.is_ok(), "Failed to retrieve user by username");
        assert_eq!(by_username.unwrap().id, *user_id);
        
        // Retrieve by email
        let by_email = user_store.get_user_by_email("retrieve@example.com");
        assert!(by_email.is_ok(), "Failed to retrieve user by email");
        assert_eq!(by_email.unwrap().id, *user_id);
    }

    #[test]
    fn test_user_update() {
        let mut user_store = UserStore::new();
        
        // Create user
        let user_data = UserData {
            id: None,
            username: "updateuser".to_string(),
            email: "update@example.com".to_string(),
            full_name: "Update User".to_string(),
            metadata: HashMap::new(),
        };
        
        let created_user = user_store.create_user(&user_data).unwrap();
        let user_id = created_user.id.clone();
        
        // Update user
        let mut update_data = UserUpdateData {
            email: Some("newemail@example.com".to_string()),
            full_name: Some("Updated Name".to_string()),
            metadata: Some(HashMap::new()),
        };
        update_data.metadata.as_mut().unwrap().insert("updated".to_string(), "true".to_string());
        
        let update_result = user_store.update_user(&user_id, &update_data);
        assert!(update_result.is_ok(), "Failed to update user");
        
        // Verify updates
        let updated_user = user_store.get_user_by_id(&user_id).unwrap();
        assert_eq!(updated_user.email, "newemail@example.com");
        assert_eq!(updated_user.full_name, "Updated Name");
        assert_eq!(updated_user.metadata.get("updated"), Some(&"true".to_string()));
        
        // Username should not change
        assert_eq!(updated_user.username, "updateuser");
    }

    #[test]
    fn test_user_deletion() {
        let mut user_store = UserStore::new();
        
        // Create user
        let user_data = UserData {
            id: None,
            username: "deleteuser".to_string(),
            email: "delete@example.com".to_string(),
            full_name: "Delete User".to_string(),
            metadata: HashMap::new(),
        };
        
        let created_user = user_store.create_user(&user_data).unwrap();
        let user_id = created_user.id.clone();
        
        // Verify user exists
        assert!(user_store.get_user_by_id(&user_id).is_ok());
        
        // Delete user
        let delete_result = user_store.delete_user(&user_id);
        assert!(delete_result.is_ok(), "Failed to delete user");
        
        // Verify user no longer exists
        let get_result = user_store.get_user_by_id(&user_id);
        assert!(get_result.is_err(), "User should not exist after deletion");
        assert!(matches!(get_result.unwrap_err(), Error::UserNotFound));
    }

    #[test]
    fn test_soft_delete() {
        let mut user_store = UserStore::new();
        
        // Create user
        let user_data = UserData {
            id: None,
            username: "softdeleteuser".to_string(),
            email: "softdelete@example.com".to_string(),
            full_name: "Soft Delete User".to_string(),
            metadata: HashMap::new(),
        };
        
        let created_user = user_store.create_user(&user_data).unwrap();
        let user_id = created_user.id.clone();
        
        // Soft delete user
        let soft_delete_result = user_store.soft_delete_user(&user_id);
        assert!(soft_delete_result.is_ok(), "Failed to soft delete user");
        
        // User should not be found in normal queries
        let get_result = user_store.get_user_by_id(&user_id);
        assert!(get_result.is_err());
        
        // But should be retrievable with include_deleted flag
        let with_deleted = user_store.get_user_by_id_including_deleted(&user_id);
        assert!(with_deleted.is_ok(), "Should find soft-deleted user");
        assert!(with_deleted.unwrap().is_deleted);
    }

    #[test]
    fn test_user_restoration() {
        let mut user_store = UserStore::new();
        
        // Create and soft delete user
        let user_data = UserData {
            id: None,
            username: "restoreuser".to_string(),
            email: "restore@example.com".to_string(),
            full_name: "Restore User".to_string(),
            metadata: HashMap::new(),
        };
        
        let created_user = user_store.create_user(&user_data).unwrap();
        let user_id = created_user.id.clone();
        user_store.soft_delete_user(&user_id).unwrap();
        
        // Restore user
        let restore_result = user_store.restore_user(&user_id);
        assert!(restore_result.is_ok(), "Failed to restore user");
        
        // User should now be found in normal queries
        let restored_user = user_store.get_user_by_id(&user_id).unwrap();
        assert!(!restored_user.is_deleted);
        assert_eq!(restored_user.username, "restoreuser");
    }
}

#[cfg(test)]
mod user_identity_tests {
    use super::*;

    #[test]
    fn test_user_identity_creation() {
        let mut identity_service = IdentityService::new();
        
        let user_id = "identity-user-123";
        let identity_result = identity_service.create_identity(user_id);
        assert!(identity_result.is_ok(), "Failed to create user identity");
        
        let identity = identity_result.unwrap();
        assert_eq!(identity.id(), user_id);
        assert!(!identity.public_key().is_empty());
    }

    #[test]
    fn test_user_signature() {
        let mut identity_service = IdentityService::new();
        
        let user_id = "signing-user";
        let identity = identity_service.create_identity(user_id).unwrap();
        
        // Sign data
        let data = b"Important message to sign";
        let signature_result = identity.sign(data);
        assert!(signature_result.is_ok(), "Failed to sign data");
        
        let signature = signature_result.unwrap();
        assert!(!signature.is_empty(), "Signature should not be empty");
        
        // Verify signature
        let verify_result = identity.verify(data, &signature);
        assert!(verify_result.is_ok(), "Failed to verify signature");
        assert!(verify_result.unwrap(), "Valid signature should verify");
        
        // Verify with wrong data should fail
        let wrong_data = b"Different message";
        let wrong_verify = identity.verify(wrong_data, &signature);
        assert!(wrong_verify.is_ok());
        assert!(!wrong_verify.unwrap(), "Signature should not verify with wrong data");
    }

    #[test]
    fn test_identity_key_rotation() {
        let mut identity_service = IdentityService::new();
        
        let user_id = "rotation-user";
        let identity = identity_service.create_identity(user_id).unwrap();
        let original_public_key = identity.public_key().to_vec();
        
        // Rotate keys
        let rotation_result = identity_service.rotate_identity_keys(user_id);
        assert!(rotation_result.is_ok(), "Failed to rotate identity keys");
        
        // Get updated identity
        let updated_identity = identity_service.get_identity(user_id).unwrap();
        let new_public_key = updated_identity.public_key();
        
        // Keys should be different
        assert_ne!(original_public_key, new_public_key, "Public key should change after rotation");
    }
}

#[cfg(test)]
mod user_search_tests {
    use super::*;

    #[test]
    fn test_user_search() {
        let mut user_store = UserStore::new();
        
        // Create multiple users
        let users = vec![
            ("alice", "alice@example.com", "Alice Anderson"),
            ("bob", "bob@example.com", "Bob Brown"),
            ("charlie", "charlie@example.com", "Charlie Chen"),
            ("david", "david@example.com", "David Davis"),
        ];
        
        for (username, email, full_name) in users {
            let user_data = UserData {
                id: None,
                username: username.to_string(),
                email: email.to_string(),
                full_name: full_name.to_string(),
                metadata: HashMap::new(),
            };
            user_store.create_user(&user_data).unwrap();
        }
        
        // Search by username prefix
        let search_results = user_store.search_users(&SearchCriteria {
            username_prefix: Some("a".to_string()),
            ..Default::default()
        });
        assert!(search_results.is_ok());
        assert_eq!(search_results.unwrap().len(), 1); // Only Alice
        
        // Search by email domain
        let domain_search = user_store.search_users(&SearchCriteria {
            email_domain: Some("example.com".to_string()),
            ..Default::default()
        });
        assert!(domain_search.is_ok());
        assert_eq!(domain_search.unwrap().len(), 4); // All users
        
        // Search by full name
        let name_search = user_store.search_users(&SearchCriteria {
            full_name_contains: Some("David".to_string()),
            ..Default::default()
        });
        assert!(name_search.is_ok());
        assert_eq!(name_search.unwrap().len(), 1); // Only David
    }

    #[test]
    fn test_paginated_user_list() {
        let mut user_store = UserStore::new();
        
        // Create many users
        for i in 0..50 {
            let user_data = UserData {
                id: None,
                username: format!("user{:03}", i),
                email: format!("user{}@example.com", i),
                full_name: format!("User {}", i),
                metadata: HashMap::new(),
            };
            user_store.create_user(&user_data).unwrap();
        }
        
        // Get first page
        let page1 = user_store.list_users(&ListOptions {
            page: 1,
            page_size: 10,
            sort_by: SortField::Username,
            sort_order: SortOrder::Ascending,
        });
        assert!(page1.is_ok());
        let page1_results = page1.unwrap();
        assert_eq!(page1_results.users.len(), 10);
        assert_eq!(page1_results.total_count, 50);
        assert_eq!(page1_results.page, 1);
        assert_eq!(page1_results.total_pages, 5);
        
        // Get second page
        let page2 = user_store.list_users(&ListOptions {
            page: 2,
            page_size: 10,
            sort_by: SortField::Username,
            sort_order: SortOrder::Ascending,
        });
        assert!(page2.is_ok());
        let page2_results = page2.unwrap();
        assert_eq!(page2_results.users.len(), 10);
        
        // Verify different users on different pages
        assert_ne!(page1_results.users[0].id, page2_results.users[0].id);
    }
}

#[cfg(test)]
mod user_bulk_operations_tests {
    use super::*;

    #[test]
    fn test_bulk_user_creation() {
        let mut user_store = UserStore::new();
        
        let users_data = vec![
            UserData {
                id: None,
                username: "bulk1".to_string(),
                email: "bulk1@example.com".to_string(),
                full_name: "Bulk User 1".to_string(),
                metadata: HashMap::new(),
            },
            UserData {
                id: None,
                username: "bulk2".to_string(),
                email: "bulk2@example.com".to_string(),
                full_name: "Bulk User 2".to_string(),
                metadata: HashMap::new(),
            },
            UserData {
                id: None,
                username: "bulk3".to_string(),
                email: "bulk3@example.com".to_string(),
                full_name: "Bulk User 3".to_string(),
                metadata: HashMap::new(),
            },
        ];
        
        let result = user_store.bulk_create_users(&users_data);
        assert!(result.is_ok(), "Failed to bulk create users");
        
        let created_users = result.unwrap();
        assert_eq!(created_users.len(), 3);
        
        // Verify all users were created
        for (i, user) in created_users.iter().enumerate() {
            assert_eq!(user.username, format!("bulk{}", i + 1));
        }
    }

    #[test]
    fn test_bulk_user_update() {
        let mut user_store = UserStore::new();
        
        // Create users first
        let mut user_ids = vec![];
        for i in 1..=3 {
            let user_data = UserData {
                id: None,
                username: format!("updatebulk{}", i),
                email: format!("updatebulk{}@example.com", i),
                full_name: format!("Update Bulk {}", i),
                metadata: HashMap::new(),
            };
            let user = user_store.create_user(&user_data).unwrap();
            user_ids.push(user.id);
        }
        
        // Bulk update
        let updates: Vec<(String, UserUpdateData)> = user_ids.iter().enumerate().map(|(i, id)| {
            let update_data = UserUpdateData {
                email: Some(format!("updated{}@example.com", i)),
                full_name: Some(format!("Updated User {}", i)),
                metadata: None,
            };
            (id.clone(), update_data)
        }).collect();
        
        let update_result = user_store.bulk_update_users(&updates);
        assert!(update_result.is_ok(), "Failed to bulk update users");
        
        // Verify updates
        for (i, id) in user_ids.iter().enumerate() {
            let user = user_store.get_user_by_id(id).unwrap();
            assert_eq!(user.email, format!("updated{}@example.com", i));
            assert_eq!(user.full_name, format!("Updated User {}", i));
        }
    }

    #[test]
    fn test_bulk_user_deletion() {
        let mut user_store = UserStore::new();
        
        // Create users to delete
        let mut user_ids = vec![];
        for i in 1..=5 {
            let user_data = UserData {
                id: None,
                username: format!("deletebulk{}", i),
                email: format!("deletebulk{}@example.com", i),
                full_name: format!("Delete Bulk {}", i),
                metadata: HashMap::new(),
            };
            let user = user_store.create_user(&user_data).unwrap();
            user_ids.push(user.id);
        }
        
        // Bulk delete
        let delete_result = user_store.bulk_delete_users(&user_ids);
        assert!(delete_result.is_ok(), "Failed to bulk delete users");
        
        // Verify all deleted
        for id in &user_ids {
            assert!(user_store.get_user_by_id(id).is_err());
        }
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;

    #[test]
    fn test_user_creation_performance() {
        let mut user_store = UserStore::new();
        
        let user_data = UserData {
            id: None,
            username: "perfuser".to_string(),
            email: "perf@example.com".to_string(),
            full_name: "Performance User".to_string(),
            metadata: HashMap::new(),
        };
        
        assert_performance!(
            || {
                user_store.create_user(&user_data).unwrap();
            },
            50 // 50ms threshold
        );
    }

    #[test]
    fn test_user_lookup_performance() {
        let mut user_store = UserStore::new();
        
        // Create many users
        let mut user_ids = vec![];
        for i in 0..1000 {
            let user_data = UserData {
                id: None,
                username: format!("lookupuser{}", i),
                email: format!("lookup{}@example.com", i),
                full_name: format!("Lookup User {}", i),
                metadata: HashMap::new(),
            };
            let user = user_store.create_user(&user_data).unwrap();
            user_ids.push(user.id);
        }
        
        // Test lookup performance
        let test_id = &user_ids[500];
        assert_performance!(
            || {
                user_store.get_user_by_id(test_id).unwrap();
            },
            5 // 5ms threshold for lookup
        );
    }

    #[test]
    fn test_concurrent_user_operations() {
        use std::sync::{Arc, Mutex};
        use std::thread;
        
        let user_store = Arc::new(Mutex::new(UserStore::new()));
        let num_threads = 10;
        let users_per_thread = 50;
        
        let (_, elapsed) = measure_time(|| {
            let mut handles = vec![];
            
            for thread_idx in 0..num_threads {
                let store_clone = Arc::clone(&user_store);
                let handle = thread::spawn(move || {
                    for user_idx in 0..users_per_thread {
                        let user_data = UserData {
                            id: None,
                            username: format!("concurrent_{}_{}", thread_idx, user_idx),
                            email: format!("concurrent{}{}@example.com", thread_idx, user_idx),
                            full_name: format!("Concurrent User {} {}", thread_idx, user_idx),
                            metadata: HashMap::new(),
                        };
                        store_clone.lock().unwrap().create_user(&user_data).unwrap();
                    }
                });
                handles.push(handle);
            }
            
            for handle in handles {
                handle.join().unwrap();
            }
        });
        
        let total_users = num_threads * users_per_thread;
        let avg_time = elapsed as f64 / total_users as f64;
        
        assert!(
            avg_time < 5.0,
            "Average user creation time too high: {:.2} ms",
            avg_time
        );
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;

    #[test]
    fn test_duplicate_username_prevention() {
        let mut user_store = UserStore::new();
        
        let user_data1 = UserData {
            id: None,
            username: "duplicate".to_string(),
            email: "first@example.com".to_string(),
            full_name: "First User".to_string(),
            metadata: HashMap::new(),
        };
        
        let user_data2 = UserData {
            id: None,
            username: "duplicate".to_string(), // Same username
            email: "second@example.com".to_string(),
            full_name: "Second User".to_string(),
            metadata: HashMap::new(),
        };
        
        // First should succeed
        assert!(user_store.create_user(&user_data1).is_ok());
        
        // Second should fail
        let result = user_store.create_user(&user_data2);
        assert!(result.is_err(), "Should not allow duplicate username");
        assert!(matches!(result.unwrap_err(), Error::DuplicateUsername));
    }

    #[test]
    fn test_duplicate_email_prevention() {
        let mut user_store = UserStore::new();
        
        let user_data1 = UserData {
            id: None,
            username: "user1".to_string(),
            email: "duplicate@example.com".to_string(),
            full_name: "First User".to_string(),
            metadata: HashMap::new(),
        };
        
        let user_data2 = UserData {
            id: None,
            username: "user2".to_string(),
            email: "duplicate@example.com".to_string(), // Same email
            full_name: "Second User".to_string(),
            metadata: HashMap::new(),
        };
        
        // First should succeed
        assert!(user_store.create_user(&user_data1).is_ok());
        
        // Second should fail
        let result = user_store.create_user(&user_data2);
        assert!(result.is_err(), "Should not allow duplicate email");
        assert!(matches!(result.unwrap_err(), Error::DuplicateEmail));
    }

    #[test]
    fn test_sql_injection_prevention() {
        let mut user_store = UserStore::new();
        
        // Attempt SQL injection in username
        let malicious_user = UserData {
            id: None,
            username: "admin'; DROP TABLE users; --".to_string(),
            email: "malicious@example.com".to_string(),
            full_name: "Malicious User".to_string(),
            metadata: HashMap::new(),
        };
        
        // Should either reject or safely handle the input
        let result = user_store.create_user(&malicious_user);
        if result.is_ok() {
            // If accepted, verify it was stored safely
            let user = result.unwrap();
            let retrieved = user_store.get_user_by_id(&user.id).unwrap();
            assert_eq!(retrieved.username, "admin'; DROP TABLE users; --");
        }
        
        // Verify store is still functional
        let normal_user = UserData {
            id: None,
            username: "normaluser".to_string(),
            email: "normal@example.com".to_string(),
            full_name: "Normal User".to_string(),
            metadata: HashMap::new(),
        };
        assert!(user_store.create_user(&normal_user).is_ok(), "Store should still be functional");
    }
}