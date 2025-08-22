//! Sink implementation for collecting and draining emissions
//!
//! A Sink collects emissions from pipes and allows them to be drained
//! for inspection, testing, or batch processing.

use crate::circuit::Sink;
use crate::subject::Resource;
use crate::pipe::{Capture, Pipe};
use crate::subject::{Substrate, Subject};
use crate::types::{Name, SubjectType, SubstratesResult, SubstratesError};
use crate::async_trait;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::sync::mpsc;

/// Basic Sink implementation that collects emissions
pub struct BasicSink<E> {
    subject: Subject,
    /// Collected captures
    captures: Arc<RwLock<Vec<Capture<E>>>>,
    /// Channel for receiving emissions
    receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<Capture<E>>>>>,
    /// Sender for the pipe to use
    sender: mpsc::UnboundedSender<Capture<E>>,
    /// Maximum number of captures to retain
    max_captures: usize,
}

impl<E> BasicSink<E> 
where
    E: Send + Sync + 'static,
{
    pub fn new(name: Name) -> Self {
        Self::with_capacity(name, 1000)
    }
    
    pub fn with_capacity(name: Name, max_captures: usize) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        let sink = Self {
            subject: Subject::new(name, SubjectType::Sink),
            captures: Arc::new(RwLock::new(Vec::new())),
            receiver: Arc::new(RwLock::new(Some(receiver))),
            sender,
            max_captures,
        };
        
        // Start background task to collect emissions
        sink.start_collector();
        sink
    }
    
    fn start_collector(&self) {
        let captures = self.captures.clone();
        let max_captures = self.max_captures;
        
        let mut receiver = self.receiver.write().take()
            .expect("Receiver already taken");
        
        tokio::spawn(async move {
            while let Some(capture) = receiver.recv().await {
                let mut captures_guard = captures.write();
                
                // Maintain max capacity by removing oldest
                if captures_guard.len() >= max_captures {
                    captures_guard.remove(0);
                }
                
                captures_guard.push(capture);
            }
        });
    }
    
    /// Create a pipe that emits to this sink
    pub fn create_pipe(&self) -> Arc<dyn Pipe<E>> {
        Arc::new(SinkPipe::new(self.sender.clone(), self.subject.clone()))
    }
    
    /// Get the current number of captured emissions
    pub fn capture_count(&self) -> usize {
        self.captures.read().len()
    }
    
    /// Peek at captures without draining
    pub fn peek(&self) -> Vec<Capture<E>> 
    where
        E: Clone,
    {
        self.captures.read().clone()
    }
}

impl<E> Substrate for BasicSink<E> {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

impl<E> Resource for BasicSink<E> {
    fn close(&mut self) {
        // Close the receiver to stop collection
        self.receiver.write().take();
    }
}

#[async_trait]
impl<E> Sink<E> for BasicSink<E>
where
    E: Send + Sync + 'static,
{
    async fn drain(&mut self) -> SubstratesResult<Vec<Capture<E>>> {
        // Swap out the captures for a new empty vector
        let mut captures_guard = self.captures.write();
        let drained = std::mem::take(&mut *captures_guard);
        Ok(drained)
    }
}

/// Pipe that emits to a sink
struct SinkPipe<E> {
    sender: mpsc::UnboundedSender<Capture<E>>,
    subject: Subject,
}

impl<E> SinkPipe<E> {
    fn new(sender: mpsc::UnboundedSender<Capture<E>>, subject: Subject) -> Self {
        Self { sender, subject }
    }
}

impl<E> std::fmt::Debug for SinkPipe<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SinkPipe")
            .field("subject", &self.subject)
            .finish()
    }
}

#[async_trait]
impl<E> Pipe<E> for SinkPipe<E>
where
    E: Send + Sync,
{
    async fn emit(&mut self, emission: E) -> SubstratesResult<()> {
        let capture = Capture::new(emission, self.subject.clone());
        self.sender
            .send(capture)
            .map_err(|_| SubstratesError::Closed("Sink closed".to_string()))?;
        Ok(())
    }
}

/// Advanced sink with filtering and transformation
pub struct FilteredSink<E, F> {
    inner: BasicSink<E>,
    filter: Arc<F>,
}

impl<E, F> FilteredSink<E, F>
where
    E: Send + Sync + 'static,
    F: Fn(&E) -> bool + Send + Sync + 'static,
{
    pub fn new(name: Name, filter: F) -> Self {
        Self {
            inner: BasicSink::new(name),
            filter: Arc::new(filter),
        }
    }
    
    /// Create a pipe that filters before emitting to the sink
    pub fn create_filtered_pipe(&self) -> Arc<dyn Pipe<E>> 
    where
        E: Clone,
    {
        Arc::new(FilteredSinkPipe::new(
            self.inner.sender.clone(),
            self.inner.subject.clone(),
            self.filter.clone(),
        ))
    }
}

impl<E, F> Substrate for FilteredSink<E, F> {
    fn subject(&self) -> &Subject {
        self.inner.subject()
    }
}

impl<E, F> Resource for FilteredSink<E, F> {
    fn close(&mut self) {
        self.inner.close()
    }
}

#[async_trait]
impl<E, F> Sink<E> for FilteredSink<E, F>
where
    E: Send + Sync + 'static,
    F: Send + Sync,
{
    async fn drain(&mut self) -> SubstratesResult<Vec<Capture<E>>> {
        self.inner.drain().await
    }
}

/// Pipe that filters before emitting to a sink
struct FilteredSinkPipe<E, F> {
    sender: mpsc::UnboundedSender<Capture<E>>,
    subject: Subject,
    filter: Arc<F>,
}

impl<E, F> FilteredSinkPipe<E, F> {
    fn new(sender: mpsc::UnboundedSender<Capture<E>>, subject: Subject, filter: Arc<F>) -> Self {
        Self { sender, subject, filter }
    }
}

impl<E, F> std::fmt::Debug for FilteredSinkPipe<E, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilteredSinkPipe")
            .field("subject", &self.subject)
            .finish()
    }
}

#[async_trait]
impl<E, F> Pipe<E> for FilteredSinkPipe<E, F>
where
    E: Send + Sync + Clone,
    F: Fn(&E) -> bool + Send + Sync,
{
    async fn emit(&mut self, emission: E) -> SubstratesResult<()> {
        if (self.filter)(&emission) {
            let capture = Capture::new(emission, self.subject.clone());
            self.sender
                .send(capture)
                .map_err(|_| SubstratesError::Closed("Sink closed".to_string()))?;
        }
        Ok(())
    }
}

/// Sink that batches captures before processing
pub struct BatchingSink<E> {
    inner: BasicSink<E>,
    batch_size: usize,
    batch_processor: Arc<dyn Fn(Vec<Capture<E>>) + Send + Sync>,
}

impl<E> BatchingSink<E>
where
    E: Send + Sync + 'static,
{
    pub fn new<F>(name: Name, batch_size: usize, processor: F) -> Self
    where
        F: Fn(Vec<Capture<E>>) + Send + Sync + 'static,
    {
        Self {
            inner: BasicSink::new(name),
            batch_size,
            batch_processor: Arc::new(processor),
        }
    }
    
    /// Process a batch if we've reached the batch size
    pub async fn process_if_ready(&mut self) -> SubstratesResult<()> {
        if self.inner.capture_count() >= self.batch_size {
            let batch = self.inner.drain().await?;
            (self.batch_processor)(batch);
        }
        Ok(())
    }
}

impl<E> Substrate for BatchingSink<E> {
    fn subject(&self) -> &Subject {
        self.inner.subject()
    }
}

impl<E> Resource for BatchingSink<E> {
    fn close(&mut self) {
        self.inner.close()
    }
}

#[async_trait]
impl<E> Sink<E> for BatchingSink<E>
where
    E: Send + Sync + Clone + 'static,
{
    async fn drain(&mut self) -> SubstratesResult<Vec<Capture<E>>> {
        let captures = self.inner.drain().await?;
        
        // Process any remaining captures
        if !captures.is_empty() {
            // Clone for the processor since we need to return the captures
            let for_processing = captures.iter()
                .map(|c| Capture::new(c.emission().clone(), c.subject().clone()))
                .collect::<Vec<_>>();
            (self.batch_processor)(for_processing);
        }
        
        Ok(captures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_basic_sink() {
        let mut sink = BasicSink::<String>::new(Name::from_part("test-sink"));
        let pipe = sink.create_pipe();
        
        // Create a mutable pipe for testing
        let mut test_pipe = SinkPipe::new(sink.sender.clone(), sink.subject.clone());
        
        // Emit some values
        test_pipe.emit("first".to_string()).await.unwrap();
        test_pipe.emit("second".to_string()).await.unwrap();
        test_pipe.emit("third".to_string()).await.unwrap();
        
        // Give background task time to collect
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        // Drain and verify
        let captures = sink.drain().await.unwrap();
        assert_eq!(captures.len(), 3);
        assert_eq!(captures[0].emission(), &"first".to_string());
        assert_eq!(captures[1].emission(), &"second".to_string());
        assert_eq!(captures[2].emission(), &"third".to_string());
        
        // Verify sink is empty after drain
        let empty = sink.drain().await.unwrap();
        assert!(empty.is_empty());
    }
    
    #[tokio::test]
    async fn test_filtered_sink() {
        let filter = |n: &i32| *n > 5;
        let mut sink = FilteredSink::new(Name::from_part("filtered-sink"), filter);
        let pipe = sink.create_filtered_pipe();
        
        // Create a mutable pipe for testing
        let mut test_pipe = FilteredSinkPipe::new(
            sink.inner.sender.clone(),
            sink.inner.subject.clone(),
            sink.filter.clone(),
        );
        
        // Emit values, only those > 5 should be captured
        for i in 1..=10 {
            test_pipe.emit(i).await.unwrap();
        }
        
        // Give background task time to collect
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        let captures = sink.drain().await.unwrap();
        assert_eq!(captures.len(), 5); // 6, 7, 8, 9, 10
        
        for capture in captures {
            assert!(*capture.emission() > 5);
        }
    }
    
    #[tokio::test]
    async fn test_sink_capacity() {
        let mut sink = BasicSink::<i32>::with_capacity(Name::from_part("capped-sink"), 3);
        let pipe = sink.create_pipe();
        
        // Create a mutable pipe for testing
        let mut test_pipe = SinkPipe::new(sink.sender.clone(), sink.subject.clone());
        
        // Emit more than capacity
        for i in 1..=5 {
            test_pipe.emit(i).await.unwrap();
        }
        
        // Give background task time to collect
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        // Should only have last 3
        let captures = sink.drain().await.unwrap();
        assert_eq!(captures.len(), 3);
        assert_eq!(*captures[0].emission(), 3);
        assert_eq!(*captures[1].emission(), 4);
        assert_eq!(*captures[2].emission(), 5);
    }
}