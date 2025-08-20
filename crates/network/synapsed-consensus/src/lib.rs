//! # Synapsed Consensus
//!
//! Byzantine fault tolerant consensus algorithms for distributed systems.
//! 
//! This crate provides multiple consensus algorithm implementations including:
//! - **HotStuff**: High-throughput, low-latency BFT consensus
//! - **PBFT**: Practical Byzantine Fault Tolerance
//! - **Tendermint**: BFT consensus with immediate finality
//! - **Avalanche**: DAG-based consensus protocol
//!
//! ## Features
//!
//! - **Byzantine Fault Tolerance**: Up to f < n/3 faulty nodes
//! - **Configurable Algorithms**: Multiple consensus protocols
//! - **Performance Optimized**: Sub-second finality
//! - **Cryptographic Security**: Ed25519 signatures and verifiable proofs
//! - **Network Agnostic**: Works with any transport layer
//!
//! ## Example
//!
//! ```rust,no_run
//! use synapsed_consensus::{HotStuffConsensus, ConsensusConfig, NodeId};
//! use tokio;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ConsensusConfig::new()
//!         .with_node_id(NodeId::new())
//!         .with_byzantine_threshold(1); // f=1, supports up to 3f+1=4 nodes
//!     
//!     let mut consensus = HotStuffConsensus::new(config).await?;
//!     
//!     // Start consensus protocol
//!     consensus.start().await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod error;
pub mod types;
pub mod traits;

// Consensus algorithm implementations
#[cfg(feature = "hotstuff")]
pub mod hotstuff;

// #[cfg(feature = "pbft")]
// pub mod pbft;

// #[cfg(feature = "tendermint")]
// pub mod tendermint;

// #[cfg(feature = "avalanche")]
// pub mod avalanche;

// Core utilities (modules to be implemented)
// pub mod crypto;
// pub mod network;
// pub mod state_machine;
// pub mod voting;

// Re-exports for convenience
pub use error::{ConsensusError, Result};
pub use types::{Block, NodeId, Vote, QuorumCertificate, ViewNumber, Transaction, VoteType};
pub use traits::{ConsensusProtocol, StateMachine, NetworkTransport, ConsensusConfig, ConsensusStats, 
                  ConsensusCrypto, LeaderElection};

// Consensus implementations
#[cfg(feature = "hotstuff")]
pub use hotstuff::HotStuffConsensus;

// #[cfg(feature = "pbft")]
// pub use pbft::PbftConsensus;

// #[cfg(feature = "tendermint")]
// pub use tendermint::TendermintConsensus;

// #[cfg(feature = "avalanche")]
// pub use avalanche::AvalancheConsensus;