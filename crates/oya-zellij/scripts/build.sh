#!/usr/bin/env bash
set -e

################################################################################
# OYA Zellij Plugin - Release Build Script
#
# LESSON LEARNED: MUST use Rust 1.83 + wasm32-wasi target
#
# WHY: Zellij 0.43.1 requires "_start" export in WASM
#      - Rust 1.84+ removed wasm32-wasi (renamed to wasm32-wasip1)
#      - wasm32-wasip1 produces WASM WITHOUT _start → Zellij can't load plugin
#      - Rust 1.83 is the last version with wasm32-wasi support
#
# ERROR IF WRONG: "failed to find function export '_start'"
#
# VERIFICATION: llvm-nm target/wasm32-wasi/release/oya_zellij.wasm | grep " T _start"
#              Should show: 00000c4d T _start
################################################################################

# Navigate to workspace root (for repo detection)
WORKSPACE_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

# Build from plugin directory (standalone workspace avoids edition2024 conflicts)
cd "${WORKSPACE_ROOT}/crates/oya-zellij"

# Build with Rust 1.83 + wasm32-wasi target
# ⚠️  DO NOT change these parameters!
rustup run 1.83 cargo build --release --target wasm32-wasi
