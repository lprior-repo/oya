# Restate Documentation Compass

Navigation guide for Restate documentation. Use this to find relevant documentation quickly.

## I'm trying to...

### Understand Restate basics
- [Key Concepts](docs/foundations/key-concepts.md) - Core building blocks
- [Services](docs/foundations/services.md) - Service types (Objects, Workflows, Services)
- [Handlers](docs/foundations/handlers.md) - Handler types and invocation
- [Invocations](docs/foundations/invocations.md) - How to invoke services
- [Actions](docs/foundations/actions.md) - Essential context actions

### Develop with Rust (OYA's SDK)
- [Rust SDK Index](docs/develop/rust/index.md) - Primary reference with examples
- [Rust SDK Source](docs/develop/rust/src/) - Complete v0.8.0 source code
- [Rust SDK Examples](docs/develop/rust/examples/) - Example implementations
- [lib.rs](docs/develop/rust/src/lib.rs) - Main entry point, macros, traits
- [context/mod.rs](docs/develop/rust/src/context/mod.rs) - Execution contexts
- [errors.rs](docs/develop/rust/src/errors.rs) - HandlerError and error types

### Configure and deploy services
- [Service Configuration](docs/services/configuration.md) - Retries, timeouts, retention
- [Versioning](docs/services/versioning.md) - Deployment and versioning
- [Kubernetes](docs/services/deploy/kubernetes.md) - K8s deployment
- [Standalone](docs/services/deploy/standalone.md) - Standalone services

### Operate Restate server
- [Server Overview](docs/server/overview.md) - Self-hosted Restate
- [Configuration](docs/server/configuration.md) - Server config options
- [Networking](docs/server/networking.md) - Ports and listeners
- [Monitoring](docs/server/monitoring/metrics.md) - Prometheus metrics
- [Clustering](docs/server/clusters.md) - Multi-node deployment

### Use Admin API
- [Health Check](docs/admin-api/health/health-check-endpoint.md) - `/health` endpoint
- [List Services](docs/admin-api/service/list-services.md) - Service discovery
- [List Deployments](docs/admin-api/deployment/list-deployments.md) - Deployment status
- [Invocation Management](docs/admin-api/invocation/cancel-an-invocation.md) - Cancel/kill/pause

### Build AI agents
- [AI Overview](docs/ai/index.md) - Durable AI patterns
- [Vercel AI SDK](docs/ai/sdk-integrations/vercel-ai-sdk.md) - Integration guide
- [OpenAI SDK](docs/ai/sdk-integrations/openai-agents-sdk.md) - Integration guide
- [Chat UI Integration](docs/ai/patterns/chat-ui-integration.md) - Frontend patterns
- [Human-in-the-loop](docs/ai/patterns/human-in-the-loop.md) - Approval workflows

### Handle errors and failures
- [Error Handling Guide](docs/guides/error-handling.md) - Comprehensive guide
- [Error Codes](docs/references/errors.md) - Error code reference
- [Sagas](docs/guides/sagas.md) - Compensating transactions

### Monitor and debug
- [Introspection](docs/services/introspection.md) - Service/invocation inspection
- [SQL Introspection](docs/references/sql-introspection.md) - SQL queries
- [Logging](docs/server/monitoring/logging.md) - Log configuration
- [Tracing](docs/server/monitoring/tracing.md) - OTEL traces

## Category Overview

```
docs/
├── admin-api/          # Admin API reference (20+ docs)
│   ├── cluster_health/
│   ├── deployment/
│   ├── health/
│   ├── invocation/
│   ├── service/
│   ├── service_handler/
│   └── subscription/
├── ai/                 # AI agent patterns (15+ docs)
│   ├── patterns/
│   └── sdk-integrations/
├── cloud/              # Restate Cloud docs
├── develop/            # SDK docs (35+ docs)
│   ├── java/
│   ├── python/
│   ├── rust/           # <-- OYA's SDK (complete source + examples)
│   └── ts/
├── foundations/        # Core concepts (5 docs)
├── guides/             # How-to guides (15+ docs)
├── references/         # API references (10 docs)
├── server/             # Server ops (15+ docs)
│   ├── deploy/
│   └── monitoring/
├── services/           # Service lifecycle (15+ docs)
│   ├── deploy/
│   └── invocation/
├── tour/               # Learning tours (5 docs)
└── use-cases/          # Use case guides (4 docs)
```

## OYA Integration Points

OYA's `Oya` uses these Restate features:

| OYA Feature | Restate Concept | Documentation |
|-------------|-----------------|---------------|
| `#[restate_sdk::object]` | Virtual Object | [Services](docs/foundations/services.md) |
| `ObjectContext<'_>` | Object context | [Handlers](docs/foundations/handlers.md) |
| `ctx.sleep()` | Durable timer | [src/context/mod.rs](docs/develop/rust/src/context/mod.rs) |
| `HandlerError` | Error handling | [src/errors.rs](docs/develop/rust/src/errors.rs) |
| `HttpServer::listen_and_serve()` | Service serving | [src/http_server.rs](docs/develop/rust/src/http_server.rs) |
| `OYA_BIND_ADDR=127.0.0.1:9080` | Networking | [Networking](docs/server/networking.md) |
| Health check at `:9070/health` | Admin health | [Health Check](docs/admin-api/health/health-check-endpoint.md) |

## Quick Commands

```bash
# View full documentation
cat llms-full.txt | less

# Search for specific topic
grep -r "ObjectContext" docs/

# Search Rust SDK source
grep -r "ObjectContext" docs/develop/rust/src/

# View Rust SDK structure
ls -la docs/develop/rust/

# View category counts
cat INDEX.json | jq '.categories | to_entries | map({key, count: (.value | length)})'
```
