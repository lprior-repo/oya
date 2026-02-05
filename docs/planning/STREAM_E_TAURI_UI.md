# Stream E: Tauri Desktop UI + axum Backend

**Duration**: Weeks 3-6 (4 weeks)
**LOC**: ~3.5k
**Priority**: High (user-facing MVP component)

## Overview

Step Functions-like DAG visualization with real-time WebSocket updates in a native desktop app using Tauri + Leptos + axum.

---

## Bead Breakdown

**Total Beads**: 6
**Planning Session**: stream-e-tauri-ui (COMPLETE)

| # | Bead ID | Title | Type | Priority | Effort | Description |
|---|---------|-------|------|----------|--------|-------------|
| 1 | intent-cli-20260201020059-ttfabixq | backend: Implement axum REST API with tower middleware | feature | 1 | 4hr | Implement axum HTTP server with 4 REST endpoints (create workflow, query status, cancel, health check) and tower middleware (CORS, tracing, compression). Provides backend API for Tauri frontend. |
| 2 | intent-cli-20260201020059-hwlgqn0s | backend: Implement WebSocket server with bincode binary streaming | feature | 1 | 4hr | Implement WebSocket server for real-time bead event streaming using bincode binary protocol. Broadcasts BeadEvent updates to all connected clients with <50ms latency. |
| 3 | intent-cli-20260201020059-o7uvwmxl | ui: Implement Tauri 2.0 scaffold with Leptos CSR frontend | feature | 1 | 4hr | Create Tauri 2.0 desktop app with Leptos client-side rendering (CSR) frontend. Sets up project structure, build pipeline, HTTP client, and WebSocket connection to axum backend. |
| 4 | intent-cli-20260201020059-xrlsf0mp | ui: Implement DAG visualization with Leptos components | feature | 2 | 4hr | Build interactive DAG visualization using Leptos components and Canvas API. Renders bead nodes with state colors, dependency edges, pan/zoom controls. Force-directed layout for 50+ node graphs. |
| 5 | intent-cli-20260201020059-4eih0cdy | ui: Integrate WebSocket for real-time bead status updates | feature | 1 | 2hr | Connect Leptos frontend to WebSocket server. Subscribe to bincode event stream, deserialize BeadEvent updates, reactively update DAG visualization with <50ms latency using Leptos signals. |
| 6 | intent-cli-20260201020059-n6vt99rk | ui: Implement execution timeline and manual bead controls | feature | 2 | 4hr | Build Step Functions-like execution timeline component with Leptos. Shows phase progress, event history, manual actions (cancel/retry buttons), error visualization, responsive layout. |

---

## Architecture

### Frontend: Tauri + Leptos
- **Tauri 2.0**: Native desktop window (cross-platform: Linux, macOS, Windows)
- **Leptos 0.7 CSR**: Type-safe reactive UI in Rust WASM
- **Communication**: HTTP/REST + WebSocket to axum backend

### Backend: axum + tower
- **axum 0.7**: Web framework for REST API + WebSocket server
- **tower**: Middleware (CORS, tracing, compression)
- **bincode**: Binary serialization for WebSocket frames

### Data Flow
```
Leptos Frontend (WASM)
  ↓ HTTP POST
axum Backend (REST API) → Orchestrator (create bead)
  ↓ WebSocket (bincode)
Leptos Frontend (WASM) → Reactive UI update (<50ms)
```

---

## Technical Details

### REST API Endpoints
1. `POST /api/workflows` - Create new bead
2. `GET /api/beads/:id` - Query bead status
3. `POST /api/beads/:id/cancel` - Cancel execution
4. `GET /api/health` - System health check

### WebSocket Protocol
- **URL**: `ws://localhost:8080/api/ws`
- **Format**: bincode binary frames
- **Messages**: Streamed `BeadEvent` updates
- **Latency**: <50ms from event to UI update

### DAG Visualization Features
- Force-directed graph layout (Canvas API)
- State-based node colors:
  - Pending: gray
  - Scheduled: yellow
  - Running: blue
  - Completed: green
  - Failed: red
- Dependency edges with arrows
- Pan/zoom controls (mouse events)
- Interactive node selection

### Execution Timeline Features
- Phase progress indicator (15 phases for TDD15)
- Event history (chronological list)
- Manual controls:
  - Cancel running bead
  - Retry failed bead
  - View event log
- Error visualization (conditional rendering)
- Responsive layout (Tailwind or Leptos styling)

---

## Quality Gates

### Per-Bead Gates
- All tests pass (happy path + error path)
- CUE schema validation passes
- No unwraps, no panics (clippy forbid)
- Railway-Oriented Programming (Result<T, Error>)
- Moon quick check passes (6-7ms with cache)

### Integration Gates
- Tauri app launches and connects to axum
- WebSocket establishes connection
- REST API responds to all 4 endpoints
- DAG renders 50-node graph correctly
- Real-time updates <50ms latency
- Manual controls work (cancel, retry)
- Full user workflow complete (create → visualize → watch → view results)

---

## Success Criteria

### MVP Complete When:
- ✅ Tauri desktop app launches
- ✅ axum backend serves REST API + WebSocket
- ✅ Leptos frontend renders DAG visualization
- ✅ Real-time WebSocket updates working
- ✅ Manual bead controls functional (cancel, retry)
- ✅ Execution timeline displays phase progress
- ✅ <50ms UI update latency
- ✅ All 6 beads implemented and tested
- ✅ User acceptance test passes (full workflow)

---

## Dependencies

### Rust Crates
```toml
# Backend
axum = "0.7"
tower = "0.5"
tower-http = "0.6"
bincode = "1.3"

# Frontend (Tauri app)
tauri = "2.0"
leptos = { version = "0.7", features = ["csr"] }
gloo-net = "0.6"  # WebSocket client
web-sys = "0.3"   # Canvas API
```

### External Tools
- Node.js (for Tauri build)
- Rust nightly (for Leptos WASM)

---

## Critical Files

### Backend (axum)
- `crates/oya-web/src/lib.rs` - axum server setup
- `crates/oya-web/src/routes.rs` - REST API routes
- `crates/oya-web/src/websocket.rs` - WebSocket server
- `crates/oya-web/Cargo.toml` - Backend dependencies

### Frontend (Tauri + Leptos)
- `crates/oya-ui/src-tauri/main.rs` - Tauri wrapper
- `crates/oya-ui/src/app.rs` - Leptos frontend app
- `crates/oya-ui/src/components/dag_viz.rs` - DAG visualization component
- `crates/oya-ui/src/components/timeline.rs` - Execution timeline component
- `crates/oya-ui/src/websocket.rs` - WebSocket client
- `crates/oya-ui/Cargo.toml` - Frontend dependencies
- `crates/oya-ui/tauri.conf.json` - Tauri configuration

### Tests
- `crates/oya-web/tests/rest_api_test.rs` - REST API integration tests
- `crates/oya-web/tests/websocket_test.rs` - WebSocket integration tests
- `crates/oya-ui/tests/ui_test.rs` - Frontend component tests

---

## Risk Mitigation

### Risk 1: Tauri + Leptos Integration
**Mitigation**:
- Follow official Tauri + Leptos template
- Use WebSocket for loose coupling (Tauri backend doesn't know about Leptos)
- If WebSocket issues, fallback to Tauri IPC commands
- If Leptos issues, fallback to vanilla JS + Vite

### Risk 2: Canvas Performance (50+ nodes)
**Mitigation**:
- Use force-directed layout with caching
- Render on RequestAnimationFrame
- Implement viewport culling (only render visible nodes)
- If too slow, fallback to SVG with d3.js

### Risk 3: WebSocket Latency
**Mitigation**:
- Use bincode binary protocol (faster than JSON)
- Batch events if >100/sec
- Debounce UI updates (max 60 FPS)
- Monitor latency in production

---

## Planning Session Details

**Session ID**: stream-e-tauri-ui
**Status**: COMPLETE
**Created**: 2026-02-01T01:38:29
**Tasks Generated**: 6
**Beads Created**: 6
**CUE Schemas**: 6 (all in `.beads/schemas/`)

**State File**: `~/.local/share/planner/sessions/stream-e-tauri-ui.yml`
