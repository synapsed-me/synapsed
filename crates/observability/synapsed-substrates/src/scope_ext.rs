//! Extension trait for Scope with generic methods
//! This pattern allows the core Scope trait to remain object-safe while providing the full Java API

use crate::circuit::{Closure, Scope};
use crate::subject::Resource;
use crate::types::SubstratesResult;
use std::sync::Arc;

/// Extension trait providing generic methods for Scope
/// This trait is automatically implemented for all types that implement Scope
pub trait ScopeExt: Scope {
    /// Creates a closure for the specified resource within this scope
    fn closure<R>(&self, _resource: R) -> SubstratesResult<Arc<dyn Closure<Resource = R>>>
    where
        R: Resource + Send + Sync + 'static,
    {
        // Default implementation would create a closure that manages the resource lifecycle
        todo!("Default implementation for closure")
    }
    
    /// Registers a resource with this scope for lifecycle management
    fn register<R>(&self, _resource: R) -> SubstratesResult<R>
    where
        R: Resource + Send + Sync + 'static,
    {
        // Default implementation would add resource to scope's managed resources
        todo!("Default implementation for register")
    }
}

// Automatically implement ScopeExt for all types that implement Scope
impl<T: Scope + ?Sized> ScopeExt for T {}