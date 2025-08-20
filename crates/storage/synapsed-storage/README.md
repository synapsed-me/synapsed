# Synapsed Storage

A flexible, high-performance storage abstraction layer for the Synapsed ecosystem.

## Features

- **Multiple Storage Backends**: Memory, RocksDB, Sled, SQLite, Redis
- **Pluggable Architecture**: Mix and match backends with enhancement layers
- **Compression Support**: LZ4, Zstandard, Snappy with adaptive selection
- **Caching Layer**: LRU, LFU, ARC with configurable policies
- **Distributed Storage**: Raft consensus, replication, and partitioning
- **Async-First**: Built on Tokio for high-performance async operations
- **Zero-Copy**: Uses `bytes::Bytes` to minimize memory copying

## Quick Start

```rust
use synapsed_storage::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Create a storage instance with caching and compression
    let storage = StorageBuilder::new(
        StorageConfig::RocksDb(RocksDbConfig {
            path: "./data".into(),
            options: Default::default(),
        })
    )
    .with_cache(CacheConfig {
        cache_type: CacheType::Lru,
        max_entries: 10_000,
        ..Default::default()
    })
    .with_compression(CompressionConfig {
        enabled: true,
        algorithm: CompressionAlgorithm::Lz4,
        min_size: 1024,
        level: 3,
    })
    .build()
    .await?;

    // Use the storage
    storage.put(b"key", b"value").await?;
    let value = storage.get(b"key").await?;
    
    Ok(())
}
```

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed design documentation.

## Development

```bash
# Run tests
cargo test

# Run benchmarks
cargo bench

# Build with all features
cargo build --all-features

# Build with specific backend
cargo build --features rocksdb,compression
```

## License

Dual-licensed under MIT and Apache 2.0.