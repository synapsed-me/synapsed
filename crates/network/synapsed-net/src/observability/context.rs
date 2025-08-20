//! Observability context management.

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use uuid::Uuid;

thread_local! {
    /// Thread-local observability context.
    pub static OBSERVABILITY_CONTEXT: RefCell<Option<ObservabilityContext>> = RefCell::new(None);
}

/// Context for observability operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityContext {
    /// Trace ID for distributed tracing
    pub trace_id: Uuid,
    
    /// Span ID for this operation
    pub span_id: Uuid,
    
    /// Parent span ID if this is a child operation
    pub parent_span: Option<Uuid>,
    
    /// Baggage items for context propagation
    pub baggage: HashMap<String, String>,
    
    /// Privacy level for this context
    pub privacy_level: PrivacyLevel,
}

impl ObservabilityContext {
    /// Creates a new root context.
    pub fn new() -> Self {
        Self {
            trace_id: Uuid::new_v4(),
            span_id: Uuid::new_v4(),
            parent_span: None,
            baggage: HashMap::new(),
            privacy_level: PrivacyLevel::Standard,
        }
    }
    
    /// Creates a child context from this context.
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id,
            span_id: Uuid::new_v4(),
            parent_span: Some(self.span_id),
            baggage: self.baggage.clone(),
            privacy_level: self.privacy_level,
        }
    }
    
    /// Adds a baggage item to the context.
    pub fn add_baggage(&mut self, key: String, value: String) {
        self.baggage.insert(key, value);
    }
    
    /// Sets the privacy level for this context.
    pub fn with_privacy_level(mut self, level: PrivacyLevel) -> Self {
        self.privacy_level = level;
        self
    }
    
    /// Gets the current context from thread-local storage.
    pub fn current() -> Option<ObservabilityContext> {
        OBSERVABILITY_CONTEXT.with(|ctx| ctx.borrow().clone())
    }
    
    /// Sets the current context in thread-local storage.
    pub fn set_current(context: ObservabilityContext) {
        OBSERVABILITY_CONTEXT.with(|ctx| {
            *ctx.borrow_mut() = Some(context);
        });
    }
    
    /// Clears the current context from thread-local storage.
    pub fn clear_current() {
        OBSERVABILITY_CONTEXT.with(|ctx| {
            *ctx.borrow_mut() = None;
        });
    }
    
    /// Runs a function with this context set as current.
    pub fn run<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let previous = Self::current();
        Self::set_current(self.clone());
        
        let result = f();
        
        match previous {
            Some(ctx) => Self::set_current(ctx),
            None => Self::clear_current(),
        }
        
        result
    }
}

impl Default for ObservabilityContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Privacy levels for observability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivacyLevel {
    /// No special privacy requirements
    None,
    
    /// Standard privacy (default)
    Standard,
    
    /// High privacy - minimal logging
    High,
    
    /// Maximum privacy - no identifying information
    Maximum,
}

impl PrivacyLevel {
    /// Checks if a field should be logged at this privacy level.
    pub fn should_log_field(&self, field: &str) -> bool {
        match self {
            PrivacyLevel::None => true,
            PrivacyLevel::Standard => !STANDARD_PRIVATE_FIELDS.contains(&field),
            PrivacyLevel::High => HIGH_PRIVACY_ALLOWED_FIELDS.contains(&field),
            PrivacyLevel::Maximum => MAXIMUM_PRIVACY_ALLOWED_FIELDS.contains(&field),
        }
    }
    
    /// Checks if metrics should be collected at this privacy level.
    pub fn should_collect_metrics(&self) -> bool {
        match self {
            PrivacyLevel::None | PrivacyLevel::Standard => true,
            PrivacyLevel::High => true,
            PrivacyLevel::Maximum => false,
        }
    }
}

// Fields that should not be logged at standard privacy level
const STANDARD_PRIVATE_FIELDS: &[&str] = &[
    "ip_address",
    "user_id",
    "email",
    "phone",
    "name",
    "address",
    "credit_card",
    "ssn",
    "private_key",
    "password",
];

// Fields allowed at high privacy level
const HIGH_PRIVACY_ALLOWED_FIELDS: &[&str] = &[
    "timestamp",
    "duration",
    "count",
    "error_type",
    "protocol",
    "transport_type",
];

// Fields allowed at maximum privacy level
const MAXIMUM_PRIVACY_ALLOWED_FIELDS: &[&str] = &[
    "timestamp",
    "count",
];