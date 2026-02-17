# QA Restate Validation Learnings

## Purpose

This document captures what we validated in live QA against Restate and how to re-run it reliably.

## What We Learned

- Restate admin endpoints on `9070` are the fastest smoke check for service registration.
- Long-running `start` invocations can make workflow-level probes feel hung while the object is busy.
- A lightweight object handler is required for deterministic liveness checks during QA.
- The orchestrator now includes `ping` for that purpose.

## Live Validation Flow (Docker-First Default)

1. Start runtime with Docker Restate + local OYA service:

```bash
scripts/dev-up.sh
```

2. Validate Restate admin health:

```bash
curl -sS -i http://127.0.0.1:9070/health
curl -sS -i http://127.0.0.1:9070/services
```

Expected:

- `200 OK` from `/health`
- `200 OK` from `/services` and an `OyaOrchestrator` entry

3. Validate ingress liveness:

```bash
curl -sS -i http://127.0.0.1:8080/restate/health
```

Expected:

- `200 OK`

4. Start one pipeline run:

```bash
scripts/pipeline-run.sh qa-run-001 qa-bead-001 "qa validation"
```

5. Validate input contract failure path:

```bash
curl -sS -i -X POST http://127.0.0.1:8080/OyaOrchestrator/qa-invalid-json/start \
  -H "content-type: application/json" \
  -d "not-json"
```

Expected: `400 Bad Request` with decode error details.

## Reliability Guardrails Added

- External command execution in orchestrator stages now has explicit timeout boundaries.
- Timeout failures return actionable output instead of stalling the stage indefinitely.

## Runtime Commands

- Start: `scripts/dev-up.sh`
- Stop: `scripts/dev-down.sh`
- Full reset (including Restate local data volume): `scripts/dev-reset.sh`

Local Docker runtime sets `OYA_SKIP_ZJJ_GATE=1` by default, so ship-gate does not require being
inside a zjj workspace during pipeline validation.

## Replay Safety Note

If workflow code changes while old invocations are still replaying, Restate can raise replay
mismatch errors (for example: previous journal recorded `clear state` but current code emits
`set state`).

In local development, when this happens:

```bash
scripts/dev-reset.sh
scripts/dev-up.sh
```

This clears stale invocation/journal state so the new workflow revision can run deterministically.

## Recommended QA Gate Set

```bash
moon run :quick
moon run :ci
```

Run the live Restate flow above after gates pass.
