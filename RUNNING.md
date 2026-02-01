# Running OYA Application

## Prerequisites

Ensure you have the required tools installed via mise:
- trunk (for Leptos frontend): `mise use -g cargo:trunk@latest`
- Rust toolchain (already configured)

## Quick Start

### Option 1: Run Full Stack (Frontend + Backend)

Open two terminal windows:

**Terminal 1 - API Server:**
```bash
moon run :serve-api
# Server will start on http://localhost:3000
```

**Terminal 2 - UI Server:**
```bash
moon run :serve-ui
# UI will start on http://localhost:8080
# Opens browser automatically
```

### Option 2: Development Workflow

```bash
# Fast checks before running
moon run :quick  # 6-7ms with cache

# Build and run API server
cargo run --bin oya-server

# In another terminal, serve the UI
cd crates/oya-ui && trunk serve --open
```

## Architecture

- **Frontend**: Leptos 0.7 WASM app (port 8080)
  - Client-side rendering
  - Canvas-based graph visualization
  - Hot reload enabled

- **Backend**: Axum REST API (port 3000)
  - Health checks
  - Workflow endpoints
  - Beads integration

## Available Endpoints

### API (http://localhost:3000)
- `GET /health` - Health check

### UI (http://localhost:8080)
- `/` - Home
- `/dashboard` - Dashboard view
- `/tasks` - Tasks list
- `/beads` - Beads view

## Moon Tasks

```bash
moon run :serve-ui    # Start Leptos frontend
moon run :serve-api   # Start Axum backend
moon run :quick       # Fast lint check
moon run :ci          # Full CI pipeline
moon run :build       # Release build
```
