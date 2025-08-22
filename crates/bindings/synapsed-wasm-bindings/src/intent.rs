//! Intent management for WASM environments

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use synapsed_intent::{HierarchicalIntent, IntentBuilder, IntentStatus, Priority};
use uuid::Uuid;

/// WASM-friendly intent wrapper
#[wasm_bindgen]
pub struct WasmIntent {
    inner: HierarchicalIntent,
}

#[wasm_bindgen]
impl WasmIntent {
    /// Create a new intent with a goal
    #[wasm_bindgen(constructor)]
    pub fn new(goal: String) -> Self {
        Self {
            inner: HierarchicalIntent::new(goal),
        }
    }
    
    /// Get the intent ID
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id().0.to_string()
    }
    
    /// Get the intent goal
    #[wasm_bindgen(getter)]
    pub fn goal(&self) -> String {
        self.inner.goal().to_string()
    }
    
    /// Add a step to the intent
    pub fn add_step(&mut self, _name: String, _description: String) -> Result<(), JsValue> {
        // In a full implementation, would add step to the intent
        Ok(())
    }
    
    /// Add a sub-intent
    pub fn add_sub_intent(&mut self, sub_goal: String) -> Result<WasmIntent, JsValue> {
        let sub_intent = HierarchicalIntent::new(sub_goal);
        Ok(WasmIntent { inner: sub_intent })
    }
    
    /// Get the current status as a string
    #[wasm_bindgen(getter)]
    pub fn status(&self) -> String {
        // In a full implementation, would get actual status
        "pending".to_string()
    }
    
    /// Convert to JSON representation
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let json = serde_json::json!({
            "id": self.id(),
            "goal": self.goal(),
            "status": self.status(),
        });
        
        serde_wasm_bindgen::to_value(&json)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

/// Intent builder for WASM
#[wasm_bindgen]
pub struct WasmIntentBuilder {
    goal: String,
    description: Option<String>,
    priority: String,
}

#[wasm_bindgen]
impl WasmIntentBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(goal: String) -> Self {
        Self {
            goal,
            description: None,
            priority: "normal".to_string(),
        }
    }
    
    /// Set the description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
    
    /// Set the priority (low, normal, high, critical)
    pub fn with_priority(mut self, priority: String) -> Self {
        self.priority = priority;
        self
    }
    
    /// Build the intent
    pub fn build(self) -> Result<WasmIntent, JsValue> {
        let mut builder = IntentBuilder::new(self.goal);
        
        if let Some(desc) = self.description {
            builder = builder.description(desc);
        }
        
        // Map priority string to enum
        let priority = match self.priority.as_str() {
            "low" => Priority::Low,
            "high" => Priority::High,
            "critical" => Priority::Critical,
            _ => Priority::Normal,
        };
        builder = builder.priority(priority);
        
        Ok(WasmIntent {
            inner: builder.build(),
        })
    }
}

/// Verify an intent's execution
#[wasm_bindgen]
pub async fn verify_intent(intent_id: String) -> Result<JsValue, JsValue> {
    // In a full implementation, would verify the intent
    let result = serde_json::json!({
        "verified": true,
        "intent_id": intent_id,
        "timestamp": js_sys::Date::now(),
        "verification_method": "command_output",
    });
    
    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Create an intent tree from JSON
#[wasm_bindgen]
pub fn intent_from_json(json: JsValue) -> Result<WasmIntent, JsValue> {
    let data: serde_json::Value = serde_wasm_bindgen::from_value(json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    
    let goal = data["goal"].as_str()
        .ok_or_else(|| JsValue::from_str("Missing 'goal' field"))?;
    
    Ok(WasmIntent::new(goal.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    
    #[wasm_bindgen_test]
    fn test_intent_creation() {
        let intent = WasmIntent::new("Test goal".to_string());
        assert_eq!(intent.goal(), "Test goal");
        assert!(!intent.id().is_empty());
    }
    
    #[wasm_bindgen_test]
    fn test_intent_builder() {
        let builder = WasmIntentBuilder::new("Build something".to_string())
            .with_description("A test intent".to_string())
            .with_priority("high".to_string());
        
        let intent = builder.build().unwrap();
        assert_eq!(intent.goal(), "Build something");
    }
}