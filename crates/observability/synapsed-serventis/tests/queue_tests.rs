use synapsed_serventis::*;
use synapsed_substrates::SubjectType;
use std::sync::{Arc, Mutex};

// Test implementation of QueueMonitor
#[derive(Debug)]
struct TestQueueMonitor {
    subject: Subject,
    events: Arc<Mutex<Vec<(String, QueueEventType, Option<usize>)>>>,
}

impl TestQueueMonitor {
    fn new(events: Arc<Mutex<Vec<(String, QueueEventType, Option<usize>)>>>) -> Self {
        Self {
            subject: Subject::new(Name::from_part("test-queue"), SubjectType::Channel),
            events,
        }
    }
}

#[async_trait::async_trait]
impl Pipe<Box<dyn QueueEvent>> for TestQueueMonitor {
    async fn emit(&mut self, emission: Box<dyn QueueEvent>) -> synapsed_substrates::types::SubstratesResult<()> {
        self.events.lock().unwrap().push((
            emission.queue_id().to_string(),
            emission.event_type(),
            emission.queue_depth(),
        ));
        Ok(())
    }
}

impl Substrate for TestQueueMonitor {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

impl QueueMonitor for TestQueueMonitor {}

// Helper to track queue events
#[derive(Default)]
struct QueueEventTracker {
    events: Arc<Mutex<Vec<(String, QueueEventType, Option<usize>)>>>,
}

impl QueueEventTracker {
    fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    fn get_events(&self) -> Vec<(String, QueueEventType, Option<usize>)> {
        self.events.lock().unwrap().clone()
    }
    
    fn create_monitor(&self) -> TestQueueMonitor {
        TestQueueMonitor::new(self.events.clone())
    }
}

#[tokio::test]
async fn test_basic_queue_event() {
    let event = BasicQueueEvent::new("queue1".to_string(), QueueEventType::Enqueue, Some(10));
    
    assert_eq!(event.queue_id(), "queue1");
    assert_eq!(event.event_type(), QueueEventType::Enqueue);
    assert_eq!(event.queue_depth(), Some(10));
}

#[tokio::test]
async fn test_queue_monitor_enqueue_events() {
    let tracker = QueueEventTracker::new();
    let mut monitor = tracker.create_monitor();
    
    // Test enqueue events
    for i in 1..=5 {
        let event = BasicQueueEvent::new("queue1".to_string(), QueueEventType::Enqueue, Some(i));
        monitor.emit(Box::new(event)).await.unwrap();
    }
    
    let events = tracker.get_events();
    assert_eq!(events.len(), 5);
    
    // Verify queue is growing
    for i in 0..5 {
        assert_eq!(events[i].0, "queue1");
        assert_eq!(events[i].1, QueueEventType::Enqueue);
        assert_eq!(events[i].2, Some(i + 1));
    }
}

#[tokio::test]
async fn test_queue_monitor_dequeue_events() {
    let tracker = QueueEventTracker::new();
    let mut monitor = tracker.create_monitor();
    
    // Start with full queue
    let enqueue_event = BasicQueueEvent::new("queue1".to_string(), QueueEventType::Enqueue, Some(10));
    monitor.emit(Box::new(enqueue_event)).await.unwrap();
    
    // Test dequeue events
    for i in 1..=5 {
        let remaining = 10 - i;
        let event = BasicQueueEvent::new("queue1".to_string(), QueueEventType::Dequeue, Some(remaining));
        monitor.emit(Box::new(event)).await.unwrap();
    }
    
    let events = tracker.get_events();
    assert_eq!(events.len(), 6); // 1 enqueue + 5 dequeue
    
    // Verify queue is shrinking
    for i in 1..6 {
        assert_eq!(events[i].0, "queue1");
        assert_eq!(events[i].1, QueueEventType::Dequeue);
        assert_eq!(events[i].2, Some(10 - i));
    }
}

#[tokio::test]
async fn test_queue_monitor_mixed_operations() {
    let tracker = QueueEventTracker::new();
    let mut monitor = tracker.create_monitor();
    
    // Simulate mixed queue operations
    let operations = vec![
        (QueueEventType::Enqueue, Some(1)),
        (QueueEventType::Enqueue, Some(3)),
        (QueueEventType::Dequeue, Some(2)),
        (QueueEventType::Enqueue, Some(5)),
        (QueueEventType::Dequeue, Some(3)),
        (QueueEventType::Enqueue, Some(6)),
    ];
    
    for (event_type, depth) in &operations {
        let event = BasicQueueEvent::new("queue1".to_string(), *event_type, *depth);
        monitor.emit(Box::new(event)).await.unwrap();
    }
    
    let events = tracker.get_events();
    assert_eq!(events.len(), 6);
    
    // Verify all operations were recorded
    for i in 0..6 {
        assert_eq!(events[i].0, "queue1");
        assert_eq!(events[i].1, operations[i].0);
        assert_eq!(events[i].2, operations[i].1);
    }
}

#[tokio::test]
async fn test_queue_monitor_overflow() {
    let tracker = QueueEventTracker::new();
    let mut monitor = tracker.create_monitor();
    
    // Test queue overflow scenario
    let overflow_event = BasicQueueEvent::new("queue1".to_string(), QueueEventType::Overflow, Some(100));
    monitor.emit(Box::new(overflow_event)).await.unwrap();
    
    let events = tracker.get_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, "queue1");
    assert_eq!(events[0].1, QueueEventType::Overflow);
    assert_eq!(events[0].2, Some(100));
}

#[tokio::test]
async fn test_queue_monitor_empty() {
    let tracker = QueueEventTracker::new();
    let mut monitor = tracker.create_monitor();
    
    // Test empty queue
    let empty_event = BasicQueueEvent::new("queue1".to_string(), QueueEventType::Empty, None);
    monitor.emit(Box::new(empty_event)).await.unwrap();
    
    let events = tracker.get_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, "queue1");
    assert_eq!(events[0].1, QueueEventType::Empty);
    assert_eq!(events[0].2, None);
}

#[tokio::test]
async fn test_queue_event_type_states() {
    let tracker = QueueEventTracker::new();
    let mut monitor = tracker.create_monitor();
    
    // Test all queue event types
    let event_types = vec![
        (QueueEventType::Enqueue, Some(10)),
        (QueueEventType::Dequeue, Some(9)),
        (QueueEventType::Full, Some(100)),
        (QueueEventType::Empty, None),
        (QueueEventType::Overflow, Some(101)),
        (QueueEventType::Underflow, None),
    ];
    
    for (event_type, depth) in &event_types {
        let event = BasicQueueEvent::new("queue1".to_string(), *event_type, *depth);
        monitor.emit(Box::new(event)).await.unwrap();
    }
    
    let events = tracker.get_events();
    assert_eq!(events.len(), 6);
    
    // Verify all event types were recorded
    for i in 0..6 {
        assert_eq!(events[i].0, "queue1");
        assert_eq!(events[i].1, event_types[i].0);
        assert_eq!(events[i].2, event_types[i].1);
    }
}

#[tokio::test]
async fn test_queue_event_type_equality() {
    // Test QueueEventType equality
    assert_eq!(QueueEventType::Enqueue, QueueEventType::Enqueue);
    assert_ne!(QueueEventType::Enqueue, QueueEventType::Dequeue);
    assert_eq!(QueueEventType::Full, QueueEventType::Full);
    assert_ne!(QueueEventType::Empty, QueueEventType::Full);
}

#[tokio::test]
async fn test_multiple_queues() {
    let tracker = QueueEventTracker::new();
    let mut monitor = tracker.create_monitor();
    
    // Test events from multiple queues
    let queues = vec!["queue1", "queue2", "queue3"];
    
    for queue_id in &queues {
        let event = BasicQueueEvent::new(queue_id.to_string(), QueueEventType::Enqueue, Some(5));
        monitor.emit(Box::new(event)).await.unwrap();
    }
    
    let events = tracker.get_events();
    assert_eq!(events.len(), 3);
    
    // Verify each queue was tracked
    for i in 0..3 {
        assert_eq!(events[i].0, queues[i]);
        assert_eq!(events[i].1, QueueEventType::Enqueue);
        assert_eq!(events[i].2, Some(5));
    }
}

#[tokio::test]
async fn test_queue_monitor_async_behavior() {
    use tokio::time::{Duration, sleep};
    
    let tracker = QueueEventTracker::new();
    let mut monitor = tracker.create_monitor();
    
    let start = std::time::Instant::now();
    
    // Simulate async queue operations
    for i in 0..5 {
        let event = BasicQueueEvent::new("queue1".to_string(), QueueEventType::Enqueue, Some(i));
        monitor.emit(Box::new(event)).await.unwrap();
        sleep(Duration::from_millis(10)).await;
    }
    
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(50));
    
    let events = tracker.get_events();
    assert_eq!(events.len(), 5);
}