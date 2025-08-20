//! Runtime abstractions and utilities for async operations.
//!
//! This module provides common runtime abstractions and utilities that can be
//! used across all Synapsed components.

use crate::{SynapsedError, SynapsedResult};
use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Trait for async runtime operations
#[async_trait]
pub trait AsyncRuntime: Send + Sync {
    /// Spawn a task
    fn spawn<F>(&self, future: F) -> Box<dyn JoinHandle<F::Output>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static;

    /// Spawn a blocking task
    fn spawn_blocking<F, R>(&self, f: F) -> Box<dyn JoinHandle<R>>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static;

    /// Sleep for a duration
    async fn sleep(&self, duration: Duration);

    /// Set a timeout for a future
    async fn timeout<F>(&self, duration: Duration, future: F) -> SynapsedResult<F::Output>
    where
        F: Future + Send,
        F::Output: Send;
}

/// Join handle for spawned tasks
pub trait JoinHandle<T>: Send + Sync {
    /// Wait for the task to complete
    fn join(self: Box<Self>) -> Pin<Box<dyn Future<Output = SynapsedResult<T>> + Send>>;

    /// Abort the task
    fn abort(&self);

    /// Check if the task is finished
    fn is_finished(&self) -> bool;
}

/// Tokio-based async runtime implementation
#[derive(Debug, Clone)]
pub struct TokioRuntime {
    handle: tokio::runtime::Handle,
}

impl TokioRuntime {
    /// Create a new Tokio runtime wrapper
    #[must_use] pub fn new() -> Self {
        Self {
            handle: tokio::runtime::Handle::current(),
        }
    }

    /// Create with explicit handle
    #[must_use] pub fn with_handle(handle: tokio::runtime::Handle) -> Self {
        Self { handle }
    }
}

impl Default for TokioRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// Tokio join handle wrapper
pub struct TokioJoinHandle<T> {
    handle: tokio::task::JoinHandle<T>,
}

impl<T: Send + 'static> JoinHandle<T> for TokioJoinHandle<T> {
    fn join(self: Box<Self>) -> Pin<Box<dyn Future<Output = SynapsedResult<T>> + Send>> {
        Box::pin(async move {
            self.handle.await
                .map_err(|e| SynapsedError::internal(format!("Task join error: {e}")))
        })
    }

    fn abort(&self) {
        self.handle.abort();
    }

    fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }
}

#[async_trait]
impl AsyncRuntime for TokioRuntime {
    fn spawn<F>(&self, future: F) -> Box<dyn JoinHandle<F::Output>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let handle = self.handle.spawn(future);
        Box::new(TokioJoinHandle { handle })
    }

    fn spawn_blocking<F, R>(&self, f: F) -> Box<dyn JoinHandle<R>>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let handle = self.handle.spawn_blocking(f);
        Box::new(TokioJoinHandle { handle })
    }

    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }

    async fn timeout<F>(&self, duration: Duration, future: F) -> SynapsedResult<F::Output>
    where
        F: Future + Send,
        F::Output: Send,
    {
        tokio::time::timeout(duration, future)
            .await
            .map_err(|_| SynapsedError::timeout(format!("Operation timed out after {duration:?}")))
    }
}

/// Task executor trait
#[async_trait]
pub trait TaskExecutor: Send + Sync {
    /// Execute a task
    async fn execute<F, T>(&self, task: F) -> SynapsedResult<T>
    where
        F: Future<Output = SynapsedResult<T>> + Send,
        T: Send;

    /// Execute with retry logic
    async fn execute_with_retry<F, T>(&self, mut task: F, max_retries: usize) -> SynapsedResult<T>
    where
        F: FnMut() -> Pin<Box<dyn Future<Output = SynapsedResult<T>> + Send>> + Send + Sync,
        T: Send;
}

/// Simple task executor implementation
#[derive(Debug, Clone)]
pub struct SimpleTaskExecutor {
    runtime: Arc<TokioRuntime>,
}

impl SimpleTaskExecutor {
    /// Create a new simple task executor
    #[must_use] pub fn new(runtime: Arc<TokioRuntime>) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl TaskExecutor for SimpleTaskExecutor {
    async fn execute<F, T>(&self, task: F) -> SynapsedResult<T>
    where
        F: Future<Output = SynapsedResult<T>> + Send,
        T: Send,
    {
        task.await
    }

    async fn execute_with_retry<F, T>(&self, mut task: F, max_retries: usize) -> SynapsedResult<T>
    where
        F: FnMut() -> Pin<Box<dyn Future<Output = SynapsedResult<T>> + Send>> + Send + Sync,
        T: Send,
    {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts <= max_retries {
            match task().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    if !error.is_retryable() || attempts == max_retries {
                        return Err(error);
                    }
                    
                    last_error = Some(error);
                    attempts += 1;
                    
                    // Exponential backoff
                    let delay = Duration::from_millis(100 * 2_u64.pow(attempts as u32 - 1));
                    self.runtime.sleep(delay).await;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| SynapsedError::internal("Unexpected retry failure")))
    }
}

/// Cancellation token for cooperative task cancellation
#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: Arc<RwLock<bool>>,
}

impl CancellationToken {
    /// Create a new cancellation token
    #[must_use] pub fn new() -> Self {
        Self {
            cancelled: Arc::new(RwLock::new(false)),
        }
    }

    /// Cancel the token
    pub async fn cancel(&self) {
        let mut cancelled = self.cancelled.write().await;
        *cancelled = true;
    }

    /// Check if the token is cancelled
    pub async fn is_cancelled(&self) -> bool {
        *self.cancelled.read().await
    }

    /// Wait for cancellation
    pub async fn cancelled(&self) {
        loop {
            if self.is_cancelled().await {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Create a child token that cancels when this token cancels
    #[must_use] pub fn child(&self) -> Self {
        Self {
            cancelled: self.cancelled.clone(),
        }
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Task state for tracking task execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is pending
    Pending,
    /// Task is running
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
    /// Task was cancelled
    Cancelled,
}

/// Task metadata
#[derive(Debug, Clone)]
pub struct TaskMetadata {
    /// Task ID
    pub id: uuid::Uuid,
    /// Task name
    pub name: String,
    /// Current state
    pub state: TaskState,
    /// Start time
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// End time
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl TaskMetadata {
    /// Create new task metadata
    #[must_use] pub fn new(name: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name: name.to_string(),
            state: TaskState::Pending,
            started_at: None,
            ended_at: None,
            error: None,
        }
    }

    /// Mark task as started
    pub fn start(&mut self) {
        self.state = TaskState::Running;
        self.started_at = Some(chrono::Utc::now());
    }

    /// Mark task as completed
    pub fn complete(&mut self) {
        self.state = TaskState::Completed;
        self.ended_at = Some(chrono::Utc::now());
    }

    /// Mark task as failed
    pub fn fail(&mut self, error: &str) {
        self.state = TaskState::Failed;
        self.ended_at = Some(chrono::Utc::now());
        self.error = Some(error.to_string());
    }

    /// Mark task as cancelled
    pub fn cancel(&mut self) {
        self.state = TaskState::Cancelled;
        self.ended_at = Some(chrono::Utc::now());
    }

    /// Get task duration
    #[must_use] pub fn duration(&self) -> Option<chrono::Duration> {
        match (self.started_at, self.ended_at) {
            (Some(start), Some(end)) => Some(end - start),
            _ => None,
        }
    }

    /// Check if task is finished
    #[must_use] pub fn is_finished(&self) -> bool {
        matches!(
            self.state,
            TaskState::Completed | TaskState::Failed | TaskState::Cancelled
        )
    }
}

/// Async task wrapper with metadata and cancellation support
pub struct Task<T> {
    metadata: Arc<RwLock<TaskMetadata>>,
    cancellation_token: CancellationToken,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Task<T> {
    /// Create a new task
    #[must_use] pub fn new(name: &str) -> Self {
        Self {
            metadata: Arc::new(RwLock::new(TaskMetadata::new(name))),
            cancellation_token: CancellationToken::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get task metadata
    pub async fn metadata(&self) -> TaskMetadata {
        self.metadata.read().await.clone()
    }

    /// Get cancellation token
    #[must_use] pub fn cancellation_token(&self) -> &CancellationToken {
        &self.cancellation_token
    }

    /// Cancel the task
    pub async fn cancel(&self) {
        let mut metadata = self.metadata.write().await;
        metadata.cancel();
        self.cancellation_token.cancel().await;
    }

    /// Check if task is cancelled
    pub async fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled().await
    }

    /// Execute the task with the given future
    pub async fn execute<F>(self, future: F) -> SynapsedResult<T>
    where
        F: Future<Output = SynapsedResult<T>>,
    {
        // Mark as started
        {
            let mut metadata = self.metadata.write().await;
            metadata.start();
        }

        // Execute with cancellation support
        let result = tokio::select! {
            result = future => result,
            () = self.cancellation_token.cancelled() => {
                let mut metadata = self.metadata.write().await;
                metadata.cancel();
                return Err(SynapsedError::internal("Task was cancelled"));
            }
        };

        // Update metadata based on result and store it
        {
            let mut metadata = self.metadata.write().await;
            match &result {
                Ok(_) => metadata.complete(),
                Err(error) => metadata.fail(&error.to_string()),
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_tokio_runtime() {
        let runtime = TokioRuntime::new();
        
        // Test spawn  
        let handle = runtime.spawn(async { 42 });
        let result = handle.join().await.unwrap();
        assert_eq!(result, 42);
        
        // Test spawn_blocking
        let handle = runtime.spawn_blocking(|| 24);
        let result = handle.join().await.unwrap();
        assert_eq!(result, 24);
        
        // Test sleep (just ensure it doesn't panic)
        runtime.sleep(Duration::from_millis(1)).await;
        
        // Test timeout - success case
        let result = runtime.timeout(
            Duration::from_millis(100),
            async { tokio::time::sleep(Duration::from_millis(10)).await; 42 }
        ).await;
        assert_eq!(result.unwrap(), 42);
        
        // Test timeout - timeout case
        let result = runtime.timeout(
            Duration::from_millis(10),
            async { tokio::time::sleep(Duration::from_millis(100)).await; 42 }
        ).await;
        assert!(result.is_err());
    }

    #[tokio::test] 
    async fn test_cancellation_token() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled().await);
        
        token.cancel().await;
        assert!(token.is_cancelled().await);
        
        // Test child token
        let parent = CancellationToken::new();
        let child = parent.child();
        
        parent.cancel().await;
        assert!(child.is_cancelled().await);
    }

    #[tokio::test]
    async fn test_task_metadata() {
        let mut metadata = TaskMetadata::new("test_task");
        assert_eq!(metadata.name, "test_task");
        assert_eq!(metadata.state, TaskState::Pending);
        assert!(!metadata.is_finished());
        
        metadata.start();
        assert_eq!(metadata.state, TaskState::Running);
        assert!(metadata.started_at.is_some());
        
        metadata.complete();
        assert_eq!(metadata.state, TaskState::Completed);
        assert!(metadata.ended_at.is_some());
        assert!(metadata.is_finished());
        assert!(metadata.duration().is_some());
    }

    #[tokio::test]
    async fn test_task_execution() {
        let task = Task::new("test_task");
        
        let result = task.execute(async { Ok::<i32, SynapsedError>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_task_cancellation() {
        let task = Task::new("test_task");
        
        // Cancel the task before execution
        task.cancel().await;
        assert!(task.is_cancelled().await);
        
        let result = task.execute(async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok::<i32, SynapsedError>(42)
        }).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_simple_task_executor() {
        let runtime = Arc::new(TokioRuntime::new());
        let executor = SimpleTaskExecutor::new(runtime);
        
        // Test successful execution
        let result = executor.execute(async { Ok::<i32, SynapsedError>(42) }).await;
        assert_eq!(result.unwrap(), 42);
        
        // Test retry with eventual success
        use std::sync::{Arc, Mutex};
        let attempts = Arc::new(Mutex::new(0));
        let attempts_clone = attempts.clone();
        let result = executor.execute_with_retry(
            move || {
                let mut count = attempts_clone.lock().unwrap();
                *count += 1;
                let current_attempts = *count;
                Box::pin(async move {
                    if current_attempts < 3 {
                        Err(SynapsedError::network("temporary failure"))
                    } else {
                        Ok(42)
                    }
                })
            },
            5
        ).await;
        assert_eq!(result.unwrap(), 42);
        assert_eq!(*attempts.lock().unwrap(), 3);
    }
}