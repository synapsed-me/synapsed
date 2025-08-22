//! Percept and Composer implementations
//!
//! Percepts are type-safe wrappers created by Composers around channels.
//! A Composer transforms a Channel<E> into a percept type P.

use crate::circuit::Channel;
use crate::pipe::Pipe;
use crate::types::SubstratesResult;
use std::sync::Arc;

/// Transforms a Channel into a percept type
/// Direct port of Java Substrates Composer interface
pub trait Composer<P, E>: Send + Sync {
    /// Composes a percept from a channel
    fn compose(&self, channel: Arc<dyn Channel<E>>) -> P;
}

/// Identity composer that returns the channel itself
pub struct IdentityComposer<E> {
    _phantom: std::marker::PhantomData<E>,
}

impl<E> IdentityComposer<E> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<E> Default for IdentityComposer<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> Composer<Arc<dyn Channel<E>>, E> for IdentityComposer<E> 
where
    E: Send + Sync + 'static,
{
    fn compose(&self, channel: Arc<dyn Channel<E>>) -> Arc<dyn Channel<E>> {
        channel
    }
}

/// Composer that creates a Pipe-based percept
pub struct PipeComposer<E> {
    _phantom: std::marker::PhantomData<E>,
}

impl<E> PipeComposer<E> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<E> Default for PipeComposer<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> Composer<Arc<dyn Pipe<E>>, E> for PipeComposer<E>
where
    E: Send + Sync + 'static,
{
    fn compose(&self, channel: Arc<dyn Channel<E>>) -> Arc<dyn Pipe<E>> {
        // Get a pipe from the channel
        Channel::pipe(&*channel).expect("Failed to get pipe from channel")
    }
}

/// Mapping composer that applies a function to the result of another composer
pub struct MappingComposer<P1, P2, E, F> {
    inner: Arc<dyn Composer<P1, E>>,
    mapper: F,
    _phantom: std::marker::PhantomData<(P1, P2, E)>,
}

impl<P1, P2, E, F> MappingComposer<P1, P2, E, F>
where
    F: Fn(P1) -> P2 + Send + Sync,
{
    pub fn new(inner: Arc<dyn Composer<P1, E>>, mapper: F) -> Self {
        Self {
            inner,
            mapper,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<P1, P2, E, F> Composer<P2, E> for MappingComposer<P1, P2, E, F>
where
    F: Fn(P1) -> P2 + Send + Sync,
    E: Send + Sync + 'static,
    P1: Send + Sync,
    P2: Send + Sync,
{
    fn compose(&self, channel: Arc<dyn Channel<E>>) -> P2 {
        let p1 = self.inner.compose(channel);
        (self.mapper)(p1)
    }
}

/// Example custom percept that wraps a channel with additional functionality
pub struct TypedPercept<E> {
    channel: Arc<dyn Channel<E>>,
    type_name: &'static str,
}

impl<E> TypedPercept<E> 
where
    E: Send + Sync + 'static,
{
    pub fn new(channel: Arc<dyn Channel<E>>, type_name: &'static str) -> Self {
        Self { channel, type_name }
    }
    
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }
    
    pub fn get_pipe(&self) -> SubstratesResult<Arc<dyn Pipe<E>>> {
        Channel::pipe(&*self.channel)
    }
}

/// Composer that creates typed percepts
pub struct TypedComposer<E> {
    type_name: &'static str,
    _phantom: std::marker::PhantomData<E>,
}

impl<E> TypedComposer<E> {
    pub fn new(type_name: &'static str) -> Self {
        Self {
            type_name,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<E> Composer<TypedPercept<E>, E> for TypedComposer<E>
where
    E: Send + Sync + 'static,
{
    fn compose(&self, channel: Arc<dyn Channel<E>>) -> TypedPercept<E> {
        TypedPercept::new(channel, self.type_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::BasicChannel;
    use crate::types::Name;
    
    #[test]
    fn test_identity_composer() {
        let channel = Arc::new(BasicChannel::<String>::new(Name::from_part("test")));
        let composer = IdentityComposer::new();
        let percept = composer.compose(channel.clone());
        
        // Identity composer returns the same channel
        assert!(Arc::ptr_eq(&percept, &(channel as Arc<dyn Channel<String>>)));
    }
    
    #[test]
    fn test_typed_composer() {
        let channel = Arc::new(BasicChannel::<i32>::new(Name::from_part("metrics")));
        let composer = TypedComposer::new("MetricsChannel");
        let percept = composer.compose(channel as Arc<dyn Channel<i32>>);
        
        assert_eq!(percept.type_name(), "MetricsChannel");
    }
    
    #[tokio::test]
    async fn test_pipe_composer() {
        let channel = Arc::new(BasicChannel::<String>::new(Name::from_part("test")));
        let composer = PipeComposer::new();
        let pipe = composer.compose(channel.clone() as Arc<dyn Channel<String>>);
        
        // Create a mutable pipe for testing
        let mut test_pipe = crate::channel::ChannelPipe::new(channel.sender().clone());
        
        // Should be able to emit through the pipe
        test_pipe.emit("test message".to_string()).await.unwrap();
    }
}