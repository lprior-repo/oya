#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$REPO_ROOT"

echo "[oya] Resetting local runtime state..."
systemctl --user stop oya-manual.service >/dev/null 2>&1 || true
systemctl --user stop opencode-manual.service >/dev/null 2>&1 || true
systemctl --user stop restate.service >/dev/null 2>&1 || true
docker compose down --remove-orphans >/dev/null
docker rm -f oya-restate >/dev/null 2>&1 || true
docker volume rm oya_oya_restate_data >/dev/null 2>&1 || true

echo "[oya] Clean state reset complete"
echo "[oya] Run scripts/dev-up.sh to start fresh"
