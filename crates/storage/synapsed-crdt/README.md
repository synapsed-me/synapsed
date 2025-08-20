# Synapsed CRDT

Conflict-free Replicated Data Types for distributed collaboration and synchronization in the Synapsed ecosystem.

## Overview

This crate provides mathematically proven data structures that automatically resolve conflicts in distributed systems without requiring consensus protocols. All CRDTs are designed for eventual consistency and partition tolerance.

## Supported CRDTs

### State-based CRDTs (CvRDTs)

#### Last-Writer-Wins Register (LWW-Register)
- **Use Case**: Configuration values, user preferences
- **Properties**: Simple conflict resolution, low memory overhead
- **Example**: Distributed configuration management

#### Observed-Remove Set (OR-Set)
- **Use Case**: Collaborative document editing, user lists
- **Properties**: Add/remove semantics, no phantom additions
- **Example**: Real-time collaboration platforms

#### PN-Counter
- **Use Case**: Metrics, counters, statistics
- **Properties**: Increment/decrement operations, eventual consistency
- **Example**: Distributed analytics systems

### Operation-based CRDTs (CmRDTs)

#### Replicated Growable Array (RGA)
- **Use Case**: Collaborative text editing, ordered lists
- **Properties**: Maintains insertion order, supports concurrent edits
- **Example**: Google Docs-style collaborative editing

## Quick Start

```rust
use synapsed_crdt::{LwwRegister, OrSet, PnCounter, ActorId};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let actor = ActorId::new();
    
    // LWW Register for simple values
    let mut register = LwwRegister::new(actor.clone());
    register.set("config_value", 123).await?;
    
    // OR-Set for collections
    let mut set = OrSet::new(actor.clone());
    set.add("item1").await?;
    set.add("item2").await?;
    
    // PN-Counter for metrics
    let mut counter = PnCounter::new(actor.clone());
    counter.increment(5).await?;
    counter.decrement(2).await?;
    
    println!("Counter value: {}", counter.value());
    Ok(())
}
```

## Collaborative Text Editing

```rust
use synapsed_crdt::{Rga, ActorId};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let alice = ActorId::from_string("alice");
    let bob = ActorId::from_string("bob");
    
    // Create two document replicas
    let mut doc_alice = Rga::new(alice);
    let mut doc_bob = Rga::new(bob);
    
    // Concurrent edits
    doc_alice.insert(0, 'H').await?;
    doc_alice.insert(1, 'e').await?;
    doc_alice.insert(2, 'l').await?;
    doc_alice.insert(3, 'l').await?;
    doc_alice.insert(4, 'o').await?;
    
    doc_bob.insert(0, 'W').await?;
    doc_bob.insert(1, 'o').await?;
    doc_bob.insert(2, 'r').await?;
    doc_bob.insert(3, 'l').await?;
    doc_bob.insert(4, 'd').await?;
    
    // Merge documents - automatic conflict resolution
    doc_alice.merge(&doc_bob).await?;
    doc_bob.merge(&doc_alice).await?;
    
    // Both documents now have the same content
    println!("Alice's doc: {}", doc_alice.to_string());
    println!("Bob's doc: {}", doc_bob.to_string());
    
    Ok(())
}
```

## Synchronization

### Delta Synchronization
```rust
use synapsed_crdt::{OrSet, Delta};

// Efficient synchronization using deltas
let mut set1 = OrSet::new(actor1);
let mut set2 = OrSet::new(actor2);

// Track changes as deltas
set1.start_delta_tracking().await?;
set1.add("item1").await?;
set1.add("item2").await?;

// Get delta and apply to other replica
let delta = set1.get_delta().await?;
set2.apply_delta(delta).await?;

// Synchronized!
assert_eq!(set1.elements(), set2.elements());
```

### Merkle Tree Verification
```rust
use synapsed_crdt::{OrSet, merkle::MerkleSync};

let mut set1 = OrSet::new(actor1);
let mut set2 = OrSet::new(actor2);

// Add items to both sets
set1.add("common1").await?;
set1.add("unique1").await?;
set2.add("common1").await?;
set2.add("unique2").await?;

// Efficient synchronization with Merkle trees
let sync = MerkleSync::new();
let diff = sync.compute_diff(&set1, &set2).await?;
sync.apply_diff(&mut set1, &mut set2, diff).await?;

// Now synchronized
assert_eq!(set1.elements().len(), 3); // common1, unique1, unique2
```

## Architecture

### CRDT Trait
Core interface for all CRDTs:
```rust
#[async_trait]
pub trait Crdt<T>: Clone + Send + Sync {
    type Delta;
    
    async fn merge(&mut self, other: &Self) -> Result<()>;
    async fn value(&self) -> T;
    fn actor_id(&self) -> &ActorId;
}
```

### Mergeable Trait
For efficient synchronization:
```rust
#[async_trait]
pub trait Mergeable {
    type Delta;
    
    async fn compute_delta(&self, other: &Self) -> Result<Self::Delta>;
    async fn apply_delta(&mut self, delta: Self::Delta) -> Result<()>;
}
```

## Performance Characteristics

| CRDT Type | Memory | Merge Time | Use Case |
|-----------|--------|------------|----------|
| LWW-Register | O(1) | O(1) | Simple values |
| OR-Set | O(n) | O(n) | Sets, collections |
| PN-Counter | O(a) | O(a) | Counters, metrics |
| RGA | O(n) | O(n log n) | Text editing |

Where:
- n = number of elements
- a = number of actors

## Mathematical Properties

### Convergence (CvRDTs)
All state-based CRDTs implement a join-semilattice where:
- **Commutative**: A ⊔ B = B ⊔ A
- **Associative**: (A ⊔ B) ⊔ C = A ⊔ (B ⊔ C)
- **Idempotent**: A ⊔ A = A

### Delivery Guarantees (CmRDTs) 
Operation-based CRDTs require:
- **Causal Delivery**: Operations applied in causal order
- **Exactly-Once Delivery**: No duplicate operations
- **Concurrent Commutativity**: Concurrent operations commute

## Testing

```bash
# Unit tests
cargo test

# Property-based testing
cargo test --features proptest

# Concurrent execution tests
cargo test --test concurrent_tests

# Performance benchmarks
cargo bench
```

## Features

- `default`: Enables all basic CRDT types
- `lww`: Last-Writer-Wins registers
- `orset`: Observed-Remove sets
- `pncounter`: PN-Counters
- `rga`: Replicated Growable Arrays
- `merkle-tree`: Merkle tree synchronization

## Dependencies

### Core Dependencies
- `dashmap`: Concurrent hash maps
- `uuid`: Unique identifiers
- `chrono`: Timestamp handling

### Internal Dependencies
- `synapsed-core`: Shared utilities
- `synapsed-crypto`: Cryptographic verification

## License

Licensed under either of:
- Apache License, Version 2.0
- MIT License

## References

1. Shapiro, M., et al. "Conflict-free Replicated Data Types" (2011)
2. Roh, H., et al. "RGA: A Line-based Collaborative Editing Algorithm" (2011)
3. Bieniusa, A., et al. "An Optimized Conflict-free Replicated Set" (2012)