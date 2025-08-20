//! Tests for ServiceExt trait

use synapsed_serventis::{BasicService, ServiceExt};
use synapsed_substrates::types::{Name, SubjectType};
use synapsed_serventis::Subject;

#[tokio::test]
async fn test_service_dispatch() {
    let subject = Subject::new(Name::from_part("test-service"), SubjectType::Source);
    let mut service = BasicService::new(subject);
    
    // Test successful dispatch
    let result = service
        .dispatch(|| -> Result<i32, &'static str> { Ok(42) })
        .await;
    assert_eq!(result, Ok(42));
    
    // Test failed dispatch
    let result = service
        .dispatch(|| -> Result<i32, &'static str> { Err("test error") })
        .await;
    assert_eq!(result, Err("test error"));
}

#[tokio::test]
async fn test_service_execute() {
    let subject = Subject::new(Name::from_part("test-service"), SubjectType::Source);
    let mut service = BasicService::new(subject);
    
    // Test successful execution
    let result = service
        .execute(|| -> Result<String, &'static str> { Ok("success".to_string()) })
        .await;
    assert_eq!(result, Ok("success".to_string()));
    
    // Test failed execution
    let result = service
        .execute(|| -> Result<String, &'static str> { Err("execution failed") })
        .await;
    assert_eq!(result, Err("execution failed"));
}

#[tokio::test]
async fn test_service_with_handler() {
    use std::sync::{Arc, Mutex};
    use synapsed_serventis::Signal;
    
    let subject = Subject::new(Name::from_part("test-service"), SubjectType::Source);
    let signals = Arc::new(Mutex::new(Vec::new()));
    let signals_clone = signals.clone();
    
    let mut service = BasicService::with_handler(subject, move |signal: Signal| {
        signals_clone.lock().unwrap().push(signal);
    });
    
    // Execute a successful operation
    let _result = service
        .dispatch(|| -> Result<i32, &'static str> { Ok(100) })
        .await;
    
    // Check that appropriate signals were emitted
    let emitted_signals = signals.lock().unwrap();
    assert!(emitted_signals.contains(&Signal::Call));
    assert!(emitted_signals.contains(&Signal::Success));
}