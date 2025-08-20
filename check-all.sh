#!/bin/bash

# Check compilation status of all crates

echo "🔍 Checking compilation status of all crates..."
echo ""

CRATES=(
    "synapsed-core"
    "synapsed-crypto"
    "synapsed-gpu"
    "synapsed-substrates"
    "synapsed-serventis"
    "synapsed-storage"
    "synapsed-crdt"
    "synapsed-net"
    "synapsed-consensus"
    "synapsed-routing"
    "synapsed-identity"
    "synapsed-safety"
    "synapsed-wasm"
    "synapsed-neural-core"
    "synapsed-payments"
    "synapsed-intent"
)

SUCCESS=()
FAILED=()

for crate in "${CRATES[@]}"; do
    printf "Checking %-25s ... " "$crate"
    if cargo check -p "$crate" 2>/dev/null; then
        echo "✅"
        SUCCESS+=("$crate")
    else
        echo "❌"
        FAILED+=("$crate")
    fi
done

echo ""
echo "📊 Summary:"
echo "  ✅ Success: ${#SUCCESS[@]} crates"
echo "  ❌ Failed:  ${#FAILED[@]} crates"

if [ ${#SUCCESS[@]} -gt 0 ]; then
    echo ""
    echo "Working crates:"
    for crate in "${SUCCESS[@]}"; do
        echo "  - $crate"
    done
fi

if [ ${#FAILED[@]} -gt 0 ]; then
    echo ""
    echo "Crates with issues:"
    for crate in "${FAILED[@]}"; do
        echo "  - $crate"
    done
fi