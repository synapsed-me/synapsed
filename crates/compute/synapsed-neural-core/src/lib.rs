//! # Synapsed Neural Core
//!
//! Neural network architectures and cognitive patterns for AI agents and distributed intelligence.
//! 
//! This crate provides ephemeral neural networks with dynamic architecture creation,
//! 27+ cognitive patterns, and WASM-compatible execution for cross-platform deployment.
//!
//! ## Supported Architectures
//!
//! - **Feedforward Networks**: Basic multi-layer perceptrons
//! - **Recurrent Networks**: LSTM, GRU, and vanilla RNN implementations
//! - **Transformer Networks**: Self-attention mechanisms for sequence processing
//! - **Convolutional Networks**: Spatial pattern recognition
//! - **Cognitive Patterns**: 27+ specialized thinking patterns
//!
//! ## Features
//!
//! - **Ephemeral Creation**: Dynamic neural network construction
//! - **Cognitive Patterns**: Specialized reasoning patterns
//! - **WASM Compatible**: Runs in browser and server environments
//! - **SIMD Acceleration**: Hardware-accelerated computation
//! - **Memory Efficient**: Optimized for resource-constrained environments
//!
//! ## Example
//!
//! ```rust,no_run
//! use synapsed_neural_core::{NeuralNetwork, Architecture, CognitivePattern};
//! use tokio;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a feedforward network
//!     let architecture = Architecture::feedforward()
//!         .input_size(784)
//!         .hidden_layers(vec![256, 128, 64])
//!         .output_size(10)
//!         .activation("relu");
//!     
//!     let mut network = NeuralNetwork::create(architecture).await?;
//!     
//!     // Apply cognitive pattern
//!     let pattern = CognitivePattern::Convergent;
//!     network.apply_cognitive_pattern(pattern).await?;
//!     
//!     // Train on data
//!     let input = vec![0.1; 784];
//!     let target = vec![0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
//!     
//!     network.train(&input, &target).await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod types;
pub mod traits;

// Core neural components
pub mod activation;
pub mod layer;
pub mod network;
pub mod optimizer;

// Architecture implementations
#[cfg(feature = "feedforward")]
pub mod feedforward;

#[cfg(feature = "recurrent")]
pub mod recurrent;

#[cfg(feature = "transformer")]
pub mod transformer;

#[cfg(feature = "convolutional")]
pub mod convolutional;

// Cognitive patterns
#[cfg(feature = "cognitive-patterns")]
pub mod cognitive;

// Utilities
pub mod math;
pub mod memory;

#[cfg(feature = "wasm")]
pub mod wasm_runtime;

#[cfg(feature = "simd")]
pub mod simd_ops;

// Re-exports for convenience
pub use error::{NeuralError, Result};
pub use types::{Architecture, Tensor, Weight, Bias};
pub use traits::{NeuralNetwork, Layer, Optimizer, ActivationFunction};
pub use network::EphemeralNetwork;

#[cfg(feature = "cognitive-patterns")]
pub use cognitive::CognitivePattern;