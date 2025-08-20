//! Extension trait for Subject with generic methods
//! This pattern allows the core traits to remain object-safe while providing the full Java API

use crate::subject::{Component, Context, Registrar, Subscriber};
use crate::types::SubstratesResult;
use std::sync::Arc;

/// Extension trait providing generic methods for Component
pub trait ComponentExt: Component {
    /// Gets the source with proper type information
    fn source_typed<E>(&self) -> Option<&dyn crate::source::Source<E>>
    where
        Self::Emission: 'static,
        E: 'static,
    {
        if std::any::TypeId::of::<Self::Emission>() == std::any::TypeId::of::<E>() {
            // Safe because we checked the types match
            unsafe {
                Some(std::mem::transmute::<
                    &dyn crate::source::Source<Self::Emission>,
                    &dyn crate::source::Source<E>,
                >(self.source()))
            }
        } else {
            None
        }
    }
}

/// Extension trait providing generic methods for Context
pub trait ContextExt: Context {
    /// Gets the source with proper type information
    fn source_typed<E>(&self) -> Option<&dyn crate::source::Source<E>>
    where
        Self::Emission: 'static,
        E: 'static,
    {
        if std::any::TypeId::of::<Self::Emission>() == std::any::TypeId::of::<E>() {
            // Safe because we checked the types match
            unsafe {
                Some(std::mem::transmute::<
                    &dyn crate::source::Source<Self::Emission>,
                    &dyn crate::source::Source<E>,
                >(self.source()))
            }
        } else {
            None
        }
    }
}

/// Extension trait providing generic methods for Subscriber
pub trait SubscriberExt: Subscriber {
    /// Creates a subscriber from a function
    fn from_function<F>(func: F) -> crate::source::FunctionSubscriber<Self::Emission, F>
    where
        F: Fn(&crate::Subject, &mut dyn Registrar<Emission = Self::Emission>) -> SubstratesResult<()>
            + Send
            + Sync,
    {
        crate::source::FunctionSubscriber::new(func)
    }
}

/// Extension trait providing generic methods for Registrar  
pub trait RegistrarExt: Registrar {
    /// Registers a pipe with type checking
    fn register_typed<E>(&mut self, pipe: Arc<dyn crate::pipe::Pipe<E>>) -> bool
    where
        Self::Emission: 'static,
        E: 'static,
    {
        if std::any::TypeId::of::<Self::Emission>() == std::any::TypeId::of::<E>() {
            // Safe because we checked the types match
            unsafe {
                let pipe_transmuted = std::mem::transmute::<
                    Arc<dyn crate::pipe::Pipe<E>>,
                    Arc<dyn crate::pipe::Pipe<Self::Emission>>,
                >(pipe);
                self.register(pipe_transmuted);
                true
            }
        } else {
            false
        }
    }
}

// Automatically implement extension traits
impl<T: Component + ?Sized> ComponentExt for T {}
impl<T: Context + ?Sized> ContextExt for T {}
impl<T: Subscriber + ?Sized> SubscriberExt for T {}
impl<T: Registrar + ?Sized> RegistrarExt for T {}