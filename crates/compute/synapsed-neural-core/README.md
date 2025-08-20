# Synapsed Neural Core

Neural network architectures and cognitive patterns for AI agents and distributed intelligence in the Synapsed ecosystem.

## Overview

This crate provides ephemeral neural networks with dynamic architecture creation, 27+ cognitive patterns, and WASM-compatible execution. Designed for AI agents that need to adapt their thinking patterns dynamically.

## Supported Architectures

### Feedforward Networks
- **Multi-Layer Perceptrons**: Basic dense networks
- **Deep Networks**: Arbitrary depth with skip connections
- **Residual Networks**: ResNet-style architectures
- **Use Case**: Classification, regression, function approximation

### Recurrent Networks
- **LSTM**: Long Short-Term Memory networks
- **GRU**: Gated Recurrent Units
- **Vanilla RNN**: Simple recurrent networks
- **Use Case**: Sequence processing, time series, natural language

### Transformer Networks
- **Self-Attention**: Multi-head attention mechanisms
- **Positional Encoding**: Position-aware sequence processing
- **Layer Normalization**: Stable training for deep networks
- **Use Case**: Language models, sequence-to-sequence tasks

### Convolutional Networks
- **2D Convolution**: Spatial pattern recognition
- **Pooling Layers**: Dimension reduction
- **Batch Normalization**: Training stability
- **Use Case**: Image processing, computer vision

## Cognitive Patterns

The system includes 27+ specialized cognitive patterns:

### Analytical Patterns
- **Convergent**: Focused problem-solving
- **Divergent**: Creative exploration
- **Critical**: Systematic evaluation
- **Logical**: Step-by-step reasoning

### Creative Patterns
- **Lateral**: Non-linear thinking
- **Analogical**: Pattern-based reasoning
- **Synthetic**: Combining disparate concepts
- **Intuitive**: Gut-feeling based decisions

### Systems Patterns
- **Holistic**: Big-picture thinking
- **Reductionist**: Breaking down complexity
- **Network**: Relationship-focused analysis
- **Emergent**: Pattern recognition in complex systems

## Quick Start

```rust
use synapsed_neural_core::{
    NeuralNetwork, Architecture, CognitivePattern, 
    Tensor, Optimizer
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a feedforward network
    let architecture = Architecture::feedforward()
        .input_size(784)                    // MNIST image size
        .hidden_layers(vec![256, 128, 64])  // Hidden layer sizes
        .output_size(10)                    // Number of classes
        .activation("relu")                 // ReLU activation
        .dropout(0.2);                      // 20% dropout
    
    let mut network = NeuralNetwork::create(architecture).await?;
    
    // Apply convergent thinking pattern
    network.apply_cognitive_pattern(CognitivePattern::Convergent).await?;
    
    // Set up optimizer
    let optimizer = Optimizer::adam()
        .learning_rate(0.001)
        .beta1(0.9)
        .beta2(0.999);
    
    network.set_optimizer(optimizer).await?;
    
    // Training data
    let input = Tensor::from_vec(vec![0.1; 784], vec![1, 784]);
    let target = Tensor::from_vec(vec![0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], vec![1, 10]);
    
    // Train the network
    let loss = network.train_step(&input, &target).await?;
    println!("Training loss: {:.4}", loss);
    
    // Make predictions
    let output = network.forward(&input).await?;
    println!("Prediction: {:?}", output.argmax());
    
    Ok(())
}
```

## Cognitive Pattern Integration

```rust
use synapsed_neural_core::{CognitivePattern, NeuralNetwork};

// Apply different thinking patterns dynamically
let mut network = NeuralNetwork::create(architecture).await?;

// For creative tasks
network.apply_cognitive_pattern(CognitivePattern::Divergent).await?;
let creative_output = network.forward(&input).await?;

// For analytical tasks
network.apply_cognitive_pattern(CognitivePattern::Critical).await?;
let analytical_output = network.forward(&input).await?;

// For systems thinking
network.apply_cognitive_pattern(CognitivePattern::Holistic).await?;
let systems_output = network.forward(&input).await?;
```

## WASM Deployment

```rust
use synapsed_neural_core::wasm_runtime::WasmNetwork;

// Create WASM-compatible network
let network = WasmNetwork::create(architecture).await?;

// Deploy to browser or Node.js
network.compile_to_wasm().await?;
```

## Ephemeral Networks

```rust
use synapsed_neural_core::EphemeralNetwork;

// Create temporary network for specific task
let ephemeral = EphemeralNetwork::spawn()
    .for_task("image_classification")
    .with_lifetime(300)  // 5 minutes
    .with_auto_cleanup(true);

let mut network = ephemeral.create().await?;

// Network automatically cleans up after lifetime expires
// Memory is reclaimed, preventing resource leaks
```

## Architecture Specifications

### Layer Configuration
```rust
let config = LayerConfig::dense()
    .input_size(512)
    .output_size(256)
    .activation(ActivationFunction::ReLU)
    .weight_init(WeightInit::Xavier)
    .bias_init(BiasInit::Zero)
    .dropout(0.1);
```

### Advanced Architectures
```rust
// Transformer with self-attention
let transformer = Architecture::transformer()
    .num_layers(6)
    .hidden_size(512)
    .num_heads(8)
    .feedforward_size(2048)
    .max_sequence_length(1024)
    .positional_encoding(true);

// Convolutional neural network
let cnn = Architecture::convolutional()
    .input_channels(3)
    .conv_layer(32, 3, 1)  // 32 filters, 3x3 kernel, stride 1
    .conv_layer(64, 3, 1)
    .max_pool(2, 2)        // 2x2 pooling
    .conv_layer(128, 3, 1)
    .global_avg_pool()
    .dense(10);            // Final classification layer
```

## Performance Optimization

### SIMD Acceleration
```rust
use synapsed_neural_core::simd_ops::{simd_matmul, simd_activation};

// Hardware-accelerated operations
let output = simd_matmul(&input, &weights).await?;
let activated = simd_activation(&output, ActivationFunction::ReLU).await?;
```

### Memory Management
```rust
use synapsed_neural_core::memory::MemoryPool;

// Efficient memory allocation
let pool = MemoryPool::new()
    .with_initial_size(1024 * 1024)  // 1MB initial
    .with_growth_factor(2.0)
    .with_max_size(100 * 1024 * 1024); // 100MB max

let network = NeuralNetwork::with_memory_pool(architecture, pool).await?;
```

## Performance Characteristics

| Architecture | Training Speed | Inference Speed | Memory Usage | Use Case |
|--------------|----------------|-----------------|--------------|----------|
| Feedforward | Fast | Very Fast | Low | Simple tasks |
| LSTM | Medium | Medium | Medium | Sequences |
| Transformer | Slow | Fast | High | Language |
| CNN | Medium | Fast | Medium | Images |

## Cognitive Pattern Effects

| Pattern | Exploration | Focus | Creativity | Accuracy |
|---------|-------------|-------|------------|----------|
| Convergent | Low | High | Low | High |
| Divergent | High | Low | High | Medium |
| Critical | Medium | High | Low | High |
| Lateral | High | Medium | High | Medium |
| Holistic | Medium | Medium | Medium | Medium |

## Testing

```bash
# Unit tests
cargo test

# Neural network specific tests
cargo test --test neural_tests

# Cognitive pattern tests
cargo test --test cognitive_tests

# WASM compatibility tests
cargo test --target wasm32-unknown-unknown

# Performance benchmarks
cargo bench
```

## Features

- `default`: All basic neural architectures and cognitive patterns
- `feedforward`: Multi-layer perceptron networks
- `recurrent`: LSTM, GRU, and RNN implementations
- `transformer`: Self-attention mechanisms
- `convolutional`: CNN architectures
- `cognitive-patterns`: 27+ thinking patterns
- `wasm`: WebAssembly compatibility
- `simd`: SIMD acceleration

## Dependencies

### Core Dependencies
- `ndarray`: Multi-dimensional arrays
- `rand`: Random number generation
- `wide`: SIMD operations

### Optional Dependencies
- `wasm-bindgen`: WebAssembly bindings
- `web-sys`: Browser APIs

### Internal Dependencies
- `synapsed-core`: Shared utilities
- `synapsed-crypto`: Secure random generation

## License

Licensed under either of:
- Apache License, Version 2.0
- MIT License

## Research References

1. Vaswani, A., et al. "Attention Is All You Need" (2017)
2. Hochreiter, S., et al. "Long Short-Term Memory" (1997)
3. He, K., et al. "Deep Residual Learning for Image Recognition" (2016)
4. Guilford, J.P. "The Nature of Human Intelligence" (1967)