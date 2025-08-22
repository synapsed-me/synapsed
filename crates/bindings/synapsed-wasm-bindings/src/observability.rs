//! Observability hooks for WASM using Substrates patterns

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use web_sys::console;

/// Event emitter for WASM environments
#[wasm_bindgen]
pub struct WasmEventEmitter {
    circuit_id: String,
    events: Vec<WasmEvent>,
    listeners: Vec<js_sys::Function>,
}

/// Event structure for observability
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmEvent {
    id: String,
    event_type: String,
    timestamp: f64,
    data: String,
    source: String,
}

#[wasm_bindgen]
impl WasmEvent {
    #[wasm_bindgen(constructor)]
    pub fn new(event_type: String, data: String, source: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            event_type,
            timestamp: js_sys::Date::now(),
            data,
            source,
        }
    }
    
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }
    
    #[wasm_bindgen(getter)]
    pub fn event_type(&self) -> String {
        self.event_type.clone()
    }
    
    #[wasm_bindgen(getter)]
    pub fn timestamp(&self) -> f64 {
        self.timestamp
    }
    
    #[wasm_bindgen(getter)]
    pub fn data(&self) -> String {
        self.data.clone()
    }
    
    #[wasm_bindgen(getter)]
    pub fn source(&self) -> String {
        self.source.clone()
    }
}

#[wasm_bindgen]
impl WasmEventEmitter {
    #[wasm_bindgen(constructor)]
    pub fn new(circuit_id: String) -> Self {
        console::log_1(&format!("Creating event emitter for circuit: {}", circuit_id).into());
        Self {
            circuit_id,
            events: Vec::new(),
            listeners: Vec::new(),
        }
    }
    
    /// Emit an event
    pub fn emit(&mut self, event: WasmEvent) -> Result<(), JsValue> {
        // Log to console
        console::log_1(&format!(
            "[{}] Event: {} - {}",
            self.circuit_id,
            event.event_type(),
            event.data()
        ).into());
        
        // Store event
        self.events.push(event.clone());
        
        // Notify all listeners
        for listener in &self.listeners {
            let event_js = serde_wasm_bindgen::to_value(&serde_json::json!({
                "id": event.id(),
                "type": event.event_type(),
                "timestamp": event.timestamp(),
                "data": event.data(),
                "source": event.source(),
            }))?;
            
            listener.call1(&JsValue::NULL, &event_js)?;
        }
        
        Ok(())
    }
    
    /// Add an event listener (JavaScript function)
    pub fn add_listener(&mut self, listener: js_sys::Function) {
        self.listeners.push(listener);
    }
    
    /// Get all events as JSON
    pub fn get_events(&self) -> Result<JsValue, JsValue> {
        let events_json: Vec<_> = self.events.iter()
            .map(|e| serde_json::json!({
                "id": e.id(),
                "type": e.event_type(),
                "timestamp": e.timestamp(),
                "data": e.data(),
                "source": e.source(),
            }))
            .collect();
        
        serde_wasm_bindgen::to_value(&events_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    
    /// Clear all events
    pub fn clear_events(&mut self) {
        self.events.clear();
    }
}

/// Metrics collector for WASM
#[wasm_bindgen]
pub struct WasmMetricsCollector {
    metrics: Vec<WasmMetric>,
    start_time: f64,
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmMetric {
    name: String,
    value: f64,
    unit: String,
    timestamp: f64,
    labels: String, // JSON string of labels
}

#[wasm_bindgen]
impl WasmMetric {
    #[wasm_bindgen(constructor)]
    pub fn new(name: String, value: f64, unit: String) -> Self {
        Self {
            name,
            value,
            unit,
            timestamp: js_sys::Date::now(),
            labels: "{}".to_string(),
        }
    }
    
    /// Add labels as JSON string
    pub fn with_labels(mut self, labels: String) -> Self {
        self.labels = labels;
        self
    }
    
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }
    
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> f64 {
        self.value
    }
    
    #[wasm_bindgen(getter)]
    pub fn unit(&self) -> String {
        self.unit.clone()
    }
    
    #[wasm_bindgen(getter)]
    pub fn timestamp(&self) -> f64 {
        self.timestamp
    }
}

#[wasm_bindgen]
impl WasmMetricsCollector {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            metrics: Vec::new(),
            start_time: js_sys::Date::now(),
        }
    }
    
    /// Record a metric
    pub fn record(&mut self, metric: WasmMetric) {
        console::log_1(&format!(
            "Metric: {} = {} {}",
            metric.name(),
            metric.value(),
            metric.unit()
        ).into());
        
        self.metrics.push(metric);
    }
    
    /// Record execution time
    pub fn record_execution_time(&mut self, operation: String, duration_ms: f64) {
        let metric = WasmMetric::new(
            format!("{}_duration", operation),
            duration_ms,
            "ms".to_string(),
        );
        self.record(metric);
    }
    
    /// Record a count
    pub fn record_count(&mut self, name: String, count: f64) {
        let metric = WasmMetric::new(name, count, "count".to_string());
        self.record(metric);
    }
    
    /// Get all metrics as JSON
    pub fn get_metrics(&self) -> Result<JsValue, JsValue> {
        let metrics_json: Vec<_> = self.metrics.iter()
            .map(|m| serde_json::json!({
                "name": m.name(),
                "value": m.value(),
                "unit": m.unit(),
                "timestamp": m.timestamp(),
                "labels": serde_json::from_str::<serde_json::Value>(&m.labels).unwrap_or(serde_json::json!({})),
            }))
            .collect();
        
        serde_wasm_bindgen::to_value(&metrics_json)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    
    /// Get summary statistics
    pub fn get_summary(&self) -> Result<JsValue, JsValue> {
        let total_metrics = self.metrics.len();
        let uptime_ms = js_sys::Date::now() - self.start_time;
        
        let summary = serde_json::json!({
            "total_metrics": total_metrics,
            "uptime_ms": uptime_ms,
            "start_time": self.start_time,
        });
        
        serde_wasm_bindgen::to_value(&summary)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

/// Trace span for WASM
#[wasm_bindgen]
pub struct WasmTraceSpan {
    id: String,
    name: String,
    start_time: f64,
    end_time: Option<f64>,
    parent_id: Option<String>,
    attributes: String, // JSON string
}

#[wasm_bindgen]
impl WasmTraceSpan {
    #[wasm_bindgen(constructor)]
    pub fn new(name: String) -> Self {
        console::log_1(&format!("Starting span: {}", name).into());
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            start_time: js_sys::Date::now(),
            end_time: None,
            parent_id: None,
            attributes: "{}".to_string(),
        }
    }
    
    /// Create a child span
    pub fn child(&self, name: String) -> WasmTraceSpan {
        let mut child = WasmTraceSpan::new(name);
        child.parent_id = Some(self.id.clone());
        child
    }
    
    /// End the span
    pub fn end(&mut self) {
        self.end_time = Some(js_sys::Date::now());
        let duration = self.end_time.unwrap() - self.start_time;
        console::log_1(&format!(
            "Ending span: {} (duration: {:.2}ms)",
            self.name,
            duration
        ).into());
    }
    
    /// Set attributes as JSON string
    pub fn set_attributes(&mut self, attributes: String) -> Result<(), JsValue> {
        // Validate it's valid JSON
        let _: serde_json::Value = serde_json::from_str(&attributes)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.attributes = attributes;
        Ok(())
    }
    
    /// Get span info as JSON
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let json = serde_json::json!({
            "id": self.id,
            "name": self.name,
            "start_time": self.start_time,
            "end_time": self.end_time,
            "duration_ms": self.end_time.map(|e| e - self.start_time),
            "parent_id": self.parent_id,
            "attributes": serde_json::from_str::<serde_json::Value>(&self.attributes).unwrap_or(serde_json::json!({})),
        });
        
        serde_wasm_bindgen::to_value(&json)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

/// Observable wrapper for any operation
#[wasm_bindgen]
pub struct WasmObservable {
    name: String,
    emitter: WasmEventEmitter,
    metrics: WasmMetricsCollector,
    current_span: Option<WasmTraceSpan>,
}

#[wasm_bindgen]
impl WasmObservable {
    #[wasm_bindgen(constructor)]
    pub fn new(name: String) -> Self {
        Self {
            name: name.clone(),
            emitter: WasmEventEmitter::new(name.clone()),
            metrics: WasmMetricsCollector::new(),
            current_span: None,
        }
    }
    
    /// Start a traced operation
    pub fn start_operation(&mut self, operation_name: String) -> Result<(), JsValue> {
        let span = WasmTraceSpan::new(operation_name.clone());
        
        // Emit start event
        let event = WasmEvent::new(
            "operation_started".to_string(),
            operation_name,
            self.name.clone(),
        );
        self.emitter.emit(event)?;
        
        self.current_span = Some(span);
        Ok(())
    }
    
    /// End the current operation
    pub fn end_operation(&mut self) -> Result<(), JsValue> {
        if let Some(mut span) = self.current_span.take() {
            span.end();
            
            // Record duration metric
            if let Some(end_time) = span.end_time {
                let duration = end_time - span.start_time;
                self.metrics.record_execution_time(span.name.clone(), duration);
            }
            
            // Emit end event
            let event = WasmEvent::new(
                "operation_ended".to_string(),
                span.name.clone(),
                self.name.clone(),
            );
            self.emitter.emit(event)?;
        }
        Ok(())
    }
    
    /// Get a summary of all observations
    pub fn get_summary(&self) -> Result<JsValue, JsValue> {
        let events_js = self.emitter.get_events()?;
        let metrics_js = self.metrics.get_metrics()?;
        
        // Convert JsValue back to serde_json::Value
        let events: serde_json::Value = serde_wasm_bindgen::from_value(events_js)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let metrics: serde_json::Value = serde_wasm_bindgen::from_value(metrics_js)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        let summary = serde_json::json!({
            "name": self.name,
            "events": events,
            "metrics": metrics,
        });
        
        serde_wasm_bindgen::to_value(&summary)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    
    #[wasm_bindgen_test]
    fn test_event_emitter() {
        let mut emitter = WasmEventEmitter::new("test-circuit".to_string());
        let event = WasmEvent::new(
            "test_event".to_string(),
            "test data".to_string(),
            "test".to_string(),
        );
        
        emitter.emit(event).unwrap();
        let events = emitter.get_events().unwrap();
        assert!(!events.is_null());
    }
    
    #[wasm_bindgen_test]
    fn test_metrics_collector() {
        let mut collector = WasmMetricsCollector::new();
        collector.record_count("test_count".to_string(), 42.0);
        collector.record_execution_time("test_op".to_string(), 100.5);
        
        let metrics = collector.get_metrics().unwrap();
        assert!(!metrics.is_null());
    }
}