#!/usr/bin/env bash
set -e

################################################################################
# OYA Zellij Plugin - Install Script
#
# Copies built WASM to Zellij plugins directory
#
# EXPECTS: WASM at crates/oya-zellij/target/wasm32-wasi/release/oya_zellij.wasm
#          (built with Rust 1.83 + wasm32-wasi - see build.sh)
#
# OUTPUT:  ~/.local/share/zellij/plugins/oya_zellij.wasm
################################################################################

PLUGIN_NAME="oya_zellij"
WORKSPACE_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
WASM_PATH="${WORKSPACE_ROOT}/crates/oya-zellij/target/wasm32-wasi/release/${PLUGIN_NAME}.wasm"
INSTALL_DIR="${HOME}/.local/share/zellij/plugins"

echo "🔨 Installing ${PLUGIN_NAME} WASM plugin..."

# Check if WASM file exists
if [ ! -f "${WASM_PATH}" ]; then
    echo "❌ Error: ${WASM_PATH} not found!"
    echo ""
    echo "Build the plugin first:"
    echo "  bash crates/oya-zellij/scripts/build.sh"
    echo ""
    echo "Or use Moon:"
    echo "  moon run oya-zellij:build"
    exit 1
fi

# Create install directory if it doesn't exist
mkdir -p "${INSTALL_DIR}"

# Copy WASM file
cp "${WASM_PATH}" "${INSTALL_DIR}/${PLUGIN_NAME}.wasm"

# Get file size
FILE_SIZE=$(ls -lh "${INSTALL_DIR}/${PLUGIN_NAME}.wasm" | awk '{print $5}')

echo "✅ Plugin installed successfully!"
echo "📦 Location: ${INSTALL_DIR}/${PLUGIN_NAME}.wasm"
echo "📊 Size: ${FILE_SIZE}"
echo ""
echo "To use the plugin, run:"
echo "  zellij --layout oya"
