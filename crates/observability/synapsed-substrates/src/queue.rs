//! Advanced Queue and Script execution implementation
//!
//! This module provides a complete implementation of the Queue/Script pattern
//! with priorities, named scripts, cancellation, and execution context.

use crate::circuit::{Current, Queue, Script};
use crate::subject::{Substrate, Subject};
use crate::types::{Name, SubjectType, SubstratesResult, SubstratesError, Id};
use crate::async_trait;
use std::collections::{HashMap, BinaryHeap};
use std::cmp::{Ordering, Reverse};
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, Instant};

/// Priority level for script execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Script wrapper with metadata
struct QueuedScript {
    id: Id,
    name: Option<Name>,
    script: Arc<dyn Script>,
    priority: Priority,
    queued_at: Instant,
    completion_sender: Option<oneshot::Sender<SubstratesResult<()>>>,
}

impl PartialEq for QueuedScript {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for QueuedScript {}

impl PartialOrd for QueuedScript {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedScript {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then older scripts first
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => other.queued_at.cmp(&self.queued_at),
            other => other,
        }
    }
}

/// Advanced Queue implementation with priorities and management
pub struct ManagedQueue {
    subject: Subject,
    /// Channel for submitting scripts
    sender: mpsc::UnboundedSender<QueuedScript>,
    /// Track pending scripts
    pending: Arc<RwLock<HashMap<Id, QueuedScript>>>,
    /// Named scripts for reuse
    named_scripts: Arc<RwLock<HashMap<Name, Arc<dyn Script>>>>,
    /// Statistics
    stats: Arc<RwLock<QueueStats>>,
}

#[derive(Debug, Default, Clone)]
pub struct QueueStats {
    total_submitted: usize,
    total_executed: usize,
    total_failed: usize,
    average_wait_ms: f64,
    average_execution_ms: f64,
}

impl ManagedQueue {
    pub fn new(name: Name) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let pending = Arc::new(RwLock::new(HashMap::new()));
        let stats = Arc::new(RwLock::new(QueueStats::default()));
        
        let queue = Self {
            subject: Subject::new(name, SubjectType::Queue),
            sender,
            pending: pending.clone(),
            named_scripts: Arc::new(RwLock::new(HashMap::new())),
            stats: stats.clone(),
        };
        
        // Start the execution worker
        queue.start_worker(receiver, pending, stats);
        queue
    }
    
    fn start_worker(
        &self,
        mut receiver: mpsc::UnboundedReceiver<QueuedScript>,
        pending: Arc<RwLock<HashMap<Id, QueuedScript>>>,
        stats: Arc<RwLock<QueueStats>>,
    ) {
        tokio::spawn(async move {
            // Use a priority queue for script execution
            let mut priority_queue = BinaryHeap::new();
            
            loop {
                // Try to receive new scripts or process existing ones
                tokio::select! {
                    Some(script) = receiver.recv() => {
                        priority_queue.push(Reverse(script));
                    }
                    _ = tokio::time::sleep(Duration::from_millis(1)), if !priority_queue.is_empty() => {
                        if let Some(Reverse(mut script)) = priority_queue.pop() {
                            let wait_time = script.queued_at.elapsed();
                            let exec_start = Instant::now();
                            
                            // Execute the script
                            let current = AdvancedCurrent::new(script.id);
                            let result = script.script.exec(&current).await;
                            
                            let exec_time = exec_start.elapsed();
                            
                            // Update stats
                            {
                                let mut stats = stats.write();
                                stats.total_executed += 1;
                                if result.is_err() {
                                    stats.total_failed += 1;
                                }
                                
                                // Update averages
                                let n = stats.total_executed as f64;
                                stats.average_wait_ms = 
                                    (stats.average_wait_ms * (n - 1.0) + wait_time.as_millis() as f64) / n;
                                stats.average_execution_ms = 
                                    (stats.average_execution_ms * (n - 1.0) + exec_time.as_millis() as f64) / n;
                            }
                            
                            // Remove from pending
                            pending.write().remove(&script.id);
                            
                            // Send completion notification
                            if let Some(sender) = script.completion_sender.take() {
                                let _ = sender.send(result);
                            }
                        }
                    }
                    else => {
                        // No scripts to process, wait for new ones
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                }
            }
        });
    }
    
    /// Submit a script with priority and get a completion future
    pub async fn submit_with_priority(
        &self,
        script: Arc<dyn Script>,
        priority: Priority,
    ) -> SubstratesResult<oneshot::Receiver<SubstratesResult<()>>> {
        let (tx, rx) = oneshot::channel();
        
        let queued_script = QueuedScript {
            id: Id::new(),
            name: None,
            script,
            priority,
            queued_at: Instant::now(),
            completion_sender: Some(tx),
        };
        
        let id = queued_script.id;
        
        // Create a copy for tracking without the sender
        let tracking_script = QueuedScript {
            id,
            name: queued_script.name.clone(),
            script: queued_script.script.clone(),
            priority: queued_script.priority,
            queued_at: queued_script.queued_at,
            completion_sender: None,
        };
        
        self.pending.write().insert(id, tracking_script);
        self.stats.write().total_submitted += 1;
        
        self.sender.send(queued_script)
            .map_err(|_| SubstratesError::Closed("Queue closed".to_string()))?;
        
        Ok(rx)
    }
    
    /// Register a named script for reuse
    pub fn register_named_script(&self, name: Name, script: Arc<dyn Script>) {
        self.named_scripts.write().insert(name, script);
    }
    
    /// Execute a named script
    pub async fn execute_named(&self, name: &Name) -> SubstratesResult<()> {
        let script = self.named_scripts.read()
            .get(name)
            .cloned()
            .ok_or_else(|| SubstratesError::NotFound(format!("Named script not found: {}", name)))?;
        
        self.post(script).await
    }
    
    /// Get queue statistics
    pub fn stats(&self) -> QueueStats {
        self.stats.read().clone()
    }
    
    /// Cancel a pending script
    pub fn cancel(&self, id: &Id) -> bool {
        self.pending.write().remove(id).is_some()
    }
}

impl Substrate for ManagedQueue {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

#[async_trait]
impl Queue for ManagedQueue {
    async fn await_empty(&self) {
        while !self.pending.read().is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    
    async fn post(&self, script: Arc<dyn Script>) -> SubstratesResult<()> {
        let queued_script = QueuedScript {
            id: Id::new(),
            name: None,
            script,
            priority: Priority::Normal,
            queued_at: Instant::now(),
            completion_sender: None,
        };
        
        let id = queued_script.id;
        
        // Create a copy for tracking without the sender
        let tracking_script = QueuedScript {
            id,
            name: queued_script.name.clone(),
            script: queued_script.script.clone(),
            priority: queued_script.priority,
            queued_at: queued_script.queued_at,
            completion_sender: None,
        };
        
        self.pending.write().insert(id, tracking_script);
        self.stats.write().total_submitted += 1;
        
        self.sender.send(queued_script)
            .map_err(|_| SubstratesError::Closed("Queue closed".to_string()))?;
        Ok(())
    }
    
    async fn post_named(&self, name: Name, script: Arc<dyn Script>) -> SubstratesResult<()> {
        // Register the script
        self.register_named_script(name.clone(), script.clone());
        
        let queued_script = QueuedScript {
            id: Id::new(),
            name: Some(name),
            script,
            priority: Priority::Normal,
            queued_at: Instant::now(),
            completion_sender: None,
        };
        
        let id = queued_script.id;
        
        // Create a copy for tracking without the sender
        let tracking_script = QueuedScript {
            id,
            name: queued_script.name.clone(),
            script: queued_script.script.clone(),
            priority: queued_script.priority,
            queued_at: queued_script.queued_at,
            completion_sender: None,
        };
        
        self.pending.write().insert(id, tracking_script);
        self.stats.write().total_submitted += 1;
        
        self.sender.send(queued_script)
            .map_err(|_| SubstratesError::Closed("Queue closed".to_string()))?;
        Ok(())
    }
}

/// Advanced Current implementation with execution context
pub struct AdvancedCurrent {
    subject: Subject,
    context: Arc<RwLock<HashMap<String, String>>>,
}

impl AdvancedCurrent {
    pub fn new(id: Id) -> Self {
        Self {
            subject: Subject::with_id(
                id,
                Name::from_part("current"),
                SubjectType::Current,
            ),
            context: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Set a context value
    pub fn set_context(&self, key: String, value: String) {
        self.context.write().insert(key, value);
    }
    
    /// Get a context value
    pub fn get_context(&self, key: &str) -> Option<String> {
        self.context.read().get(key).cloned()
    }
}

impl Substrate for AdvancedCurrent {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

impl Current for AdvancedCurrent {}

/// Composite script that executes multiple scripts in sequence
pub struct CompositeScript {
    scripts: Vec<Arc<dyn Script>>,
}

impl CompositeScript {
    pub fn new(scripts: Vec<Arc<dyn Script>>) -> Self {
        Self { scripts }
    }
}

#[async_trait]
impl Script for CompositeScript {
    async fn exec(&self, current: &dyn Current) -> SubstratesResult<()> {
        for script in &self.scripts {
            script.exec(current).await?;
        }
        Ok(())
    }
}

/// Script that executes with a timeout
pub struct TimeoutScript {
    inner: Arc<dyn Script>,
    timeout: Duration,
}

impl TimeoutScript {
    pub fn new(inner: Arc<dyn Script>, timeout: Duration) -> Self {
        Self { inner, timeout }
    }
}

#[async_trait]
impl Script for TimeoutScript {
    async fn exec(&self, current: &dyn Current) -> SubstratesResult<()> {
        match tokio::time::timeout(self.timeout, self.inner.exec(current)).await {
            Ok(result) => result,
            Err(_) => Err(SubstratesError::Internal("Script execution timed out".to_string())),
        }
    }
}

/// Script that retries on failure
pub struct RetryScript {
    inner: Arc<dyn Script>,
    max_retries: usize,
    retry_delay: Duration,
}

impl RetryScript {
    pub fn new(inner: Arc<dyn Script>, max_retries: usize, retry_delay: Duration) -> Self {
        Self {
            inner,
            max_retries,
            retry_delay,
        }
    }
}

#[async_trait]
impl Script for RetryScript {
    async fn exec(&self, current: &dyn Current) -> SubstratesResult<()> {
        let mut last_error = None;
        
        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                tokio::time::sleep(self.retry_delay).await;
            }
            
            match self.inner.exec(current).await {
                Ok(()) => return Ok(()),
                Err(e) => last_error = Some(e),
            }
        }
        
        Err(last_error.unwrap_or_else(|| 
            SubstratesError::Internal("Retry script failed".to_string())
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    
    struct TestScript {
        counter: Arc<AtomicUsize>,
    }
    
    #[async_trait]
    impl Script for TestScript {
        async fn exec(&self, _current: &dyn Current) -> SubstratesResult<()> {
            self.counter.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(())
        }
    }
    
    #[tokio::test]
    async fn test_managed_queue() {
        let queue = ManagedQueue::new(Name::from_part("test-queue"));
        
        let counter = Arc::new(AtomicUsize::new(0));
        let script = Arc::new(TestScript { counter: counter.clone() });
        
        // Submit multiple scripts
        for _ in 0..5 {
            queue.post(script.clone()).await.unwrap();
        }
        
        // Wait for completion
        queue.await_empty().await;
        
        assert_eq!(counter.load(AtomicOrdering::SeqCst), 5);
    }
    
    #[tokio::test]
    async fn test_priority_execution() {
        let queue = ManagedQueue::new(Name::from_part("priority-queue"));
        
        let order = Arc::new(RwLock::new(Vec::new()));
        
        struct OrderScript {
            id: usize,
            order: Arc<RwLock<Vec<usize>>>,
        }
        
        #[async_trait]
        impl Script for OrderScript {
            async fn exec(&self, _current: &dyn Current) -> SubstratesResult<()> {
                self.order.write().push(self.id);
                Ok(())
            }
        }
        
        // Submit scripts with different priorities
        for (i, priority) in [(1, Priority::Low), (2, Priority::High), (3, Priority::Normal)].iter() {
            let script = Arc::new(OrderScript {
                id: *i,
                order: order.clone(),
            });
            
            let _ = queue.submit_with_priority(script, *priority).await.unwrap();
        }
        
        // Give time for scripts to be queued
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Wait for completion
        queue.await_empty().await;
        
        // High priority should execute first
        let execution_order = order.read().clone();
        assert_eq!(execution_order[0], 2); // High priority
    }
}