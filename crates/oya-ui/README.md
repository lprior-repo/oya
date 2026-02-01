# oya-ui

Leptos 0.7 CSR frontend for OYA graph visualization.

## Overview

This crate provides a client-side rendered web UI for visualizing the OYA task dependency graph using:
- **Leptos 0.7** with CSR (Client-Side Rendering) only
- **WASM** compilation target (wasm32-unknown-unknown)
- **Canvas-based** graph rendering
- **WebSocket** communication for real-time updates

## Architecture

```
src/
├── lib.rs          # Main entry point and App component
├── models/         # Data structures (Graph, Node, Edge)
├── components/     # Leptos UI components
├── layout/         # Graph layout algorithms
└── utils/          # Canvas helpers and utilities
```

## Development Workflow

### Prerequisites

Install the WASM target:
```bash
rustup target add wasm32-unknown-unknown
```

Install trunk (for WASM development):
```bash
cargo install trunk
```

### Building

Check compilation:
```bash
cargo check --package oya-ui --target wasm32-unknown-unknown
```

Build WASM bundle:
```bash
cd crates/oya-ui
trunk build
```

### Development Server

Run with auto-reload:
```bash
cd crates/oya-ui
trunk serve --open
```

### Testing

Run tests:
```bash
cargo test --package oya-ui
```

Run WASM tests:
```bash
wasm-pack test --headless --firefox
```

## Code Quality

This crate follows strict quality standards:
- **Zero unwraps**: All error handling uses `Result<T, E>`
- **Zero panics**: No `panic!`, `todo!`, or `unimplemented!`
- **Functional patterns**: Extensive use of `map`, `and_then`, `?` operator
- **Type safety**: Strong typing with custom error types

## Module Overview

### models
Data structures for graph representation:
- `GraphNode`: Represents a node with position and metadata
- `GraphEdge`: Represents connections between nodes
- `Graph`: Complete graph structure

### components
Leptos UI components:
- `GraphCanvas`: Canvas element for rendering
- `ControlPanel`: User controls for graph interaction
- `InfoPanel`: Display selected node details

### layout
Graph layout algorithms:
- `force_directed`: Force-directed graph layout

### utils
Helper functions:
- `canvas`: Canvas manipulation utilities

## Dependencies

Core dependencies:
- `leptos = "0.7"` with `csr` feature
- `web-sys` with canvas features
- `wasm-bindgen` for JS interop

## Integration

This crate is designed to work with:
- `oya-core`: Core data structures
- `oya-web`: Backend WebSocket server
- Tauri 2.0: Desktop application wrapper (future)

## License

MIT
