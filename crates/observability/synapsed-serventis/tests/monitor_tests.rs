use synapsed_serventis::*;
use synapsed_substrates::types::{Name, SubjectType};
use std::sync::{Arc, Mutex};

// Helper struct to track status emissions
#[derive(Default)]
struct StatusTracker {
    statuses: Arc<Mutex<Vec<(Condition, Confidence)>>>,
}

impl StatusTracker {
    fn new() -> Self {
        Self {
            statuses: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    fn get_statuses(&self) -> Vec<(Condition, Confidence)> {
        self.statuses.lock().unwrap().clone()
    }
    
    fn create_monitor(&self) -> BasicMonitor {
        let statuses = self.statuses.clone();
        let subject = Subject::new(Name::from_part("test-monitor"), SubjectType::Channel);
        BasicMonitor::with_handler(subject, move |condition, confidence| {
            statuses.lock().unwrap().push((condition, confidence));
        })
    }
}

#[tokio::test]
async fn test_monitor_basic_status_emission() {
    let tracker = StatusTracker::new();
    let mut monitor = tracker.create_monitor();
    
    // Test emitting a status with condition and confidence
    monitor.status(Condition::Stable, Confidence::Confirmed).await.unwrap();
    
    let statuses = tracker.get_statuses();
    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0], (Condition::Stable, Confidence::Confirmed));
}

#[tokio::test]
async fn test_monitor_all_conditions() {
    let tracker = StatusTracker::new();
    let mut monitor = tracker.create_monitor();
    
    // Test all condition types
    let conditions = vec![
        Condition::Converging,
        Condition::Stable,
        Condition::Diverging,
        Condition::Erratic,
        Condition::Degraded,
        Condition::Defective,
        Condition::Down,
    ];
    
    for condition in &conditions {
        monitor.status(*condition, Confidence::Measured).await.unwrap();
    }
    
    let statuses = tracker.get_statuses();
    assert_eq!(statuses.len(), 7);
    
    for (i, condition) in conditions.iter().enumerate() {
        assert_eq!(statuses[i].0, *condition);
        assert_eq!(statuses[i].1, Confidence::Measured);
    }
}

#[tokio::test]
async fn test_monitor_all_confidence_levels() {
    let tracker = StatusTracker::new();
    let mut monitor = tracker.create_monitor();
    
    // Test all confidence levels
    let confidence_levels = vec![
        Confidence::Tentative,
        Confidence::Measured,
        Confidence::Confirmed,
    ];
    
    for confidence in &confidence_levels {
        monitor.status(Condition::Stable, *confidence).await.unwrap();
    }
    
    let statuses = tracker.get_statuses();
    assert_eq!(statuses.len(), 3);
    
    for (i, confidence) in confidence_levels.iter().enumerate() {
        assert_eq!(statuses[i].0, Condition::Stable);
        assert_eq!(statuses[i].1, *confidence);
    }
}

#[tokio::test]
async fn test_condition_health_checks() {
    // Test is_healthy method
    assert!(Condition::Converging.is_healthy());
    assert!(Condition::Stable.is_healthy());
    assert!(!Condition::Diverging.is_healthy());
    assert!(!Condition::Erratic.is_healthy());
    assert!(!Condition::Degraded.is_healthy());
    assert!(!Condition::Defective.is_healthy());
    assert!(!Condition::Down.is_healthy());
    
    // Test is_unhealthy method
    assert!(!Condition::Converging.is_unhealthy());
    assert!(!Condition::Stable.is_unhealthy());
    assert!(!Condition::Diverging.is_unhealthy());
    assert!(!Condition::Erratic.is_unhealthy());
    assert!(Condition::Degraded.is_unhealthy());
    assert!(Condition::Defective.is_unhealthy());
    assert!(Condition::Down.is_unhealthy());
    
    // Test is_unstable method
    assert!(!Condition::Converging.is_unstable());
    assert!(!Condition::Stable.is_unstable());
    assert!(Condition::Diverging.is_unstable());
    assert!(Condition::Erratic.is_unstable());
    assert!(!Condition::Degraded.is_unstable());
    assert!(!Condition::Defective.is_unstable());
    assert!(!Condition::Down.is_unstable());
}

#[tokio::test]
async fn test_condition_severity_scores() {
    assert_eq!(Condition::Stable.severity(), 0);
    assert_eq!(Condition::Converging.severity(), 1);
    assert_eq!(Condition::Diverging.severity(), 2);
    assert_eq!(Condition::Erratic.severity(), 3);
    assert_eq!(Condition::Degraded.severity(), 4);
    assert_eq!(Condition::Defective.severity(), 5);
    assert_eq!(Condition::Down.severity(), 6);
}

#[tokio::test]
async fn test_monitor_state_transitions() {
    let tracker = StatusTracker::new();
    let mut monitor = tracker.create_monitor();
    
    // Simulate a system going from healthy to unhealthy
    monitor.status(Condition::Stable, Confidence::Confirmed).await.unwrap();
    monitor.status(Condition::Diverging, Confidence::Tentative).await.unwrap();
    monitor.status(Condition::Diverging, Confidence::Measured).await.unwrap();
    monitor.status(Condition::Erratic, Confidence::Measured).await.unwrap();
    monitor.status(Condition::Degraded, Confidence::Confirmed).await.unwrap();
    monitor.status(Condition::Defective, Confidence::Confirmed).await.unwrap();
    monitor.status(Condition::Down, Confidence::Confirmed).await.unwrap();
    
    let statuses = tracker.get_statuses();
    assert_eq!(statuses.len(), 7);
    
    // Verify the progression
    let conditions: Vec<_> = statuses.iter().map(|(c, _)| *c).collect();
    assert_eq!(conditions, vec![
        Condition::Stable,
        Condition::Diverging,
        Condition::Diverging,
        Condition::Erratic,
        Condition::Degraded,
        Condition::Defective,
        Condition::Down,
    ]);
    
    // Verify confidence progression
    assert_eq!(statuses[0].1, Confidence::Confirmed);
    assert_eq!(statuses[1].1, Confidence::Tentative);
    assert_eq!(statuses[2].1, Confidence::Measured);
}

#[tokio::test]
async fn test_monitor_recovery_sequence() {
    let tracker = StatusTracker::new();
    let mut monitor = tracker.create_monitor();
    
    // Simulate a system recovering from failure
    monitor.status(Condition::Down, Confidence::Confirmed).await.unwrap();
    monitor.status(Condition::Defective, Confidence::Measured).await.unwrap();
    monitor.status(Condition::Degraded, Confidence::Measured).await.unwrap();
    monitor.status(Condition::Converging, Confidence::Tentative).await.unwrap();
    monitor.status(Condition::Converging, Confidence::Measured).await.unwrap();
    monitor.status(Condition::Stable, Confidence::Confirmed).await.unwrap();
    
    let statuses = tracker.get_statuses();
    assert_eq!(statuses.len(), 6);
    
    // Verify severity decreases during recovery
    let severities: Vec<_> = statuses.iter()
        .map(|(c, _)| c.severity())
        .collect();
    
    // Check that severity generally decreases (recovery pattern)
    assert_eq!(severities[0], 6); // Down
    assert_eq!(severities[1], 5); // Defective
    assert_eq!(severities[2], 4); // Degraded
    assert_eq!(severities[3], 1); // Converging
    assert_eq!(severities[4], 1); // Converging
    assert_eq!(severities[5], 0); // Stable
}

#[tokio::test]
async fn test_monitor_confidence_progression() {
    let tracker = StatusTracker::new();
    let mut monitor = tracker.create_monitor();
    
    // Test typical confidence progression for a detected issue
    monitor.status(Condition::Diverging, Confidence::Tentative).await.unwrap();
    monitor.status(Condition::Diverging, Confidence::Measured).await.unwrap();
    monitor.status(Condition::Diverging, Confidence::Confirmed).await.unwrap();
    
    let statuses = tracker.get_statuses();
    assert_eq!(statuses.len(), 3);
    
    // All should have same condition but increasing confidence
    assert!(statuses.iter().all(|(c, _)| *c == Condition::Diverging));
    assert_eq!(statuses[0].1, Confidence::Tentative);
    assert_eq!(statuses[1].1, Confidence::Measured);
    assert_eq!(statuses[2].1, Confidence::Confirmed);
}

#[tokio::test]
async fn test_monitor_rapid_condition_changes() {
    let tracker = StatusTracker::new();
    let mut monitor = tracker.create_monitor();
    
    // Simulate rapid condition changes (system instability)
    for _ in 0..10 {
        monitor.status(Condition::Stable, Confidence::Tentative).await.unwrap();
        monitor.status(Condition::Diverging, Confidence::Tentative).await.unwrap();
    }
    
    let statuses = tracker.get_statuses();
    assert_eq!(statuses.len(), 20);
    
    // Count transitions
    let stable_count = statuses.iter().filter(|(c, _)| *c == Condition::Stable).count();
    let diverging_count = statuses.iter().filter(|(c, _)| *c == Condition::Diverging).count();
    
    assert_eq!(stable_count, 10);
    assert_eq!(diverging_count, 10);
}

#[tokio::test]
async fn test_basic_status_implementation() {
    let status = BasicStatus::new(Condition::Stable, Confidence::Confirmed);
    
    assert_eq!(status.condition(), Condition::Stable);
    assert_eq!(status.confidence(), Confidence::Confirmed);
    
    // Test with different combinations
    let status2 = BasicStatus::new(Condition::Down, Confidence::Tentative);
    assert_eq!(status2.condition(), Condition::Down);
    assert_eq!(status2.confidence(), Confidence::Tentative);
}

#[tokio::test]
async fn test_monitor_async_behavior() {
    use tokio::time::Duration;
    
    let tracker = StatusTracker::new();
    let mut monitor = tracker.create_monitor();
    
    // Test that status emission is async
    let start = std::time::Instant::now();
    
    // Emit multiple statuses
    for i in 0..5 {
        let condition = if i % 2 == 0 { Condition::Stable } else { Condition::Diverging };
        monitor.status(condition, Confidence::Measured).await.unwrap();
        // Small delay to simulate real monitoring
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(50));
    
    let statuses = tracker.get_statuses();
    assert_eq!(statuses.len(), 5);
}