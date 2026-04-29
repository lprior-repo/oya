# Oya Frontend

The Oya frontend is a Dioxus 0.7 web application for designing and inspecting
Oya/Restate workflows. It runs in the browser through WebAssembly, but it is not
a standalone product: release validation assumes the local Oya runtime is
available for Restate-backed panels and execution flows.

## Runtime Ports

- Frontend dev/E2E server: `http://127.0.0.1:8081`
- Restate ingress/API: `http://127.0.0.1:8080`
- Restate admin/API: `http://127.0.0.1:9070`
- Oya handler service: `http://127.0.0.1:9180`

Port `909` is not the local rootless default. The backend CLI starts managed
`restate-server` on `8080` via `oya init`.

## Development Commands

Use Moon tasks only from the workspace root:

```bash
moon run frontend:serve
moon run frontend:build-web
moon run frontend:e2e
moon run frontend:ci
```

Do not run `dx`, `cargo`, or `npm` directly for frontend build/test/lint work;
the Moon tasks provide the supported command surface.

## Release Verification

The current browser E2E gate is Playwright in headless Chromium:

```bash
moon run frontend:e2e
```

The release-blocker fix for `oya-45t` verified the flow editor suite passes
8/8 tests, including sidebar search, context-menu palette behavior, node
selection/deletion, extension-panel actions, drag behavior, and an adversarial
seeded interaction loop.

## Project Structure

- `src/main.rs`: Dioxus application entry point.
- `src/ui/`: UI components for shell, graph canvas, panels, and Restate status.
- `src/graph/`: Workflow graph model, execution runtime, validation, and layout.
- `src/hooks/`: Dioxus signal/store hooks for workflow, selection, canvas, and sync.
- `e2e/`: Playwright browser tests run by `moon run frontend:e2e`.
- `specs/`: Flow and linter specifications.

## Quality Gates

- Fast frontend checks: `moon run frontend:ci`
- Browser workflow checks: `moon run frontend:e2e`
- Production web bundle: `moon run frontend:build-web`
- Full repository gate: `moon run :ci --force`
