//! Resources API - placeholder for future implementation
//! Direct port of Java Serventis Resources interface

use crate::{async_trait, Arc, Composer, Pipe, Substrate};

/// The Resources interface - entry point into the Serventis Resources API
/// Direct port of Java Serventis Resources interface
pub trait Resources: Composer<Arc<dyn ResourceMonitor>, Box<dyn ResourceEvent>> + Send + Sync {}

/// ResourceMonitor interface for emitting signals about resource interactions
/// Direct port of Java Serventis ResourceMonitor interface
#[async_trait]
pub trait ResourceMonitor: Pipe<Box<dyn ResourceEvent>> + Substrate + Send + Sync {}

/// ResourceEvent interface representing interactions with shared resources
/// Direct port of Java Serventis ResourceEvent interface (placeholder)
pub trait ResourceEvent: Send + Sync {
    /// Get the resource identifier
    fn resource_id(&self) -> &str;
    
    /// Get the event type
    fn event_type(&self) -> ResourceEventType;
}

/// Types of resource events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceEventType {
    Acquire,
    Release,
    Lock,
    Unlock,
    Read,
    Write,
    Create,
    Delete,
}

/// Basic implementation of ResourceEvent
#[derive(Debug, Clone)]
pub struct BasicResourceEvent {
    resource_id: String,
    event_type: ResourceEventType,
}

impl BasicResourceEvent {
    pub fn new(resource_id: String, event_type: ResourceEventType) -> Self {
        Self {
            resource_id,
            event_type,
        }
    }
}

impl ResourceEvent for BasicResourceEvent {
    fn resource_id(&self) -> &str {
        &self.resource_id
    }
    
    fn event_type(&self) -> ResourceEventType {
        self.event_type
    }
}