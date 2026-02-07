#!/usr/bin/env bash
set -e

################################################################################
# OYA Zellij Plugin - Quick Update Script
#
# Development workflow: rebuild + reinstall in one command
#
# Use this while iterating on plugin code (faster than manual commands)
################################################################################

echo "🔄 Quick rebuild and reinstall..."

# Build with Rust 1.83 + wasm32-wasi
bash crates/oya-zellij/scripts/build.sh

# Install to Zellij
bash crates/oya-zellij/scripts/install-zellij-plugin.sh

echo ""
echo "🎉 Update complete! Restart Zellij to see changes."
echo ""
echo "Test with:"
echo "  zellij --layout oya"
