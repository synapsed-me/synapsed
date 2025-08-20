use synapsed_serventis::*;
use synapsed_substrates::SubjectType;
use std::sync::{Arc, Mutex};

// Simple probe implementation for testing
#[derive(Debug)]
struct TestProbe {
    subject: Subject,
    measurements: Arc<Mutex<Vec<(f64, String)>>>,
}

impl TestProbe {
    fn new(measurements: Arc<Mutex<Vec<(f64, String)>>>) -> Self {
        Self {
            subject: Subject::new(Name::from_part("test-probe"), SubjectType::Channel),
            measurements,
        }
    }
}

#[async_trait::async_trait]
impl Pipe<Box<dyn Measurement>> for TestProbe {
    async fn emit(&mut self, emission: Box<dyn Measurement>) -> synapsed_substrates::types::SubstratesResult<()> {
        self.measurements.lock().unwrap().push((emission.value(), emission.unit().to_string()));
        Ok(())
    }
}

impl Substrate for TestProbe {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

impl Probe for TestProbe {}

// Helper struct to track measurements
#[derive(Default)]
struct MeasurementTracker {
    measurements: Arc<Mutex<Vec<(f64, String)>>>,
}

impl MeasurementTracker {
    fn new() -> Self {
        Self {
            measurements: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    fn get_measurements(&self) -> Vec<(f64, String)> {
        self.measurements.lock().unwrap().clone()
    }
    
    fn create_probe(&self) -> TestProbe {
        TestProbe::new(self.measurements.clone())
    }
}

#[tokio::test]
async fn test_probe_basic_measurement() {
    let tracker = MeasurementTracker::new();
    let mut probe = tracker.create_probe();
    
    // Test emitting a simple measurement
    let measurement = BasicMeasurement::new(42.5, "celsius".to_string());
    probe.emit(Box::new(measurement)).await.unwrap();
    
    let measurements = tracker.get_measurements();
    assert_eq!(measurements.len(), 1);
    assert_eq!(measurements[0].0, 42.5);
    assert_eq!(measurements[0].1, "celsius");
}

#[tokio::test]
async fn test_probe_multiple_measurements() {
    let tracker = MeasurementTracker::new();
    let mut probe = tracker.create_probe();
    
    // Test emitting multiple measurements
    let values = vec![1.0, 2.5, 3.7, 4.2, 5.9];
    for value in &values {
        let measurement = BasicMeasurement::new(*value, "ms".to_string());
        probe.emit(Box::new(measurement)).await.unwrap();
    }
    
    let measurements = tracker.get_measurements();
    assert_eq!(measurements.len(), 5);
    let values_only: Vec<f64> = measurements.iter().map(|(v, _)| *v).collect();
    assert_eq!(values_only, values);
}

#[tokio::test]
async fn test_probe_edge_cases() {
    let tracker = MeasurementTracker::new();
    let mut probe = tracker.create_probe();
    
    // Test edge case values
    let edge_values = vec![
        0.0,                    // Zero
        -1.0,                   // Negative
        f64::MAX,               // Maximum
        f64::MIN,               // Minimum
        f64::MIN_POSITIVE,      // Smallest positive
        1e-10,                  // Very small
        1e10,                   // Very large
    ];
    
    for value in &edge_values {
        let measurement = BasicMeasurement::new(*value, "units".to_string());
        probe.emit(Box::new(measurement)).await.unwrap();
    }
    
    let measurements = tracker.get_measurements();
    assert_eq!(measurements.len(), edge_values.len());
    
    // Verify all values were captured correctly
    for (i, value) in edge_values.iter().enumerate() {
        assert_eq!(measurements[i].0, *value);
    }
}

#[tokio::test]
async fn test_measurement_implementation() {
    // Test BasicMeasurement directly
    let measurement = BasicMeasurement::new(123.45, "kg".to_string());
    assert_eq!(measurement.value(), 123.45);
    assert_eq!(measurement.unit(), "kg");
    
    // Test with different values
    let measurement2 = BasicMeasurement::new(-67.89, "m/s".to_string());
    assert_eq!(measurement2.value(), -67.89);
    assert_eq!(measurement2.unit(), "m/s");
}

#[tokio::test]
async fn test_probe_high_frequency_measurements() {
    let tracker = MeasurementTracker::new();
    let mut probe = tracker.create_probe();
    
    // Simulate high-frequency measurements
    for i in 0..100 {
        let value = i as f64 * 0.1;
        let measurement = BasicMeasurement::new(value, "Hz".to_string());
        probe.emit(Box::new(measurement)).await.unwrap();
    }
    
    let measurements = tracker.get_measurements();
    assert_eq!(measurements.len(), 100);
    
    // Verify measurements are in order
    for i in 0..100 {
        assert!((measurements[i].0 - (i as f64 * 0.1)).abs() < f64::EPSILON);
    }
}

#[tokio::test]
async fn test_probe_statistical_measurements() {
    let tracker = MeasurementTracker::new();
    let mut probe = tracker.create_probe();
    
    // Generate some statistical data
    let values = vec![10.0, 20.0, 30.0, 40.0, 50.0];
    
    for value in &values {
        let measurement = BasicMeasurement::new(*value, "points".to_string());
        probe.emit(Box::new(measurement)).await.unwrap();
    }
    
    let measurements = tracker.get_measurements();
    
    // Calculate statistics
    let values: Vec<f64> = measurements.iter().map(|(v, _)| *v).collect();
    let sum: f64 = values.iter().sum();
    let mean = sum / values.len() as f64;
    let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    
    assert_eq!(mean, 30.0);
    assert_eq!(min, 10.0);
    assert_eq!(max, 50.0);
}

#[tokio::test]
async fn test_probe_async_behavior() {
    use tokio::time::Duration;
    
    let tracker = MeasurementTracker::new();
    let mut probe = tracker.create_probe();
    
    // Test async measurement emissions
    let start = std::time::Instant::now();
    
    for i in 0..5 {
        let measurement = BasicMeasurement::new(i as f64, "count".to_string());
        probe.emit(Box::new(measurement)).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(50));
    
    let measurements = tracker.get_measurements();
    assert_eq!(measurements.len(), 5);
}

#[tokio::test]
async fn test_probe_pattern_detection() {
    let tracker = MeasurementTracker::new();
    let mut probe = tracker.create_probe();
    
    // Create a sinusoidal pattern
    for i in 0..20 {
        let angle = (i as f64) * std::f64::consts::PI / 10.0;
        let value = angle.sin();
        let measurement = BasicMeasurement::new(value, "radians".to_string());
        probe.emit(Box::new(measurement)).await.unwrap();
    }
    
    let measurements = tracker.get_measurements();
    assert_eq!(measurements.len(), 20);
    
    // Verify the pattern (should oscillate between -1 and 1)
    for (value, _) in &measurements {
        assert!(*value >= -1.0 && *value <= 1.0);
    }
}

#[tokio::test]
async fn test_probe_threshold_detection() {
    let tracker = MeasurementTracker::new();
    let mut probe = tracker.create_probe();
    
    let threshold = 50.0;
    let values = vec![10.0, 30.0, 55.0, 45.0, 60.0, 35.0];
    
    for value in &values {
        let measurement = BasicMeasurement::new(*value, "percent".to_string());
        probe.emit(Box::new(measurement)).await.unwrap();
    }
    
    let measurements = tracker.get_measurements();
    
    // Count values above threshold
    let above_threshold = measurements.iter()
        .filter(|(v, _)| *v > threshold)
        .count();
    
    assert_eq!(above_threshold, 2); // 55.0 and 60.0
}

#[tokio::test]
async fn test_probe_rate_of_change() {
    let tracker = MeasurementTracker::new();
    let mut probe = tracker.create_probe();
    
    // Simulate increasing values
    for i in 0..10 {
        let value = (i * i) as f64; // Quadratic growth
        let measurement = BasicMeasurement::new(value, "m^2".to_string());
        probe.emit(Box::new(measurement)).await.unwrap();
    }
    
    let measurements = tracker.get_measurements();
    
    // Calculate rate of change
    let mut rates = Vec::new();
    for i in 1..measurements.len() {
        let rate = measurements[i].0 - measurements[i-1].0;
        rates.push(rate);
    }
    
    // Verify increasing rate of change (acceleration)
    for i in 1..rates.len() {
        assert!(rates[i] > rates[i-1]);
    }
}