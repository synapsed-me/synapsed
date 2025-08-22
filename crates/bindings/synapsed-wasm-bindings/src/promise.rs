//! Promise Theory implementation for WASM

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use synapsed_promise::{Promise, PromiseState, TrustLevel};

/// WASM-friendly promise wrapper
#[wasm_bindgen]
pub struct WasmPromise {
    id: String,
    body: String,
    promiser: String,
    promisee: String,
    state: String,
}

#[wasm_bindgen]
impl WasmPromise {
    /// Create a new promise
    #[wasm_bindgen(constructor)]
    pub fn new(body: String, promiser: String, promisee: String) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            id,
            body,
            promiser,
            promisee,
            state: "pending".to_string(),
        }
    }
    
    /// Get the promise ID
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }
    
    /// Get the promise body
    #[wasm_bindgen(getter)]
    pub fn body(&self) -> String {
        self.body.clone()
    }
    
    /// Get the promiser
    #[wasm_bindgen(getter)]
    pub fn promiser(&self) -> String {
        self.promiser.clone()
    }
    
    /// Get the promisee
    #[wasm_bindgen(getter)]
    pub fn promisee(&self) -> String {
        self.promisee.clone()
    }
    
    /// Get the current state
    #[wasm_bindgen(getter)]
    pub fn state(&self) -> String {
        self.state.clone()
    }
    
    /// Accept the promise (as promisee)
    pub fn accept(&mut self) -> Result<(), JsValue> {
        if self.state != "pending" {
            return Err(JsValue::from_str("Promise is not pending"));
        }
        self.state = "accepted".to_string();
        Ok(())
    }
    
    /// Fulfill the promise (as promiser)
    pub fn fulfill(&mut self) -> Result<(), JsValue> {
        if self.state != "accepted" {
            return Err(JsValue::from_str("Promise must be accepted first"));
        }
        self.state = "fulfilled".to_string();
        Ok(())
    }
    
    /// Break the promise
    pub fn break_promise(&mut self, reason: String) -> Result<(), JsValue> {
        self.state = format!("broken: {}", reason);
        Ok(())
    }
    
    /// Convert to JSON
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let json = serde_json::json!({
            "id": self.id,
            "body": self.body,
            "promiser": self.promiser,
            "promisee": self.promisee,
            "state": self.state,
        });
        
        serde_wasm_bindgen::to_value(&json)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

/// Trust model for agents
#[wasm_bindgen]
pub struct WasmTrustModel {
    agents: Vec<String>,
    trust_levels: Vec<f64>,
}

#[wasm_bindgen]
impl WasmTrustModel {
    /// Create a new trust model
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            trust_levels: Vec::new(),
        }
    }
    
    /// Add an agent with initial trust level
    pub fn add_agent(&mut self, agent_id: String, initial_trust: f64) -> Result<(), JsValue> {
        if initial_trust < 0.0 || initial_trust > 1.0 {
            return Err(JsValue::from_str("Trust level must be between 0 and 1"));
        }
        
        self.agents.push(agent_id);
        self.trust_levels.push(initial_trust);
        Ok(())
    }
    
    /// Get trust level for an agent
    pub fn get_trust(&self, agent_id: String) -> f64 {
        self.agents.iter()
            .position(|a| a == &agent_id)
            .and_then(|i| self.trust_levels.get(i))
            .copied()
            .unwrap_or(0.5) // Default trust
    }
    
    /// Update trust based on promise fulfillment
    pub fn update_trust(&mut self, agent_id: String, fulfilled: bool) -> Result<(), JsValue> {
        if let Some(pos) = self.agents.iter().position(|a| a == &agent_id) {
            if let Some(trust) = self.trust_levels.get_mut(pos) {
                if fulfilled {
                    *trust = (*trust * 0.9 + 0.1).min(1.0); // Increase trust
                } else {
                    *trust = (*trust * 0.9).max(0.0); // Decrease trust
                }
            }
        }
        Ok(())
    }
    
    /// Get all agents and their trust levels as JSON
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let mut agents_trust = Vec::new();
        for (i, agent) in self.agents.iter().enumerate() {
            if let Some(trust) = self.trust_levels.get(i) {
                agents_trust.push(serde_json::json!({
                    "agent": agent,
                    "trust": trust,
                }));
            }
        }
        
        serde_wasm_bindgen::to_value(&agents_trust)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

/// Cooperation protocol between agents
#[wasm_bindgen]
pub struct WasmCooperationProtocol {
    protocol_id: String,
    participants: Vec<String>,
    promises: Vec<WasmPromise>,
}

#[wasm_bindgen]
impl WasmCooperationProtocol {
    /// Create a new cooperation protocol
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            protocol_id: uuid::Uuid::new_v4().to_string(),
            participants: Vec::new(),
            promises: Vec::new(),
        }
    }
    
    /// Add a participant
    pub fn add_participant(&mut self, agent_id: String) -> Result<(), JsValue> {
        if !self.participants.contains(&agent_id) {
            self.participants.push(agent_id);
        }
        Ok(())
    }
    
    /// Create a promise between participants
    pub fn create_promise(
        &mut self, 
        body: String, 
        promiser: String, 
        promisee: String
    ) -> Result<WasmPromise, JsValue> {
        if !self.participants.contains(&promiser) {
            return Err(JsValue::from_str("Promiser not in protocol"));
        }
        if !self.participants.contains(&promisee) {
            return Err(JsValue::from_str("Promisee not in protocol"));
        }
        
        let promise = WasmPromise::new(body, promiser, promisee);
        self.promises.push(promise.clone());
        Ok(promise)
    }
    
    /// Get all promises as JSON
    pub fn get_promises(&self) -> Result<JsValue, JsValue> {
        let promises_json: Vec<_> = self.promises.iter()
            .map(|p| serde_json::json!({
                "id": p.id(),
                "body": p.body(),
                "promiser": p.promiser(),
                "promisee": p.promisee(),
                "state": p.state(),
            }))
            .collect();
        
        serde_wasm_bindgen::to_value(&promises_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

impl Clone for WasmPromise {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            body: self.body.clone(),
            promiser: self.promiser.clone(),
            promisee: self.promisee.clone(),
            state: self.state.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    
    #[wasm_bindgen_test]
    fn test_promise_creation() {
        let promise = WasmPromise::new(
            "Deliver data".to_string(),
            "agent1".to_string(),
            "agent2".to_string()
        );
        
        assert_eq!(promise.body(), "Deliver data");
        assert_eq!(promise.promiser(), "agent1");
        assert_eq!(promise.promisee(), "agent2");
        assert_eq!(promise.state(), "pending");
    }
    
    #[wasm_bindgen_test]
    fn test_trust_model() {
        let mut trust = WasmTrustModel::new();
        trust.add_agent("agent1".to_string(), 0.7).unwrap();
        
        assert_eq!(trust.get_trust("agent1".to_string()), 0.7);
        assert_eq!(trust.get_trust("unknown".to_string()), 0.5); // Default
        
        trust.update_trust("agent1".to_string(), true).unwrap();
        assert!(trust.get_trust("agent1".to_string()) > 0.7);
    }
}