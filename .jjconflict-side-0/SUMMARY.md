# OYA Application - Build Summary

## What We Built

### Single Binary Full Stack Application ✅

**Frontend + Backend in ONE binary** served on http://localhost:3000

### Architecture

```
┌─────────────────────────────────────┐
│   oya-server (Axum + Tower HTTP)    │
│  Single Binary on Port 3000          │
├─────────────────────────────────────┤
│                                      │
│  ┌──────────────┐  ┌──────────────┐ │
│  │  Frontend    │  │  API         │ │
│  │  (Leptos     │  │  (REST)      │ │
│  │   WASM)      │  │              │ │
│  │  Served as   │  │  /api/*      │ │
│  │  static files│  │              │ │
│  └──────────────┘  └──────────────┘ │
│                                      │
└─────────────────────────────────────┘
```

## Key Features

### Frontend (Leptos 0.7 WASM)
- ✅ Client-side rendering
- ✅ Type-safe routing
- ✅ Canvas-based graph visualization components
- ✅ Task list with filtering
- ✅ Pages: Home, Dashboard, Tasks, Beads
- ✅ Compiled to WASM and served as static files

### Backend (Axum REST API)
- ✅ Health endpoint (`/api/health`)
- ✅ Workflow endpoints (`/api/workflows`)
- ✅ Beads integration (`/api/beads/{id}`)
- ✅ Tower middleware: CORS, compression, tracing
- ✅ Serves both static frontend AND API routes

### Development Tools
- ✅ **Moon CI/CD**: 6-7ms cached builds with bazel-remote
- ✅ **Mise**: Tool version management (trunk, rust, etc.)
- ✅ **Trunk**: Leptos WASM bundler
- ✅ **Playwright E2E**: Docker-based testing

## Running the App

### Start Server
```bash
# Using moon (recommended)
moon run :serve

# Or directly
cargo run --release -p oya-web --bin oya-server
```

Then open: **http://localhost:3000**

### Development Workflow

1. **Edit Frontend** (`crates/oya-ui/src/**`):
   ```bash
   moon run :build-ui          # Rebuild WASM
   cargo build --release -p oya-web
   ./target/release/oya-server
   ```

2. **Edit Backend** (`crates/oya-web/src/**`):
   ```bash
   cargo build --release -p oya-web
   ./target/release/oya-server
   ```

3. **Fast Checks**:
   ```bash
   moon run :quick  # 6-7ms with cache!
   ```

### E2E Testing
```bash
docker compose -f docker-compose.playwright.yml run --rm playwright
```

## Project Structure

```
crates/
├── oya-ui/          # Leptos 0.7 frontend (standalone workspace)
│   ├── src/
│   │   ├── app.rs         # Main app component
│   │   ├── router.rs      # Route definitions
│   │   ├── pages/         # Page components
│   │   ├── components/    # Reusable components
│   │   └── models/        # Data models
│   ├── dist/              # Built WASM (trunk build output)
│   └── index.html         # HTML template
│
└── oya-web/         # Axum backend
    ├── src/
    │   ├── routes/        # API routes
    │   ├── actors/        # State management
    │   ├── server.rs      # Server setup
    │   └── bin/
    │       └── server.rs  # Binary entry point
    └── Cargo.toml

tests/
└── e2e/
    └── app.spec.ts        # Playwright tests
```

## Tech Stack

| Component | Technology | Why |
|-----------|-----------|-----|
| Frontend | Leptos 0.7 | Type-safe reactive UI in Rust → WASM |
| Backend | Axum 0.8 | Fast, ergonomic Rust web framework |
| Build | Moon | Cached builds (98.5% faster) |
| Bundler | Trunk | Leptos WASM bundler |
| Testing | Playwright | E2E browser testing |
| Tools | Mise | Version management |

## Zero Configuration

- ❌ No nginx
- ❌ No reverse proxy
- ❌ No separate frontend server
- ✅ Just one binary!

## Next Steps

1. Add more API endpoints
2. Implement real graph data
3. Add WebSocket for real-time updates
4. Expand E2E test coverage
5. Add CSS framework (optional)
