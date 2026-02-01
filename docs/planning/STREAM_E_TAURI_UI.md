# Stream E: Tauri Desktop UI + axum Backend

**Timeline**: Weeks 3-6
**Goal**: Step Functions-like DAG visualization with real-time WebSocket updates in native desktop app
**LOC Target**: ~3.5k LOC
**Status**: Session initialized, beads NOT YET CREATED

## Overview

Stream E implements the user interface for Oya:
- Native desktop app using Tauri 2.0
- Rust WASM frontend using Leptos (type-safe reactive UI)
- axum backend (REST API + WebSocket server)
- Real-time bead status updates via WebSocket
- Step Functions-like DAG visualization and execution history

## Planning Session

**Session ID**: `stream-e-tauri-ui`
**Session File**: `~/.local/share/planner/sessions/stream-e-tauri-ui.yml`
**Status**: INITIALIZED - Beads NOT yet generated

## Architecture

### Tech Stack
- **Frontend**: Tauri (native window) + Leptos (Rust WASM)
- **Backend**: axum (REST + WebSocket) running in orchestrator process
- **Communication**: HTTP/WebSocket from Tauri frontend to axum backend
- **Serialization**: bincode for WebSocket binary frames
- **Real-time**: WebSocket subscriptions to bead events

### Component Structure
```
crates/oya-ui/              # Tauri + Leptos frontend
├── src-tauri/              # Tauri wrapper (Rust)
│   └── main.rs
└── src/                    # Leptos app (WASM)
    ├── app.rs              # Main app component
    ├── components/
    │   ├── dag_viz.rs      # DAG visualization
    │   ├── timeline.rs     # Execution history
    │   └── controls.rs     # Manual actions
    └── websocket.rs        # WebSocket client

crates/oya-web/             # axum backend
├── src/
│   ├── lib.rs              # Server setup
│   ├── routes.rs           # REST API routes
│   └── websocket.rs        # WebSocket server
```

## Planned Features (Week-by-Week)

### Week 3: axum Backend + Tauri Scaffold
**Beads to create**:
1. axum REST API (5 endpoints: create/query/cancel/health/list)
2. WebSocket server (bincode binary frames, event streaming)
3. Tauri 2.0 scaffold (window setup, frontend build)
4. Leptos CSR setup (client-side rendering, routing)

### Week 4: DAG Visualization
**Beads to create**:
1. Leptos DAG component (force-directed graph rendering)
2. Canvas/SVG rendering (nodes as circles, edges as arrows)
3. Interactive controls (pan, zoom, node selection)
4. State coloring (Pending=gray, Running=blue, Completed=green, Failed=red)

### Week 5: Real-Time Status Updates
**Beads to create**:
1. WebSocket client in Leptos (connect to axum)
2. Event subscription and deserialization (bincode)
3. Reactive UI updates (Leptos signals for state changes)
4. Phase progress display (current phase out of 15 TDD15 phases)

### Week 6: Step Functions-Like Features
**Beads to create**:
1. Execution history timeline (vertical list with Leptos `For`)
2. Bead detail panel (status, logs, checkpoints, events)
3. Manual actions (cancel button, retry button, view logs)
4. Error visualization (conditional rendering with Leptos `Show`)

## REST API Specification

### Endpoints
```
POST /api/workflows         - Create new workflow/bead
GET  /api/beads/:id         - Query bead status
POST /api/beads/:id/cancel  - Cancel bead execution
POST /api/beads/:id/retry   - Retry failed bead
GET  /api/beads/:id/events  - Get event log for bead
GET  /api/health            - System health check
GET  /api/workflows         - List all workflows
WS   /api/ws                - WebSocket for real-time updates
```

### WebSocket Protocol
- **Message Format**: bincode binary frames
- **Event Types**: BeadStateChanged, PhaseCompleted, BeadFailed, WorkflowCompleted
- **Subscription**: Client subscribes to specific workflow or all events
- **Heartbeat**: Ping/pong every 30s to detect disconnection

## Success Criteria

- ✅ crates/oya-web/ with axum + tower backend
- ✅ crates/oya-ui/ with Tauri + Leptos frontend
- ✅ REST API (5+ endpoints)
- ✅ WebSocket server with bincode binary frames
- ✅ DAG visualization (force-directed graph in Leptos)
- ✅ Real-time status updates (<50ms latency)
- ✅ Step Functions-like execution history
- ✅ Manual bead control (cancel, retry)
- ✅ Type-safe frontend (Leptos + Rust WASM)
- ✅ Native desktop app (Tauri)

## Dependencies

**Rust Crates**:
- `tauri = "2.0"` - Desktop app framework
- `leptos = { version = "0.7", features = ["csr"] }` - Reactive UI
- `axum = "0.7"` - Web framework
- `tower = "0.5"` - Middleware
- `tower-http = "0.6"` - HTTP middleware (CORS, compression)
- `tokio-tungstenite = "0.24"` - WebSocket for axum
- `gloo-net = "0.6"` - WebSocket client for Leptos/WASM

## Critical Files (To Be Created)

```
crates/oya-web/src/lib.rs                   - axum server
crates/oya-web/src/routes.rs                - REST API
crates/oya-web/src/websocket.rs             - WebSocket server
crates/oya-ui/src-tauri/main.rs             - Tauri wrapper
crates/oya-ui/src/app.rs                    - Leptos app
crates/oya-ui/src/components/dag_viz.rs     - DAG visualization
crates/oya-ui/src/websocket.rs              - WebSocket client
```

## Next Steps

1. **Continue planning**: Create 8-10 atomic beads for Stream E
2. **Task breakdown**: REST API, WebSocket, Tauri setup, Leptos components, DAG viz
3. **CUE schemas**: Generate validation schemas for each bead
4. **Implementation order**: Backend first (axum), then frontend (Tauri + Leptos)

## Notes

- **Defer complexity**: Start with simple list view before complex DAG visualization
- **Tauri is mandatory**: Desktop app is part of MVP (not optional)
- **Type safety**: Leptos provides compile-time guarantees for UI
- **Performance**: Target <50ms latency for WebSocket updates
- **Integration**: UI is last stream to implement (depends on A, B, C, D)
