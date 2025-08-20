use synapsed_serventis::*;
use synapsed_substrates::types::{Name, SubjectType};
use std::sync::{Arc, Mutex};

// Helper struct to track signal emissions
#[derive(Default)]
struct SignalTracker {
    signals: Arc<Mutex<Vec<Signal>>>,
}

impl SignalTracker {
    fn new() -> Self {
        Self {
            signals: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    fn get_signals(&self) -> Vec<Signal> {
        self.signals.lock().unwrap().clone()
    }
    
    fn create_service(&self) -> BasicService {
        let signals = self.signals.clone();
        let subject = Subject::new(Name::from_part("test"), SubjectType::Source);
        BasicService::with_handler(subject, move |signal| {
            signals.lock().unwrap().push(signal);
        })
    }
}

#[tokio::test]
async fn test_service_start_stop_signals() {
    let tracker = SignalTracker::new();
    let mut service = tracker.create_service();
    
    // Test start signal
    service.start().await.unwrap();
    
    // Test stop signal
    service.stop().await.unwrap();
    
    let signals = tracker.get_signals();
    assert_eq!(signals.len(), 2);
    assert_eq!(signals[0], Signal::Start);
    assert_eq!(signals[1], Signal::Stop);
}

#[tokio::test]
async fn test_service_call_success_fail_signals() {
    let tracker = SignalTracker::new();
    let mut service = tracker.create_service();
    
    // Test call signal
    service.call().await.unwrap();
    
    // Test success signal
    service.success().await.unwrap();
    
    // Test fail signal
    service.fail().await.unwrap();
    
    let signals = tracker.get_signals();
    assert_eq!(signals.len(), 3);
    assert_eq!(signals[0], Signal::Call);
    assert_eq!(signals[1], Signal::Success);
    assert_eq!(signals[2], Signal::Fail);
}

#[tokio::test]
async fn test_service_orientation_release_vs_receipt() {
    let tracker = SignalTracker::new();
    let mut service = tracker.create_service();
    
    // Test release signals (actions taken by the service)
    service.start().await.unwrap();     // Release
    service.call().await.unwrap();      // Release
    service.success().await.unwrap();   // Release
    service.stop().await.unwrap();      // Release
    
    // Test receipt signals (acknowledgments)
    service.started().await.unwrap();   // Receipt
    service.called().await.unwrap();    // Receipt
    service.succeeded().await.unwrap(); // Receipt
    service.stopped().await.unwrap();   // Receipt
    
    let signals = tracker.get_signals();
    assert_eq!(signals.len(), 8);
    
    // Verify release signals
    assert_eq!(signals[0], Signal::Start);
    assert_eq!(signals[1], Signal::Call);
    assert_eq!(signals[2], Signal::Success);
    assert_eq!(signals[3], Signal::Stop);
    
    // Verify receipt signals
    assert_eq!(signals[4], Signal::Started);
    assert_eq!(signals[5], Signal::Called);
    assert_eq!(signals[6], Signal::Succeeded);
    assert_eq!(signals[7], Signal::Stopped);
}

#[tokio::test]
async fn test_service_all_signal_types() {
    let tracker = SignalTracker::new();
    let mut service = tracker.create_service();
    
    // Test all signal pairs in order
    service.start().await.unwrap();
    service.started().await.unwrap();
    service.stop().await.unwrap();
    service.stopped().await.unwrap();
    service.call().await.unwrap();
    service.called().await.unwrap();
    service.success().await.unwrap();
    service.succeeded().await.unwrap();
    service.fail().await.unwrap();
    service.failed().await.unwrap();
    service.recourse().await.unwrap();
    service.recoursed().await.unwrap();
    service.redirect().await.unwrap();
    service.redirected().await.unwrap();
    service.expire().await.unwrap();
    service.expired().await.unwrap();
    service.retry().await.unwrap();
    service.retried().await.unwrap();
    service.reject().await.unwrap();
    service.rejected().await.unwrap();
    service.discard().await.unwrap();
    service.discarded().await.unwrap();
    service.delay().await.unwrap();
    service.delayed().await.unwrap();
    service.schedule().await.unwrap();
    service.scheduled().await.unwrap();
    service.resume().await.unwrap();
    service.resumed().await.unwrap();
    service.disconnect().await.unwrap();
    service.disconnected().await.unwrap();
    
    let signals = tracker.get_signals();
    assert_eq!(signals.len(), 30); // 15 pairs of signals
    
    // Verify all signals were emitted in order
    let expected_signals = vec![
        Signal::Start, Signal::Started,
        Signal::Stop, Signal::Stopped,
        Signal::Call, Signal::Called,
        Signal::Success, Signal::Succeeded,
        Signal::Fail, Signal::Failed,
        Signal::Recourse, Signal::Recoursed,
        Signal::Redirect, Signal::Redirected,
        Signal::Expire, Signal::Expired,
        Signal::Retry, Signal::Retried,
        Signal::Reject, Signal::Rejected,
        Signal::Discard, Signal::Discarded,
        Signal::Delay, Signal::Delayed,
        Signal::Schedule, Signal::Scheduled,
        Signal::Resume, Signal::Resumed,
        Signal::Disconnect, Signal::Disconnected,
    ];
    
    assert_eq!(signals, expected_signals);
}

#[tokio::test]
async fn test_service_dispatch_method() {
    let tracker = SignalTracker::new();
    let mut service = tracker.create_service();
    
    // Test successful dispatch
    let result: Result<i32, &str> = service.dispatch(|| Ok(42)).await;
    assert_eq!(result.unwrap(), 42);
    
    let signals = tracker.get_signals();
    assert_eq!(signals.len(), 2);
    assert_eq!(signals[0], Signal::Call);
    assert_eq!(signals[1], Signal::Success);
}

#[tokio::test]
async fn test_service_dispatch_method_error() {
    let tracker = SignalTracker::new();
    let mut service = tracker.create_service();
    
    // Test failed dispatch
    let result: Result<i32, &str> = service.dispatch(|| Err("error")).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "error");
    
    let signals = tracker.get_signals();
    assert_eq!(signals.len(), 2);
    assert_eq!(signals[0], Signal::Call);
    assert_eq!(signals[1], Signal::Fail);
}

#[tokio::test]
async fn test_service_execute_method() {
    let tracker = SignalTracker::new();
    let mut service = tracker.create_service();
    
    // Test successful execution
    let result: Result<String, &str> = service.execute(|| Ok("success".to_string())).await;
    assert_eq!(result.unwrap(), "success");
    
    let signals = tracker.get_signals();
    assert_eq!(signals.len(), 3);
    assert_eq!(signals[0], Signal::Start);
    assert_eq!(signals[1], Signal::Success);
    assert_eq!(signals[2], Signal::Stop);
}

#[tokio::test]
async fn test_service_execute_method_error() {
    let tracker = SignalTracker::new();
    let mut service = tracker.create_service();
    
    // Test failed execution
    let result: Result<i32, String> = service.execute(|| Err("failed".to_string())).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "failed");
    
    let signals = tracker.get_signals();
    assert_eq!(signals.len(), 3);
    assert_eq!(signals[0], Signal::Start);
    assert_eq!(signals[1], Signal::Fail);
    assert_eq!(signals[2], Signal::Stop);
}

#[tokio::test]
async fn test_service_complex_workflow() {
    let tracker = SignalTracker::new();
    let mut service = tracker.create_service();
    
    // Simulate a complex service workflow
    service.schedule().await.unwrap();
    service.scheduled().await.unwrap();
    service.start().await.unwrap();
    service.started().await.unwrap();
    
    // First attempt fails
    service.call().await.unwrap();
    service.called().await.unwrap();
    service.fail().await.unwrap();
    service.failed().await.unwrap();
    
    // Retry
    service.retry().await.unwrap();
    service.retried().await.unwrap();
    service.call().await.unwrap();
    service.called().await.unwrap();
    
    // Success this time
    service.success().await.unwrap();
    service.succeeded().await.unwrap();
    service.stop().await.unwrap();
    service.stopped().await.unwrap();
    
    let signals = tracker.get_signals();
    assert_eq!(signals.len(), 16);
    
    // Verify the workflow sequence
    let expected = vec![
        Signal::Schedule, Signal::Scheduled,
        Signal::Start, Signal::Started,
        Signal::Call, Signal::Called,
        Signal::Fail, Signal::Failed,
        Signal::Retry, Signal::Retried,
        Signal::Call, Signal::Called,
        Signal::Success, Signal::Succeeded,
        Signal::Stop, Signal::Stopped,
    ];
    
    assert_eq!(signals, expected);
}

#[tokio::test]
async fn test_service_async_behavior() {
    use tokio::time::Duration;
    
    let tracker = SignalTracker::new();
    let mut service = tracker.create_service();
    
    // Test that dispatch is truly async
    let start = std::time::Instant::now();
    let result: Result<(), ()> = service.dispatch(|| {
        std::thread::sleep(Duration::from_millis(100));
        Ok(())
    }).await;
    let elapsed = start.elapsed();
    
    assert!(result.is_ok());
    assert!(elapsed >= Duration::from_millis(100));
    
    let signals = tracker.get_signals();
    assert_eq!(signals.len(), 2);
    assert_eq!(signals[0], Signal::Call);
    assert_eq!(signals[1], Signal::Success);
}

#[tokio::test]
async fn test_service_multiple_signals() {
    let tracker = SignalTracker::new();
    let mut service = tracker.create_service();
    
    // Test multiple signal emissions in sequence
    for i in 0..5 {
        match i % 3 {
            0 => service.call().await.unwrap(),
            1 => service.success().await.unwrap(),
            _ => service.fail().await.unwrap(),
        }
    }
    
    let signals = tracker.get_signals();
    assert_eq!(signals.len(), 5);
    
    // Count signal types
    let call_count = signals.iter().filter(|s| **s == Signal::Call).count();
    let success_count = signals.iter().filter(|s| **s == Signal::Success).count();
    let fail_count = signals.iter().filter(|s| **s == Signal::Fail).count();
    
    assert_eq!(call_count, 2); // i=0, i=3
    assert_eq!(success_count, 2); // i=1, i=4
    assert_eq!(fail_count, 1); // i=2
}