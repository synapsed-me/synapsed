//! Probes API - Direct port of Java Serventis Probes interface
//!
//! The Probes API provides a structured framework for monitoring and reporting
//! communication outcomes in distributed systems. It enables precise observation
//! of operations across client-server boundaries.

use crate::{async_trait, Arc, Composer, Pipe, Subject, Substrate};
use serde::{Deserialize, Serialize};
use std::fmt;
use synapsed_substrates::types::SubstratesResult;

/// The Probes interface - entry point into the Serventis Probes API
/// Direct port of Java Serventis Probes interface
pub trait Probes: Composer<Arc<dyn Probe>, Observation> + Send + Sync {}

/// An Observation is a record of a communication event, capturing:
/// - What happened (outcome)
/// - Where it happened (origin)
/// - What was happening (operation)
///
/// Direct port of Java Serventis Observation interface
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Observation {
    operation: Operation,
    origin: Origin,
    outcome: Outcome,
}

impl Observation {
    /// Create a new observation
    pub fn new(origin: Origin, operation: Operation, outcome: Outcome) -> Self {
        Self {
            operation,
            origin,
            outcome,
        }
    }

    /// Returns the type of operation being observed
    pub fn operation(&self) -> Operation {
        self.operation
    }

    /// Returns the origin where the observation was made
    pub fn origin(&self) -> Origin {
        self.origin
    }

    /// Returns the outcome of the observed operation
    pub fn outcome(&self) -> Outcome {
        self.outcome
    }
}

impl fmt::Display for Observation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}:{:?}:{:?}", self.origin, self.operation, self.outcome)
    }
}

/// A Probe is an instrument that emits observations about communication operations
/// Direct port of Java Serventis Probe interface
#[async_trait]
pub trait Probe: Pipe<Observation> + Substrate + Send + Sync {
    /// Emits a CLIENT observation with the specified outcome and operation
    async fn client(&mut self, operation: Operation, outcome: Outcome) -> SubstratesResult<()> {
        self.observation(Origin::Client, operation, outcome).await
    }

    /// Emits an observation for a client-side CLOSE operation
    async fn close_client(&mut self, outcome: Outcome) -> SubstratesResult<()> {
        self.observation(Origin::Client, Operation::Close, outcome).await
    }

    /// Emits an observation for a CLOSE operation with specified origin
    async fn close(&mut self, origin: Origin, outcome: Outcome) -> SubstratesResult<()> {
        self.observation(origin, Operation::Close, outcome).await
    }

    /// Emits an observation for a server-side CLOSE operation
    async fn close_server(&mut self, outcome: Outcome) -> SubstratesResult<()> {
        self.observation(Origin::Server, Operation::Close, outcome).await
    }

    /// Emits an observation for a client-side CONNECT operation
    async fn connect_client(&mut self, outcome: Outcome) -> SubstratesResult<()> {
        self.observation(Origin::Client, Operation::Connect, outcome).await
    }

    /// Emits an observation for a CONNECT operation with specified origin
    async fn connect(&mut self, origin: Origin, outcome: Outcome) -> SubstratesResult<()> {
        self.observation(origin, Operation::Connect, outcome).await
    }

    /// Emits an observation for a server-side CONNECT operation
    async fn connect_server(&mut self, outcome: Outcome) -> SubstratesResult<()> {
        self.observation(Origin::Server, Operation::Connect, outcome).await
    }

    /// Emits a custom observation
    async fn emit_observation(&mut self, observation: Observation) -> SubstratesResult<()> {
        self.emit(observation).await
    }

    /// Emits an observation for a FAILURE outcome
    async fn failure(&mut self, origin: Origin, operation: Operation) -> SubstratesResult<()> {
        self.observation(origin, operation, Outcome::Failure).await
    }

    /// Emits an observation with specified origin, operation, and outcome
    async fn observation(
        &mut self,
        origin: Origin,
        operation: Operation,
        outcome: Outcome,
    ) -> SubstratesResult<()> {
        self.emit(Observation::new(origin, operation, outcome)).await
    }

    /// Emits an observation for a client-side PROCESS operation
    async fn process_client(&mut self, outcome: Outcome) -> SubstratesResult<()> {
        self.observation(Origin::Client, Operation::Process, outcome).await
    }

    /// Emits an observation for a PROCESS operation with specified origin
    async fn process(&mut self, origin: Origin, outcome: Outcome) -> SubstratesResult<()> {
        self.observation(origin, Operation::Process, outcome).await
    }

    /// Emits an observation for a server-side PROCESS operation
    async fn process_server(&mut self, outcome: Outcome) -> SubstratesResult<()> {
        self.observation(Origin::Server, Operation::Process, outcome).await
    }

    /// Emits an observation for a client-side RECEIVE operation
    async fn receive_client(&mut self, outcome: Outcome) -> SubstratesResult<()> {
        self.observation(Origin::Client, Operation::Receive, outcome).await
    }

    /// Emits an observation for a RECEIVE operation with specified origin
    async fn receive(&mut self, origin: Origin, outcome: Outcome) -> SubstratesResult<()> {
        self.observation(origin, Operation::Receive, outcome).await
    }

    /// Emits an observation for a server-side RECEIVE operation
    async fn receive_server(&mut self, outcome: Outcome) -> SubstratesResult<()> {
        self.observation(Origin::Server, Operation::Receive, outcome).await
    }

    /// Emits an observation for a client-side SEND operation
    async fn send_client(&mut self, outcome: Outcome) -> SubstratesResult<()> {
        self.observation(Origin::Client, Operation::Send, outcome).await
    }

    /// Emits an observation for a SEND operation with specified origin
    async fn send(&mut self, origin: Origin, outcome: Outcome) -> SubstratesResult<()> {
        self.observation(origin, Operation::Send, outcome).await
    }

    /// Emits an observation for a server-side SEND operation
    async fn send_server(&mut self, outcome: Outcome) -> SubstratesResult<()> {
        self.observation(Origin::Server, Operation::Send, outcome).await
    }

    /// Emits a SERVER observation with the specified operation and outcome
    async fn server(&mut self, operation: Operation, outcome: Outcome) -> SubstratesResult<()> {
        self.observation(Origin::Server, operation, outcome).await
    }

    /// Emits an observation for a SUCCESS outcome
    async fn success(&mut self, origin: Origin, operation: Operation) -> SubstratesResult<()> {
        self.observation(origin, operation, Outcome::Success).await
    }
}

/// The Outcome enum represents the result of an observed operation
/// Direct port of Java Serventis Outcome enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Outcome {
    /// The operation completed successfully as expected
    Success,
    /// The operation failed to complete as expected
    Failure,
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Outcome::Success => write!(f, "SUCCESS"),
            Outcome::Failure => write!(f, "FAILURE"),
        }
    }
}

/// The Origin enum identifies where in the distributed system an observation was made
/// Direct port of Java Serventis Origin enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Origin {
    /// The observation was made at the client side of the communication
    Client,
    /// The observation was made at the server side of the communication
    Server,
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Origin::Client => write!(f, "CLIENT"),
            Origin::Server => write!(f, "SERVER"),
        }
    }
}

/// The Operation enum identifies the type of activity being performed
/// Direct port of Java Serventis Operation enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Operation {
    /// Establishing a connection or session between communicating parties
    Connect,
    /// Transmitting data from one party to another
    Send,
    /// Accepting data transmitted from another party
    Receive,
    /// Processing or handling received data or requests
    Process,
    /// Terminating a connection or session
    Close,
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operation::Connect => write!(f, "CONNECT"),
            Operation::Send => write!(f, "SEND"),
            Operation::Receive => write!(f, "RECEIVE"),
            Operation::Process => write!(f, "PROCESS"),
            Operation::Close => write!(f, "CLOSE"),
        }
    }
}

/// Basic implementation of Probe
#[derive(Debug)]
pub struct BasicProbe {
    subject: Arc<Subject>,
    observations: Vec<Observation>,
}

impl BasicProbe {
    pub fn new(subject: Arc<Subject>) -> Self {
        Self {
            subject,
            observations: Vec::new(),
        }
    }

    pub fn observations(&self) -> &[Observation] {
        &self.observations
    }
}

#[async_trait]
impl Pipe<Observation> for BasicProbe {
    async fn emit(&mut self, value: Observation) -> SubstratesResult<()> {
        self.observations.push(value);
        Ok(())
    }
}

#[async_trait]
impl Substrate for BasicProbe {
    fn subject(&self) -> &Subject {
        &self.subject
    }
}

#[async_trait]
impl Probe for BasicProbe {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Subject;
    use synapsed_substrates::types::{Name, SubjectType};

    #[tokio::test]
    async fn test_probe_observations() {
        let subject = Arc::new(Subject::new(
            Name::from("test-probe"),
            SubjectType::Service,
        ));
        let mut probe = BasicProbe::new(subject);

        // Test client operations
        probe.client(Operation::Connect, Outcome::Success).await.unwrap();
        probe.client(Operation::Send, Outcome::Success).await.unwrap();
        probe.client(Operation::Receive, Outcome::Failure).await.unwrap();

        // Test server operations
        probe.server(Operation::Process, Outcome::Success).await.unwrap();
        probe.server(Operation::Close, Outcome::Success).await.unwrap();

        let observations = probe.observations();
        assert_eq!(observations.len(), 5);
        
        assert_eq!(observations[0], Observation::new(
            Origin::Client,
            Operation::Connect,
            Outcome::Success
        ));
        
        assert_eq!(observations[4], Observation::new(
            Origin::Server,
            Operation::Close,
            Outcome::Success
        ));
    }

    #[tokio::test]
    async fn test_observation_display() {
        let obs = Observation::new(Origin::Client, Operation::Send, Outcome::Success);
        assert_eq!(obs.to_string(), "Client:Send:Success");
        
        let obs = Observation::new(Origin::Server, Operation::Process, Outcome::Failure);
        assert_eq!(obs.to_string(), "Server:Process:Failure");
    }
}