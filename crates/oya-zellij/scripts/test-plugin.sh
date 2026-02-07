#!/usr/bin/env bash
set -e

################################################################################
# OYA Zellij Plugin - Test Script
#
# Verifies WASM has required _start export and other symbols
# Run this after build to catch configuration errors early
################################################################################

# Navigate to workspace root
WORKSPACE_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "${WORKSPACE_ROOT}"

echo "🔍 Checking WASM exports..."

if command -v llvm-nm >/dev/null 2>&1; then
    exports=$(llvm-nm crates/oya-zellij/target/wasm32-wasi/release/oya_zellij.wasm 2>/dev/null | grep " T " | grep -v "^_" || echo "")

    if echo "$exports" | grep -q "_start"; then
        echo "✅ Plugin has _start export"
    else
        echo "❌ Plugin missing _start export!"
        echo "This usually means the plugin was built with wrong Rust version or target."
        echo "Fix: bash crates/oya-zellij/scripts/build.sh"
        exit 1
    fi

    if echo "$exports" | grep -q "load"; then
        echo "✅ Plugin has load export"
    else
        echo "❌ Plugin missing load export!"
        exit 1
    fi

    echo "📊 All exports:"
    echo "$exports"
else
    echo "⚠️  llvm-nm not found, skipping export verification"
    echo "Install llvm for full verification (optional)"
fi

echo ""
echo "✅ Plugin looks good!"
