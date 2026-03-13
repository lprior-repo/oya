# Restate Rust SDK (v0.8.0)

The Rust SDK for [Restate](https://restate.dev), the durable execution runtime.

## Source Files

This directory contains the complete source code of `restate-sdk` v0.8.0 for local reference:

```
rust/
├── README.md              # SDK overview and quickstart
├── src/
│   ├── lib.rs             # Main entry point with macros and traits
│   ├── context/           # Execution contexts (ObjectContext, WorkflowContext, etc.)
│   ├── endpoint/          # Endpoint builder and HTTP server
│   ├── errors.rs          # HandlerError and error types
│   ├── http_server.rs     # HTTP server implementation
│   ├── lambda.rs          # AWS Lambda support
│   ├── serde.rs           # Serialization utilities
│   └── ...
└── examples/              # Example implementations
```

## Key Imports for OYA

```rust
use restate_sdk::endpoint::Endpoint;
use restate_sdk::http_server::HttpServer;
use restate_sdk::prelude::*;
```

## Core Concepts

### Service Types

1. **Virtual Object** (`#[restate_sdk::object]`) - Stateful, keyed services
   - OYA uses this for `Oya`
   - One instance per key with persistent state

2. **Workflow** (`#[restate_sdk::workflow]`) - Long-running processes
   - Supports durable timers and event waiting

3. **Service** (`#[restate_sdk::service]`) - Stateless request-response
   - No persistent state between invocations

### Context Types

| Context | Service Type | Key Features |
|---------|-------------|--------------|
| `ObjectContext<'_>` | Virtual Object | `get()`, `set()`, `clear()` for state |
| `WorkflowContext<'_>` | Workflow | `sleep()`, `run()`, `promise()` |
| `Context<'_>` | Service | Basic request handling |

### Handler Types

```rust
#[restate_sdk::object]
pub trait MyService {
    // Regular handler
    async fn my_handler(input: String) -> Result<String, HandlerError>;
    
    // Exclusive handler (serialized execution per key)
    #[exclusive]
    async fn exclusive_handler(input: String) -> Result<String, HandlerError>;
}
```

## OYA Integration Example

```rust
use restate_sdk::prelude::*;

#[restate_sdk::object]
pub trait Oya {
    async fn start(request: String) -> Result<String, HandlerError>;
    async fn get_status() -> Result<String, HandlerError>;
    async fn ping() -> Result<String, HandlerError>;
}

pub struct OyaOrchestratorImpl;

impl Oya for OyaOrchestratorImpl {
    async fn start(
        &self,
        ctx: ObjectContext<'_>,
        request: String,
    ) -> Result<String, HandlerError> {
        // Get the object key
        let run_id = ctx.key();
        
        // Durable sleep (survives crashes)
        ctx.sleep(std::time::Duration::from_secs(10)).await;
        
        // Call another service
        let result = ctx.service_call(
            "OtherService",
            "handler",
            vec!["arg".into()],
        ).await?;
        
        Ok(run_id.to_string())
    }
    
    async fn get_status(&self, ctx: ObjectContext<'_>) -> Result<String, HandlerError> {
        // Read state
        let state: Option<String> = ctx.get("status").await?;
        Ok(state.unwrap_or_default())
    }
    
    async fn ping(&self, _ctx: ObjectContext<'_>) -> Result<String, HandlerError> {
        Ok(r#"{"status":"ok"}"#.to_string())
    }
}

// Start the server
#[tokio::main]
async fn main() {
    let endpoint = Endpoint::builder()
        .bind(OyaOrchestratorImpl.serve())
        .build();
    
    HttpServer::new(endpoint)
        .listen_and_serve("127.0.0.1:9080".parse().unwrap())
        .await;
}
```

## Key Files to Read

1. **`src/lib.rs`** - Main SDK entry point, macros, and public API
2. **`src/context/mod.rs`** - Context traits and implementations
3. **`src/endpoint/mod.rs`** - Endpoint builder
4. **`src/errors.rs`** - Error handling

## Official Documentation

- **API Docs**: https://docs.rs/restate-sdk/latest/restate_sdk/
- **Restate Docs**: https://docs.restate.dev
- **Examples**: https://github.com/restatedev/sdk-typescript/tree/main/examples

## Version

This is `restate-sdk` v0.8.0, the version used by OYA.

```toml
[dependencies]
restate-sdk = "0.8"
```
