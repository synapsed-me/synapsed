#!/bin/bash
# Build script for synapsed-wasm

set -e

echo "Building synapsed-wasm..."

# Install wasm-pack if not already installed
if ! command -v wasm-pack &> /dev/null; then
    echo "Installing wasm-pack..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi

# Build for different targets
echo "Building for bundler..."
wasm-pack build --target bundler --out-dir pkg/bundler

echo "Building for web..."
wasm-pack build --target web --out-dir pkg/web

echo "Building for nodejs..."
wasm-pack build --target nodejs --out-dir pkg/node

echo "Build complete!"
echo "Packages available in:"
echo "  - pkg/bundler/ (for webpack/rollup)"
echo "  - pkg/web/ (for direct browser usage)"
echo "  - pkg/node/ (for Node.js)"