#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$REPO_ROOT"

echo "[oya] Starting Restate (Docker)..."
docker rm -f oya-restate >/dev/null 2>&1 || true
docker compose up -d restate

echo "[oya] Waiting for Restate admin health..."
for _ in $(seq 1 30); do
	if curl -fsS "http://127.0.0.1:9070/health" >/dev/null 2>&1; then
		break
	fi
	sleep 1
done

echo "[oya] Building OYA binary..."
moon run :build

echo "[oya] Starting OYA service (systemd transient unit)..."
systemctl --user stop oya-manual.service >/dev/null 2>&1 || true
systemctl --user reset-failed oya-manual.service >/dev/null 2>&1 || true

MOON_BIN="${MOON_PATH:-$(command -v moon)}"
OPENCODE_BIN="${OPENCODE_PATH:-${HOME}/.local/share/mise/installs/github-sst-opencode/1.2.6/opencode}"

# Ensure an isolated OpenCode HTTP server is running for OYA to talk to.
# Generate a strong password unless one is provided in the environment.
if [ -z "${OPENCODE_SERVER_PASSWORD:-}" ]; then
	if command -v openssl >/dev/null 2>&1; then
		OPENCODE_SERVER_PASSWORD=$(openssl rand -hex 16)
	elif command -v uuidgen >/dev/null 2>&1; then
		OPENCODE_SERVER_PASSWORD=$(uuidgen)
	else
		OPENCODE_SERVER_PASSWORD="$(date +%s)-$$"
	fi
fi

echo "[oya] Starting isolated OpenCode HTTP server (systemd transient)..."
systemctl --user stop opencode-manual.service >/dev/null 2>&1 || true
systemctl --user reset-failed opencode-manual.service >/dev/null 2>&1 || true
systemd-run --user --unit opencode-manual \
	-E OPENCODE_SERVER_PASSWORD="$OPENCODE_SERVER_PASSWORD" \
	--working-directory "$REPO_ROOT" \
	"$OPENCODE_BIN" serve --port 4097 --hostname 127.0.0.1 --print-logs >/dev/null 2>&1 || true

# Wait for HTTP server to be available (short loop)
for i in $(seq 1 20); do
	if curl -fsS "http://127.0.0.1:4097" >/dev/null 2>&1; then
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
	-E OYA_OPENCODE_BASE_URL="http://127.0.0.1:4097" \
	-E OYA_OPENCODE_PASSWORD="$OPENCODE_SERVER_PASSWORD" \
	-E OYA_SKIP_ZJJ_GATE="${OYA_SKIP_ZJJ_GATE:-1}" \
	--working-directory "$REPO_ROOT" \
	"$REPO_ROOT/target/release/oya" >/dev/null

echo "[oya] Registering deployment..."
curl -fsS -X POST "http://127.0.0.1:9070/deployments" --json '{"uri":"http://localhost:9080"}' >/dev/null || true

echo "[oya] Runtime ready"
echo "  Admin:   http://127.0.0.1:9070"
echo "  Ingress: http://127.0.0.1:8080"
echo "  Service: http://127.0.0.1:9080"
