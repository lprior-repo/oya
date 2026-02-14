# Running OYA Application

## Quick Start - Single Binary! 🚀

The entire application (frontend + API) runs from a single binary:

```bash
# Option 1: Using moon (recommended)
moon run :serve

# Option 2: Direct cargo
cargo run --release -p oya-web --bin oya-server
```

Then open **http://localhost:3000** in your browser.

- **Frontend**: http://localhost:3000
- **API**: http://localhost:3000/api

## Prerequisites

- trunk (for building frontend): `mise use -g cargo:trunk@latest`
- Rust toolchain (already configured via mise)

## Development Workflow

### 1. Make changes to the frontend
```bash
# After editing crates/oya-ui/src/**
cd crates/oya-ui
mise exec -- trunk build --release

# Then rebuild and restart the server
cargo build --release -p oya-web
./target/release/oya-server
```

### 2. Make changes to the backend
```bash
# After editing crates/oya-web/src/**
cargo build --release -p oya-web
./target/release/oya-server
```

### 3. Fast checks before committing
```bash
moon run :quick  # 6-7ms with cache!
moon run :ci     # Full pipeline if tests pass
```

## Architecture

**Single Binary Full Stack:**
- Frontend: Leptos 0.7 WASM (compiled to /crates/oya-ui/dist)
- Backend: Axum REST API + Static File Server
- Everything served from port 3000

## Available Endpoints

### Frontend (http://localhost:3000)
- `/` - Home
- `/dashboard` - Dashboard view
- `/tasks` - Tasks list
- `/beads` - Beads view

### API (http://localhost:3000/api)
- `GET /api/health` - Health check
- `GET /api/workflows` - Workflow endpoints
- `GET /api/beads` - Beads integration

## Moon Tasks

```bash
moon run :serve       # Start full stack server (single binary)
moon run :quick       # Fast lint check (6-7ms)
moon run :ci          # Full CI pipeline
moon run :build       # Release build all crates
```

## Combative Ralph Loop

Run a long Red-Queen style hardening loop that repeatedly adds adversarial tests, patches code, and re-runs quality gates with Scott Wlaschin + Dan North review criteria.

```bash
# Default (opencode + GLM-5, min=30, max=200)
bash scripts/run_ralph_combative_loop.sh

# Or via Moon
moon run :ralph-combative-loop

# Override defaults
RALPH_MODEL="zai-coding-plan/glm-5" \
RALPH_MIN_ITERATIONS=30 \
RALPH_MAX_ITERATIONS=200 \
bash scripts/run_ralph_combative_loop.sh
```

Useful environment variables:

- `RALPH_MODEL` - model id
- `RALPH_AGENT` - `opencode` / `claude-code` / `codex`
- `RALPH_MIN_ITERATIONS`, `RALPH_MAX_ITERATIONS` - loop bounds
- `RALPH_NO_COMMIT` - `1` (default) avoids auto-commit, set `0` to allow commits
- `RALPH_ALLOW_ALL` - `1` (default) auto-approves tools, set `0` for prompts
- `RALPH_PROMPT_FILE` - custom prompt path

The default completion promise is `COMBATIVE_LOOP_COMPLETE`.

## How It Works

1. **Frontend Build**: `trunk build --release` compiles Leptos to WASM → `crates/oya-ui/dist/`
2. **Backend Build**: `cargo build -p oya-web` includes tower-http static file serving
3. **Single Server**: Axum serves API routes under `/api` and static files for everything else
4. **Zero Configuration**: No nginx, no reverse proxy, just one binary!
