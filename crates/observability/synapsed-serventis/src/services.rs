//! Services API - direct port of Java Serventis Services interface
//!
//! Services is a novel approach to monitoring service-to-service interactions based on 
//! signaling theory and social systems regulated by local and remote status assessment.

use crate::{async_trait, Arc, Composer, Pipe, Subject, Substrate};
use serde::{Deserialize, Serialize};
use std::fmt;
use synapsed_substrates::types::SubstratesResult;

/// The Services interface - entry point into the Serventis Services API
/// Direct port of Java Serventis Services interface
pub trait Services: Composer<Arc<dyn Service>, Signal> + Send + Sync {}

/// A service interface representing a composition of functions or operations
/// Direct port of Java Serventis Service interface
#[async_trait]
pub trait Service: Pipe<Signal> + Substrate + Send + Sync {
    /// A signal released indicating the request (call) for work to be done
    async fn call(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Call).await
    }
    
    /// A signal received indicating the request (call) for work to be done
    async fn called(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Called).await
    }
    
    /// A signal released indicating the delay of work
    async fn delay(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Delay).await
    }
    
    /// A signal received indicating the delay of work
    async fn delayed(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Delayed).await
    }
    
    /// A signal released indicating the dropping of work
    async fn discard(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Discard).await
    }
    
    /// A signal received indicating the dropping of work
    async fn discarded(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Discarded).await
    }
    
    /// A signal released indicating the disconnection of work
    async fn disconnect(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Disconnect).await
    }
    
    /// A signal received indicating the disconnection of work
    async fn disconnected(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Disconnected).await
    }
    
    /// A signal released indicating the expiration of work
    async fn expire(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Expire).await
    }
    
    /// A signal received indicating the expiration of work
    async fn expired(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Expired).await
    }
    
    /// A signal released indicating failure to complete work
    async fn fail(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Fail).await
    }
    
    /// A signal received indicating failure to complete work
    async fn failed(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Failed).await
    }
    
    /// A signal released indicating activation of some recourse strategy
    async fn recourse(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Recourse).await
    }
    
    /// A signal received indicating activation of some recourse strategy
    async fn recoursed(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Recoursed).await
    }
    
    /// A signal released indicating redirection of work to another service
    async fn redirect(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Redirect).await
    }
    
    /// A signal received indicating redirection of work to another service
    async fn redirected(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Redirected).await
    }
    
    /// A signal released indicating rejection of work
    async fn reject(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Reject).await
    }
    
    /// A signal received indicating rejection of work
    async fn rejected(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Rejected).await
    }
    
    /// A signal released indicating resumption of work
    async fn resume(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Resume).await
    }
    
    /// A signal received indicating resumption of work
    async fn resumed(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Resumed).await
    }
    
    /// A signal received indicating retry of work
    async fn retried(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Retried).await
    }
    
    /// A signal released indicating retry of work
    async fn retry(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Retry).await
    }
    
    /// A signal released indicating scheduling of work
    async fn schedule(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Schedule).await
    }
    
    /// A signal received indicating scheduling of work
    async fn scheduled(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Scheduled).await
    }
    
    /// A signal released indicating start of work
    async fn start(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Start).await
    }
    
    /// A signal received indicating start of work
    async fn started(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Started).await
    }
    
    /// A signal released indicating completion of work
    async fn stop(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Stop).await
    }
    
    /// A signal received indicating completion of work
    async fn stopped(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Stopped).await
    }
    
    /// A signal received indicating successful completion of work
    async fn succeeded(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Succeeded).await
    }
    
    /// A signal released indicating successful completion of work
    async fn success(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Success).await
    }
    
    /// A signal released indicating suspension of work
    async fn suspend(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Suspend).await
    }
    
    /// A signal received indicating suspension of work
    async fn suspended(&mut self) -> SubstratesResult<()> {
        self.emit(Signal::Suspended).await
    }
}

/// Signal enum representing various types of signals services can emit
/// Direct port of Java Serventis Signal enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Signal {
    /// A signal released indicating the start of work
    Start,
    /// A signal received indicating the start of work
    Started,
    /// A signal released indicating the completion of work
    Stop,
    /// A signal received indicating the completion of work
    Stopped,
    /// A signal released indicating the request for work to be done
    Call,
    /// A signal received indicating the request for work to be done
    Called,
    /// A signal released indicating successful completion of work
    Success,
    /// A signal received indicating successful completion of work
    Succeeded,
    /// A signal released indicating failure to complete work
    Fail,
    /// A signal received indicating failure to complete work
    Failed,
    /// A signal released indicating activation of recourse strategy
    Recourse,
    /// A signal received indicating activation of recourse strategy
    Recoursed,
    /// A signal released indicating redirection of work
    Redirect,
    /// A signal received indicating redirection of work
    Redirected,
    /// A signal released indicating expiration of work
    Expire,
    /// A signal received indicating expiration of work
    Expired,
    /// A signal released indicating retry of work
    Retry,
    /// A signal received indicating retry of work
    Retried,
    /// A signal released indicating rejection of work
    Reject,
    /// A signal received indicating rejection of work
    Rejected,
    /// A signal released indicating dropping of work
    Discard,
    /// A signal received indicating dropping of work
    Discarded,
    /// A signal released indicating delay of work
    Delay,
    /// A signal received indicating delay of work
    Delayed,
    /// A signal released indicating scheduling of work
    Schedule,
    /// A signal received indicating scheduling of work
    Scheduled,
    /// A signal released indicating suspension of work
    Suspend,
    /// A signal received indicating suspension of work
    Suspended,
    /// A signal released indicating resumption of work
    Resume,
    /// A signal received indicating resumption of work
    Resumed,
    /// A signal released indicating disconnection of work
    Disconnect,
    /// A signal received indicating disconnection of work
    Disconnected,
}

impl Signal {
    /// Get the sign (operation type) of this signal
    pub fn sign(&self) -> Sign {
        match self {
            Signal::Start | Signal::Started => Sign::Start,
            Signal::Stop | Signal::Stopped => Sign::Stop,
            Signal::Call | Signal::Called => Sign::Call,
            Signal::Success | Signal::Succeeded => Sign::Success,
            Signal::Fail | Signal::Failed => Sign::Fail,
            Signal::Recourse | Signal::Recoursed => Sign::Recourse,
            Signal::Redirect | Signal::Redirected => Sign::Redirect,
            Signal::Expire | Signal::Expired => Sign::Expire,
            Signal::Retry | Signal::Retried => Sign::Retry,
            Signal::Reject | Signal::Rejected => Sign::Reject,
            Signal::Discard | Signal::Discarded => Sign::Discard,
            Signal::Delay | Signal::Delayed => Sign::Delay,
            Signal::Schedule | Signal::Scheduled => Sign::Schedule,
            Signal::Suspend | Signal::Suspended => Sign::Suspend,
            Signal::Resume | Signal::Resumed => Sign::Resume,
            Signal::Disconnect | Signal::Disconnected => Sign::Disconnect,
        }
    }
    
    /// Get the orientation (perspective) of this signal
    pub fn orientation(&self) -> Orientation {
        match self {
            Signal::Start | Signal::Stop | Signal::Call | Signal::Success | Signal::Fail
            | Signal::Recourse | Signal::Redirect | Signal::Expire | Signal::Retry
            | Signal::Reject | Signal::Discard | Signal::Delay | Signal::Schedule
            | Signal::Suspend | Signal::Resume | Signal::Disconnect => Orientation::Release,
            
            Signal::Started | Signal::Stopped | Signal::Called | Signal::Succeeded
            | Signal::Failed | Signal::Recoursed | Signal::Redirected | Signal::Expired
            | Signal::Retried | Signal::Rejected | Signal::Discarded | Signal::Delayed
            | Signal::Scheduled | Signal::Suspended | Signal::Resumed | Signal::Disconnected => {
                Orientation::Receipt
            }
        }
    }
}

/// Sign classifies operations, transitions, and outcomes
/// Direct port of Java Serventis Sign enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Sign {
    /// Indicates the start of work to be done
    Start,
    /// Indicates the completion of work
    Stop,
    /// Indicates the request (call) for work to be done
    Call,
    /// Indicates successful completion of work
    Success,
    /// Indicates failure to complete work
    Fail,
    /// Indicates activation of degraded work mode after failure
    Recourse,
    /// Indicates forwarding of work to another service
    Redirect,
    /// Indicates expiration of time budget for work
    Expire,
    /// Indicates automatic retry of work on error
    Retry,
    /// Indicates rejection of work
    Reject,
    /// Indicates discarding of work
    Discard,
    /// Indicates delaying of work
    Delay,
    /// Indicates scheduling of work
    Schedule,
    /// Indicates suspension of work
    Suspend,
    /// Indicates resumption of work
    Resume,
    /// Indicates inability to issue work
    Disconnect,
}

/// Orientation classifies the method of signal recording
/// Direct port of Java Serventis Orientation enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Orientation {
    /// Emission of a sign from a self-perspective
    Release,
    /// Reception of a sign observed from other-perspective
    Receipt,
}

impl fmt::Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Display for Sign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Display for Orientation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// A wrapper type for signal handlers that implements Debug
struct SignalHandler {
    handler: Arc<dyn Fn(Signal) + Send + Sync>,
}

impl SignalHandler {
    fn new<F>(handler: F) -> Self
    where
        F: Fn(Signal) + Send + Sync + 'static,
    {
        Self {
            handler: Arc::new(handler),
        }
    }
    
    fn handle(&self, signal: Signal) {
        (self.handler)(signal);
    }
}

impl fmt::Debug for SignalHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignalHandler")
            .field("handler", &"<function>")
            .finish()
    }
}

/// Basic service implementation
#[derive(Debug)]
pub struct BasicService {
    subject: Subject,
    signal_handler: Option<SignalHandler>,
}

impl BasicService {
    pub fn new(subject: Subject) -> Self {
        Self {
            subject,
            signal_handler: None,
        }
    }
    
    pub fn with_handler<F>(subject: Subject, handler: F) -> Self
    where
        F: Fn(Signal) + Send + Sync + 'static,
    {
        Self {
            subject,
            signal_handler: Some(SignalHandler::new(handler)),
        }
    }
}

impl Substrate for BasicService {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

#[async_trait]
impl Pipe<Signal> for BasicService {
    async fn emit(&mut self, emission: Signal) -> SubstratesResult<()> {
        if let Some(handler) = &self.signal_handler {
            handler.handle(emission);
        }
        Ok(())
    }
}

#[async_trait]
impl Service for BasicService {}

#[cfg(test)]
mod tests {
    use super::*;
    use synapsed_substrates::types::{Name, SubjectType};
    
    #[tokio::test]
    async fn test_service_signals() {
        let subject = Subject::new(Name::from_part("test-service"), SubjectType::Source);
        let mut service = BasicService::new(subject);
        
        // Test basic signal emission
        service.start().await.unwrap();
        service.success().await.unwrap();
        service.stop().await.unwrap();
    }
    
    #[test]
    fn test_signal_properties() {
        let signal = Signal::Start;
        assert_eq!(signal.sign(), Sign::Start);
        assert_eq!(signal.orientation(), Orientation::Release);
        
        let signal = Signal::Started;
        assert_eq!(signal.sign(), Sign::Start);
        assert_eq!(signal.orientation(), Orientation::Receipt);
        
        let signal = Signal::Success;
        assert_eq!(signal.sign(), Sign::Success);
        assert_eq!(signal.orientation(), Orientation::Release);
        
        let signal = Signal::Succeeded;
        assert_eq!(signal.sign(), Sign::Success);
        assert_eq!(signal.orientation(), Orientation::Receipt);
    }
}