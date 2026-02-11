#!/bin/bash
cd /home/lewis/src/oya && rustc --version && cargo --version
echo "=== Checking rustc args ==="
cargo clippy -p orchestrator --print cfg 2>&1 | grep unreachable || echo "No cfg for unreachable"
echo "=== Env vars ==="
env | grep -i rust
