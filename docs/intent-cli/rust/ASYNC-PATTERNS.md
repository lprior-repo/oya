# Async Patterns for ZJJ Contributors

## Overview

ZJJ migrated from synchronous `rusqlite` to asynchronous `sqlx` in version 0.3.0. All database operations and command handlers now use async/await patterns with Tokio runtime.

## Core Concepts

### Async Runtime

All async code runs on Tokio runtime, initialized in `main.rs`:

```rust
#[tokio::main]
async fn main() {
    if let Err(err) = run_cli().await {
        eprintln!("Error: {}", format_error(&err));
        process::exit(1);
    }
}
```

### Database Connection Pooling

Instead of opening new connections, we use a connection pool:

```rust
// OLD (synchronous rusqlite)
let conn = Connection::open(path)?;
let sessions = conn.query_map("SELECT * FROM sessions", |row| ...)?;

// NEW (asynchronous sqlx with pooling)
let pool = SqlitePool::connect(&db_url).await?;
let sessions = sqlx::query_as!("SELECT * FROM sessions")
    .fetch_all(&pool)
    .await?;
```

### Async Command Handlers

All command handlers are now async:

```rust
// crates/zjj/src/commands/status/mod.rs
pub async fn run(args: Args, ctx: &CommandContext) -> Result<()> {
    let db = get_session_db().await?;
    let session = db.get(&args.name).await?;
    ctx.output_json(&session)
}
```

## Common Patterns

### Pattern 1: Database Query with Error Handling

```rust
pub async fn get_session(db: &SqlitePool, name: &str) -> Result<Option<Session>> {
    sqlx::query_as!(
        "SELECT id, name, status, workspace_path, branch, created_at FROM sessions WHERE name = ?",
        name
    )
    .fetch_optional(db)
    .await
    .map_err(|e| Error::database(format!("Failed to fetch session: {}", e)))
}
```

### Pattern 2: Transaction with Multiple Operations

```rust
pub async fn update_session_status(db: &SqlitePool, name: &str, status: Status) -> Result<()> {
    let mut tx = db.begin().await?;

    sqlx::query!("UPDATE sessions SET status = ? WHERE name = ?", status, name)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}
```

### Pattern 3: Async File Operations

```rust
use tokio::fs;

pub async fn read_workspace_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path).await?;
    toml::from_str(&content)
        .map_err(|e| Error::validation(format!("Invalid config: {}", e)))
}
```

### Pattern 4: Spawning Background Tasks

```rust
tokio::spawn(async move {
    // Background task runs independently
    while let Ok(msg) = rx.recv().await {
        process_message(msg).await?;
    }
    Ok::<(), Error>(())
});
```

### Pattern 5: Blocking Calls in Async Context

When calling blocking code from async context:

```rust
let result = tokio::task::spawn_blocking(move || {
    // Blocking operation
    heavy_computation(data)
})
.await??;
```

## Error Handling

### Railway-Oriented Programming

Use `Result<T>` and propagate errors with `?`:

```rust
pub async fn create_session(db: &SqlitePool, name: &str) -> Result<Session> {
    // Each .await is a "track" in railway
    let workspace_path = resolve_workspace_path(name).await?;

    validate_name(name).map_err(|e| Error::validation(e))?;

    let session = db_insert_session(db, name, &workspace_path).await?;

    Ok(session)  // Final station on railway
}
```

### Custom Error Types

```rust
pub enum Error {
    Validation(String),
    Database(String),
    Io(String),
    Config(String),
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Error::Database(format!("Database error: {}", e))
    }
}
```

## Testing

### Async Tests

```rust
#[tokio::test]
async fn test_create_session() {
    let pool = create_test_pool().await;

    let result = create_session(&pool, "test-session").await;

    assert!(result.is_ok());
    let session = result.unwrap();
    assert_eq!(session.name, "test-session");
}
```

### Mocking Database

```rust
fn mock_db() -> SqlitePool {
    // Create in-memory database for tests
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    run_migrations(&pool).await.unwrap();
    pool
}
```

## Migration Guide for Contributors

If you're working with pre-0.3.0 synchronous code:

1. **Change function signatures**:
   - `fn foo() -> Result<()>` → `async fn foo() -> Result<()>`
   - Add `#[tokio::test]` to test functions

2. **Replace rusqlite with sqlx**:
   - `Connection::open()` → `SqlitePool::connect().await`
   - `conn.query_map()` → `sqlx::query_as!().fetch_all().await`
   - `conn.execute()` → `sqlx::query!().execute().await`

3. **Add `.await` to all async operations**:
   - Database calls
   - File I/O (`tokio::fs`)
   - Tokio spawned tasks

4. **Use connection pool**:
   - Pass `&SqlitePool` instead of `&Connection`
   - Pool handles connection lifecycle

5. **Run tests**:
   - `moon run :test` - All async tests
   - `moon run :quick` - Format and type check

## Common Pitfalls

### Pitfall 1: Blocking Event Loop

```rust
// BAD: Blocks entire async runtime
let files = std::fs::read_dir(".").unwrap();

// GOOD: Offloads to blocking thread pool
let files = tokio::fs::read_dir(".").await?;
```

### Pitfall 2: Forgetting `.await`

```rust
// BAD: Function returns Future, never executes
let sessions = db.list();

// GOOD: Actually awaits result
let sessions = db.list().await?;
```

### Pitfall 3: Holding Across Await Points

```rust
// BAD: Lock held across await, causes deadlock
let mutex = Mutex::new(vec);
let guard = mutex.lock().unwrap();
let data = fetch_remote(guard).await?;

// GOOD: Drop guard before await
{
    let guard = mutex.lock().unwrap();
    let local_data = guard.clone();
}
let data = fetch_remote(local_data).await?;
```

## Further Reading

- [Tokio Documentation](https://tokio.rs/)
- [SQLx Documentation](https://docs.rs/sqlx/)
- [Railway-Oriented Programming](https://fsharpforfunandprofit.com/rop/)
- [Async Rust Book](https://rust-lang.github.io/async-book/)
