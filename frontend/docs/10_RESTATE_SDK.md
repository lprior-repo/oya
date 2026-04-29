# Restate SDK v0.8.0 Complete API Reference

> **Purpose**: Comprehensive reference of all Restate SDK calls for building UI abstractions and code generators.

## Table of Contents

1. [Overview](#overview)
2. [Service Types](#service-types)
3. [Context Types](#context-types)
4. [Core Context Traits](#core-context-traits)
   - [ContextClient (Service Communication)](#contextclient-service-communication)
   - [ContextTimers (Scheduling)](#contexttimers-scheduling--timers)
   - [ContextSideEffects (Journaling)](#contextsideeffects-journaling-results)
   - [ContextReadState (Reading State)](#contextreadstate-reading-state)
   - [ContextWriteState (Writing State)](#contextwritestate-writing-state)
   - [ContextPromises (Durable Promises)](#contextpromises-durable-promises)
   - [ContextAwakeables (Callbacks)](#contextawakeables-callback-pattern)
5. [Request Building](#request-building)
6. [Error Handling](#error-handling)
7. [Method Availability Matrix](#method-availability-matrix)
8. [Common Patterns](#common-patterns)

---

## Overview

Restate is a system for building resilient applications with durable execution. The Rust SDK (v0.8.0) provides type-safe handlers that can be part of three service types, each with different capabilities for state management, concurrency, and communication patterns.

**Key Concepts:**
- **Durable Execution**: Code execution is tracked in a journal; failures trigger replay from the last checkpoint
- **Stateful Handlers**: Virtual Objects and Workflows have built-in key-value state
- **Exactly-Once Semantics**: All operations are idempotent by default through journaling

---

## Service Types

| Type | Description | State | Concurrency |
|------|-------------|-------|-------------|
| **Service** | Stateless RPC handlers | None | Unlimited parallel invocations |
| **Virtual Object** | Stateful key-addressable entities | Key-value store | Serial per key |
| **Workflow** | Long-running processes with lifecycle | Per-execution | Serial per key |

### Defining Services

```rust
use restate_sdk::prelude::*;

// Basic Service
#[restate_sdk::service]
trait MyService {
    async fn my_handler(greeting: String) -> Result<String, HandlerError>;
}

// Virtual Object
#[restate_sdk::object]
trait MyVirtualObject {
    async fn my_handler(name: String) -> Result<String, HandlerError>;
}

// Workflow
#[restate_sdk::workflow]
trait MyWorkflow {
    async fn run(req: String) -> Result<String, HandlerError>;
}
```

---

## Context Types

All handlers receive a context object that provides access to Restate's capabilities. The context type determines which operations are available.

```
Context (base - all handlers)
├── Context (Service handlers)
│   └── ContextClient, ContextTimers, ContextSideEffects, ContextAwakeables
├── ObjectContext (Exclusive object handlers)
│   └── ContextClient, ContextTimers, ContextSideEffects, ContextAwakeables
│   └── ContextReadState, ContextWriteState
├── SharedObjectContext (Concurrent object handlers)
│   └── ContextClient, ContextTimers, ContextSideEffects, ContextAwakeables
│   └── ContextReadState (read-only)
├── WorkflowContext (Main workflow handler)
│   └── ContextClient, ContextTimers, ContextSideEffects, ContextAwakeables
│   └── ContextPromises
│   └── ContextReadState, ContextWriteState
└── SharedWorkflowContext (Workflow interaction handlers)
    └── ContextClient, ContextTimers, ContextSideEffects, ContextAwakeables
    └── ContextPromises
    └── ContextReadState (read-only)
```

### Base Context Methods

Available on **all** context types:

```rust
// Get the unique invocation ID for this execution
ctx.invocation_id() -> &str

// Access request headers (immutable)
ctx.headers() -> &HeaderMap

// Access request headers (mutable)
ctx.headers_mut() -> &HeaderMap
```

### Object-Specific Methods

```rust
// ObjectContext, SharedObjectContext
ctx.key() -> &str  // Get the object key

// WorkflowContext, SharedWorkflowContext  
ctx.key() -> &str  // Get the workflow key
```

---

## Core Context Traits

### ContextClient (Service Communication)

Service communication supports three invocation patterns:

| Pattern | Method | Description | Use Case |
|---------|--------|-------------|----------|
| Request-Response | `.call()` | Wait for response | Synchronous API calls |
| One-Way | `.send()` | Fire and forget | Async notifications |
| Delayed | `.send_after()` | Schedule for later | Timed operations |

#### Client Creation

```rust
// For Services (stateless)
ctx.service_client::<ServiceClient>() -> ServiceClient

// For Virtual Objects (stateful, key-addressed)
ctx.object_client::<ObjectClient>(key: impl Into<String>) -> ObjectClient

// For Workflows (long-running)
ctx.workflow_client::<WorkflowClient>(key: impl Into<String>) -> WorkflowClient

// Low-level request builder
ctx.request(request_target: RequestTarget, req: Req) -> Request<'ctx, Req, Res>

// Attach to existing invocation
ctx.invocation_handle(invocation_id: String) -> impl InvocationHandle + 'ctx
```

#### Call Semantics by Target

| Target | `.call()` | `.send()` | `.send_after()` |
|--------|-----------|-----------|----------------|
| Service | ✅ Request-response | ✅ One-way | ✅ Delayed |
| Virtual Object | ✅ Request-response | ✅ One-way | ✅ Delayed |
| Workflow (run) | ✅ Request-response | ✅ One-way | ✅ Delayed |
| Workflow (handler) | ✅ Request-response | ✅ One-way | ✅ Delayed |

#### Examples

```rust
// Request-Response (bidirectional)
let response = ctx.service_client::<OtherServiceClient>()
    .handler(request)
    .call()
    .await?;

// One-Way (fire-and-forget)
ctx.object_client::<MyObjectClient>("my-key")
    .process(payload)
    .send();

// Delayed (scheduled)
ctx.workflow_client::<OrderWorkflowClient>("order-123")
    .send_reminder()
    .send_after(Duration::from_hours(24));
```

---

### ContextTimers (Scheduling & Timers)

```rust
// Durable sleep - handler suspends during sleep (cost-saving on FaaS)
// The invocation is paused and automatically resumed after the duration
ctx.sleep(duration: Duration) -> impl DurableFuture<Output = Result<(), TerminalError>>

// Example: Wait 10 seconds
ctx.sleep(Duration::from_secs(10)).await?;
```

**Note**: Virtual Objects are blocked during sleep. Use delayed calls instead if other invocations need to proceed.

---

### ContextSideEffects (Journaling Results)

Restate journals non-deterministic operations to ensure deterministic replay. Use these methods for operations that produce different results on each execution.

#### Journaled Actions

```rust
// Execute a non-deterministic block and store result in the journal
// On replay, the stored result is returned instead of re-executing
ctx.run(|| async {
    // Cannot use ctx inside here
    external_api_call().await
}) -> impl RunFuture<Result<T, TerminalError>>

// With custom retry policy for the wrapped operation
ctx.run(|| operation())
    .retry_policy(RunRetryPolicy::default()
        .initial_delay(Duration::from_millis(100))
        .exponentiation_factor(2.0)
        .max_delay(Duration::from_millis(1000))
        .max_attempts(10)
        .max_duration(Duration::from_secs(10)))
    .await?;
```

#### Deterministic Random

```rust
// Random seed stable across retries (derived from invocation ID)
ctx.random_seed() -> u64

// Random number generator (seeded with invocation ID)
// Returns same sequence on every retry
ctx.rand() -> &mut StdRng
let value: u32 = ctx.rand().random();

// UUID generation (stable across retries)
ctx.rand_uuid() -> Uuid
```

---

### ContextReadState (Reading State)

**Available on**: ObjectContext, WorkflowContext, SharedObjectContext (read-only), SharedWorkflowContext (read-only)

```rust
// Get a state value by key
// Returns None if key doesn't exist
ctx.get<T: Deserialize>(key: &str) -> impl Future<Output = Result<Option<T>, TerminalError>>

// Get all state keys for this object/workflow
ctx.get_keys() -> impl Future<Output = Result<Vec<String>, TerminalError>>

// Example
let count: i32 = ctx.get::<i32>("counter")
    .await?
    .unwrap_or(0);
```

---

### ContextWriteState (Writing State)

**Available on**: ObjectContext, WorkflowContext (not SharedObjectContext)

```rust
// Set a state value (overwrites existing)
ctx.set<T: Serialize>(key: &str, value: T)

// Delete a single state key
ctx.clear(key: &str)

// Delete all state for this object/workflow
ctx.clear_all()

// Example: Increment counter atomically
let current = ctx.get::<i32>("counter").await?.unwrap_or(0);
ctx.set("counter", current + 1);
```

---

### ContextPromises (Durable Promises)

**Available on**: WorkflowContext, SharedWorkflowContext

Promises provide a durable mechanism for workflow-to-workflow or handler-to-workflow communication.

```rust
// Create a promise - durable distributed oneshot channel
// The workflow can wait for this promise to be resolved
ctx.promise<T: Deserialize>(key: &str) -> impl DurableFuture<Output = Result<T, TerminalError>>

// Check promise status without waiting
ctx.peek_promise<T: Deserialize>(key: &str) -> impl Future<Output = Result<Option<T>, TerminalError>>

// Resolve the promise from any handler or external code
ctx.resolve_promise<T: Serialize>(key: &str, value: T)

// Reject the promise with an error
ctx.reject_promise(key: &str, failure: TerminalError)
```

---

### ContextAwakeables (Callback Pattern)

**Available on**: All context types

Awakeables enable pausing an invocation while waiting for an external process to complete a task (callback pattern).

#### Creating Awakeables

```rust
// Create an awakeable - returns (identifier, promise)
// The identifier is used by external code to complete the awakeable
ctx.awakeable<T: Deserialize>() -> (
    String,  // ID to pass to external system
    impl DurableFuture<Output = Result<T, TerminalError>> + Send  // Promise to wait on
)

// Full example
let (id, callback) = ctx.awakeable::<String>();

// Trigger external task and include the ID
ctx.run(|| trigger_external_workflow(id.clone(), request)).await?;

// Wait for external system to call back
let result = callback.await?;
```

#### Completing Awakeables

From another handler:
```rust
// Resolve with a value
ctx.resolve_awakeable(&id, "result".to_string());

// Reject with an error
ctx.reject_awakeable(&id, TerminalError::new("something went wrong"));
```

Via HTTP:
```bash
# Resolve
curl -X POST localhost:909/restate/awakeables/{id}/resolve \
  -H 'content-type: application/json' \
  -d '{"result": "data"}'

# Reject
curl -X POST localhost:909/restate/awakeables/{id}/reject \
  -H 'content-type: text/plain' \
  -d 'error message'
```

---

## Request Building

### Low-Level Request Builder

For fine-grained control over requests:

```rust
// Create a request with custom target
ctx.request(request_target: RequestTarget, req: Req) -> Request<'ctx, Req, Res>

// Configure the request
request
    .header("x-custom-header", "value")     // Add custom header
    .idempotency_key("unique-key")          // Ensure exactly-once semantics
    .call()                                  // Execute and wait
    .send()                                  // Execute without waiting
    .send_after(Duration::from_secs(60))     // Schedule for later
```

### RequestTarget Enum

```rust
// Target a service handler
RequestTarget::Service { 
    name: String,      // Service name 
    handler: String    // Handler name
}

// Target a virtual object handler
RequestTarget::Object { 
    name: String,      // Object name
    key: String,       // Object key
    handler: String    // Handler name
}

// Target a workflow handler
RequestTarget::Workflow { 
    name: String,      // Workflow name  
    key: String,       // Workflow key
    handler: String    // Handler name
}
```

---

## Error Handling

### TerminalError

Return a `TerminalError` to stop retries and fail the invocation:

```rust
// Simple terminal error
TerminalError::new("descriptive error message")

// Terminal error with error code
TerminalError::from_code("PAYMENT_FAILED", "Insufficient funds")

// Inspect error
error.is_terminal() -> bool
error.code() -> Option<&str>
error.message() -> &str
```

All context operations that can fail return `Result<T, TerminalError>`.

---

## Method Availability Matrix

| Method | Service | Object | Shared Object | Workflow | Shared Workflow |
|--------|---------|--------|---------------|----------|-----------------|
| `invocation_id()` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `headers()` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `key()` | ❌ | ✅ | ✅ | ✅ | ✅ |
| `service_client()` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `object_client()` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `workflow_client()` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `request()` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `invocation_handle()` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `sleep()` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `run()` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `random_seed()` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `rand()` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `rand_uuid()` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `get()` | ❌ | ✅ | ✅ | ✅ | ✅ |
| `get_keys()` | ❌ | ✅ | ✅ | ✅ | ✅ |
| `set()` | ❌ | ✅ | ❌ | ✅ | ❌ |
| `clear()` | ❌ | ✅ | ❌ | ✅ | ❌ |
| `clear_all()` | ❌ | ✅ | ❌ | ✅ | ❌ |
| `promise()` | ❌ | ❌ | ❌ | ✅ | ✅ |
| `peek_promise()` | ❌ | ❌ | ❌ | ✅ | ✅ |
| `resolve_promise()` | ❌ | ❌ | ❌ | ✅ | ✅ |
| `reject_promise()` | ❌ | ❌ | ❌ | ✅ | ✅ |
| `awakeable()` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `resolve_awakeable()` | ✅ | ✅ | ✅ | ✅ | ✅ |
| `reject_awakeable()` | ✅ | ✅ | ✅ | ✅ | ✅ |

---

## Common Patterns

### Service-to-Service Communication

```rust
// Bidirectional call with response
let result = ctx.service_client::<PaymentServiceClient>()
    .process(payment_request)
    .call()
    .await?;

// Fire-and-forget notification
ctx.service_client::<NotificationServiceClient>()
    .send_email(notification)
    .send();

// Schedule background task
ctx.service_client::<BackgroundJobClient>()
    .enqueue(job)
    .send_after(Duration::from_minutes(5));
```

### Virtual Object State Management

```rust
// State operations are atomic per object
async fn increment_counter(ctx: ObjectContext<'_>, delta: i32) -> Result<i32, HandlerError> {
    let current = ctx.get::<i32>("counter").await?.unwrap_or(0);
    let new_value = current + delta;
    ctx.set("counter", new_value)?;
    Ok(new_value)
}
```

### Workflow with Promises

```rust
// In workflow run() handler - wait for external signal
async fn process_order(ctx: WorkflowContext<'_>, order: Order) -> Result<OrderResult, HandlerError> {
    // Create promise for async processing
    let (promise_key, promise) = ctx.promise::<ProcessingResult>("processing").await?;
    
    // Trigger external processor with promise key
    ctx.service_client::<ProcessorClient>()
        .process(order.clone())
        .send();
    
    // Wait for result
    let result = promise.await?;
    
    ctx.set("result", &result)?;
    Ok(result)
}

// In workflow shared handler - resolve promise
async fn handle_processor_callback(ctx: SharedWorkflowContext<'_>, result: ProcessingResult) {
    ctx.resolve_promise("processing", result).ok();
}
```

### External Callback Pattern

```rust
// Pause workflow, wait for external webhook
async fn wait_for_approval(ctx: WorkflowContext<'_>, request_id: String) -> Result<Approval, HandlerError> {
    let (callback_id, callback) = ctx.awakeable::<Approval>();
    
    // Store callback ID for webhook handler
    ctx.set("pending_approval", &callback_id)?;
    
    // Send to external system
    ctx.run(|| send_approval_request(callback_id, request_id)).await?;
    
    // Wait for webhook to call back
    callback.await?
}
```

---

## Additional Resources

| Resource | URL |
|----------|-----|
| SDK Documentation | https://docs.rs/restate-sdk/latest/restate_sdk/ |
| Official Guides | https://docs.restate.dev/ |
| GitHub Repository | https://github.com/restatedev/sdk-rust |
| Rust SDK Release | https://crates.io/crates/restate-sdk |
