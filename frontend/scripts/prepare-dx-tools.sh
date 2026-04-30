#!/usr/bin/env sh
set -eu

dx_home="${DX_HOME:?DX_HOME must be set before preparing Dioxus tools}"
wasm_opt_dir="$dx_home/tools/binaryen-129/bin"

mkdir -p "$wasm_opt_dir"
cp scripts/wasm-opt-level0 "$wasm_opt_dir/wasm-opt"
chmod +x "$wasm_opt_dir/wasm-opt"
