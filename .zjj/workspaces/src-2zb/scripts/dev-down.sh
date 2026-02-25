#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$REPO_ROOT"

echo "[oya] Stopping OYA transient unit..."
systemctl --user stop oya-manual.service >/dev/null 2>&1 || true

echo "[oya] Stopping OpenCode transient unit..."
systemctl --user stop opencode-manual.service >/dev/null 2>&1 || true

echo "[oya] Stopping user restate.service (single-runtime enforcement)..."
systemctl --user stop restate.service >/dev/null 2>&1 || true

echo "[oya] Stopping Restate (Docker)..."
docker compose stop restate >/dev/null

echo "[oya] Runtime stopped"
