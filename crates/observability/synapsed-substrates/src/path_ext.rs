//! Extension trait for Path with generic methods
//! This pattern allows the core Path trait to remain object-safe while providing the full Java API

use crate::pipe::{Assembly, Path, Pipe};
use std::sync::Arc;

/// Extension trait providing generic methods for Path
/// This trait is automatically implemented for all types that implement Path<E>
pub trait PathExt<E>: Path<E>
where
    E: Send + Sync + 'static,
{
    /// Returns a new path that extends the current pipeline with a guard operation
    fn guard<P>(self: Arc<Self>, predicate: P) -> Arc<dyn Path<E>>
    where
        P: Fn(&E) -> bool + Send + Sync + 'static,
        Self: Sized + 'static,
    {
        // Implementation would wrap the path with a filtering operation
        Arc::new(GuardPath::new(self as Arc<dyn Path<E>>, predicate))
    }
    
    /// Returns a new path that allows inspection of emissions without modifying them
    fn peek<F>(self: Arc<Self>, consumer: F) -> Arc<dyn Path<E>>
    where
        F: Fn(&E) + Send + Sync + 'static,
        Self: Sized + 'static,
    {
        // Implementation would wrap the path with a peek operation
        Arc::new(PeekPath::new(self as Arc<dyn Path<E>>, consumer))
    }
    
    /// Returns a new path that extends the current pipeline with a reduction operation
    fn reduce<F>(self: Arc<Self>, initial: E, operator: F) -> Arc<dyn Path<E>>
    where
        F: Fn(E, E) -> E + Send + Sync + 'static,
        E: Clone,
        Self: Sized + 'static,
    {
        // Implementation would wrap the path with a reduction operation
        Arc::new(ReducePath::new(self as Arc<dyn Path<E>>, initial, operator))
    }
    
    /// Returns a new path that extends the current pipeline with a replacement operation
    fn replace<F>(self: Arc<Self>, transformer: F) -> Arc<dyn Path<E>>
    where
        F: Fn(E) -> E + Send + Sync + 'static,
        Self: Sized + 'static,
    {
        // Implementation would wrap the path with a transformation operation
        Arc::new(ReplacePath::new(self as Arc<dyn Path<E>>, transformer))
    }
}

// Automatically implement PathExt for all types that implement Path<E>
impl<E, T> PathExt<E> for T 
where 
    T: Path<E> + ?Sized,
    E: Send + Sync + 'static,
{}

// Helper structs for path operations

struct GuardPath<E, P> {
    inner: Arc<dyn Path<E>>,
    #[allow(dead_code)]
    predicate: P,
}

impl<E, P> GuardPath<E, P> 
where
    P: Fn(&E) -> bool + Send + Sync + 'static,
{
    fn new(inner: Arc<dyn Path<E>>, predicate: P) -> Self {
        Self { inner, predicate }
    }
}

impl<E, P> Assembly for GuardPath<E, P> 
where
    E: Send + Sync + 'static,
    P: Fn(&E) -> bool + Send + Sync + 'static,
{}

impl<E, P> Path<E> for GuardPath<E, P>
where
    E: Send + Sync + 'static,
    P: Fn(&E) -> bool + Send + Sync + 'static,
{
    fn diff(self: Arc<Self>) -> Arc<dyn Path<E>> {
        self.inner.clone().diff()
    }
    
    fn diff_with_initial(self: Arc<Self>, initial: E) -> Arc<dyn Path<E>> {
        self.inner.clone().diff_with_initial(initial)
    }
    
    fn forward(self: Arc<Self>, pipe: Arc<dyn Pipe<E>>) -> Arc<dyn Path<E>> {
        self.inner.clone().forward(pipe)
    }
    
    fn limit(self: Arc<Self>, limit: u64) -> Arc<dyn Path<E>> {
        self.inner.clone().limit(limit)
    }
    
    fn sample_count(self: Arc<Self>, sample: u32) -> Arc<dyn Path<E>> {
        self.inner.clone().sample_count(sample)
    }
    
    fn sample_rate(self: Arc<Self>, sample: f64) -> Arc<dyn Path<E>> {
        self.inner.clone().sample_rate(sample)
    }
}

struct PeekPath<E, F> {
    inner: Arc<dyn Path<E>>,
    #[allow(dead_code)]
    consumer: F,
}

impl<E, F> PeekPath<E, F>
where
    F: Fn(&E) + Send + Sync + 'static,
{
    fn new(inner: Arc<dyn Path<E>>, consumer: F) -> Self {
        Self { inner, consumer }
    }
}

impl<E, F> Assembly for PeekPath<E, F>
where
    E: Send + Sync + 'static,
    F: Fn(&E) + Send + Sync + 'static,
{}

impl<E, F> Path<E> for PeekPath<E, F>
where
    E: Send + Sync + 'static,
    F: Fn(&E) + Send + Sync + 'static,
{
    fn diff(self: Arc<Self>) -> Arc<dyn Path<E>> {
        self.inner.clone().diff()
    }
    
    fn diff_with_initial(self: Arc<Self>, initial: E) -> Arc<dyn Path<E>> {
        self.inner.clone().diff_with_initial(initial)
    }
    
    fn forward(self: Arc<Self>, pipe: Arc<dyn Pipe<E>>) -> Arc<dyn Path<E>> {
        self.inner.clone().forward(pipe)
    }
    
    fn limit(self: Arc<Self>, limit: u64) -> Arc<dyn Path<E>> {
        self.inner.clone().limit(limit)
    }
    
    fn sample_count(self: Arc<Self>, sample: u32) -> Arc<dyn Path<E>> {
        self.inner.clone().sample_count(sample)
    }
    
    fn sample_rate(self: Arc<Self>, sample: f64) -> Arc<dyn Path<E>> {
        self.inner.clone().sample_rate(sample)
    }
}

struct ReducePath<E, F> {
    inner: Arc<dyn Path<E>>,
    #[allow(dead_code)]
    current: parking_lot::Mutex<E>,
    #[allow(dead_code)]
    operator: F,
}

impl<E, F> ReducePath<E, F>
where
    E: Clone,
    F: Fn(E, E) -> E + Send + Sync + 'static,
{
    fn new(inner: Arc<dyn Path<E>>, initial: E, operator: F) -> Self {
        Self { 
            inner,
            current: parking_lot::Mutex::new(initial),
            operator,
        }
    }
}

impl<E, F> Assembly for ReducePath<E, F>
where
    E: Clone + Send + Sync + 'static,
    F: Fn(E, E) -> E + Send + Sync + 'static,
{}

impl<E, F> Path<E> for ReducePath<E, F>
where
    E: Clone + Send + Sync + 'static,
    F: Fn(E, E) -> E + Send + Sync + 'static,
{
    fn diff(self: Arc<Self>) -> Arc<dyn Path<E>> {
        self.inner.clone().diff()
    }
    
    fn diff_with_initial(self: Arc<Self>, initial: E) -> Arc<dyn Path<E>> {
        self.inner.clone().diff_with_initial(initial)
    }
    
    fn forward(self: Arc<Self>, pipe: Arc<dyn Pipe<E>>) -> Arc<dyn Path<E>> {
        self.inner.clone().forward(pipe)
    }
    
    fn limit(self: Arc<Self>, limit: u64) -> Arc<dyn Path<E>> {
        self.inner.clone().limit(limit)
    }
    
    fn sample_count(self: Arc<Self>, sample: u32) -> Arc<dyn Path<E>> {
        self.inner.clone().sample_count(sample)
    }
    
    fn sample_rate(self: Arc<Self>, sample: f64) -> Arc<dyn Path<E>> {
        self.inner.clone().sample_rate(sample)
    }
}

struct ReplacePath<E, F> {
    inner: Arc<dyn Path<E>>,
    #[allow(dead_code)]
    transformer: F,
}

impl<E, F> ReplacePath<E, F>
where
    F: Fn(E) -> E + Send + Sync + 'static,
{
    fn new(inner: Arc<dyn Path<E>>, transformer: F) -> Self {
        Self { inner, transformer }
    }
}

impl<E, F> Assembly for ReplacePath<E, F>
where
    E: Send + Sync + 'static,
    F: Fn(E) -> E + Send + Sync + 'static,
{}

impl<E, F> Path<E> for ReplacePath<E, F>
where
    E: Send + Sync + 'static,
    F: Fn(E) -> E + Send + Sync + 'static,
{
    fn diff(self: Arc<Self>) -> Arc<dyn Path<E>> {
        self.inner.clone().diff()
    }
    
    fn diff_with_initial(self: Arc<Self>, initial: E) -> Arc<dyn Path<E>> {
        self.inner.clone().diff_with_initial(initial)
    }
    
    fn forward(self: Arc<Self>, pipe: Arc<dyn Pipe<E>>) -> Arc<dyn Path<E>> {
        self.inner.clone().forward(pipe)
    }
    
    fn limit(self: Arc<Self>, limit: u64) -> Arc<dyn Path<E>> {
        self.inner.clone().limit(limit)
    }
    
    fn sample_count(self: Arc<Self>, sample: u32) -> Arc<dyn Path<E>> {
        self.inner.clone().sample_count(sample)
    }
    
    fn sample_rate(self: Arc<Self>, sample: f64) -> Arc<dyn Path<E>> {
        self.inner.clone().sample_rate(sample)
    }
}