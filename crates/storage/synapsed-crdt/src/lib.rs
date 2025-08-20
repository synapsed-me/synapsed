//! # Synapsed CRDT
//!
//! Conflict-free Replicated Data Types for distributed collaboration and synchronization.
//! 
//! This crate provides mathematically proven data structures that automatically resolve
//! conflicts in distributed systems without requiring consensus protocols.
//!
//! ## Supported CRDTs
//!
//! - **LWW-Register**: Last-Writer-Wins register for simple values
//! - **OR-Set**: Observed-Remove Set for distributed sets
//! - **PN-Counter**: Increment/Decrement counter
//! - **RGA**: Replicated Growable Array for collaborative text editing
//! - **Merkle Tree**: Efficient synchronization with cryptographic verification
//!
//! ## Features
//!
//! - **Conflict-Free**: Automatic conflict resolution without coordination
//! - **Eventually Consistent**: All replicas converge to the same state
//! - **Partition Tolerant**: Works during network splits
//! - **Cryptographically Verified**: Optional Merkle tree verification
//! - **High Performance**: Optimized for low-latency operations
//!
//! ## Example
//!
//! ```rust,no_run
//! use synapsed_crdt::{LwwRegister, ActorId};
//! use tokio;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let actor1 = ActorId::new();
//!     let actor2 = ActorId::new();
//!     
//!     // Create two replicas
//!     let mut replica1 = LwwRegister::new(actor1);
//!     let mut replica2 = LwwRegister::new(actor2);
//!     
//!     // Concurrent updates
//!     replica1.set("Hello", 1).await?;
//!     replica2.set("World", 2).await?;
//!     
//!     // Merge replicas - automatically resolves conflicts
//!     replica1.merge(&replica2).await?;
//!     replica2.merge(&replica1).await?;
//!     
//!     // Both replicas now have consistent state
//!     assert_eq!(replica1.get(), replica2.get());
//!     
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod types;
pub mod traits;

// CRDT implementations
#[cfg(feature = "lww")]
pub mod lww_register;

#[cfg(feature = "orset")]
pub mod or_set;

#[cfg(feature = "pncounter")]
pub mod pn_counter;

#[cfg(feature = "rga")]
pub mod rga;

// Utilities
pub mod clock;
pub mod sync;

#[cfg(feature = "merkle-tree")]
pub mod merkle;

// Re-exports for convenience
pub use error::{CrdtError, Result};
pub use types::{ActorId, Timestamp, VectorClock, Delta};
pub use traits::{Crdt, Mergeable, Synchronizable};

#[cfg(feature = "lww")]
pub use lww_register::LwwRegister;

#[cfg(feature = "orset")]
pub use or_set::OrSet;

#[cfg(feature = "pncounter")]
pub use pn_counter::PnCounter;

#[cfg(feature = "rga")]
pub use rga::Rga;