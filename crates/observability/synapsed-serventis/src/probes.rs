//! Probes API - placeholder for future implementation
//! Direct port of Java Serventis Probes interface

use crate::{async_trait, Arc, Composer, Pipe, Substrate};

/// The Probes interface - entry point into the Serventis Probes API
/// Direct port of Java Serventis Probes interface
pub trait Probes: Composer<Arc<dyn Probe>, Box<dyn Measurement>> + Send + Sync {}

/// Probe interface for monitoring and reporting communication outcomes
/// Direct port of Java Serventis Probe interface
#[async_trait]
pub trait Probe: Pipe<Box<dyn Measurement>> + Substrate + Send + Sync {}

/// Measurement interface representing communication outcomes
/// Direct port of Java Serventis Measurement interface (placeholder)
pub trait Measurement: Send + Sync {
    /// Get the measurement value
    fn value(&self) -> f64;
    
    /// Get the measurement unit
    fn unit(&self) -> &str;
}

/// Basic implementation of Measurement
#[derive(Debug, Clone)]
pub struct BasicMeasurement {
    value: f64,
    unit: String,
}

impl BasicMeasurement {
    pub fn new(value: f64, unit: String) -> Self {
        Self { value, unit }
    }
}

impl Measurement for BasicMeasurement {
    fn value(&self) -> f64 {
        self.value
    }
    
    fn unit(&self) -> &str {
        &self.unit
    }
}