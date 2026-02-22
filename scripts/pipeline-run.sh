#!/usr/bin/env bash
set -euo pipefail

echo "[oya] scripts/pipeline-run.sh is temporarily disabled."
echo "[oya] Pipeline execution is blocked until re-enabled."
exit 1

if [[ $# -lt 2 ]]; then
	echo "Usage: scripts/pipeline-run.sh <run_id> <bead_id> [context]"
	exit 1
fi

RUN_ID="$1"
BEAD_ID="$2"
CONTEXT="${3:-local docker validation}"

if ! br show "$BEAD_ID" --json >/dev/null 2>&1; then
	echo "[oya] ERROR: bead '$BEAD_ID' not found (run 'br list' to choose a valid id)"
	exit 1
fi

curl -fsS -X POST "http://127.0.0.1:8080/OyaOrchestrator/${RUN_ID}/run/send" \
	-H "content-type: application/json" \
	-d "{\"bead_id\":\"${BEAD_ID}\",\"context\":\"${CONTEXT}\"}" >/dev/null

echo "[oya] Started run_id=${RUN_ID} bead_id=${BEAD_ID}"

while true; do
	INV_JSON="$(curl -fsS http://127.0.0.1:9070/query --json "{\"query\":\"select id, status, completion_result, journal_size, modified_at from sys_invocation where target_service_name = 'OyaOrchestrator' and target_service_key = '${RUN_ID}' and target_handler_name = 'run' order by modified_at desc limit 1;\"}")"
	INV_ID="$(printf '%s' "$INV_JSON" | python -c 'import json,sys; rows=json.load(sys.stdin).get("rows", []); print(rows[0]["id"] if rows else "")')"
	INV_STATUS="$(printf '%s' "$INV_JSON" | python -c 'import json,sys; rows=json.load(sys.stdin).get("rows", []); print(rows[0].get("status", "") if rows else "")')"
	INV_RESULT="$(printf '%s' "$INV_JSON" | python -c 'import json,sys; rows=json.load(sys.stdin).get("rows", []); print(rows[0].get("completion_result", "") if rows else "")')"
	JOURNAL_SIZE="$(printf '%s' "$INV_JSON" | python -c 'import json,sys; rows=json.load(sys.stdin).get("rows", []); print(rows[0].get("journal_size", "") if rows else "")')"

	echo "[oya] invocation=${INV_ID} status=${INV_STATUS} result=${INV_RESULT} journal=${JOURNAL_SIZE}"

	if [[ "$INV_STATUS" == "completed" ]]; then
		if [[ "$INV_RESULT" != "success" ]]; then
			FAILURE="$(curl -fsS http://127.0.0.1:9070/query --json "{\"query\":\"select completion_failure from sys_invocation where id = '${INV_ID}' limit 1;\"}" | python -c 'import json,sys; rows=json.load(sys.stdin).get("rows", []); print(rows[0].get("completion_failure", "") if rows else "")')"
			echo "[oya] ERROR: invocation failed: ${FAILURE}"
			exit 1
		fi

		STATE_STATUS="$(curl -fsS http://127.0.0.1:9070/query --json "{\"query\":\"select value_utf8 from state where service_name = 'OyaOrchestrator' and service_key = '${RUN_ID}' and key = 'state' limit 1;\"}" | python -c 'import json,sys; rows=json.load(sys.stdin).get("rows", []); raw=rows[0].get("value_utf8", "") if rows else ""; outer=json.loads(raw) if raw else "{}"; state=json.loads(outer) if isinstance(outer, str) else outer; print(state.get("status", ""))')"
		if [[ "$STATE_STATUS" != "shipped" ]]; then
			echo "[oya] ERROR: run completed at transport level but orchestration status is '${STATE_STATUS}' (expected 'shipped')"
			exit 1
		fi

		break
	fi

	sleep 5
done

echo "[oya] Final state captured for ${RUN_ID}"
