# QA Restate Validation Learnings

## Purpose

This document captures what we validated in live QA against Restate and how to re-run it reliably.

## What We Learned

- Restate admin endpoints on `9070` are the fastest smoke check for service registration.
- Long-running `start` invocations can make workflow-level probes feel hung while the object is busy.
- A lightweight object handler is required for deterministic liveness checks during QA.
- The orchestrator now includes `ping` for that purpose.

## Live Validation Flow

1. Start the OYA endpoint:

```bash
OYA_BIND_ADDR=127.0.0.1:9080 moon run :run
```

2. Validate Restate admin health:

```bash
curl -sS -i http://127.0.0.1:9070/health
curl -sS -i http://127.0.0.1:9070/services
```

Expected:

- `200 OK` from `/health`
- `200 OK` from `/services` and an `OyaOrchestrator` entry

3. Validate object-level liveness through ingress:

```bash
curl -sS -i -X POST http://127.0.0.1:8080/OyaOrchestrator/qa-ping/ping
```

Expected body shape:

```json
"{\"status\":\"ok\",\"service\":\"OyaOrchestrator\"}"
```

If you receive handler-not-found for `ping`, the active Restate deployment is stale and still
serving an older revision. Redeploy/re-register the endpoint before continuing validation.

4. Validate input contract failure path:

```bash
curl -sS -i -X POST http://127.0.0.1:8080/OyaOrchestrator/qa-invalid-json/start \
  -H "content-type: application/json" \
  -d "not-json"
```

Expected: `400 Bad Request` with decode error details.

## Reliability Guardrails Added

- External command execution in orchestrator stages now has explicit timeout boundaries.
- Timeout failures return actionable output instead of stalling the stage indefinitely.

## Current Environment Observation

During QA, Restate registry still exposed only `start` and `get_status` for
`OyaOrchestrator` (revision `2`), so `ping` was not yet routable via ingress. The binary starts
locally on `127.0.0.1:9080`, but ingress behavior is pinned to currently active deployment
metadata.

## Recommended QA Gate Set

```bash
moon run :quick
moon run :ci
```

Run the live Restate flow above after gates pass.
