//! Circuit, Conduit, and Channel implementations - core of Substrates API

use crate::pipe::{Pipe, Path, Sequencer};
use crate::subject::{Component, Resource, Substrate};
use crate::types::{Name, State, SubjectType, SubstratesError, SubstratesResult};
use crate::{async_trait, Subject};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Computational network of conduits, containers, clocks, channels, and pipes
/// Direct port of Java Substrates Circuit interface
#[async_trait]
pub trait Circuit: Substrate + Resource + Send + Sync {
    /// Returns a clock that will use this circuit to emit clock cycle events
    async fn clock(&self) -> SubstratesResult<Arc<dyn Clock>>;
    
    /// Returns a named clock
    async fn clock_named(&self, name: Name) -> SubstratesResult<Arc<dyn Clock>>;
    
    // Generic methods moved to CircuitExt trait for object-safety
    /// Returns a Queue that can be used to coordinate execution
    fn queue(&self) -> Arc<dyn Queue>;
}

/// Component that emits clock ticks
/// Direct port of Java Substrates Clock interface
#[async_trait]
pub trait Clock: Substrate + Resource + Send + Sync {
    /// Subscribe a pipe to events of a particular cycle
    async fn consume(
        &self,
        name: Name,
        cycle: ClockCycle,
        pipe: Arc<dyn Pipe<DateTime<Utc>>>,
    ) -> SubstratesResult<Arc<dyn crate::subject::Subscription>>;
}

/// Clock cycle enumeration
/// Direct port of Java Substrates Clock.Cycle enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClockCycle {
    /// Emitted on every millisecond passing
    Millisecond,
    /// Emitted on every second passing  
    Second,
    /// Emitted on every minute passing
    Minute,
}

impl ClockCycle {
    /// Returns the number of time units this cycle represents
    pub fn units(&self) -> u64 {
        match self {
            ClockCycle::Millisecond => 1,
            ClockCycle::Second => 1000,
            ClockCycle::Minute => 1000 * 60,
        }
    }
}

/// Composer that forms percepts around a channel
/// Direct port of Java Substrates Composer interface
pub trait Composer<P, E>: Send + Sync {
    /// Composes a channel into a percept
    fn compose(&self, channel: Arc<dyn Channel<E>>) -> SubstratesResult<P>;
}

/// Identity composer implementation
#[derive(Debug)]
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
    fn compose(&self, channel: Arc<dyn Channel<E>>) -> SubstratesResult<Arc<dyn Channel<E>>> {
        Ok(channel)
    }
}

/// Pipe composer implementation
#[derive(Debug)]
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
    fn compose(&self, channel: Arc<dyn Channel<E>>) -> SubstratesResult<Arc<dyn Pipe<E>>> {
        Channel::pipe(&*channel)
    }
}

/// Creates percepts that emit captured data into pipes
/// Direct port of Java Substrates Conduit interface
pub trait Conduit<P, E>: Container<P, E> {}

/// A (subject) named pipe managed by a conduit
/// Direct port of Java Substrates Channel interface
#[async_trait]
pub trait Channel<E>: Substrate + Inlet<E> {
    /// Returns a pipe that will use this channel to emit values
    fn pipe(&self) -> SubstratesResult<Arc<dyn Pipe<E>>>;
    
    /// Returns a pipe with sequencer
    fn pipe_with_sequencer(
        &self,
        sequencer: Arc<dyn Sequencer<dyn Path<E>>>,
    ) -> SubstratesResult<Arc<dyn Pipe<E>>>;
}

/// Interface that provides access to a pipe for emitting values
/// Direct port of Java Substrates Inlet interface
pub trait Inlet<E>: Send + Sync {
    /// Returns a pipe that this inlet holds
    fn pipe(&self) -> SubstratesResult<Arc<dyn Pipe<E>>>;
}

/// Creates and manages an instance pool and notifies of events
/// Direct port of Java Substrates Container interface
pub trait Container<P, E>: Pool<P> + Component<Emission = E> {}

/// Manages instances of a pooled type by name
/// Direct port of Java Substrates Pool interface
pub trait Pool<T>: Send + Sync {
    /// Returns an instance of the pooled type for a given substrate
    fn get_by_substrate(&self, substrate: &dyn Substrate) -> SubstratesResult<T> {
        self.get_by_subject(substrate.subject())
    }
    
    /// Returns an instance of the pooled type for a given subject
    fn get_by_subject(&self, subject: &Subject) -> SubstratesResult<T> {
        self.get(subject.name())
    }
    
    /// Returns an instance of the pooled type for a given name
    fn get(&self, name: &Name) -> SubstratesResult<T>;
}

/// Interface used to coordinate the processing of queued events
/// Direct port of Java Substrates Queue interface
#[async_trait]
pub trait Queue: Send + Sync {
    /// Suspends the current thread of execution until the queue is empty
    async fn await_empty(&self);
    
    /// Posts a Script to the queue
    async fn post(&self, script: Arc<dyn Script>) -> SubstratesResult<()>;
    
    /// Posts a named Script to the queue
    async fn post_named(&self, name: Name, script: Arc<dyn Script>) -> SubstratesResult<()>;
}

/// Executable unit of work that can be scheduled for execution
/// Direct port of Java Substrates Script interface
#[async_trait]
pub trait Script: Send + Sync {
    /// Executes this script within the context of the provided current
    async fn exec(&self, current: &dyn Current) -> SubstratesResult<()>;
}

/// Interface that provides efficient access to a circuit's work queue
/// Direct port of Java Substrates Current interface
#[async_trait]
pub trait Current: Substrate {
    // Generic method moved to CurrentExt for object-safety
}

/// Utility interface for scoping the work performed against a resource
/// Direct port of Java Substrates Closure interface
/// Note: Generic parameter R moved to associated type for object-safety
#[async_trait]
pub trait Closure: Send + Sync {
    /// The resource type managed by this closure
    type Resource: Resource;
    
    // Generic method moved to ClosureExt for object-safety
}

/// Represents a resource management scope
/// Direct port of Java Substrates Scope interface
#[async_trait]
pub trait Scope: Substrate + Send + Sync {
    /// Close the scope
    async fn close(&mut self);
    
    // Generic methods moved to ScopeExt for object-safety
    
    /// Creates a new named child scope within this scope
    fn scope_named(&self, name: Name) -> SubstratesResult<Arc<dyn Scope>>;
    
    /// Creates a new anonymous child scope within this scope
    fn scope(&self) -> SubstratesResult<Arc<dyn Scope>>;
}

/// In-memory buffer of captures
/// Direct port of Java Substrates Sink interface
#[async_trait]
pub trait Sink<E>: Substrate + Resource {
    /// Returns events that have accumulated since the sink was created or last drain
    async fn drain(&mut self) -> SubstratesResult<Vec<crate::pipe::Capture<E>>>;
}

/// Method chaining utility trait
/// Direct port of Java Substrates Tap interface
pub trait Tap<T>: Send + Sync {
    /// Apply a function and return self for method chaining
    fn tap<F>(self, consumer: F) -> Self
    where
        F: FnOnce(&Self),
        Self: Sized,
    {
        consumer(&self);
        self
    }
}

/// Basic implementation of Circuit
#[derive(Debug)]
pub struct BasicCircuit {
    subject: Subject,
    #[allow(dead_code)]
    channels: RwLock<HashMap<Name, Arc<dyn std::any::Any + Send + Sync>>>,
    queue: Arc<BasicQueue>,
}

impl BasicCircuit {
    pub fn new(name: Name) -> Self {
        Self {
            subject: Subject::new(name, SubjectType::Circuit),
            channels: RwLock::new(HashMap::new()),
            queue: Arc::new(BasicQueue::new()),
        }
    }
}

impl Substrate for BasicCircuit {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

impl Resource for BasicCircuit {
    fn close(&mut self) {
        // Clean up resources
    }
}

#[async_trait]
impl Component for BasicCircuit {
    type Emission = State;
    
    fn source(&self) -> &dyn crate::source::Source<Self::Emission> {
        // This would return a source that emits State changes
        // For now, we panic to indicate unimplemented
        unimplemented!("BasicCircuit source not yet implemented")
    }
}

// Removed Tap implementation due to dyn compatibility issues

#[async_trait]
impl Circuit for BasicCircuit {
    async fn clock(&self) -> SubstratesResult<Arc<dyn Clock>> {
        Err(SubstratesError::InvalidOperation(
            "Clock not yet implemented".to_string()
        ))
    }
    
    async fn clock_named(&self, _name: Name) -> SubstratesResult<Arc<dyn Clock>> {
        Err(SubstratesError::InvalidOperation(
            "Named clock not yet implemented".to_string()
        ))
    }
    
    // Generic methods are implemented in CircuitExt trait
    
    fn queue(&self) -> Arc<dyn Queue> {
        self.queue.clone()
    }
}

/// Basic Current implementation for script execution context
#[derive(Debug, Clone)]
pub struct BasicCurrent {
    subject: Subject,
}

impl BasicCurrent {
    pub fn new() -> Self {
        Self {
            subject: Subject::new(Name::from_part("queue-current"), SubjectType::Script),
        }
    }
}

impl Substrate for BasicCurrent {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

impl Current for BasicCurrent {}

impl Default for BasicCurrent {
    fn default() -> Self {
        Self::new()
    }
}

/// Basic Queue implementation
#[derive(Debug)]
pub struct BasicQueue {
    sender: mpsc::UnboundedSender<Arc<dyn Script>>,
    pending_count: Arc<std::sync::atomic::AtomicUsize>,
}

impl BasicQueue {
    pub fn new() -> Self {
        let (sender, mut receiver) = mpsc::unbounded_channel::<Arc<dyn Script>>();
        let pending_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let pending_count_clone = pending_count.clone();
        
        // Spawn a task to process scripts
        tokio::spawn(async move {
            while let Some(script) = receiver.recv().await {
                // Create a basic Current context for script execution
                let current = BasicCurrent::new();
                if let Err(e) = script.exec(&current).await {
                    tracing::error!("Script execution failed: {}", e);
                }
                pending_count_clone.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            }
        });
        
        Self { sender, pending_count }
    }
}

impl Default for BasicQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Queue for BasicQueue {
    async fn await_empty(&self) {
        // Wait until pending count reaches zero
        while self.pending_count.load(std::sync::atomic::Ordering::SeqCst) > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }
    
    async fn post(&self, script: Arc<dyn Script>) -> SubstratesResult<()> {
        self.pending_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.sender
            .send(script)
            .map_err(|_| {
                self.pending_count.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                SubstratesError::ChannelError("Queue closed".to_string())
            })?;
        Ok(())
    }
    
    async fn post_named(&self, _name: Name, script: Arc<dyn Script>) -> SubstratesResult<()> {
        self.post(script).await
    }
}

/// Basic Scope implementation
pub struct BasicScope {
    subject: Subject,
    resources: parking_lot::RwLock<Vec<Box<dyn Resource + Send + Sync>>>,
}

impl std::fmt::Debug for BasicScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicScope")
            .field("subject", &self.subject)
            .field("resources_count", &self.resources.read().len())
            .finish()
    }
}

impl BasicScope {
    pub fn new(name: Name) -> Self {
        Self {
            subject: Subject::new(name, SubjectType::Scope),
            resources: parking_lot::RwLock::new(Vec::new()),
        }
    }
}

impl Substrate for BasicScope {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

#[async_trait]
impl Scope for BasicScope {
    async fn close(&mut self) {
        // Close all managed resources
        let mut resources = self.resources.write();
        for resource in resources.iter_mut() {
            resource.close();
        }
        resources.clear();
    }
    
    fn scope_named(&self, name: Name) -> SubstratesResult<Arc<dyn Scope>> {
        Ok(Arc::new(BasicScope::new(name)))
    }
    
    fn scope(&self) -> SubstratesResult<Arc<dyn Scope>> {
        let name = Name::from_part("child");
        self.scope_named(name)
    }
}

/// Basic implementation of Sink
pub struct BasicSink<E> {
    subject: Subject,
    captures: parking_lot::RwLock<Vec<crate::pipe::Capture<E>>>,
}

impl<E> std::fmt::Debug for BasicSink<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicSink")
            .field("subject", &self.subject)
            .field("captures_count", &self.captures.read().len())
            .finish()
    }
}

impl<E> BasicSink<E> {
    pub fn new(subject: Subject) -> Self {
        Self {
            subject,
            captures: parking_lot::RwLock::new(Vec::new()),
        }
    }
    
    /// Create a pipe that forwards emissions to this sink
    pub fn create_pipe(self: Arc<Self>) -> Arc<dyn Pipe<E>>
    where
        E: Send + Sync + 'static,
    {
        Arc::new(SinkPipe {
            sink: self,
        })
    }
}

impl<E> Substrate for BasicSink<E> {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

impl<E> Resource for BasicSink<E> {
    fn close(&mut self) {
        self.captures.write().clear();
    }
}

#[async_trait]
impl<E> Sink<E> for BasicSink<E>
where
    E: Send + Sync + 'static,
{
    async fn drain(&mut self) -> SubstratesResult<Vec<crate::pipe::Capture<E>>> {
        let mut captures = self.captures.write();
        let drained = std::mem::take(&mut *captures);
        Ok(drained)
    }
}

/// Pipe that forwards emissions to a sink
struct SinkPipe<E> {
    sink: Arc<BasicSink<E>>,
}

impl<E> std::fmt::Debug for SinkPipe<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SinkPipe").finish()
    }
}

#[async_trait]
impl<E> Pipe<E> for SinkPipe<E>
where
    E: Send + Sync + 'static,
{
    async fn emit(&mut self, emission: E) -> SubstratesResult<()> {
        let capture = crate::pipe::Capture::new(emission, self.sink.subject.clone());
        self.sink.captures.write().push(capture);
        Ok(())
    }
}

/// Basic implementation of Conduit
pub struct BasicConduit<P, E> {
    subject: Subject,
    composer: Arc<dyn Composer<P, E>>,
    circuit: Subject,
    sequencer: Option<Arc<dyn Sequencer<dyn Path<E>>>>,
    source: crate::source::BasicSource<E>,
}

impl<P, E> std::fmt::Debug for BasicConduit<P, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicConduit")
            .field("subject", &self.subject)
            .field("circuit", &self.circuit)
            .finish()
    }
}

impl<P, E> BasicConduit<P, E> {
    pub fn new(subject: Subject, composer: Arc<dyn Composer<P, E>>, circuit: Subject) -> Self {
        let source = crate::source::BasicSource::new(subject.clone());
        Self {
            subject,
            composer,
            circuit,
            sequencer: None,
            source,
        }
    }
    
    pub fn set_sequencer(&mut self, sequencer: Arc<dyn Sequencer<dyn Path<E>>>) {
        self.sequencer = Some(sequencer);
    }
}

impl<P, E> Substrate for BasicConduit<P, E> {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

impl<P, E> Resource for BasicConduit<P, E> {
    fn close(&mut self) {
        // Clean up resources
    }
}

impl<P, E> Component for BasicConduit<P, E>
where
    E: Send + Sync + 'static,
{
    type Emission = E;
    
    fn source(&self) -> &dyn crate::source::Source<E> {
        &self.source
    }
}

impl<P, E> Pool<P> for BasicConduit<P, E>
where
    P: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn get(&self, name: &Name) -> SubstratesResult<P> {
        let channel = BasicChannel::<E>::new(name.clone());
        self.composer.compose(Arc::new(channel))
    }
}

impl<P, E> Container<P, E> for BasicConduit<P, E>
where
    P: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
}

impl<P, E> Conduit<P, E> for BasicConduit<P, E>
where
    P: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
}

/// Basic implementation of Container
pub struct BasicContainer<P, E> {
    subject: Subject,
    composer: Arc<dyn Composer<P, E>>,
    circuit: Subject,
    sequencer: Option<Arc<dyn Sequencer<dyn Path<E>>>>,
    source: crate::source::BasicSource<E>,
    channels: parking_lot::RwLock<HashMap<Name, Arc<dyn Channel<E>>>>,
}

impl<P, E> std::fmt::Debug for BasicContainer<P, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicContainer")
            .field("subject", &self.subject)
            .field("circuit", &self.circuit)
            .field("channels_count", &self.channels.read().len())
            .finish()
    }
}

impl<P, E> BasicContainer<P, E> {
    pub fn new(subject: Subject, composer: Arc<dyn Composer<P, E>>, circuit: Subject) -> Self {
        let source = crate::source::BasicSource::new(subject.clone());
        Self {
            subject,
            composer,
            circuit,
            sequencer: None,
            source,
            channels: parking_lot::RwLock::new(HashMap::new()),
        }
    }
    
    pub fn set_sequencer(&mut self, sequencer: Arc<dyn Sequencer<dyn Path<E>>>) {
        self.sequencer = Some(sequencer);
    }
}

impl<P, E> Substrate for BasicContainer<P, E> {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

impl<P, E> Resource for BasicContainer<P, E> {
    fn close(&mut self) {
        self.channels.write().clear();
    }
}

impl<P, E> Component for BasicContainer<P, E>
where
    E: Send + Sync + 'static,
{
    type Emission = E;
    
    fn source(&self) -> &dyn crate::source::Source<E> {
        &self.source
    }
}

impl<P, E> Pool<P> for BasicContainer<P, E>
where
    P: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn get(&self, name: &Name) -> SubstratesResult<P> {
        let mut channels = self.channels.write();
        let channel = channels.entry(name.clone())
            .or_insert_with(|| Arc::new(BasicChannel::<E>::new(name.clone())))
            .clone();
        
        self.composer.compose(channel)
    }
}

impl<P, E> Container<P, E> for BasicContainer<P, E>
where
    P: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
}

/// Basic implementation of Channel
pub struct BasicChannel<E> {
    subject: Subject,
    pipes: parking_lot::RwLock<Vec<Arc<dyn Pipe<E>>>>,
}

impl<E> std::fmt::Debug for BasicChannel<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicChannel")
            .field("subject", &self.subject)
            .field("pipes_count", &self.pipes.read().len())
            .finish()
    }
}

impl<E> BasicChannel<E> {
    pub fn new(name: Name) -> Self {
        Self {
            subject: Subject::new(name, SubjectType::Channel),
            pipes: parking_lot::RwLock::new(Vec::new()),
        }
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
        Ok(Arc::new(ChannelPipe {
            channel: self.subject.clone(),
            _phantom: std::marker::PhantomData,
        }))
    }
}

#[async_trait]
impl<E> Channel<E> for BasicChannel<E>
where
    E: Send + Sync + 'static,
{
    fn pipe(&self) -> SubstratesResult<Arc<dyn Pipe<E>>> {
        Inlet::pipe(self)
    }
    
    fn pipe_with_sequencer(
        &self,
        sequencer: Arc<dyn Sequencer<dyn Path<E>>>,
    ) -> SubstratesResult<Arc<dyn Pipe<E>>> {
        // Create a pipe that applies the sequencer
        let (sender, mut receiver) = mpsc::unbounded_channel::<E>();
        let channel = self.subject.clone();
        
        // Spawn task to process sequenced emissions
        tokio::spawn(async move {
            while let Some(emission) = receiver.recv().await {
                // In a real implementation, sequencer would transform emissions
                tracing::debug!("Processing sequenced emission for channel {:?}", channel);
                drop(emission);
            }
        });
        
        Ok(Arc::new(SequencedPipe {
            channel: self.subject.clone(),
            sequencer,
            sender,
        }))
    }
}

/// Pipe implementation for channels
struct ChannelPipe<E> {
    channel: Subject,
    _phantom: std::marker::PhantomData<E>,
}

/// Sequenced pipe that applies a sequencer to emissions
struct SequencedPipe<E> {
    channel: Subject,
    sequencer: Arc<dyn Sequencer<dyn Path<E>>>,
    sender: mpsc::UnboundedSender<E>,
}

impl<E> std::fmt::Debug for ChannelPipe<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChannelPipe")
            .field("channel", &self.channel)
            .finish()
    }
}

impl<E> std::fmt::Debug for SequencedPipe<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SequencedPipe")
            .field("channel", &self.channel)
            .finish()
    }
}

#[async_trait]
impl<E> Pipe<E> for SequencedPipe<E>
where
    E: Send + Sync + 'static,
{
    async fn emit(&mut self, emission: E) -> SubstratesResult<()> {
        // Send emission through the channel for sequenced processing
        self.sender
            .send(emission)
            .map_err(|_| SubstratesError::ChannelError("Sequenced pipe closed".to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl<E> Pipe<E> for ChannelPipe<E>
where
    E: Send + Sync + 'static,
{
    async fn emit(&mut self, emission: E) -> SubstratesResult<()> {
        // Emit to the channel's subject
        // In a real implementation, this would route through the conduit
        tracing::debug!("Emitting to channel {:?}", self.channel);
        // Store the emission in the subject's state if needed
        drop(emission); // Consume the emission
        Ok(())
    }
}