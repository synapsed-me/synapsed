//! Core types for Substrates API

use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/// Result type for Substrates operations
pub type SubstratesResult<T> = Result<T, SubstratesError>;

/// Errors that can occur in Substrates operations
#[derive(Debug, thiserror::Error)]
pub enum SubstratesError {
    #[error("Resource not found: {0}")]
    NotFound(String),
    #[error("Resource closed: {0}")]
    Closed(String),
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    #[error("Channel error: {0}")]
    ChannelError(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Unique identifier for components
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Id(Uuid);

impl Id {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
    
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
    
    pub fn from_string(s: &str) -> Self {
        // Try to parse as UUID, or create new one if it fails
        let uuid = Uuid::parse_str(s).unwrap_or_else(|_| Uuid::new_v4());
        Self(uuid)
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Hierarchical name similar to Java Substrates Name interface
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Name {
    parts: Vec<String>,
}

impl Name {
    pub const SEPARATOR: char = '.';
    
    pub fn new() -> Self {
        Self { parts: Vec::new() }
    }
    
    pub fn from_part(part: impl Into<String>) -> Self {
        Self { parts: vec![part.into()] }
    }
    
    pub fn parse(path: &str) -> Self {
        if path.is_empty() {
            return Self::new();
        }
        Self {
            parts: path.split(Self::SEPARATOR).map(|s| s.to_string()).collect(),
        }
    }
    
    pub fn from_parts(parts: Vec<String>) -> Self {
        Self { parts }
    }
    
    pub fn append(&self, part: impl Into<String>) -> Self {
        let mut parts = self.parts.clone();
        parts.push(part.into());
        Self { parts }
    }
    
    pub fn append_name(&self, other: &Name) -> Self {
        let mut parts = self.parts.clone();
        parts.extend_from_slice(&other.parts);
        Self { parts }
    }
    
    pub fn parent(&self) -> Option<Self> {
        if self.parts.len() <= 1 {
            return None;
        }
        Some(Self {
            parts: self.parts[..self.parts.len() - 1].to_vec(),
        })
    }
    
    pub fn parts(&self) -> &[String] {
        &self.parts
    }
    
    pub fn depth(&self) -> usize {
        self.parts.len()
    }
    
    pub fn to_path(&self) -> String {
        self.parts.join(&Self::SEPARATOR.to_string())
    }
}

impl Default for Name {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_path())
    }
}

impl From<&str> for Name {
    fn from(s: &str) -> Self {
        Self::parse(s)
    }
}

/// Trait for values that can be stored in State
pub trait StateValue: fmt::Debug + Send + Sync {
    fn clone_box(&self) -> Box<dyn StateValue>;
    fn type_id(&self) -> TypeId;
    fn type_name(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
}

impl<T> StateValue for T
where
    T: fmt::Debug + Clone + Send + Sync + 'static,
{
    fn clone_box(&self) -> Box<dyn StateValue> {
        Box::new(self.clone())
    }
    
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    
    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Clone for Box<dyn StateValue> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// A typed slot containing a named value
#[derive(Debug, Clone)]
pub struct Slot<T> {
    name: Name,
    value: T,
}

impl<T> Slot<T> {
    pub fn new(name: Name, value: T) -> Self {
        Self { name, value }
    }
    
    pub fn name(&self) -> &Name {
        &self.name
    }
    
    pub fn value(&self) -> &T {
        &self.value
    }
    
    pub fn into_value(self) -> T {
        self.value
    }
}

/// Collection of named slots containing typed values
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(skip)]
    slots: HashMap<Name, Box<dyn StateValue>>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_slot<T>(name: Name, value: T) -> Self
    where
        T: StateValue + 'static,
    {
        let mut state = Self::new();
        state.set(name, value);
        state
    }
    
    pub fn set<T>(&mut self, name: Name, value: T)
    where
        T: StateValue + 'static,
    {
        self.slots.insert(name, Box::new(value));
    }
    
    pub fn get<T>(&self, name: &Name) -> Option<&T>
    where
        T: 'static,
    {
        self.slots.get(name)?.as_any().downcast_ref::<T>()
    }
    
    pub fn contains(&self, name: &Name) -> bool {
        self.slots.contains_key(name)
    }
    
    pub fn remove(&mut self, name: &Name) -> Option<Box<dyn StateValue>> {
        self.slots.remove(name)
    }
    
    pub fn len(&self) -> usize {
        self.slots.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
    
    pub fn merge(&self, other: &State) -> State {
        let mut merged = self.clone();
        for (name, value) in &other.slots {
            merged.slots.insert(name.clone(), value.clone());
        }
        merged
    }
    
    pub fn compact(&self) -> State {
        self.clone()
    }
}

/// Subject type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubjectType {
    Channel,
    Circuit,
    Clock,
    Conduit,
    Container,
    Current,
    Queue,
    Source,
    Scope,
    Script,
    Sink,
    Subscriber,
    Subscription,
}

impl fmt::Display for SubjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}