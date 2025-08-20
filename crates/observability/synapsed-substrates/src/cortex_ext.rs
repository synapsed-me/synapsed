//! Extension trait for Cortex with generic methods
//! This pattern allows the core Cortex trait to remain object-safe while providing the full Java API

use crate::circuit::{Pool, Sink};
use crate::cortex::Cortex;
use crate::pipe::Capture;
use crate::source::Source;
use crate::subject::Context;
use crate::types::{Name, Slot, SubstratesResult};
use crate::Subject;
use std::sync::Arc;

/// Extension trait providing generic methods for Cortex
/// This trait is automatically implemented for all types that implement Cortex
pub trait CortexExt: crate::Cortex {
    /// Creates a capture of an emitted value with its associated subject
    fn capture<E>(&self, subject: Subject, emission: E) -> Capture<E> {
        Capture::new(emission, subject)
    }
    
    /// Returns a name from iterating over string values
    fn name_from_iter<I>(&self, it: I) -> Name
    where
        I: IntoIterator<Item = String>,
    {
        let parts: Vec<String> = it.into_iter().collect();
        Name::from_parts(parts)
    }
    
    /// Returns a name from iterating over values mapped to strings
    fn name_from_iter_mapped<I, T, F>(&self, it: I, mapper: F) -> Name
    where
        I: IntoIterator<Item = T>,
        F: Fn(T) -> String,
    {
        let parts: Vec<String> = it.into_iter().map(mapper).collect();
        Name::from_parts(parts)
    }
    
    /// Creates a name from a type
    fn name_from_type<T>(&self) -> Name
    where
        T: 'static,
    {
        Name::from_part(std::any::type_name::<T>())
    }
    
    /// Creates a pool that always returns the same singleton instance
    fn pool_singleton<T>(&self, singleton: T) -> Arc<dyn Pool<T>>
    where
        T: Clone + Send + Sync + 'static,
    {
        Arc::new(SingletonPool::new(singleton))
    }
    
    /// Creates a Sink instance for the given context's source
    fn sink_from_context<E, C>(&self, _context: &C) -> SubstratesResult<Arc<dyn Sink<E>>>
    where
        E: Send + Sync + 'static,
        C: Context<Emission = E>,
    {
        todo!("Implement sink from context")
    }
    
    /// Creates a Sink object from the given Source object
    fn sink_from_source<E>(&self, _source: Arc<dyn Source<E>>) -> SubstratesResult<Arc<dyn Sink<E>>>
    where
        E: Send + Sync + 'static,
    {
        // Implementation would create a sink that subscribes to the source
        todo!("Implement sink from source")
    }
    
    /// Creates a slot with a generic value
    fn slot<T>(&self, name: Name, value: T) -> Slot<T>
    where
        T: Send + Sync + 'static,
    {
        Slot::new(name, value)
    }
    
    /// Creates a subscriber with a function
    fn subscriber_with_function<E, F>(
        &self,
        _substrate: Subject,
        _function: F,
    ) -> Arc<dyn crate::subject::DynSubscriber>
    where
        E: Send + Sync + 'static,
        F: Fn(&Subject, &mut dyn std::any::Any) + Send + Sync + 'static,
    {
        // This would create a type-erased subscriber
        todo!("Implement function subscriber")
    }
    
    /// Creates a subscriber with a pool
    fn subscriber_with_pool<E>(
        &self,
        _substrate: Subject,
        _pool: Arc<dyn Pool<dyn crate::pipe::Pipe<E>>>,
    ) -> Arc<dyn crate::subject::DynSubscriber>
    where
        E: Send + Sync + 'static,
    {
        // Implementation would create a subscriber that uses the pool
        todo!("Implement subscriber with pool")
    }
}

// Automatically implement CortexExt for all types that implement Cortex
impl<T: Cortex + ?Sized> CortexExt for T {}

// Helper struct for singleton pools
struct SingletonPool<T> {
    value: T,
}

impl<T: Clone> SingletonPool<T> {
    fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T: Clone + Send + Sync> Pool<T> for SingletonPool<T> {
    fn get(&self, _name: &Name) -> SubstratesResult<T> {
        Ok(self.value.clone())
    }
}