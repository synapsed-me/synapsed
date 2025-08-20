#!/bin/bash

# Migration script to copy crates from playground repository to synapsed-me organization structure

set -e

PLAYGROUND_PATH="/tmp/playground/synapsed/core"
SYNAPSED_PATH="/tmp/synapsed-me/synapsed/crates"

echo "🚀 Starting migration of Synapsed crates..."

# Observability crates
echo "📊 Migrating observability crates..."
cp -r "$PLAYGROUND_PATH/synapsed-substrates" "$SYNAPSED_PATH/observability/"
cp -r "$PLAYGROUND_PATH/synapsed-serventis" "$SYNAPSED_PATH/observability/"

# Core infrastructure
echo "🏗️ Migrating core infrastructure..."
cp -r "$PLAYGROUND_PATH/synapsed-core" "$SYNAPSED_PATH/core/"
cp -r "$PLAYGROUND_PATH/synapsed-crypto" "$SYNAPSED_PATH/core/"
cp -r "$PLAYGROUND_PATH/synapsed-gpu" "$SYNAPSED_PATH/core/"

# Storage & Data
echo "💾 Migrating storage crates..."
cp -r "$PLAYGROUND_PATH/synapsed-storage" "$SYNAPSED_PATH/storage/"
cp -r "$PLAYGROUND_PATH/synapsed-crdt" "$SYNAPSED_PATH/storage/"

# Networking
echo "🌐 Migrating network crates..."
cp -r "$PLAYGROUND_PATH/synapsed-net" "$SYNAPSED_PATH/network/"
cp -r "$PLAYGROUND_PATH/synapsed-consensus" "$SYNAPSED_PATH/network/"
cp -r "$PLAYGROUND_PATH/synapsed-routing" "$SYNAPSED_PATH/network/"

# Security & Identity
echo "🔐 Migrating security crates..."
cp -r "$PLAYGROUND_PATH/synapsed-identity" "$SYNAPSED_PATH/security/"
cp -r "$PLAYGROUND_PATH/synapsed-safety" "$SYNAPSED_PATH/security/"

# Compute & Runtime
echo "⚡ Migrating compute crates..."
cp -r "$PLAYGROUND_PATH/synapsed-wasm" "$SYNAPSED_PATH/compute/"
cp -r "$PLAYGROUND_PATH/synapsed-neural-core" "$SYNAPSED_PATH/compute/"

# Applications
echo "📱 Migrating application crates..."
cp -r "$PLAYGROUND_PATH/synapsed-payments" "$SYNAPSED_PATH/applications/"

echo "✅ Migration of existing crates complete!"

# Create directories for IntentProof modules
echo "🎯 Creating IntentProof module directories..."
mkdir -p "$SYNAPSED_PATH/intent/synapsed-intent"
mkdir -p "$SYNAPSED_PATH/intent/synapsed-promise"
mkdir -p "$SYNAPSED_PATH/intent/synapsed-verify"
mkdir -p "$SYNAPSED_PATH/intent/synapsed-enforce"

# Create directories for new application crates
echo "🛠️ Creating new application crate directories..."
mkdir -p "$SYNAPSED_PATH/applications/synapsed-mcp"
mkdir -p "$SYNAPSED_PATH/applications/synapsed-cli"

echo "📁 Directory structure created!"

# Update paths in Cargo.toml files to reflect new structure
echo "🔧 Updating dependency paths..."
find "$SYNAPSED_PATH" -name "Cargo.toml" -type f | while read -r cargo_file; do
    # Update internal dependency paths
    sed -i 's|path = "../synapsed-|path = "../../|g' "$cargo_file" 2>/dev/null || true
    sed -i 's|synapsed_|synapsed-|g' "$cargo_file" 2>/dev/null || true
done

echo "✨ Migration complete! Next steps:"
echo "1. Split IntentProof modules into the intent/ directory"
echo "2. Create MCP and CLI applications"
echo "3. Update all Cargo.toml files with correct paths"
echo "4. Run 'cargo build --all' to verify everything compiles"