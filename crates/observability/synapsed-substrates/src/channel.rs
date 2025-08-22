//! Channel implementation - proper emission flow through Pipes
//!
//! This module implements the correct Substrates pattern where:
//! - Channels are subject-based ports into conduits
//! - Emissions flow through Pipes, not directly from Subjects
//! - Channels create Pipes which handle the actual emission

use crate::circuit::{Channel, Inlet};
use crate::pipe::{Pipe, Path, Sequencer};
use crate::subject::{Substrate, Subject};
use crate::types::{Name, SubjectType, SubstratesResult, SubstratesError};
use std::sync::Arc;
use tokio::sync::mpsc;
use parking_lot::RwLock;
use crate::async_trait;

/// Basic implementation of Channel
/// A Channel is a subject-based port that creates Pipes for emission
pub struct BasicChannel<E> {
    subject: Subject,
    /// Internal sender for this channel's pipeline
    sender: mpsc::UnboundedSender<E>,
    /// Receiver is stored to prevent channel from closing
    _receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<E>>>>,
}

impl<E> BasicChannel<E> {
    /// Create a new BasicChannel
    pub fn new(name: Name) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        Self {
            subject: Subject::new(name, SubjectType::Channel),
            sender,
            _receiver: Arc::new(RwLock::new(Some(receiver))),
        }
    }
    
    /// Create a channel with a parent subject
    pub fn with_parent(name: Name, parent: Subject) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        Self {
            subject: Subject::with_parent(name, SubjectType::Channel, parent),
            sender,
            _receiver: Arc::new(RwLock::new(Some(receiver))),
        }
    }
    
    #[cfg(test)]
    pub(crate) fn sender(&self) -> &mpsc::UnboundedSender<E> {
        &self.sender
    }
}

impl<E> Substrate for BasicChannel<E> {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

impl<E> Inlet<E> for BasicChannel<E> 
where
    E: Send + Sync + 'static,
{
    fn pipe(&self) -> SubstratesResult<Arc<dyn Pipe<E>>> {
        Ok(Arc::new(ChannelPipe::new(self.sender.clone())))
    }
}

#[async_trait]
impl<E> Channel<E> for BasicChannel<E> 
where
    E: Send + Sync + 'static,
{
    /// Returns a pipe that will use this channel to emit values
    /// This is the PRIMARY way emissions happen - through Pipes, not directly
    fn pipe(&self) -> SubstratesResult<Arc<dyn Pipe<E>>> {
        // Use the Inlet implementation
        Inlet::pipe(self)
    }
    
    /// Returns a pipe with a custom sequencer
    fn pipe_with_sequencer(
        &self,
        _sequencer: Arc<dyn Sequencer<dyn Path<E>>>,
    ) -> SubstratesResult<Arc<dyn Pipe<E>>> {
        // For now, ignore sequencer and return basic pipe
        // TODO: Implement sequencer support
        Channel::pipe(self)
    }
}

/// Pipe implementation that emits through a channel
pub(crate) struct ChannelPipe<E> {
    sender: mpsc::UnboundedSender<E>,
}

impl<E> ChannelPipe<E> {
    pub(crate) fn new(sender: mpsc::UnboundedSender<E>) -> Self {
        Self { sender }
    }
}

impl<E> std::fmt::Debug for ChannelPipe<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChannelPipe").finish()
    }
}

#[async_trait]
impl<E> Pipe<E> for ChannelPipe<E> 
where
    E: Send + Sync,
{
    /// Emit a value through this pipe into the channel's pipeline
    async fn emit(&mut self, emission: E) -> SubstratesResult<()> {
        self.sender
            .send(emission)
            .map_err(|_| SubstratesError::Closed("Channel closed".to_string()))
    }
}

/// Conduit implementation that creates channels and manages their lifecycle
pub struct BasicConduit<P, E> {
    subject: Subject,
    channels: Arc<RwLock<Vec<Arc<BasicChannel<E>>>>>,
    _phantom: std::marker::PhantomData<P>,
}

impl<P, E> BasicConduit<P, E> {
    pub fn new(name: Name) -> Self {
        Self {
            subject: Subject::new(name, SubjectType::Conduit),
            channels: Arc::new(RwLock::new(Vec::new())),
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Create a new channel within this conduit
    pub fn create_channel(&self, name: Name) -> Arc<BasicChannel<E>> {
        let channel = Arc::new(BasicChannel::with_parent(
            name,
            self.subject.clone(),
        ));
        self.channels.write().push(channel.clone());
        channel
    }
}

impl<P, E> Substrate for BasicConduit<P, E> {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_channel_emits_through_pipe() {
        // Create a channel
        let channel = BasicChannel::<String>::new(Name::from_part("test-channel"));
        
        // Get a pipe from the channel using Channel trait explicitly
        let pipe = Channel::pipe(&channel).unwrap();
        
        // Create a mutable pipe for testing
        let mut test_pipe = ChannelPipe::new(channel.sender().clone());
        
        // Emit through the pipe (NOT directly on subject or channel!)
        test_pipe.emit("Hello through pipe!".to_string()).await.unwrap();
        
        // In a real implementation, a subscriber would receive this emission
        // For now, we just verify no panic
    }
    
    #[tokio::test]
    async fn test_conduit_creates_channels() {
        let conduit = BasicConduit::<(), String>::new(Name::from_part("test-conduit"));
        
        let channel1 = conduit.create_channel(Name::from_part("channel-1"));
        let channel2 = conduit.create_channel(Name::from_part("channel-2"));
        
        // Verify channels have correct parent
        assert!(channel1.subject().enclosure().is_some());
        assert!(channel2.subject().enclosure().is_some());
        
        // Verify we can get pipes from channels using Channel trait
        let _pipe1 = Channel::pipe(&*channel1).unwrap();
        let _pipe2 = Channel::pipe(&*channel2).unwrap();
    }
}