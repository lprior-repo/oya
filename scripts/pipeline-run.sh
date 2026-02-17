#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
	echo "Usage: scripts/pipeline-run.sh <run_id> <bead_id> [context]"
	exit 1
fi

RUN_ID="$1"
BEAD_ID="$2"
CONTEXT="${3:-local docker validation}"

curl -fsS -X POST "http://127.0.0.1:8080/OyaOrchestrator/${RUN_ID}/start/send" \
	-H "content-type: application/json" \
	-d "{\"bead_id\":\"${BEAD_ID}\",\"context\":\"${CONTEXT}\"}" >/dev/null

echo "[oya] Started run_id=${RUN_ID} bead_id=${BEAD_ID}"

while true; do
	INV_JSON="$(curl -fsS http://127.0.0.1:9070/query --json "{\"query\":\"select id, status, completion_result, journal_size, modified_at from sys_invocation where target_service_name = 'OyaOrchestrator' and target_service_key = '${RUN_ID}' and target_handler_name = 'start' order by modified_at desc limit 1;\"}")"
	INV_ID="$(printf '%s' "$INV_JSON" | python -c 'import json,sys; rows=json.load(sys.stdin).get("rows", []); print(rows[0]["id"] if rows else "")')"
	INV_STATUS="$(printf '%s' "$INV_JSON" | python -c 'import json,sys; rows=json.load(sys.stdin).get("rows", []); print(rows[0].get("status", "") if rows else "")')"
	INV_RESULT="$(printf '%s' "$INV_JSON" | python -c 'import json,sys; rows=json.load(sys.stdin).get("rows", []); print(rows[0].get("completion_result", "") if rows else "")')"
	JOURNAL_SIZE="$(printf '%s' "$INV_JSON" | python -c 'import json,sys; rows=json.load(sys.stdin).get("rows", []); print(rows[0].get("journal_size", "") if rows else "")')"

	echo "[oya] invocation=${INV_ID} status=${INV_STATUS} result=${INV_RESULT} journal=${JOURNAL_SIZE}"

	if [[ "$INV_STATUS" == "completed" ]]; then
		break
	fi

	sleep 5
done

echo "[oya] Final state captured for ${RUN_ID}"
