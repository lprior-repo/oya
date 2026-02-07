#!/usr/bin/env bash
set -e

################################################################################
# OYA Zellij Plugin - Debug Build Script
#
# LESSON LEARNED: MUST use Rust 1.83 + wasm32-wasi target
#
# WHY: See build.sh for full explanation
#      - Debug mode is faster for development iteration
#      - Still requires wasm32-wasi for _start export
#
# Use this during development, run build.sh for release builds
################################################################################

# Navigate to workspace root
WORKSPACE_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

# Build from plugin directory
cd "${WORKSPACE_ROOT}/crates/oya-zellij"

# Debug build with Rust 1.83 + wasm32-wasi
# ⚠️  DO NOT change Rust version or target!
rustup run 1.83 cargo build --target wasm32-wasi
