# Message Passing Patterns in ractor

This directory demonstrates the three core message passing patterns in ractor:

## Overview

The actor model in ractor supports three primary communication patterns, each suited for different use cases:

1. **Call Pattern (Request-Response)** - Synchronous RPC-style communication
2. **Cast Pattern (Fire-and-Forget)** - Asynchronous command sending
3. **Send Pattern (Async Message Passing)** - Non-blocking message delivery

## File Structure

```
messaging/
├── mod.rs          # Module exports and overview
├── calculator.rs   # Demonstrates call and cast patterns
├── logger.rs       # Demonstrates send pattern
└── README.md       # This file
```

## Pattern 1: Call (Request-Response)

**Use when:** You need a response back from the actor

**Characteristics:**
- Synchronous: Caller blocks until response is received
- Type-safe: Response type is enforced at compile time
- Timeout support: Can specify maximum wait time
- Error handling: Errors are returned in the Result

### Example: Calculator Actor

```rust
use ractor::call;

// Send request and wait for response
let result = call!(
    calculator,
    CalculatorMessage::Add {
        a: 10,
        b: 20,
        reply: ractor::RpcReplyPort::new()
    },
    Some(tokio::time::Duration::from_millis(100))
)?;

assert_eq!(result, Ok(30));
```

### Best For:
- Queries (getting data from actor state)
- Calculations (arithmetic, data processing)
- Validation (checking if an operation would succeed)
- Any operation where you need immediate feedback

## Pattern 2: Cast (Fire-and-Forget)

**Use when:** You don't need a response, just want to send a command

**Characteristics:**
- Asynchronous: Returns immediately after sending
- No blocking: Caller continues without waiting
- Best-effort: No guarantee message was processed
- Stateless: Returns nothing

### Example: Reset Command

```rust
use ractor::cast;

// Send command without waiting for response
cast!(calculator, CalculatorMessage::Reset)?;

// Continue immediately - don't wait for confirmation
```

### Best For:
- State mutations (setting values, clearing data)
- Commands (start, stop, reset operations)
- Notifications (actor should react but doesn't need to reply)
- High-throughput scenarios where blocking would be too slow

## Pattern 3: Send (Async Message Passing)

**Use when:** You want non-blocking delivery with optional async response

**Characteristics:**
- Non-blocking: Returns immediately
- Flexible: Can use oneshot channels for async responses
- Message queuing: Messages are queued in actor mailbox
- Concurrent: Safe to send from multiple tasks

### Example: Logger Actor

```rust
// Send message asynchronously
logger.send_message(LoggerMessage::Log {
    msg: "Processing started".to_string()
})?;

// Can optionally get response via oneshot channel
let (tx, rx) = tokio::sync::oneshot::channel();
logger.send_message(LoggerMessage::GetAll { reply: tx })?;
let entries = rx.await?;
```

### Best For:
- Logging (fire-and-forget with optional retrieval)
- Event streaming (sending events to actors)
- Progress updates (notifications without blocking)
- Scenarios requiring concurrent message sending

## Comparison Table

| Pattern  | Blocking | Response | Use Case                     | Example            |
|----------|----------|----------|------------------------------|--------------------|
| call()   | Yes      | Yes      | Queries, calculations        | Get statistics     |
| cast()   | No       | No       | Commands, state mutations    | Reset calculator   |
| send()   | No       | Optional | Logging, events              | Log message        |

## Design Principles

### Zero Panics, Zero Unwraps

All examples follow functional Rust principles:

```rust
// ✓ GOOD: Use Result for error handling
let result = a.checked_add(b).ok_or(CalculatorError::Overflow)?;

// ✗ BAD: Never use unwrap
let result = a.checked_add(b).unwrap(); // FORBIDDEN
```

### Type Safety

Message types enforce communication patterns at compile time:

```rust
// Query messages must have reply ports
pub enum CalculatorMessage {
    Add {
        a: i64,
        b: i64,
        reply: RpcReplyPort<Result<i64, CalculatorError>>, // Required for call!
    },

    // Command messages have no reply
    Reset,  // For cast! or send()
}
```

### Persistent State

State is immutable - updates create new instances:

```rust
pub struct LoggerState {
    entries: Vector<LogEntry, ArcK>,  // Persistent vector
}

// Functional update - returns new state
pub fn add_entry(self, entry: LogEntry) -> Self {
    Self {
        entries: self.entries.push_back(entry),
    }
}
```

## Running Tests

```bash
# Run all messaging pattern tests
moon run :test -- orchestrator messaging_patterns

# Run specific test
moon run :test -- orchestrator call_pattern_request_response_arithmetic

# Run with output
moon run :test -- orchestrator -- --nocapture messaging_patterns
```

## Examples by Use Case

### 1. Need to Calculate Something? Use Call!

```rust
let sum = call!(calc, CalculatorMessage::Add { a: 5, b: 3, reply }, timeout)?;
```

### 2. Need to Change State? Use Cast!

```rust
cast!(calc, CalculatorMessage::SetValue { value: 42 })?;
```

### 3. Need to Log Something? Use Send!

```rust
logger.send_message(LoggerMessage::Log {
    msg: "Operation complete".to_string()
})?;
```

## Performance Considerations

- **call()**: Blocks caller, use sparingly in hot paths
- **cast()**: Non-blocking, ideal for high-throughput commands
- **send()**: Non-blocking, best for events and logging

## Error Handling

All patterns return `Result` for proper error handling:

```rust
// Call pattern returns result of operation
let result = call!(calc, CalculatorMessage::Divide { a: 10, b: 0, reply }, timeout)?;
assert_eq!(result, Err(CalculatorError::DivisionByZero));

// Cast pattern returns SendError if actor is dead
cast!(calc, CalculatorMessage::Reset)?;

// Send pattern returns SendError if actor is dead
logger.send_message(LoggerMessage::Clear)?;
```

## See Also

- `/crates/orchestrator/src/actors/examples/ping_pong/` - Basic actor communication
- `/crates/orchestrator/src/actors/` - Production actor implementations
- [ractor documentation](https://docs.rs/ractor/)
