//! Subject abstraction - direct port of Java Substrates Subject interface

use crate::types::{Id, Name, State, SubjectType};
use std::fmt;
use std::sync::Arc;

/// A subject represents a referent that maintains identity and state
/// Direct port of the Java Substrates Subject interface
#[derive(Debug, Clone)]
pub struct Subject {
    id: Id,
    name: Name,
    subject_type: SubjectType,
    state: State,
    parent: Option<Box<Subject>>,
}

impl Subject {
    /// Create a new subject
    pub fn new(name: Name, subject_type: SubjectType) -> Self {
        Self {
            id: Id::new(),
            name,
            subject_type,
            state: State::new(),
            parent: None,
        }
    }
    
    /// Create a subject with a specific ID
    pub fn with_id(id: Id, name: Name, subject_type: SubjectType) -> Self {
        Self {
            id,
            name,
            subject_type,
            state: State::new(),
            parent: None,
        }
    }
    
    /// Create a subject with a parent (enclosure)
    pub fn with_parent(name: Name, subject_type: SubjectType, parent: Subject) -> Self {
        Self {
            id: Id::new(),
            name,
            subject_type,
            state: State::new(),
            parent: Some(Box::new(parent)),
        }
    }
    
    /// Returns a unique identifier for this subject
    pub fn id(&self) -> &Id {
        &self.id
    }
    
    /// The Name associated with this subject
    pub fn name(&self) -> &Name {
        &self.name
    }
    
    /// Returns the subject type
    pub fn subject_type(&self) -> &SubjectType {
        &self.subject_type
    }
    
    /// Returns the current state of this subject
    pub fn state(&self) -> &State {
        &self.state
    }
    
    /// Get mutable reference to state
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
    
    /// Returns the (parent/prefix) subject that encloses this subject
    pub fn enclosure(&self) -> Option<&Subject> {
        self.parent.as_ref().map(|p| p.as_ref())
    }
    
    /// Returns the outermost (extreme) subject
    pub fn extremity(&self) -> &Subject {
        match &self.parent {
            None => self,
            Some(parent) => parent.extremity(),
        }
    }
    
    /// Returns the depth of this subject within all enclosures
    pub fn depth(&self) -> usize {
        match &self.parent {
            None => 0,
            Some(parent) => parent.depth() + 1,
        }
    }
    
    /// Returns a representation of just this subject
    pub fn part(&self) -> String {
        format!(
            "Subject[name={},type={},id={}]",
            self.name, self.subject_type, self.id
        )
    }
    
    /// Returns a representation of the subject, including enclosing subjects
    pub fn path(&self) -> String {
        self.path_with_separator('/')
    }
    
    /// Returns a representation with custom separator
    pub fn path_with_separator(&self, separator: char) -> String {
        match &self.parent {
            None => self.name.to_path(),
            Some(parent) => {
                format!("{}{}{}", parent.path_with_separator(separator), separator, self.name.to_path())
            }
        }
    }
    
    /// Returns true if this Subject is directly or indirectly enclosed within the enclosure parameter
    pub fn within(&self, enclosure: &Subject) -> bool {
        let mut current = self.enclosure();
        while let Some(parent) = current {
            if parent.id() == enclosure.id() {
                return true;
            }
            current = parent.enclosure();
        }
        false
    }
    
    /// Create an iterator over all enclosing subjects
    pub fn ancestors(&self) -> Vec<&Subject> {
        let mut ancestors = Vec::new();
        let mut current = self.enclosure();
        while let Some(parent) = current {
            ancestors.push(parent);
            current = parent.enclosure();
        }
        ancestors
    }
}

impl PartialEq for Subject {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Subject {}

impl fmt::Display for Subject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path())
    }
}

/// Base trait for all substrate components that have an associated subject
/// Direct port of Java Substrates Substrate interface
pub trait Substrate {
    /// Returns the subject identifying this substrate
    fn subject(&self) -> &Subject;
}

/// Trait for managed event-sourcing components
/// Direct port of Java Substrates Component interface
/// Note: Generic parameter E moved to associated type for object-safety
pub trait Component: Substrate + Resource {
    /// The emission type for this component
    type Emission;
    
    /// Returns the source provided by this component
    fn source(&self) -> &dyn crate::source::Source<Self::Emission>;
}

/// Object-safe version of Component that can be used with trait objects
pub trait DynComponent: Substrate + Resource {
    /// Returns the source as a type-erased reference
    /// Use the extension trait methods for type-safe access
    fn source_dyn(&self) -> &dyn std::any::Any;
}

/// Trait for resource management
/// Direct port of Java Substrates Resource interface
pub trait Resource {
    /// Method called to indicate no more usage will be made of the instance
    fn close(&mut self) {
        // Default implementation does nothing
    }
}

/// Trait for providing access to a source
/// Direct port of Java Substrates Context interface
/// Note: Generic parameter E moved to associated type for object-safety
pub trait Context {
    /// The emission type for this context
    type Emission;
    
    /// Returns the source provided by this context
    fn source(&self) -> &dyn crate::source::Source<Self::Emission>;
}

/// Object-safe version of Context that can be used with trait objects
pub trait DynContext {
    /// Returns the source as a type-erased reference
    /// Use the extension trait methods for type-safe access  
    fn source_dyn(&self) -> &dyn std::any::Any;
}


/// Trait for connecting outlet pipes with emitting subjects within a source
/// Direct port of Java Substrates Subscriber interface
/// Note: Generic parameter E moved to associated type for object-safety
pub trait Subscriber: Send + Sync {
    /// The emission type for this subscriber
    type Emission;
    
    /// Called when a new subject emits as an emission
    fn accept(&mut self, subject: &Subject, registrar: &mut dyn Registrar<Emission = Self::Emission>);
}

/// Object-safe version of Subscriber that can be used with trait objects
pub trait DynSubscriber: Send + Sync {
    /// Called when a new subject emits as an emission
    /// Use type-erased registrar for dynamic dispatch
    fn accept_dyn(&mut self, subject: &Subject, registrar: &mut dyn std::any::Any);
}

/// Trait for linking a Subject to a Pipe
/// Direct port of Java Substrates Registrar interface
/// Note: Generic parameter E moved to associated type for object-safety
pub trait Registrar {
    /// The emission type for this registrar
    type Emission;
    
    /// Registers a Pipe with a Registrar associated with a Source
    fn register(&mut self, pipe: Arc<dyn crate::pipe::Pipe<Self::Emission>>);
}

/// Object-safe version of Registrar that can be used with trait objects
pub trait DynRegistrar {
    /// Registers a Pipe with type-erased emission type
    /// Use the extension trait methods for type-safe registration
    fn register_dyn(&mut self, pipe: Arc<dyn std::any::Any>);
}

/// Trait for managing and unregistering subscriptions
/// Direct port of Java Substrates Subscription interface
pub trait Subscription: Resource + Substrate {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_subject_creation() {
        let name = Name::from_part("test");
        let subject = Subject::new(name.clone(), SubjectType::Channel);
        
        assert_eq!(subject.name(), &name);
        assert_eq!(subject.subject_type(), &SubjectType::Channel);
        assert_eq!(subject.depth(), 0);
        assert!(subject.enclosure().is_none());
    }
    
    #[test]
    fn test_subject_hierarchy() {
        let root_name = Name::from_part("root");
        let child_name = Name::from_part("child");
        
        let root = Subject::new(root_name.clone(), SubjectType::Circuit);
        let child = Subject::with_parent(child_name.clone(), SubjectType::Channel, root.clone());
        
        assert_eq!(child.depth(), 1);
        assert_eq!(child.enclosure().unwrap().name(), &root_name);
        assert!(child.within(&root));
        assert!(!root.within(&child));
    }
    
    #[test]
    fn test_subject_path() {
        let root = Subject::new(Name::from_part("root"), SubjectType::Circuit);
        let child = Subject::with_parent(Name::from_part("child"), SubjectType::Channel, root);
        
        assert_eq!(child.path(), "root/child");
    }
}