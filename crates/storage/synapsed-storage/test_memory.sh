#!/bin/bash
# Simple test script for memory backend

cd /workspaces/playground/synapsed/core/synapsed-storage

# Run only the memory backend unit tests
cargo test --lib backends::memory::tests --no-default-features --features memory