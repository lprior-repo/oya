# Restate Documentation

AI-optimized Restate documentation for the OYA project. This documentation is sourced from [docs.restate.dev](https://docs.restate.dev) and formatted for LLM consumption.

## Files

| File | Purpose | Size |
|------|---------|------|
| `llms.txt` | Quick index of all documentation pages | ~25KB |
| `llms-full.txt` | Complete documentation in single file | ~1.6MB |
| `INDEX.json` | Machine-readable index with categories | ~72KB |
| `docs/` | Individual markdown files by category | 160+ files |

## Quick Reference for OYA

OYA uses Restate's Rust SDK (`restate-sdk = "0.8"`) for durable execution. Key patterns:

### Service Definition
```rust
#[restate_sdk::object]
pub trait Oya {
    async fn start(request: String) -> Result<String, HandlerError>;
    async fn get_status() -> Result<String, HandlerError>;
    async fn ping() -> Result<String, HandlerError>;
}
```

### Key Imports
```rust
use restate_sdk::endpoint::Endpoint;
use restate_sdk::http_server::HttpServer;
use restate_sdk::prelude::*;
```

### Durable Primitives Used
- `ObjectContext` - Virtual Object context for keyed services
- `ctx.sleep(duration)` - Durable timers
- `HandlerError` - Error handling

## Documentation Categories

| Category | Description | Docs |
|----------|-------------|------|
| `foundations/` | Core concepts, services, handlers, invocations | 5 docs |
| `develop/` | SDK documentation (Rust SDK source + TS, Python, Java) | 35+ docs |
| `admin-api/` | Admin API for cluster/deployment/service management | 20+ docs |
| `server/` | Server configuration, deployment, monitoring | 15+ docs |
| `services/` | Service lifecycle, versioning, security | 15+ docs |
| `guides/` | How-to guides for common patterns | 15+ docs |
| `ai/` | AI agent patterns and SDK integrations | 15+ docs |
| `references/` | Architecture, CLI, errors, SQL introspection | 10 docs |

## Key Documents for OYA

1. **Rust SDK**: `docs/develop/rust/` (complete v0.8.0 source + examples)
2. **Key Concepts**: `docs/foundations/key-concepts.md`
3. **Services**: `docs/foundations/services.md`
4. **Handlers**: `docs/foundations/handlers.md`
5. **Error Handling**: `docs/guides/error-handling.md`
6. **Server Config**: `docs/server/configuration.md`
7. **Admin API Health**: `docs/admin-api/health/health-check-endpoint.md`

## Usage

### For AI Agents
Read `llms.txt` first for navigation, then `llms-full.txt` for complete reference.

### For Humans
Browse `docs/` directory by category, or use `INDEX.json` for programmatic access.

### For Codanna Indexing
The `docs/` directory contains structured markdown suitable for semantic search indexing.

## Source

- Documentation: https://docs.restate.dev
- llms.txt: https://docs.restate.dev/llms.txt
- llms-full.txt: https://docs.restate.dev/llms-full.txt
- Rust SDK Crates: https://docs.rs/restate-sdk/latest/restate_sdk/
