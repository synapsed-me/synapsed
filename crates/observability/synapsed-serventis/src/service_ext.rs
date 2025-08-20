//! Extension trait for Service with generic methods
//! This pattern allows the core Service trait to remain object-safe while providing the full API

use crate::{async_trait, Service};

/// Extension trait providing generic methods for Service
/// This trait is automatically implemented for all types that implement Service
#[async_trait]
pub trait ServiceExt: Service {
    /// Method that emits appropriate signals for calling a function
    async fn dispatch<F, R, E>(&mut self, func: F) -> Result<R, E>
    where
        Self: Sized,
        F: FnOnce() -> Result<R, E> + Send,
        R: Send,
        E: Send,
    {
        self.call().await.ok();
        
        match func() {
            Ok(result) => {
                self.success().await.ok();
                Ok(result)
            }
            Err(error) => {
                self.fail().await.ok();
                Err(error)
            }
        }
    }
    
    /// Method that emits appropriate signals for executing a function
    async fn execute<F, R, E>(&mut self, func: F) -> Result<R, E>
    where
        Self: Sized,
        F: FnOnce() -> Result<R, E> + Send,
        R: Send,
        E: Send,
    {
        self.start().await.ok();
        
        let result = match func() {
            Ok(result) => {
                self.success().await.ok();
                Ok(result)
            }
            Err(error) => {
                self.fail().await.ok();
                Err(error)
            }
        };
        
        self.stop().await.ok();
        result
    }
}

// Automatically implement ServiceExt for all types that implement Service
impl<T> ServiceExt for T where T: Service + ?Sized {}