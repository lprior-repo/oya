#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$REPO_ROOT"

echo "[oya] Starting Restate (Docker)..."
echo "[oya] Ensuring single Restate runtime (stopping user restate.service if active)..."
systemctl --user stop restate.service >/dev/null 2>&1 || true
systemctl --user reset-failed restate.service >/dev/null 2>&1 || true

docker rm -f oya-restate >/dev/null 2>&1 || true
docker compose up -d restate

CONTAINER_STATUS="$(docker ps --filter name=oya-restate --format '{{.Status}}')"
if [[ -z "$CONTAINER_STATUS" ]]; then
	echo "[oya] ERROR: oya-restate container is not running"
	docker logs oya-restate --tail 80 || true
	exit 1
fi

echo "[oya] Waiting for Restate admin health..."
for _ in $(seq 1 30); do
	if curl -fsS "http://127.0.0.1:9070/health" >/dev/null 2>&1; then
		break
	fi
	sleep 1
done

if ! curl -fsS "http://127.0.0.1:9070/health" >/dev/null 2>&1; then
	echo "[oya] ERROR: Restate admin health did not become ready"
	docker logs oya-restate --tail 80 || true
	exit 1
fi

echo "[oya] Building OYA binary..."
moon run :build

systemctl --user stop oya-manual.service >/dev/null 2>&1 || true
systemctl --user reset-failed oya-manual.service >/dev/null 2>&1 || true

MOON_BIN="${MOON_PATH:-$(command -v moon)}"
OPENCODE_BIN="${OPENCODE_PATH:-${HOME}/.local/share/mise/installs/github-sst-opencode/1.2.6/opencode}"
OPENCODE_PORT="${OPENCODE_PORT:-4098}"

# Ensure an isolated OpenCode HTTP server is running for OYA to talk to.
# Local dev mode intentionally runs without HTTP auth.

echo "[oya] Starting isolated OpenCode HTTP server (systemd transient)..."
systemctl --user stop opencode-manual.service >/dev/null 2>&1 || true
systemctl --user reset-failed opencode-manual.service >/dev/null 2>&1 || true
systemd-run --user --unit opencode-manual \
	-E OPENCODE_SERVER_PASSWORD="" \
	--working-directory "$REPO_ROOT" \
	"$OPENCODE_BIN" serve --port "$OPENCODE_PORT" --hostname 127.0.0.1 --print-logs >/dev/null 2>&1 || true

# Wait for HTTP server to be available (short loop)
for i in $(seq 1 20); do
	if curl -fsS "http://127.0.0.1:${OPENCODE_PORT}" >/dev/null 2>&1; then
		break
	fi
	sleep 1
done

echo "[oya] Starting OYA service (systemd transient unit)..."
systemctl --user stop oya-manual.service >/dev/null 2>&1 || true

systemd-run --user --unit oya-manual \
	-E PATH="$PATH" \
	-E MOON_PATH="$MOON_BIN" \
	-E OPENCODE_PATH="$OPENCODE_BIN" \
	-E OYA_OPENCODE_BASE_URL="http://127.0.0.1:${OPENCODE_PORT}" \
	-E OYA_OPENCODE_PASSWORD="" \
	-E OYA_SKIP_ZJJ_GATE="${OYA_SKIP_ZJJ_GATE:-1}" \
	--working-directory "$REPO_ROOT" \
	"$REPO_ROOT/target/release/oya" >/dev/null

echo "[oya] Registering deployment..."
curl -fsS -X POST "http://127.0.0.1:9070/deployments" --json '{"uri":"http://localhost:9080"}' >/dev/null

DEPLOYMENTS_JSON="$(curl -fsS "http://127.0.0.1:9070/deployments")"
if ! python - "$DEPLOYMENTS_JSON" <<'PY'; then
import json
import sys

payload = json.loads(sys.argv[1])
deployments = payload.get("deployments", [])

for deployment in deployments:
    uri = deployment.get("uri", "")
    if not uri.startswith("http://localhost:9080"):
        continue
    service_names = {service.get("name") for service in deployment.get("services", [])}
    required = {"Oya"}
    if required.issubset(service_names):
        sys.exit(0)

print("Deployment for localhost:9080 missing required services", file=sys.stderr)
sys.exit(1)
PY
	echo "[oya] ERROR: deployment is missing one or more required services"
	exit 1
fi

echo "[oya] Runtime ready"
echo "  Admin:   http://127.0.0.1:9070"
echo "  Ingress: http://127.0.0.1:8080"
echo "  Service: http://127.0.0.1:9080"
