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
DEFAULT_OPENCODE_BIN="${HOME}/.local/share/mise/installs/github-sst-opencode/latest/opencode"
OPENCODE_BIN="${OPENCODE_PATH:-$(command -v opencode 2>/dev/null || true)}"
if [[ -z "$OPENCODE_BIN" ]]; then
	OPENCODE_BIN="$DEFAULT_OPENCODE_BIN"
fi

if [[ ! -x "$OPENCODE_BIN" ]]; then
	echo "[oya] ERROR: OpenCode binary not found or not executable: $OPENCODE_BIN"
	echo "[oya] Set OPENCODE_PATH or ensure 'opencode' is in PATH"
	exit 1
fi
OPENCODE_PORT="${OPENCODE_PORT:-4096}"

resolve_opencode_port() {
	local preferred_port="$1"
	python - "$preferred_port" <<'PY'
import socket
import sys

preferred = int(sys.argv[1])
for candidate in [preferred, preferred + 1, preferred + 2, preferred + 3]:
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    try:
        sock.bind(("127.0.0.1", candidate))
    except OSError:
        continue
    finally:
        sock.close()
    print(candidate)
    sys.exit(0)

print("")
sys.exit(1)
PY
}

OPENCODE_PORT="$(resolve_opencode_port "$OPENCODE_PORT")"
if [[ -z "$OPENCODE_PORT" ]]; then
	echo "[oya] ERROR: could not find an available OpenCode port near 4096"
	exit 1
fi

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
	if curl -fsS "http://127.0.0.1:${OPENCODE_PORT}/global/health" >/dev/null 2>&1; then
		break
	fi
	sleep 1
done

if ! curl -fsS "http://127.0.0.1:${OPENCODE_PORT}/global/health" >/dev/null 2>&1; then
	echo "[oya] ERROR: OpenCode HTTP server did not become ready on port ${OPENCODE_PORT}"
	journalctl --user -u opencode-manual.service -n 50 --no-pager || true
	exit 1
fi

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
DEPLOYMENT_RESPONSE="$(curl -fsS -X POST "http://127.0.0.1:9070/deployments" --json '{"uri":"http://localhost:9080"}')"
DEPLOYMENT_ID="$(
	python - "$DEPLOYMENT_RESPONSE" <<'PY'
import json
import sys

payload = json.loads(sys.argv[1])
print(payload.get("id", ""))
PY
)"

if [[ -z "$DEPLOYMENT_ID" ]]; then
	echo "[oya] ERROR: could not resolve deployment id from register response"
	exit 1
fi

curl -fsS -X PATCH "http://127.0.0.1:9070/deployments/${DEPLOYMENT_ID}" \
	--json '{"uri":"http://localhost:9080","overwrite":true}' >/dev/null

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
    required = {"Oya", "OyaOrchestrator"}
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
echo "  OpenCode: http://127.0.0.1:${OPENCODE_PORT}"
