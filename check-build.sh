#!/bin/bash

# Simple build check

echo "🔍 Checking if workspace can be parsed..."
if cargo metadata --no-deps > /dev/null 2>&1; then
    echo "✅ Workspace metadata is valid"
else
    echo "❌ Workspace has issues"
    cargo metadata --no-deps
    exit 1
fi

echo ""
echo "🏗️ Attempting cargo check (syntax only)..."
if cargo check --all 2>&1 | tee /tmp/cargo-check.log | grep -q "error"; then
    echo "⚠️ There are compilation errors. First few errors:"
    grep "error" /tmp/cargo-check.log | head -10
else
    echo "✅ All crates pass syntax check!"
fi

echo ""
echo "📦 Crates in workspace:"
cargo metadata --no-deps --format-version 1 | jq -r '.packages[].name' | sort

echo ""
echo "💡 To see full errors, run: cargo check --all 2>&1 | less"