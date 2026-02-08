# oya-ui - Zellij WASM Plugin for OYA SDLC

Terminal-based UI for visualizing OYA workflows, including bead status, pipeline progress, and workflow graphs.

## Architecture

```
┌─────────────────────────────────┐
│ BeadList      │ BeadDetail       │
│               ├─────────────────┤
│               │ PipelineView     │
├───────────────┴─────────────────┤
│ WorkflowGraph                   │
└─────────────────────────────────┘
```

## Building

### Prerequisites

Install the WASM target:
```bash
rustup target add wasm32-wasi
```

### Build WASM Plugin

```bash
cargo build --release --target wasm32-wasip1
```

Output: `target/wasm32-wasip1/release/oya_ui.wasm`

### Run in Zellij

```bash
# Using the layout file
zellij --layout oya.kll

# Or manually
zellij action new-tab
zellij action run-plugin "file:$(pwd)/target/wasm32-wasi/release/oya_ui.wasm"
```

## Project Structure

- `src/lib.rs` - Library entry point
- `src/plugin.rs` - Core plugin trait and event handling
- `src/layout.rs` - 3-pane layout system
- `src/render.rs` - Terminal rendering with ANSI box-drawing
- `src/components.rs` - UI components and styling

## Performance Targets

- WASM binary: <5MB
- Plugin load: <100ms
- Initial render: <50ms
- Responsive to input: <10ms

## Development

### Run Tests

```bash
cargo test
```

### Check Compilation

```bash
cargo check
```

## Next Steps

This is a scaffold implementation. Future work:

1. Integrate IPC client for communicating with oya-orchestrator
2. Implement BeadList component with real data
3. Implement BeadDetail component with pipeline visualization
4. Implement WorkflowGraph component with DAG rendering
5. Add vim-style navigation
6. Add color coding for bead status
7. Add keyboard shortcuts for common actions

## License

MIT
