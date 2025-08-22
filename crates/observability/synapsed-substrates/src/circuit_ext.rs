//! Extension trait for Circuit with generic methods
//! This pattern allows the core Circuit trait to remain object-safe while providing the full Java API

use crate::circuit::{Circuit, Closure, Conduit, Container, Current};
use crate::percept::Composer;
use crate::pipe::{Path, Sequencer};
use crate::types::{Name, SubstratesResult};
use crate::{async_trait, Subject};
use std::sync::Arc;

/// Extension trait providing generic methods for Circuit
/// This trait is automatically implemented for all types that implement Circuit
#[async_trait]
pub trait CircuitExt: Circuit {
    /// Returns a conduit that will use this circuit to process and transfer values emitted
    async fn conduit<P, E>(
        &self,
        composer: Arc<dyn Composer<P, E>>,
    ) -> SubstratesResult<Arc<dyn Conduit<P, E>>>
    where
        P: Send + Sync + 'static,
        E: Send + Sync + 'static;
    
    /// Returns a named conduit
    async fn conduit_named<P, E>(
        &self,
        name: Name,
        composer: Arc<dyn Composer<P, E>>,
    ) -> SubstratesResult<Arc<dyn Conduit<P, E>>>
    where
        P: Send + Sync + 'static,
        E: Send + Sync + 'static;
    
    /// Returns a conduit with sequencer
    async fn conduit_with_sequencer<P, E>(
        &self,
        name: Name,
        composer: Arc<dyn Composer<P, E>>,
        sequencer: Arc<dyn Sequencer<dyn Path<E>>>,
    ) -> SubstratesResult<Arc<dyn Conduit<P, E>>>
    where
        P: Send + Sync + 'static,
        E: Send + Sync + 'static;
    
    /// Returns a container that will use this circuit to create conduits
    async fn container<P, E>(
        &self,
        composer: Arc<dyn Composer<P, E>>,
    ) -> SubstratesResult<Arc<dyn Container<P, E>>>
    where
        P: Send + Sync + 'static,
        E: Send + Sync + 'static;
    
    /// Returns a named container
    async fn container_named<P, E>(
        &self,
        name: Name,
        composer: Arc<dyn Composer<P, E>>,
    ) -> SubstratesResult<Arc<dyn Container<P, E>>>
    where
        P: Send + Sync + 'static,
        E: Send + Sync + 'static;
    
    /// Returns a container with sequencer
    async fn container_with_sequencer<P, E>(
        &self,
        name: Name,
        composer: Arc<dyn Composer<P, E>>,
        sequencer: Arc<dyn Sequencer<dyn Path<E>>>,
    ) -> SubstratesResult<Arc<dyn Container<P, E>>>
    where
        P: Send + Sync + 'static,
        E: Send + Sync + 'static;
}

// Default implementations for all Circuit types
#[async_trait]
impl<T: Circuit + Sync + ?Sized> CircuitExt for T {
    async fn conduit<P, E>(
        &self,
        composer: Arc<dyn Composer<P, E>>,
    ) -> SubstratesResult<Arc<dyn Conduit<P, E>>>
    where
        P: Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        let name = Name::from_part("conduit");
        self.conduit_named(name, composer).await
    }
    
    async fn conduit_named<P, E>(
        &self,
        name: Name,
        composer: Arc<dyn Composer<P, E>>,
    ) -> SubstratesResult<Arc<dyn Conduit<P, E>>>
    where
        P: Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        use crate::circuit::BasicConduit;
        
        let subject = Subject::new(name, crate::types::SubjectType::Channel);
        let conduit = BasicConduit::new(subject, composer, self.subject().clone());
        Ok(Arc::new(conduit))
    }
    
    async fn conduit_with_sequencer<P, E>(
        &self,
        name: Name,
        composer: Arc<dyn Composer<P, E>>,
        sequencer: Arc<dyn Sequencer<dyn Path<E>>>,
    ) -> SubstratesResult<Arc<dyn Conduit<P, E>>>
    where
        P: Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        use crate::circuit::BasicConduit;
        
        let subject = Subject::new(name, crate::types::SubjectType::Channel);
        let mut conduit = BasicConduit::new(subject, composer, self.subject().clone());
        conduit.set_sequencer(sequencer);
        Ok(Arc::new(conduit))
    }
    
    async fn container<P, E>(
        &self,
        composer: Arc<dyn Composer<P, E>>,
    ) -> SubstratesResult<Arc<dyn Container<P, E>>>
    where
        P: Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        let name = Name::from_part("container");
        self.container_named(name, composer).await
    }
    
    async fn container_named<P, E>(
        &self,
        name: Name,
        composer: Arc<dyn Composer<P, E>>,
    ) -> SubstratesResult<Arc<dyn Container<P, E>>>
    where
        P: Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        use crate::circuit::BasicContainer;
        
        let subject = Subject::new(name, crate::types::SubjectType::Channel);
        let container = BasicContainer::new(subject, composer, self.subject().clone());
        Ok(Arc::new(container))
    }
    
    async fn container_with_sequencer<P, E>(
        &self,
        name: Name,
        composer: Arc<dyn Composer<P, E>>,
        sequencer: Arc<dyn Sequencer<dyn Path<E>>>,
    ) -> SubstratesResult<Arc<dyn Container<P, E>>>
    where
        P: Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        use crate::circuit::BasicContainer;
        
        let subject = Subject::new(name, crate::types::SubjectType::Channel);
        let mut container = BasicContainer::new(subject, composer, self.subject().clone());
        container.set_sequencer(sequencer);
        Ok(Arc::new(container))
    }
}

/// Extension trait providing generic methods for Current
/// This trait is automatically implemented for all types that implement Current
#[async_trait]
pub trait CurrentExt: Current {
    /// Posts a runnable to be executed asynchronously
    async fn post<F>(&self, runnable: F) -> SubstratesResult<()>
    where
        F: FnOnce() -> SubstratesResult<()> + Send + 'static;
}

// Default implementation for all Current types
#[async_trait]
impl<T: Current + Sync + ?Sized> CurrentExt for T {
    async fn post<F>(&self, runnable: F) -> SubstratesResult<()>
    where
        F: FnOnce() -> SubstratesResult<()> + Send + 'static,
    {
        // For now, just execute the function directly
        // In a real implementation, this would be queued on the circuit's queue
        runnable()
    }
}

/// Extension trait providing generic methods for Closure
/// This trait is automatically implemented for all types that implement Closure
#[async_trait]
pub trait ClosureExt: Closure {
    /// Calls a consumer, with an acquired resource, within an automatic resource management scope
    async fn consume<F>(&self, consumer: F) -> SubstratesResult<()>
    where
        F: FnOnce(&mut Self::Resource) -> SubstratesResult<()> + Send + 'static;
}

// Default implementation for all Closure types
#[async_trait]
impl<T: Closure + Sync + ?Sized> ClosureExt for T {
    async fn consume<F>(&self, _consumer: F) -> SubstratesResult<()>
    where
        F: FnOnce(&mut Self::Resource) -> SubstratesResult<()> + Send + 'static,
    {
        // This is a simplified implementation
        // In a real implementation, this would acquire the resource,
        // call the consumer, and then release the resource
        // For now, we just return Ok as the actual resource management
        // would depend on the specific Closure implementation
        Ok(())
    }
}