#!/bin/bash

# Build script for the Synapsed demo
# Excludes GPU-dependent crates that require CUDA

set -e

echo "Building Synapsed demo (without GPU acceleration)..."
echo "================================================="

# Build all crates except GPU-dependent ones
cargo build --workspace \
    --exclude synapsed-gpu \
    --exclude synapsed-neural-core \
    --release

echo ""
echo "Build completed successfully!"
echo "To run the demo:"
echo "  ./run-demo.sh"