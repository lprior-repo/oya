# oya-web

**DEPRECATED**: HTTP API server for OYA system.

> **Note**: This crate is no longer needed. Oya now uses **Zellij terminal UI** with **CLI IPC** instead of a web frontend. The web API approach has been abandoned in favor of terminal-native UI.

## Migration Path

If you need to query the orchestrator from external tools, use one of these instead:

1. **CLI commands** - `oya status`, `oya list`, `oya show <id>`
2. **Direct IPC** - Zellij plugin communicates via mpsc channels
3. **File-based API** - Read from `.beads/beads.db` (SurrealDB)

## Legacy Features

Previously provided:
- REST API with axum/tower middleware
- CORS for web frontend
- WebSocket for real-time updates
- Health check endpoints

## Status

🚧 **ABANDONED** - Use `oya-ui` (Zellij plugin) instead.

## Usage

```rust
use oya_web::{ServerConfig, create_router};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig {
        bind_address: "127.0.0.1:3000".to_string(),
        cors_origin: "tauri://localhost".to_string(),
    };

    let app = create_router(config)?;

    // Bind and serve
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

## Testing

All tests follow TDD principles with zero unwraps/panics:

```bash
# Run all tests
cargo test -p oya-web

# Run specific test
cargo test -p oya-web test_health_check_endpoint
```

### Test Coverage

- ✅ CORS preflight handling
- ✅ Request tracing
- ✅ Response compression
- ✅ Health check endpoint
- ✅ 404 handling
- ✅ Error cases (invalid origin, etc.)

## Middleware Configuration

### CORS Layer

Configured to allow:
- Origin: Configurable (default: `tauri://localhost`)
- Methods: GET, POST
- Headers: All headers

### Trace Layer

- Logs all HTTP requests/responses
- Includes: method, URI, status, latency

### Compression Layer

- Automatic gzip compression
- Applied when client sends `Accept-Encoding: gzip`

## Error Handling

All errors use `Result<T, Error>` with proper propagation:

```rust
pub enum Error {
    InvalidHeader(#[from] ::axum::http::header::InvalidHeaderValue),
    Hyper(#[from] hyper::Error),
    Io(#[from] std::io::Error),
}
```

## Quality Standards

- **Zero unwraps**: All fallible operations use `?` or proper error handling
- **Zero panics**: No `panic!`, `todo!`, or `unimplemented!`
- **Functional patterns**: Uses `map`, `and_then`, and `?` throughout
- **Test coverage**: 10 tests covering all paths
