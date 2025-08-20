//! Cortex - main entry point into the Substrates runtime
//! Direct port of Java Substrates Cortex interface
//! 
//! This is a refactored version that separates generic methods into an extension trait

use crate::circuit::{BasicCircuit, Circuit, BasicScope};
use crate::types::{Name, Slot, State, SubstratesResult};
use crate::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// The main entry point into the underlying substrates runtime
/// Direct port of Java Substrates Cortex interface
/// 
/// Note: Generic methods have been moved to the CortexExt extension trait
/// to maintain object-safety while preserving the Java API
#[async_trait]
pub trait Cortex: Send + Sync {
    /// Returns a newly created circuit instance
    async fn circuit(&self) -> SubstratesResult<Arc<dyn Circuit>>;
    
    /// Returns a newly created circuit instance with name
    async fn circuit_named(&self, name: Name) -> SubstratesResult<Arc<dyn Circuit>>;
    
    /// Returns a new name from a string path
    fn name_from_str(&self, path: &str) -> Name;
    
    /// Returns a new name from an enum
    fn name_from_enum(&self, path: &dyn std::fmt::Display) -> Name;
    
    /// Returns a new scope instance that manages a provided resource
    fn scope(&self) -> SubstratesResult<Arc<dyn crate::circuit::Scope>>;
    
    /// Returns a new named scope instance
    fn scope_named(&self, name: Name) -> SubstratesResult<Arc<dyn crate::circuit::Scope>>;
    
    /// Creates a slot with boolean value
    fn slot_bool(&self, name: Name, value: bool) -> Slot<bool>;
    
    /// Creates a slot with i32 value
    fn slot_i32(&self, name: Name, value: i32) -> Slot<i32>;
    
    /// Creates a slot with i64 value
    fn slot_i64(&self, name: Name, value: i64) -> Slot<i64>;
    
    /// Creates a slot with f64 value
    fn slot_f64(&self, name: Name, value: f64) -> Slot<f64>;
    
    /// Creates a slot with f32 value
    fn slot_f32(&self, name: Name, value: f32) -> Slot<f32>;
    
    /// Creates a slot with String value
    fn slot_string(&self, name: Name, value: String) -> Slot<String>;
    
    /// Creates a slot with Name value
    fn slot_name(&self, name: Name, value: Name) -> Slot<Name>;
    
    /// Creates a slot with State value
    fn slot_state(&self, name: Name, value: State) -> Slot<State>;
    
    /// Creates an empty state
    fn state_empty(&self) -> State;
    
    /// Creates a state with a single i32 slot
    fn state_with_i32(&self, name: Name, value: i32) -> State;
    
    /// Creates a state with a single i64 slot
    fn state_with_i64(&self, name: Name, value: i64) -> State;
    
    /// Creates a state with a single f32 slot
    fn state_with_f32(&self, name: Name, value: f32) -> State;
    
    /// Creates a state with a single f64 slot
    fn state_with_f64(&self, name: Name, value: f64) -> State;
    
    /// Creates a state with a single bool slot
    fn state_with_bool(&self, name: Name, value: bool) -> State;
    
    /// Creates a state with a single String slot
    fn state_with_string(&self, name: Name, value: String) -> State;
    
    /// Creates a state with a single Name slot
    fn state_with_name(&self, name: Name, value: Name) -> State;
    
    /// Creates a state with a single State slot
    fn state_with_state(&self, name: Name, value: State) -> State;
}

/// Default implementation of Cortex
pub struct DefaultCortex {
    circuits: parking_lot::RwLock<HashMap<Name, Arc<dyn Circuit>>>,
}

impl std::fmt::Debug for DefaultCortex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultCortex")
            .field("circuits_count", &self.circuits.read().len())
            .finish()
    }
}

impl DefaultCortex {
    /// Create a new Cortex instance
    pub fn new() -> Self {
        Self {
            circuits: parking_lot::RwLock::new(HashMap::new()),
        }
    }
}

impl Default for DefaultCortex {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Cortex for DefaultCortex {
    async fn circuit(&self) -> SubstratesResult<Arc<dyn Circuit>> {
        let name = Name::from_part("circuit");
        self.circuit_named(name).await
    }
    
    async fn circuit_named(&self, name: Name) -> SubstratesResult<Arc<dyn Circuit>> {
        // Check if circuit already exists
        if let Some(circuit) = self.circuits.read().get(&name) {
            return Ok(circuit.clone());
        }
        
        // Create new circuit
        let circuit: Arc<dyn Circuit> = Arc::new(BasicCircuit::new(name.clone()));
        self.circuits.write().insert(name, circuit.clone());
        
        Ok(circuit)
    }
    
    fn name_from_str(&self, path: &str) -> Name {
        Name::parse(path)
    }
    
    fn name_from_enum(&self, path: &dyn std::fmt::Display) -> Name {
        Name::from_part(path.to_string())
    }
    
    fn scope(&self) -> SubstratesResult<Arc<dyn crate::circuit::Scope>> {
        let name = Name::from_part("scope");
        self.scope_named(name)
    }
    
    fn scope_named(&self, name: Name) -> SubstratesResult<Arc<dyn crate::circuit::Scope>> {
        Ok(Arc::new(BasicScope::new(name)))
    }
    
    fn slot_bool(&self, name: Name, value: bool) -> Slot<bool> {
        Slot::new(name, value)
    }
    
    fn slot_i32(&self, name: Name, value: i32) -> Slot<i32> {
        Slot::new(name, value)
    }
    
    fn slot_i64(&self, name: Name, value: i64) -> Slot<i64> {
        Slot::new(name, value)
    }
    
    fn slot_f64(&self, name: Name, value: f64) -> Slot<f64> {
        Slot::new(name, value)
    }
    
    fn slot_f32(&self, name: Name, value: f32) -> Slot<f32> {
        Slot::new(name, value)
    }
    
    fn slot_string(&self, name: Name, value: String) -> Slot<String> {
        Slot::new(name, value)
    }
    
    fn slot_name(&self, name: Name, value: Name) -> Slot<Name> {
        Slot::new(name, value)
    }
    
    fn slot_state(&self, name: Name, value: State) -> Slot<State> {
        Slot::new(name, value)
    }
    
    fn state_empty(&self) -> State {
        State::new()
    }
    
    fn state_with_i32(&self, name: Name, value: i32) -> State {
        State::with_slot(name, value)
    }
    
    fn state_with_i64(&self, name: Name, value: i64) -> State {
        State::with_slot(name, value)
    }
    
    fn state_with_f32(&self, name: Name, value: f32) -> State {
        State::with_slot(name, value)
    }
    
    fn state_with_f64(&self, name: Name, value: f64) -> State {
        State::with_slot(name, value)
    }
    
    fn state_with_bool(&self, name: Name, value: bool) -> State {
        State::with_slot(name, value)
    }
    
    fn state_with_string(&self, name: Name, value: String) -> State {
        State::with_slot(name, value)
    }
    
    fn state_with_name(&self, name: Name, value: Name) -> State {
        State::with_slot(name, value)
    }
    
    fn state_with_state(&self, name: Name, value: State) -> State {
        State::with_slot(name, value)
    }
}

/// Factory function to create a new Cortex
pub fn create_cortex() -> Arc<dyn Cortex> {
    Arc::new(DefaultCortex::new())
}