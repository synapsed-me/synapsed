use synapsed_substrates::*;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_basic_circuit_creation() {
    let circuit = BasicCircuit::new(Name::from_part("test-circuit"));
    assert_eq!(circuit.subject().name().to_string(), "test-circuit");
}

#[tokio::test]
async fn test_circuit_queue_operations() {
    let circuit = BasicCircuit::new(Name::from_part("test-circuit"));
    let queue = circuit.queue();
    
    // Create a simple script
    struct TestScript {
        executed: Arc<std::sync::Mutex<bool>>,
    }
    
    #[async_trait]
    impl Script for TestScript {
        async fn exec(&self, _current: &dyn Current) -> SubstratesResult<()> {
            *self.executed.lock().unwrap() = true;
            Ok(())
        }
    }
    
    let executed = Arc::new(std::sync::Mutex::new(false));
    let script = Arc::new(TestScript { executed: executed.clone() });
    
    // Post script to queue
    queue.post(script).await.unwrap();
    
    // Wait for execution
    sleep(Duration::from_millis(50)).await;
    
    // Check script was executed
    assert!(*executed.lock().unwrap());
}

#[tokio::test]
async fn test_queue_await_empty() {
    let queue = BasicQueue::new();
    
    // Queue should be empty initially
    queue.await_empty().await;
    
    // Post multiple scripts
    for i in 0..5 {
        struct CountScript {
            count: i32,
        }
        
        #[async_trait]
        impl Script for CountScript {
            async fn exec(&self, _current: &dyn Current) -> SubstratesResult<()> {
                // Simulate work
                sleep(Duration::from_millis(10)).await;
                Ok(())
            }
        }
        
        queue.post(Arc::new(CountScript { count: i })).await.unwrap();
    }
    
    // Wait for all scripts to complete
    queue.await_empty().await;
}

#[tokio::test]
async fn test_basic_channel() {
    let channel = BasicChannel::<String>::new(Name::from_part("test-channel"));
    
    assert_eq!(channel.subject().name().to_string(), "test-channel");
    
    // Get pipe from channel using explicit trait
    let pipe = Channel::pipe(&channel).unwrap();
    assert!(Arc::strong_count(&pipe) > 0);
}

#[tokio::test]
async fn test_channel_pipe() {
    let channel = BasicChannel::<i32>::new(Name::from_part("test-channel"));
    
    // Test both Inlet::pipe and Channel::pipe work
    let inlet_pipe = Inlet::pipe(&channel).unwrap();
    let channel_pipe = Channel::pipe(&channel).unwrap();
    
    // Both should create pipes successfully
    assert!(Arc::strong_count(&inlet_pipe) > 0);
    assert!(Arc::strong_count(&channel_pipe) > 0);
}

#[tokio::test]
async fn test_scope_operations() {
    let scope = BasicScope::new(Name::from_part("test-scope"));
    
    // BasicScope stores a Subject, not direct state access
    // Test that the scope was created with correct subject
    assert_eq!(scope.subject().name().to_string(), "test-scope");
    assert_eq!(*scope.subject().subject_type(), SubjectType::Scope);
}

#[tokio::test]
async fn test_current_implementation() {
    let current = BasicCurrent::new();
    assert_eq!(current.subject().name().to_string(), "queue-current");
    assert_eq!(*current.subject().subject_type(), SubjectType::Script);
}

#[tokio::test]
async fn test_composer_identity() {
    // IdentityComposer<E> implements Composer<Arc<dyn Channel<E>>, E>
    let composer = IdentityComposer::<String>::new();
    let channel: Arc<dyn Channel<String>> = Arc::new(BasicChannel::<String>::new(Name::from_part("test")));
    let result = composer.compose(channel.clone());
    // Identity composer returns the same channel
    assert!(Arc::ptr_eq(&result, &channel));
}

#[tokio::test]
async fn test_pipe_composer() {
    #[derive(Debug)]
    struct TestPipe;
    
    #[async_trait]
    impl Pipe<String> for TestPipe {
        async fn emit(&mut self, _emission: String) -> SubstratesResult<()> {
            Ok(())
        }
    }
    
    let _pipe = Arc::new(TestPipe);
    let _composer = PipeComposer::<String>::new();
    // PipeComposer exists and can be constructed
}