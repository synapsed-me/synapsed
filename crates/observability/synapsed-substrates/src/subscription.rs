//! Complete subscription model implementation
//!
//! This module implements the full subscription pattern where:
//! - Sources maintain a registry of Subjects that can emit
//! - Subscribers register Pipes with specific Subjects
//! - When a Subject emits, the emission flows through registered Pipes

use crate::channel::{BasicChannel, BasicConduit};
use crate::circuit::Channel;
use crate::pipe::Pipe;
use crate::subject::{Registrar, Resource, Subscriber, Subscription, Substrate, Subject};
use crate::types::{Name, SubjectType, SubstratesResult, SubstratesError};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::mpsc;

/// A complete Source implementation that manages Subject->Pipe routing
pub struct ManagedSource<E> {
    subject: Subject,
    /// Map of Subject ID to channels that emit for that subject
    channels: Arc<RwLock<HashMap<String, Arc<BasicChannel<E>>>>>,
    /// Active subscriptions
    subscriptions: Arc<RwLock<Vec<Arc<ManagedSubscription<E>>>>>,
    /// Channel for internal events
    event_sender: mpsc::UnboundedSender<SourceEvent<E>>,
    event_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<SourceEvent<E>>>>>,
}

enum SourceEvent<E> {
    SubjectEmitted { subject: Subject, emission: E },
    SubscriberAdded { subscriber: Arc<dyn Subscriber<Emission = E>> },
    SubscriptionClosed { subscription_id: String },
}

impl<E> ManagedSource<E> 
where
    E: Send + Sync + Clone + 'static,
{
    pub fn new(name: Name) -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        let source = Self {
            subject: Subject::new(name, SubjectType::Source),
            channels: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(Vec::new())),
            event_sender,
            event_receiver: Arc::new(RwLock::new(Some(event_receiver))),
        };
        
        // Start event processing task
        source.start_event_processor();
        source
    }
    
    /// Register a new subject that can emit through this source
    pub fn register_subject(&self, subject: Subject) -> Arc<BasicChannel<E>> {
        let mut channels = self.channels.write();
        let channel = Arc::new(BasicChannel::new(Name::from_part(&subject.id().to_string())));
        channels.insert(subject.id().to_string(), channel.clone());
        channel
    }
    
    /// Emit a value from a specific subject
    pub async fn emit(&self, subject: &Subject, emission: E) -> SubstratesResult<()> {
        // Send event to processor
        self.event_sender
            .send(SourceEvent::SubjectEmitted {
                subject: subject.clone(),
                emission,
            })
            .map_err(|_| SubstratesError::Closed("Source event channel closed".to_string()))?;
        Ok(())
    }
    
    fn start_event_processor(&self) {
        let subscriptions = self.subscriptions.clone();
        let mut receiver = self.event_receiver.write().take()
            .expect("Event receiver already taken");
        
        tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                match event {
                    SourceEvent::SubjectEmitted { subject, emission } => {
                        // Notify all active subscriptions
                        let subs = subscriptions.read().clone();
                        for subscription in subs {
                            if subscription.is_active() {
                                subscription.notify_emission(&subject, emission.clone()).await;
                            }
                        }
                    }
                    SourceEvent::SubscriberAdded { subscriber: _ } => {
                        // Handle new subscriber
                    }
                    SourceEvent::SubscriptionClosed { subscription_id: _ } => {
                        // Clean up closed subscriptions
                        subscriptions.write().retain(|s| s.is_active());
                    }
                }
            }
        });
    }
    
    pub async fn subscribe(
        &self,
        subscriber: Arc<dyn Subscriber<Emission = E>>,
    ) -> SubstratesResult<Arc<ManagedSubscription<E>>> {
        let subscription = Arc::new(ManagedSubscription::new(
            self.subject.clone(),
            subscriber.clone(),
            self.channels.clone(),
        ));
        
        // Register the subscriber with all existing subjects
        {
            let channels = self.channels.read();
            for (subject_id, channel) in channels.iter() {
                let subject = Subject::with_id(
                    crate::types::Id::from_string(subject_id),
                    Name::from_part(subject_id),
                    SubjectType::Channel,
                );
                subscription.register_with_subject(&subject, channel.clone());
            }
        }
        
        self.subscriptions.write().push(subscription.clone());
        
        // Send event
        let _ = self.event_sender.send(SourceEvent::SubscriberAdded { subscriber });
        
        Ok(subscription)
    }
}

impl<E> Substrate for ManagedSource<E> {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

/// A managed subscription that handles Subject->Pipe routing
pub struct ManagedSubscription<E> {
    subject: Subject,
    subscriber: Arc<dyn Subscriber<Emission = E>>,
    /// Pipes registered for each subject
    pipes: Arc<RwLock<HashMap<String, Vec<Arc<dyn Pipe<E>>>>>>,
    active: Arc<RwLock<bool>>,
    channels: Arc<RwLock<HashMap<String, Arc<BasicChannel<E>>>>>,
}

impl<E> ManagedSubscription<E>
where
    E: Send + Sync + Clone + 'static,
{
    pub fn new(
        subject: Subject,
        subscriber: Arc<dyn Subscriber<Emission = E>>,
        channels: Arc<RwLock<HashMap<String, Arc<BasicChannel<E>>>>>,
    ) -> Self {
        Self {
            subject,
            subscriber,
            pipes: Arc::new(RwLock::new(HashMap::new())),
            active: Arc::new(RwLock::new(true)),
            channels,
        }
    }
    
    pub fn is_active(&self) -> bool {
        *self.active.read()
    }
    
    pub fn register_with_subject(&self, subject: &Subject, _channel: Arc<BasicChannel<E>>) {
        // Create a registrar for this subject
        let mut registrar = PipeRegistrar::new(subject.clone(), self.pipes.clone());
        
        // Let the subscriber register pipes
        let mut subscriber = self.subscriber.clone();
        Arc::get_mut(&mut subscriber).map(|s| s.accept(subject, &mut registrar));
    }
    
    pub async fn notify_emission(&self, subject: &Subject, emission: E) {
        // Clone the pipes to release the lock before await
        let pipes_to_notify = {
            let pipes = self.pipes.read();
            pipes.get(&subject.id().to_string()).cloned()
        };
        
        if let Some(subject_pipes) = pipes_to_notify {
            for pipe in subject_pipes {
                let mut pipe = pipe.clone();
                if let Some(pipe_mut) = Arc::get_mut(&mut pipe) {
                    let _ = pipe_mut.emit(emission.clone()).await;
                }
            }
        }
    }
}

impl<E> Substrate for ManagedSubscription<E> {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

impl<E> Resource for ManagedSubscription<E> {
    fn close(&mut self) {
        *self.active.write() = false;
    }
}

impl<E> Subscription for ManagedSubscription<E> {}

/// Registrar that collects pipes for a specific subject
struct PipeRegistrar<E> {
    subject: Subject,
    pipes: Arc<RwLock<HashMap<String, Vec<Arc<dyn Pipe<E>>>>>>,
}

impl<E> PipeRegistrar<E> {
    fn new(subject: Subject, pipes: Arc<RwLock<HashMap<String, Vec<Arc<dyn Pipe<E>>>>>>) -> Self {
        Self { subject, pipes }
    }
}

impl<E> Registrar for PipeRegistrar<E>
where
    E: Send + Sync + 'static,
{
    type Emission = E;
    
    fn register(&mut self, pipe: Arc<dyn Pipe<E>>) {
        let mut pipes = self.pipes.write();
        pipes.entry(self.subject.id().to_string())
            .or_insert_with(Vec::new)
            .push(pipe);
    }
}

/// Creates a Source integrated with a Conduit for complete emission flow
pub struct ConduitSource<P, E> {
    source: ManagedSource<E>,
    conduit: Arc<BasicConduit<P, E>>,
}

impl<P, E> ConduitSource<P, E>
where
    E: Send + Sync + Clone + 'static,
{
    pub fn new(name: Name) -> Self {
        let source = ManagedSource::new(name.clone());
        let conduit = Arc::new(BasicConduit::new(name));
        
        Self { source, conduit }
    }
    
    /// Create a channel within the conduit and register it with the source
    pub fn create_channel(&self, name: Name) -> Arc<BasicChannel<E>> {
        let channel = self.conduit.create_channel(name.clone());
        
        // Register this channel's subject with the source
        self.source.register_subject(channel.subject().clone());
        
        channel
    }
    
    /// Subscribe to emissions from all channels
    pub async fn subscribe(
        &self,
        subscriber: Arc<dyn Subscriber<Emission = E>>,
    ) -> SubstratesResult<Arc<ManagedSubscription<E>>> {
        self.source.subscribe(subscriber).await
    }
    
    /// Emit through a specific channel
    pub async fn emit_through_channel(
        &self,
        channel: &BasicChannel<E>,
        emission: E,
    ) -> SubstratesResult<()> {
        // Get a pipe from the channel and emit
        let mut pipe = Channel::pipe(channel)?;
        if let Some(pipe_mut) = Arc::get_mut(&mut pipe) {
            pipe_mut.emit(emission.clone()).await?;
        }
        
        // Also notify the source
        self.source.emit(channel.subject(), emission).await
    }
}

impl<P, E> Substrate for ConduitSource<P, E> {
    fn subject(&self) -> &Subject {
        self.source.subject()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipe::FunctionPipe;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    #[tokio::test]
    async fn test_managed_source_subscription() {
        let source = ManagedSource::<String>::new(Name::from_part("test-source"));
        
        // Register a subject
        let subject = Subject::new(Name::from_part("test-subject"), SubjectType::Channel);
        let _channel = source.register_subject(subject.clone());
        
        // Create a subscriber that counts emissions
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        let subscriber = Arc::new(crate::source::FunctionSubscriber::new(
            move |_subject, registrar| {
                let counter = counter_clone.clone();
                let pipe = FunctionPipe::new(move |_value: String| {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                });
                registrar.register(Arc::new(pipe));
                Ok(())
            }
        ));
        
        let _subscription = source.subscribe(subscriber).await.unwrap();
        
        // Emit some values
        source.emit(&subject, "test1".to_string()).await.unwrap();
        source.emit(&subject, "test2".to_string()).await.unwrap();
        
        // Give async tasks time to process
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
    
    #[tokio::test]
    async fn test_conduit_source_integration() {
        let conduit_source = ConduitSource::<(), String>::new(Name::from_part("test-conduit"));
        
        // Create channels
        let channel1 = conduit_source.create_channel(Name::from_part("channel-1"));
        let channel2 = conduit_source.create_channel(Name::from_part("channel-2"));
        
        // Subscribe to all channels
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        let subscriber = Arc::new(crate::source::FunctionSubscriber::new(
            move |_subject, registrar| {
                let counter = counter_clone.clone();
                let pipe = FunctionPipe::new(move |_value: String| {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                });
                registrar.register(Arc::new(pipe));
                Ok(())
            }
        ));
        
        let _subscription = conduit_source.subscribe(subscriber).await.unwrap();
        
        // Emit through both channels
        conduit_source.emit_through_channel(&channel1, "msg1".to_string()).await.unwrap();
        conduit_source.emit_through_channel(&channel2, "msg2".to_string()).await.unwrap();
        
        // Give async tasks time to process
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
}