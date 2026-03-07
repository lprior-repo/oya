#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Keep Restate Docker-only for local consistency.
systemctl --user disable --now restate.service >/dev/null 2>&1 || true
systemctl --user stop restate-manual.service >/dev/null 2>&1 || true

docker compose -f "$ROOT_DIR/docker-compose.yml" up -d restate

for _ in $(seq 1 20); do
	if curl -sf "http://127.0.0.1:909/restate/health" >/dev/null; then
		echo "[oya] Docker Restate is healthy: http://127.0.0.1:909/restate/health"
		exit 0
	fi
	sleep 1
done

echo "[oya] Restate did not become healthy within timeout" >&2
exit 1
