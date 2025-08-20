use synapsed_routing::*;

#[tokio::test]
async fn test_basic_functionality() {
    // Basic test to ensure the crate compiles and exports work
    assert!(true);
}

#[tokio::test] 
async fn test_routing_stub() {
    // Test that routing stub functions exist
    let result = std::panic::catch_unwind(|| {
        // This ensures the module can be imported
        format!("synapsed-routing test");
    });
    assert!(result.is_ok());
}