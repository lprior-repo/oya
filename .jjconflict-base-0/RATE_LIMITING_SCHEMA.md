# Rate Limiting Schema Documentation

## Overview

This implementation provides SurrealDB schema definitions and Rust type mappings for rate limiting and concurrency control, with atomic counter operations.

## Files Created

### 1. `/schema.surql` - SurrealDB Schema Definition

Complete schema definition with two tables:

#### Token Bucket Table
- **Purpose**: Rate limiting using token bucket algorithm
- **Fields**:
  - `resource_id` (string, unique): Resource identifier
  - `capacity` (int): Maximum tokens the bucket can hold
  - `current_tokens` (int): Current number of available tokens
  - `refill_rate` (float): Tokens added per second
  - `last_refill_at` (datetime): Last refill timestamp
  - `created_at`, `updated_at` (datetime): Audit timestamps

#### Concurrency Limit Table
- **Purpose**: Resource management via slot acquisition/release
- **Fields**:
  - `resource_id` (string, unique): Resource identifier
  - `max_concurrent` (int): Maximum concurrent operations allowed
  - `current_count` (int): Current number of active operations
  - `created_at`, `updated_at` (datetime): Audit timestamps

### 2. `/crates/workflow/src/schema/limits.rs` - Rust Type Mappings

Functional Rust implementation with:

#### Type-Safe Configuration
```rust
// Token bucket configuration (validated at construction)
let config = TokenBucketConfig::new(100, 10.0)?;
let bucket = TokenBucket::create("api:/v1/users".to_string(), config)?;

// Concurrency limit configuration
let config = ConcurrencyLimitConfig::new(50)?;
let limit = ConcurrencyLimit::create("db:pool".to_string(), config)?;
```

#### Atomic Operations via Query Builders
```rust
// Acquire tokens atomically
let (query, params) = build_acquire_tokens_query("bucket_id", 10, 5);

// Acquire concurrency slot atomically
let query = build_acquire_slot_query("limit_id");

// Release concurrency slot atomically
let query = build_release_slot_query("limit_id");
```

#### Pure Functions
All core logic is implemented as pure functions:
- `calculate_refill()` - Calculate tokens to add based on elapsed time
- `can_acquire()` - Check if operation would succeed (no side effects)
- `can_release()` - Check if release is valid (no side effects)

### 3. `/crates/workflow/src/schema/mod.rs` - Module Definition

Re-exports commonly used types for clean public API.

## Architecture Principles

### 1. Zero Panics, Zero Unwraps
- All operations return `Result<T, RateLimitError>`
- No `.unwrap()`, `.expect()`, or `panic!()` anywhere
- Validated at compile time with `#![deny(clippy::unwrap_used)]`

### 2. Immutability & Purity
- Configuration types are immutable after construction
- State transitions return new values
- Pure functions for all calculations
- Side effects (database updates) isolated to query execution

### 3. Railway-Oriented Programming
- Error handling via `Result` combinators
- `.map()`, `.and_then()`, `.map_err()` for chaining operations
- Semantic error types using `thiserror`

### 4. Type System Enforcement
- **Newtypes**: `ResourceId` validates non-empty strings
- **Configuration types**: Validate constraints at construction
  - `TokenBucketConfig`: capacity > 0, refill_rate > 0.0
  - `ConcurrencyLimitConfig`: max_concurrent > 0
- **Make illegal states unrepresentable**

### 5. Atomic Counter Updates
- Token acquisition checks and updates in single query
- Concurrency slot increment/decrement with WHERE clauses
- SurrealDB ensures atomicity via transaction semantics

## Success Criteria

- ✅ Token bucket with refill rate, capacity, current tokens
- ✅ Concurrency limit with max_concurrent, current_count
- ✅ Atomic increment/decrement operations via query builders
- ✅ Zero unwraps, zero panics (enforced by lints)
- ✅ Concurrency-safe counter updates (WHERE clauses + atomic updates)
- ✅ Railway-Oriented Programming patterns throughout

## Quality Standards

### Lints Enforced
```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
```

### Test Coverage
Comprehensive unit tests cover:
- Configuration validation
- Resource ID validation
- Token bucket creation and refill calculations
- Concurrency limit creation and state checks
- Query builder purity (same input → same output)

### Error Handling
All errors are semantic and enumerated:
```rust
pub enum RateLimitError {
    InvalidCapacity(i64),
    InvalidRefillRate(f64),
    InvalidTokenCount { count: i64, capacity: i64 },
    EmptyResourceId,
    InsufficientTokens { requested: i64, available: i64 },
    ConcurrencyLimitReached { current: i64, max: i64 },
    InvalidConcurrentCount { count: i64, max: i64 },
    InvalidMaxConcurrent(i64),
}
```

## Usage Examples

### Token Bucket (Rate Limiting)

```rust
use oya_workflow::schema::{TokenBucket, TokenBucketConfig};

// Create configuration
let config = TokenBucketConfig::new(100, 10.0)?;

// Create bucket
let bucket = TokenBucket::create("api:/v1/users".to_string(), config)?;

// Check if can acquire tokens (pure function)
let now = Utc::now();
if bucket.can_acquire(10, now) {
    // Execute SurrealDB query to atomically acquire
    let refill = bucket.calculate_refill(now);
    let (query, params) = build_acquire_tokens_query("bucket_id", 10, refill);
    // db.query(query).bind(params).await?;
}
```

### Concurrency Limit (Resource Management)

```rust
use oya_workflow::schema::{ConcurrencyLimit, ConcurrencyLimitConfig};

// Create configuration
let config = ConcurrencyLimitConfig::new(50)?;

// Create limit
let limit = ConcurrencyLimit::create("db:pool".to_string(), config)?;

// Check if can acquire slot (pure function)
if limit.can_acquire() {
    // Execute SurrealDB query to atomically acquire
    let query = build_acquire_slot_query("limit_id");
    // db.query(query).await?;

    // ... do work ...

    // Release slot
    let query = build_release_slot_query("limit_id");
    // db.query(query).await?;
}
```

### SurrealDB Schema Initialization

```bash
# Load schema into SurrealDB
surreal sql --endpoint http://localhost:8000 \
  --namespace oya --database workflow \
  --file schema.surql
```

Or programmatically:
```rust
use surrealdb::{Surreal, engine::local::RocksDb};

let db = Surreal::new::<RocksDb>("/path/to/db").await?;
db.use_ns("oya").use_db("workflow").await?;

// Load schema
let schema = std::fs::read_to_string("schema.surql")?;
db.query(schema).await?;
```

## Integration with Workflow Engine

The rate limiting schema integrates with the workflow engine for:

1. **Phase Execution Throttling**: Limit phase execution rate per resource
2. **Concurrent Workflow Limits**: Control how many workflows run concurrently
3. **Resource Pool Management**: Manage shared resources (DB connections, workers)

## Future Enhancements

Potential improvements tracked as separate beads:

- [ ] Distributed rate limiting (multiple instances)
- [ ] Dynamic refill rate adjustment
- [ ] Hierarchical rate limits (per-user → per-org → global)
- [ ] Rate limit metrics and observability
- [ ] Automatic cleanup of expired buckets

## References

- **Token Bucket Algorithm**: https://en.wikipedia.org/wiki/Token_bucket
- **SurrealDB Schema Docs**: https://surrealdb.com/docs/surrealql/statements/define/table
- **Railway-Oriented Programming**: https://fsharpforfunandprofit.com/rop/
- **Making Illegal States Unrepresentable**: https://www.youtube.com/watch?v=IcgmSRJHu_8
