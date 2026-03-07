# Oya Runtime Flow

This repo runs Oya with Docker Restate and fixed local ports:

- Restate ingress/API: `http://127.0.0.1:909`
- Restate admin/UI: `http://127.0.0.1:9070`
- Oya handler service: `http://127.0.0.1:9180`

`8080` and `9090` are intentionally not used by Restate.

## Prerequisites

- `docker` + `docker compose`
- `restate` CLI
- `gh` CLI authenticated
- `oya` binary available on host

## Bootstrap Runtime

```bash
oya init
```

`oya init` does all required prep each run:

1. Disables/stops user-systemd Restate services.
2. Recreates Docker Restate from `docker-compose.yml` with fresh state.
3. Restarts `oya.service` on `:9180`.
4. Waits for health/discovery checks.
5. Registers Oya handlers with Restate.
6. Verifies `Oya`, `OyaMemory`, `OyaService` are present.

Shutdown:

```bash
oya init --down
```

Validate setup invariants:

```bash
oya doctor
```

`oya doctor` checks ingress/admin/service reachability, Restate service registration,
moon task presence, and GitHub repo slug detection.
Output is emitted as JSONL (`type=check` lines + `type=summary`).

## End-to-End Lifecycle

```bash
oya lifecycle --bead <bead_id> --ingress http://127.0.0.1:909
```

Repo selection:

- If `--repo OWNER/REPO` is provided, that value is used.
- If omitted, Oya auto-detects from:

```bash
gh repo view --json nameWithOwner
```

Status/cancel:

```bash
oya status --key <workflow_key_or_bead_id> --ingress http://127.0.0.1:909
oya cancel --key <workflow_key_or_bead_id> --ingress http://127.0.0.1:909
```

## Opencode Observability (Clean JSON)

`oya status` now includes structured step details for `opencode`:

- `steps[].step == "opencode"`
- `steps[].details.events` (parsed opencode JSON events)
- `steps[].details.stderr`

Low-level trace/debug:

```bash
restate invocations describe <invocation_id>
restate sql "SELECT id,target_service_name,status,completion_result,completion_failure FROM sys_invocation ORDER BY modified_at DESC LIMIT 10"
```

## Deployment Hygiene

```bash
restate deployments list
restate deployments remove <deployment_id> --force -y
```

Steady state should be one active deployment endpoint: `http://127.0.0.1:9180/`.
