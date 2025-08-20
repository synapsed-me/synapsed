use synapsed_substrates::*;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_empty_pipe() {
    let mut pipe = EmptyPipe::<String>::new();
    
    // Should accept any emission without error
    pipe.emit("test".to_string()).await.unwrap();
    pipe.emit("another".to_string()).await.unwrap();
}

#[tokio::test]
async fn test_function_pipe() {
    let counter = Arc::new(Mutex::new(0));
    let counter_clone = counter.clone();
    
    let mut pipe = FunctionPipe::new(move |value: i32| {
        *counter_clone.lock().unwrap() += value;
        Ok(())
    });
    
    // Emit values
    pipe.emit(5).await.unwrap();
    pipe.emit(10).await.unwrap();
    pipe.emit(15).await.unwrap();
    
    // Check total
    assert_eq!(*counter.lock().unwrap(), 30);
}

#[tokio::test]
async fn test_function_pipe_error() {
    let mut pipe = FunctionPipe::new(|value: i32| {
        if value < 0 {
            Err(SubstratesError::InvalidOperation("Negative value".to_string()))
        } else {
            Ok(())
        }
    });
    
    // Positive values should work
    assert!(pipe.emit(5).await.is_ok());
    
    // Negative values should error
    assert!(pipe.emit(-5).await.is_err());
}

#[tokio::test]
async fn test_assembly_trait() {
    // Test that Assembly trait works properly
    #[derive(Debug, Clone)]
    struct TestAssembly {
        value: i32,
    }
    
    impl Assembly for TestAssembly {}
    
    let assembly = TestAssembly { value: 42 };
    // Assembly trait has no methods to test directly
    assert_eq!(assembly.value, 42);
}

#[tokio::test]
async fn test_path_trait_exists() {
    // Path trait is complex and requires full implementation
    // Just verify the trait exists and basic types implement it
    fn assert_path<T: Path<String>>(_: &T) {}
    
    // This test just verifies the trait exists
    // Real implementations would be tested in integration tests
}

#[tokio::test]
async fn test_sequencer_trait_exists() {
    // Sequencer is a marker trait
    struct TestSequencer;
    impl<P: Assembly> Sequencer<P> for TestSequencer {
        fn apply(&self, _assembly: &mut P) -> SubstratesResult<()> {
            Ok(())
        }
    }
    
    let _sequencer = TestSequencer;
    // Marker trait has no methods to test
}