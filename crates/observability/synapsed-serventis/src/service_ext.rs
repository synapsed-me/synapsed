//! Extended Service functionality matching Java API
//!
//! This module provides additional service functionality including dispatch and execute
//! methods that wrap function execution with proper signal emission.

use crate::{async_trait, Service};
use std::future::Future;
use std::pin::Pin;
use synapsed_substrates::types::SubstratesResult;

/// Extended service trait with dispatch and execute methods
#[async_trait]
pub trait ServiceExt: Service {
    /// Dispatch a function with proper signal emission (call -> success/fail)
    /// This is the async Rust equivalent of Java's dispatch(Fn) method
    async fn dispatch<F, R>(&mut self, f: F) -> Result<R, Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnOnce() -> Result<R, Box<dyn std::error::Error + Send + Sync>> + Send,
        R: Send,
    {
        // Emit call signal
        self.call().await?;
        
        // Execute the function
        match f() {
            Ok(result) => {
                // Emit success signal
                self.success().await?;
                Ok(result)
            }
            Err(e) => {
                // Emit fail signal
                self.fail().await?;
                Err(e)
            }
        }
    }
    
    /// Dispatch an async function with proper signal emission
    async fn dispatch_async<F, R>(&mut self, f: F) -> Result<R, Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnOnce() -> Pin<Box<dyn Future<Output = Result<R, Box<dyn std::error::Error + Send + Sync>>> + Send>> + Send,
        R: Send,
    {
        // Emit call signal
        self.call().await?;
        
        // Execute the async function
        match f().await {
            Ok(result) => {
                // Emit success signal
                self.success().await?;
                Ok(result)
            }
            Err(e) => {
                // Emit fail signal
                self.fail().await?;
                Err(e)
            }
        }
    }
    
    /// Execute a function with full lifecycle signals (start -> call -> success/fail -> stop)
    /// This is the async Rust equivalent of Java's execute(Fn) method
    async fn execute<F, R>(&mut self, f: F) -> Result<R, Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnOnce() -> Result<R, Box<dyn std::error::Error + Send + Sync>> + Send,
        R: Send,
    {
        // Emit start signal
        self.start().await?;
        
        // Execute the function
        let result = match f() {
            Ok(result) => {
                // Emit success signal
                self.success().await?;
                Ok(result)
            }
            Err(e) => {
                // Emit fail signal
                self.fail().await?;
                Err(e)
            }
        };
        
        // Always emit stop signal
        self.stop().await?;
        
        result
    }
    
    /// Execute an async function with full lifecycle signals
    async fn execute_async<F, R>(&mut self, f: F) -> Result<R, Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnOnce() -> Pin<Box<dyn Future<Output = Result<R, Box<dyn std::error::Error + Send + Sync>>> + Send>> + Send,
        R: Send,
    {
        // Emit start signal
        self.start().await?;
        
        // Execute the async function
        let result = match f().await {
            Ok(result) => {
                // Emit success signal
                self.success().await?;
                Ok(result)
            }
            Err(e) => {
                // Emit fail signal
                self.fail().await?;
                Err(e)
            }
        };
        
        // Always emit stop signal
        self.stop().await?;
        
        result
    }
    
    /// Execute with retry logic - emits retry signals on failure
    async fn execute_with_retry<F, R>(&mut self, f: F, max_retries: usize) -> Result<R, Box<dyn std::error::Error + Send + Sync>>
    where
        F: Fn() -> Result<R, Box<dyn std::error::Error + Send + Sync>> + Send,
        R: Send,
    {
        self.start().await?;
        
        let mut attempt = 0;
        let result = loop {
            if attempt > 0 {
                self.retry().await?;
            }
            
            match f() {
                Ok(result) => {
                    self.success().await?;
                    break Ok(result);
                }
                Err(e) => {
                    if attempt >= max_retries {
                        self.fail().await?;
                        break Err(e);
                    }
                    attempt += 1;
                }
            }
        };
        
        self.stop().await?;
        result
    }
    
    /// Execute with timeout - emits expire signal on timeout
    async fn execute_with_timeout<F, R>(
        &mut self,
        f: F,
        timeout: std::time::Duration,
    ) -> Result<R, Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnOnce() -> Pin<Box<dyn Future<Output = Result<R, Box<dyn std::error::Error + Send + Sync>>> + Send>> + Send,
        R: Send,
    {
        self.start().await?;
        
        let result = match tokio::time::timeout(timeout, f()).await {
            Ok(Ok(result)) => {
                self.success().await?;
                Ok(result)
            }
            Ok(Err(e)) => {
                self.fail().await?;
                Err(e)
            }
            Err(_) => {
                self.expire().await?;
                Err("Operation timed out".into())
            }
        };
        
        self.stop().await?;
        result
    }
    
    /// Schedule work for later execution
    async fn schedule_work(&mut self, delay: std::time::Duration) -> SubstratesResult<()> {
        self.schedule().await?;
        tokio::time::sleep(delay).await;
        self.scheduled().await
    }
    
    /// Execute with circuit breaker pattern
    async fn execute_with_circuit_breaker<F, R>(
        &mut self,
        f: F,
        _failure_threshold: usize,
        _recovery_timeout: std::time::Duration,
    ) -> Result<R, Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnOnce() -> Result<R, Box<dyn std::error::Error + Send + Sync>> + Send,
        R: Send,
    {
        // In a real implementation, would track failure count
        // For now, simplified version
        match f() {
            Ok(result) => {
                self.success().await?;
                Ok(result)
            }
            Err(e) => {
                self.fail().await?;
                // Activate recourse strategy
                self.recourse().await?;
                Err(e)
            }
        }
    }
}

// Implement ServiceExt for all types that implement Service
impl<T: Service> ServiceExt for T {}

/// Helper for creating wrapped service functions
pub struct ServiceFunction<F> {
    func: F,
}

impl<F> ServiceFunction<F> {
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BasicService, Subject};
    use synapsed_substrates::types::{Name, SubjectType};
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    #[tokio::test]
    async fn test_dispatch_success() {
        let subject = Subject::new(Name::from_part("test-service"), SubjectType::Source);
        let mut service = BasicService::new(subject);
        
        let result = service.dispatch(|| {
            Ok::<i32, Box<dyn std::error::Error + Send + Sync>>(42)
        }).await.unwrap();
        
        assert_eq!(result, 42);
    }
    
    #[tokio::test]
    async fn test_dispatch_failure() {
        let subject = Subject::new(Name::from_part("test-service"), SubjectType::Source);
        let mut service = BasicService::new(subject);
        
        let result = service.dispatch(|| {
            Err::<i32, Box<dyn std::error::Error + Send + Sync>>("error".into())
        }).await;
        
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_execute_with_retry() {
        let subject = Subject::new(Name::from_part("test-service"), SubjectType::Source);
        let mut service = BasicService::new(subject);
        
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        let result = service.execute_with_retry(move || {
            let count = counter_clone.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                Err("retry me".into())
            } else {
                Ok(42)
            }
        }, 3).await.unwrap();
        
        assert_eq!(result, 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
    
    #[tokio::test]
    async fn test_execute_with_timeout() {
        let subject = Subject::new(Name::from_part("test-service"), SubjectType::Source);
        let mut service = BasicService::new(subject);
        
        // Should succeed
        let result = service.execute_with_timeout(
            || Box::pin(async {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                Ok(42)
            }),
            std::time::Duration::from_millis(100),
        ).await.unwrap();
        
        assert_eq!(result, 42);
        
        // Should timeout
        let subject = Subject::new(Name::from_part("test-service-2"), SubjectType::Source);
        let mut service = BasicService::new(subject);
        
        let result = service.execute_with_timeout(
            || Box::pin(async {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                Ok(42)
            }),
            std::time::Duration::from_millis(50),
        ).await;
        
        assert!(result.is_err());
    }
}
