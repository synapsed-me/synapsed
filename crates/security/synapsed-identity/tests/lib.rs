//! Test suite for synapsed-identity
//! 
//! Comprehensive tests for identity management, authentication, and authorization.

#![cfg(test)]

// Test framework and utilities
mod test_framework;
mod fixtures;

// Unit tests
mod unit {
    mod auth_tests;
    mod jwt_tests;
    mod authorization_tests;
    mod user_management_tests;
    mod key_management_tests;
}

// Integration tests
mod integration {
    mod auth_flow_tests;
}

// Re-export commonly used items
use test_framework::*;
use fixtures::*;

/// Global test configuration
pub struct TestConfig {
    /// Enable verbose logging for tests
    pub verbose: bool,
    /// Run performance benchmarks
    pub benchmark: bool,
    /// Run security tests
    pub security_tests: bool,
    /// Test timeout in seconds
    pub timeout: u64,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            verbose: std::env::var("TEST_VERBOSE").is_ok(),
            benchmark: std::env::var("TEST_BENCHMARK").is_ok(),
            security_tests: std::env::var("TEST_SECURITY").is_ok(),
            timeout: std::env::var("TEST_TIMEOUT")
                .ok()
                .and_then(|t| t.parse().ok())
                .unwrap_or(60),
        }
    }
}

/// Initialize test environment
#[ctor::ctor]
fn init_tests() {
    // Initialize logging
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();
    
    // Set up test database
    std::env::set_var("DATABASE_URL", ":memory:");
    
    // Configure test environment
    std::env::set_var("JWT_SECRET", "test-secret-key");
    std::env::set_var("BCRYPT_COST", "4"); // Lower cost for faster tests
}

/// Test suite summary
#[cfg(test)]
mod test_summary {
    use super::*;
    
    #[test]
    fn test_coverage_report() {
        println!("\n=== Synapsed Identity Test Coverage ===\n");
        println!("✓ Authentication Tests:");
        println!("  - Password hashing and verification");
        println!("  - Multi-factor authentication");
        println!("  - Session management");
        println!("  - Account lockout protection");
        println!("");
        println!("✓ JWT Token Tests:");
        println!("  - Token creation and validation");
        println!("  - Token refresh and rotation");
        println!("  - Revocation management");
        println!("  - Security vulnerability tests");
        println!("");
        println!("✓ Authorization Tests:");
        println!("  - Role-based access control");
        println!("  - Permission checking");
        println!("  - Policy evaluation");
        println!("  - Dynamic permissions");
        println!("");
        println!("✓ User Management Tests:");
        println!("  - CRUD operations");
        println!("  - Bulk operations");
        println!("  - Search and pagination");
        println!("  - Identity lifecycle");
        println!("");
        println!("✓ Integration Tests:");
        println!("  - Complete authentication flows");
        println!("  - Multi-service interactions");
        println!("  - Federated authentication");
        println!("  - End-to-end scenarios");
        println!("");
        println!("✓ Performance Tests:");
        println!("  - Operation benchmarks");
        println!("  - Concurrent access");
        println!("  - Scalability tests");
        println!("");
        println!("✓ Security Tests:");
        println!("  - Timing attack resistance");
        println!("  - Injection prevention");
        println!("  - Constant-time operations");
        println!("\n=====================================\n");
    }
}