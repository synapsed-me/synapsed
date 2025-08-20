#!/bin/bash

echo "🔧 Comprehensive path fixing for all Cargo.toml files..."

# Find and fix all incorrect paths
find crates -name "Cargo.toml" -type f | while read -r file; do
    # Backup original
    cp "$file" "$file.bak"
    
    # Fix observability paths
    sed -i 's|path = "../../substrates"|path = "../../observability/synapsed-substrates"|g' "$file"
    sed -i 's|path = "../../serventis"|path = "../../observability/synapsed-serventis"|g' "$file"
    
    # Fix core paths
    sed -i 's|path = "../../core"|path = "../../core/synapsed-core"|g' "$file"
    sed -i 's|path = "../../crypto"|path = "../../core/synapsed-crypto"|g' "$file"
    sed -i 's|path = "../../gpu"|path = "../../core/synapsed-gpu"|g' "$file"
    
    # Fix storage paths
    sed -i 's|path = "../../storage"|path = "../../storage/synapsed-storage"|g' "$file"
    sed -i 's|path = "../../crdt"|path = "../../storage/synapsed-crdt"|g' "$file"
    
    # Fix network paths
    sed -i 's|path = "../../net"|path = "../../network/synapsed-net"|g' "$file"
    sed -i 's|path = "../../consensus"|path = "../../network/synapsed-consensus"|g' "$file"
    sed -i 's|path = "../../routing"|path = "../../network/synapsed-routing"|g' "$file"
    
    # Fix security paths
    sed -i 's|path = "../../identity"|path = "../../security/synapsed-identity"|g' "$file"
    sed -i 's|path = "../../safety"|path = "../../security/synapsed-safety"|g' "$file"
    
    # Fix compute paths
    sed -i 's|path = "../../wasm"|path = "../../compute/synapsed-wasm"|g' "$file"
    sed -i 's|path = "../../neural"|path = "../../compute/synapsed-neural-core"|g' "$file"
    
    # Fix application paths
    sed -i 's|path = "../../payments"|path = "../../applications/synapsed-payments"|g' "$file"
    
    # Check if file changed
    if ! diff -q "$file" "$file.bak" > /dev/null; then
        echo "  ✅ Fixed: $file"
        rm "$file.bak"
    else
        rm "$file.bak"
    fi
done

echo "✨ Path fixing complete!"