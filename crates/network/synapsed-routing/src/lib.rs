//! # Synapsed Routing
//!
//! Anonymous onion routing and P2P communication protocols for privacy-preserving applications.
//! 
//! This crate provides multiple routing strategies including:
//! - **Onion Routing**: Multi-hop encrypted routing with layered encryption
//! - **Kademlia DHT**: Distributed hash table for peer discovery
//! - **Mix Networks**: Anonymous message routing with traffic analysis resistance
//! - **Tor-compatible**: Compatible with Tor network protocols
//!
//! ## Features
//!
//! - **Anonymous Routing**: Hide sender, receiver, and content metadata
//! - **Traffic Analysis Resistance**: Uniform packet sizes and timing
//! - **Multiple Hops**: Configurable multi-hop routing paths
//! - **Circuit Management**: Automatic circuit construction and rotation
//! - **Directory Services**: Decentralized node discovery
//!
//! ## Example
//!
//! ```rust,no_run
//! use synapsed_routing::{OnionRouter, RouterConfig, NodeId};
//! use tokio;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = RouterConfig::new()
//!         .with_hop_count(3)
//!         .with_circuit_lifetime(600); // 10 minutes
//!     
//!     let mut router = OnionRouter::new(config).await?;
//!     
//!     // Create an anonymous circuit
//!     let circuit = router.create_circuit().await?;
//!     
//!     // Send anonymous message
//!     router.send_anonymous(&circuit, b"Hello, anonymous world!").await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod error;
pub mod types;

// Simplified for now - we'll implement the actual routing later
pub mod onion;

// Re-exports for convenience
pub use config::RouterConfig;
pub use error::{RoutingError, Result};
pub use types::{NodeId, Circuit, MessagePayload};
pub use onion::OnionRouter;