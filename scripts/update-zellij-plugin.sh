#!/usr/bin/env bash
set -euo pipefail

# Quick update script for Zellij plugin development
# Faster than full install - assumes directories already exist

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WASM_SOURCE="${PROJECT_ROOT}/target/wasm32-wasip1/release/oya_zellij.wasm"
WASM_DEST="${HOME}/.local/share/zellij/plugins/oya_zellij.wasm"

echo "🔄 Rebuilding Zellij plugin..."
cd "$PROJECT_ROOT"
cargo build --release --target wasm32-wasip1 -p oya-zellij 2>&1 | grep -E "(Compiling|Finished|error)" || true

echo "✅ Installing to Zellij..."
mkdir -p "${HOME}/.local/share/zellij/plugins"
cp "$WASM_SOURCE" "$WASM_DEST"

echo "✨ Done! Restart Zellij to see changes."
