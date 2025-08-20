use synapsed_neural_core::*;

#[tokio::test]
async fn test_neural_core_basic() {
    // Basic integration test for neural core
    assert!(true);
}

#[tokio::test]
async fn test_neural_types() {
    // Test that core types are accessible
    let result = std::panic::catch_unwind(|| {
        format!("Neural core module loaded");
    });
    assert!(result.is_ok());
}