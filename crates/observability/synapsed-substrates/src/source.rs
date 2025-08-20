//! Source and related abstractions - direct port of Java Substrates Source interfaces

use crate::subject::{Registrar, Resource, Subscriber, Subscription, Substrate};
use crate::types::SubstratesResult;
use crate::{async_trait, Subject};
use std::sync::Arc;

/// Trait for subscribing to source events
/// Direct port of Java Substrates Source interface
#[async_trait]
pub trait Source<E>: Substrate + Send + Sync {
    /// Subscribes a Subscriber to receive subject registrations from this source
    async fn subscribe(
        &self,
        subscriber: Arc<dyn Subscriber<Emission = E>>,
    ) -> SubstratesResult<Arc<dyn Subscription>>;
}

/// Basic source implementation
pub struct BasicSource<E> {
    subject: Subject,
    subscribers: parking_lot::RwLock<Vec<Arc<dyn Subscriber<Emission = E>>>>,
}

impl<E> std::fmt::Debug for BasicSource<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicSource")
            .field("subject", &self.subject)
            .field("subscribers_count", &self.subscribers.read().len())
            .finish()
    }
}

impl<E> BasicSource<E> {
    pub fn new(subject: Subject) -> Self {
        Self {
            subject,
            subscribers: parking_lot::RwLock::new(Vec::new()),
        }
    }
    
    /// Emit an event to all subscribers
    pub async fn emit(&self, subject: &Subject, _emission: E) -> SubstratesResult<()>
    where
        E: Clone,
    {
        let subscribers = self.subscribers.read().clone();
        
        for subscriber in subscribers {
            let mut registrar = BasicRegistrar::<E>::new();
            // Note: This is a simplified implementation
            // In a full implementation, we'd need to properly handle the registrar
            let _ = (subscriber, subject, &mut registrar);
        }
        
        Ok(())
    }
}

impl<E> Substrate for BasicSource<E> {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

#[async_trait]
impl<E> Source<E> for BasicSource<E>
where
    E: Send + Sync + 'static,
{
    async fn subscribe(
        &self,
        subscriber: Arc<dyn Subscriber<Emission = E>>,
    ) -> SubstratesResult<Arc<dyn Subscription>> {
        self.subscribers.write().push(subscriber.clone());
        
        Ok(Arc::new(BasicSubscription::new(
            self.subject.clone(),
            subscriber,
        )))
    }
}

/// Basic subscription implementation
pub struct BasicSubscription<E> {
    subject: Subject,
    #[allow(dead_code)]
    subscriber: Arc<dyn Subscriber<Emission = E>>,
    active: parking_lot::RwLock<bool>,
}

impl<E> std::fmt::Debug for BasicSubscription<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicSubscription")
            .field("subject", &self.subject)
            .field("active", &*self.active.read())
            .finish()
    }
}

impl<E> BasicSubscription<E> {
    pub fn new(subject: Subject, subscriber: Arc<dyn Subscriber<Emission = E>>) -> Self {
        Self {
            subject,
            subscriber,
            active: parking_lot::RwLock::new(true),
        }
    }
    
    pub fn is_active(&self) -> bool {
        *self.active.read()
    }
}

impl<E> Substrate for BasicSubscription<E> {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

impl<E> Resource for BasicSubscription<E> {
    fn close(&mut self) {
        *self.active.write() = false;
    }
}

impl<E> Subscription for BasicSubscription<E> {}

/// Basic registrar implementation
#[derive(Debug)]
pub struct BasicRegistrar<E> {
    pipes: Vec<Arc<dyn crate::pipe::Pipe<E>>>,
}

impl<E> BasicRegistrar<E> {
    pub fn new() -> Self {
        Self { pipes: Vec::new() }
    }
    
    pub fn pipes(&self) -> &[Arc<dyn crate::pipe::Pipe<E>>] {
        &self.pipes
    }
}

impl<E> Default for BasicRegistrar<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> Registrar for BasicRegistrar<E>
where
    E: Send + Sync + 'static,
{
    type Emission = E;
    
    fn register(&mut self, pipe: Arc<dyn crate::pipe::Pipe<E>>) {
        self.pipes.push(pipe);
    }
}

/// Function-based subscriber implementation
pub struct FunctionSubscriber<E, F>
where
    F: Fn(&Subject, &mut dyn Registrar<Emission = E>) -> SubstratesResult<()> + Send + Sync,
{
    func: F,
    _phantom: std::marker::PhantomData<E>,
}

impl<E, F> FunctionSubscriber<E, F>
where
    F: Fn(&Subject, &mut dyn Registrar<Emission = E>) -> SubstratesResult<()> + Send + Sync,
{
    pub fn new(func: F) -> Self {
        Self {
            func,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<E, F> std::fmt::Debug for FunctionSubscriber<E, F>
where
    F: Fn(&Subject, &mut dyn Registrar<Emission = E>) -> SubstratesResult<()> + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionSubscriber").finish()
    }
}

impl<E, F> Subscriber for FunctionSubscriber<E, F>
where
    F: Fn(&Subject, &mut dyn Registrar<Emission = E>) -> SubstratesResult<()> + Send + Sync,
    E: Send + Sync,
{
    type Emission = E;
    
    fn accept(&mut self, subject: &Subject, registrar: &mut dyn Registrar<Emission = E>) {
        let _ = (self.func)(subject, registrar);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Name, SubjectType};
    
    #[tokio::test]
    async fn test_basic_source() {
        let subject = Subject::new(Name::from_part("test"), SubjectType::Source);
        let source = BasicSource::<i32>::new(subject);
        
        let subscriber = Arc::new(FunctionSubscriber::new(|_subject, _registrar| Ok(())));
        
        let subscription = source.subscribe(subscriber).await.unwrap();
        assert!(subscription.subject().name().to_path() == "test");
    }
    
    #[tokio::test]
    async fn test_subscription_lifecycle() {
        let subject = Subject::new(Name::from_part("test"), SubjectType::Source);
        let source = BasicSource::<String>::new(subject);
        
        let subscriber = Arc::new(FunctionSubscriber::new(|_subject, _registrar| Ok(())));
        
        let mut subscription = source.subscribe(subscriber).await.unwrap();
        
        // Convert Arc to mutable reference for testing
        let subscription_ref = Arc::get_mut(&mut subscription).unwrap();
        subscription_ref.close();
    }
}