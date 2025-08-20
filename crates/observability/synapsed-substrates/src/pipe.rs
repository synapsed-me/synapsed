//! Pipe and Path abstractions - direct port of Java Substrates Pipe and Path interfaces

use crate::types::SubstratesResult;
use crate::{async_trait, Subject};
use std::fmt::Debug;
use std::sync::Arc;

/// Abstraction for passing typed values along a pipeline
/// Direct port of Java Substrates Pipe interface
#[async_trait]
pub trait Pipe<E>: Send + Sync + Debug {
    /// Method for passing a data value along a pipeline
    async fn emit(&mut self, emission: E) -> SubstratesResult<()>;
}

/// Empty pipe that ignores all emissions
pub struct EmptyPipe<E> {
    _phantom: std::marker::PhantomData<E>,
}

impl<E> Debug for EmptyPipe<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmptyPipe").finish()
    }
}

impl<E> EmptyPipe<E> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<E> Default for EmptyPipe<E> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<E> Pipe<E> for EmptyPipe<E>
where
    E: Send + Sync,
{
    async fn emit(&mut self, _emission: E) -> SubstratesResult<()> {
        Ok(())
    }
}

/// Function-based pipe implementation
pub struct FunctionPipe<E, F>
where
    F: Fn(E) -> SubstratesResult<()> + Send + Sync,
{
    func: F,
    _phantom: std::marker::PhantomData<E>,
}

impl<E, F> FunctionPipe<E, F>
where
    F: Fn(E) -> SubstratesResult<()> + Send + Sync,
{
    pub fn new(func: F) -> Self {
        Self {
            func,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<E, F> Debug for FunctionPipe<E, F>
where
    F: Fn(E) -> SubstratesResult<()> + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionPipe").finish()
    }
}

#[async_trait]
impl<E, F> Pipe<E> for FunctionPipe<E, F>
where
    E: Send + Sync,
    F: Fn(E) -> SubstratesResult<()> + Send + Sync,
{
    async fn emit(&mut self, emission: E) -> SubstratesResult<()> {
        (self.func)(emission)
    }
}

// TODO: Implement AsyncFunctionPipe properly
// The current implementation has issues with Sync constraints on the Future type
// /// Async function-based pipe implementation
// pub struct AsyncFunctionPipe<E, F, Fut>
// where
//     F: Fn(E) -> Fut + Send + Sync,
//     Fut: std::future::Future<Output = SubstratesResult<()>> + Send,
// {
//     func: F,
//     _phantom: std::marker::PhantomData<(E, Fut)>,
// }

/// Configurable processing pipeline for data transformation
/// Direct port of Java Substrates Path interface
pub trait Path<E>: Assembly + Send + Sync {
    /// Returns a new path that extends the current pipe with a differencing pipeline operation
    fn diff(self: Arc<Self>) -> Arc<dyn Path<E>>;
    
    /// Returns a new path that extends the current pipeline with a differencing operation with initial value
    fn diff_with_initial(self: Arc<Self>, initial: E) -> Arc<dyn Path<E>>;
    
    /// Returns a new path that forwards emissions to the specified pipe
    fn forward(self: Arc<Self>, pipe: Arc<dyn Pipe<E>>) -> Arc<dyn Path<E>>;
    
    // Generic method guard moved to PathExt for object-safety
    
    /// Returns a new path that limits the throughput to a maximum number of emitted values
    fn limit(self: Arc<Self>, limit: u64) -> Arc<dyn Path<E>>;
    
    // Generic method peek moved to PathExt for object-safety
    
    // Generic method reduce moved to PathExt for object-safety
    
    // Generic method replace moved to PathExt for object-safety
    
    /// Returns a new path that extends the current pipeline with a sampling operation
    fn sample_count(self: Arc<Self>, sample: u32) -> Arc<dyn Path<E>>;
    
    /// Returns a new path that extends the current pipeline with a sampling rate
    fn sample_rate(self: Arc<Self>, sample: f64) -> Arc<dyn Path<E>>;
}

/// Trait that serves a role in the assembly of a pipeline
/// Direct port of Java Substrates Assembly interface
pub trait Assembly: Send + Sync {}

/// Responsible for configuring and sequencing assembly components in a pipeline
/// Direct port of Java Substrates Sequencer interface
pub trait Sequencer<A>: Send + Sync
where
    A: Assembly,
{
    /// Applies configuration to the provided assembly
    fn apply(&self, assembly: &mut A) -> SubstratesResult<()>;
}

/// Filtering mechanism for values based on comparison criteria
/// Direct port of Java Substrates Sift interface
pub trait Sift<E>: Assembly + Send + Sync
where
    E: PartialOrd,
{
    /// Creates a sift that only passes values above the specified lower bound
    fn above(self: Arc<Self>, lower: E) -> Arc<dyn Sift<E>>;
    
    /// Creates a sift that only passes values below the specified upper bound
    fn below(self: Arc<Self>, upper: E) -> Arc<dyn Sift<E>>;
    
    /// Creates a sift that only passes values that represent a new high value
    fn high(self: Arc<Self>) -> Arc<dyn Sift<E>>;
    
    /// Creates a sift that only passes values that represent a new low value
    fn low(self: Arc<Self>) -> Arc<dyn Sift<E>>;
    
    /// Creates a sift that only passes values up to the specified maximum
    fn max(self: Arc<Self>, max: E) -> Arc<dyn Sift<E>>;
    
    /// Creates a sift that only passes values from the specified minimum
    fn min(self: Arc<Self>, min: E) -> Arc<dyn Sift<E>>;
    
    /// Creates a sift that only passes values within the specified range
    fn range(self: Arc<Self>, lower: E, upper: E) -> Arc<dyn Sift<E>>;
}

/// Interface that provides access to a pipe for emitting values
/// Direct port of Java Substrates Inlet interface
pub trait Inlet<E>: Send + Sync {
    /// Returns a pipe that this inlet holds
    fn pipe(&self) -> Arc<dyn Pipe<E>>;
}

/// Capture of an emitted value with its associated subject
/// Direct port of Java Substrates Capture interface
#[derive(Debug, Clone)]
pub struct Capture<E> {
    emission: E,
    subject: Subject,
}

impl<E> Capture<E> {
    pub fn new(emission: E, subject: Subject) -> Self {
        Self { emission, subject }
    }
    
    /// Returns the emitted value
    pub fn emission(&self) -> &E {
        &self.emission
    }
    
    /// Returns the subject that emitted the value
    pub fn subject(&self) -> &Subject {
        &self.subject
    }
    
    /// Consume and return the emission
    pub fn into_emission(self) -> E {
        self.emission
    }
}