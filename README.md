# Oya Runtime Flow

This repo runs Oya with Restate in a Docker-first setup and a fixed local port map:

- Restate ingress/API: `http://127.0.0.1:909`
- Restate admin/UI: `http://127.0.0.1:9070`
- Oya service endpoint: `http://127.0.0.1:9180`

The host ports `8080` and `9090` are intentionally not used by Restate.

## Prerequisites

- `docker` + `docker compose`
- `restate` CLI
- `gh` CLI authenticated to GitHub
- `oya` binary available (for local service + CLI commands)

## Runtime Bootstrap

Start or refresh runtime:

```bash
oya init
```

What `oya init` does:

1. Disables/stops user-level systemd Restate services.
2. Recreates Restate from `docker-compose.yml` with fresh state.
3. Restarts `oya.service` (local Oya handler on `:9180`).
4. Waits for health checks (`/restate/health`, `/discover`).
5. Registers handlers with Restate.
6. Validates required services: `Oya`, `OyaMemory`, `OyaService`.

Stop runtime:

```bash
oya init --down
```

## End-to-End Lifecycle Flow

Run lifecycle for a bead:

```bash
oya lifecycle --bead <bead_id> --ingress http://127.0.0.1:909
```

Repo behavior:

- If `--repo OWNER/REPO` is provided, that value is used.
- If `--repo` is omitted, Oya auto-detects the current repo using:

```bash
gh repo view --json nameWithOwner
```

Status and cancel:

```bash
oya status --key <workflow_key> --ingress http://127.0.0.1:909
oya cancel --key <workflow_key> --ingress http://127.0.0.1:909
```

## Keep Deployments Clean

Inspect deployments:

```bash
restate deployments list
```

Remove stale deployments (old endpoints/ports):

```bash
restate deployments remove <deployment_id> --force -y
```

Recommended steady state: exactly one active deployment for `http://127.0.0.1:9180/`.
