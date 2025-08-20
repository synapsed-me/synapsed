#!/bin/bash

# Fix all path dependencies in Cargo.toml files

echo "🔧 Fixing dependency paths in all Cargo.toml files..."

# Fix paths in synapsed-storage
if [ -f "crates/storage/synapsed-storage/Cargo.toml" ]; then
    sed -i 's|path = "../../substrates"|path = "../../observability/synapsed-substrates"|g' crates/storage/synapsed-storage/Cargo.toml
    sed -i 's|path = "../../serventis"|path = "../../observability/synapsed-serventis"|g' crates/storage/synapsed-storage/Cargo.toml
    sed -i 's|path = "../../core"|path = "../../core/synapsed-core"|g' crates/storage/synapsed-storage/Cargo.toml
    echo "  ✅ Fixed synapsed-storage"
fi

# Fix paths in synapsed-net
if [ -f "crates/network/synapsed-net/Cargo.toml" ]; then
    sed -i 's|path = "../../core"|path = "../../core/synapsed-core"|g' crates/network/synapsed-net/Cargo.toml
    sed -i 's|path = "../../crypto"|path = "../../core/synapsed-crypto"|g' crates/network/synapsed-net/Cargo.toml
    echo "  ✅ Fixed synapsed-net"
fi

# Fix paths in synapsed-identity
if [ -f "crates/security/synapsed-identity/Cargo.toml" ]; then
    sed -i 's|path = "../../core"|path = "../../core/synapsed-core"|g' crates/security/synapsed-identity/Cargo.toml
    sed -i 's|path = "../../crypto"|path = "../../core/synapsed-crypto"|g' crates/security/synapsed-identity/Cargo.toml
    sed -i 's|path = "../../storage"|path = "../../storage/synapsed-storage"|g' crates/security/synapsed-identity/Cargo.toml
    echo "  ✅ Fixed synapsed-identity"
fi

# Fix paths in synapsed-consensus
if [ -f "crates/network/synapsed-consensus/Cargo.toml" ]; then
    sed -i 's|path = "../../core"|path = "../../core/synapsed-core"|g' crates/network/synapsed-consensus/Cargo.toml
    sed -i 's|path = "../../crypto"|path = "../../core/synapsed-crypto"|g' crates/network/synapsed-consensus/Cargo.toml
    sed -i 's|path = "../synapsed-net"|path = "../synapsed-net"|g' crates/network/synapsed-consensus/Cargo.toml
    echo "  ✅ Fixed synapsed-consensus"
fi

# Fix paths in synapsed-crdt
if [ -f "crates/storage/synapsed-crdt/Cargo.toml" ]; then
    sed -i 's|path = "../../core"|path = "../../core/synapsed-core"|g' crates/storage/synapsed-crdt/Cargo.toml
    echo "  ✅ Fixed synapsed-crdt"
fi

# Fix paths in synapsed-safety
if [ -f "crates/security/synapsed-safety/Cargo.toml" ]; then
    sed -i 's|path = "../../core"|path = "../../core/synapsed-core"|g' crates/security/synapsed-safety/Cargo.toml
    sed -i 's|path = "../../substrates"|path = "../../observability/synapsed-substrates"|g' crates/security/synapsed-safety/Cargo.toml
    echo "  ✅ Fixed synapsed-safety"
fi

# Fix paths in synapsed-payments
if [ -f "crates/applications/synapsed-payments/Cargo.toml" ]; then
    sed -i 's|path = "../../core"|path = "../../core/synapsed-core"|g' crates/applications/synapsed-payments/Cargo.toml
    sed -i 's|path = "../../crypto"|path = "../../core/synapsed-crypto"|g' crates/applications/synapsed-payments/Cargo.toml
    sed -i 's|path = "../../identity"|path = "../../security/synapsed-identity"|g' crates/applications/synapsed-payments/Cargo.toml
    sed -i 's|path = "../../storage"|path = "../../storage/synapsed-storage"|g' crates/applications/synapsed-payments/Cargo.toml
    sed -i 's|path = "../../substrates"|path = "../../observability/synapsed-substrates"|g' crates/applications/synapsed-payments/Cargo.toml
    echo "  ✅ Fixed synapsed-payments"
fi

# Fix paths in synapsed-wasm
if [ -f "crates/compute/synapsed-wasm/Cargo.toml" ]; then
    sed -i 's|path = "../../core"|path = "../../core/synapsed-core"|g' crates/compute/synapsed-wasm/Cargo.toml
    sed -i 's|path = "../../crypto"|path = "../../core/synapsed-crypto"|g' crates/compute/synapsed-wasm/Cargo.toml
    sed -i 's|path = "../../crdt"|path = "../../storage/synapsed-crdt"|g' crates/compute/synapsed-wasm/Cargo.toml
    sed -i 's|path = "../../net"|path = "../../network/synapsed-net"|g' crates/compute/synapsed-wasm/Cargo.toml
    sed -i 's|path = "../../storage"|path = "../../storage/synapsed-storage"|g' crates/compute/synapsed-wasm/Cargo.toml
    sed -i 's|path = "../../identity"|path = "../../security/synapsed-identity"|g' crates/compute/synapsed-wasm/Cargo.toml
    echo "  ✅ Fixed synapsed-wasm"
fi

# Fix paths in synapsed-gpu
if [ -f "crates/core/synapsed-gpu/Cargo.toml" ]; then
    sed -i 's|path = "../synapsed-crypto"|path = "../synapsed-crypto"|g' crates/core/synapsed-gpu/Cargo.toml
    sed -i 's|path = "../synapsed-core"|path = "../synapsed-core"|g' crates/core/synapsed-gpu/Cargo.toml
    echo "  ✅ Fixed synapsed-gpu"
fi

echo "✨ Path fixing complete!"