# Error Handling Patterns

Comprehensive guide to idiomatic, zero-panic error handling.

## Core Pattern: Result<T, Error>

All fallible operations return `Result<T, Error>`:

```rust
pub fn operation(input: &str) -> Result<Output> {
    // implementation
}
```

Never:
- Return bare `T` for fallible operations
- Use `bool` for success/failure
- Throw exceptions (Rust doesn't have them)

## Pattern 1: The `?` Operator (Recommended)

Early return on error, continue on success:

```rust
fn process_file(path: &str) -> Result<String> {
    let content = std::fs::read_to_string(path)?;     // Returns if error
    let parsed = parse(&content)?;                     // Returns if error
    let validated = validate(&parsed)?;                // Returns if error
    Ok(validated)                                       // Success
}
```

**Why**: Concise, readable, idiomatic Rust. The `?` operator unwraps on `Ok` and returns on `Err`.

## Pattern 2: Match Expressions (Explicit)

When you need to handle both cases explicitly:

```rust
match operation() {
    Ok(value) => {
        println!("Success: {}", value);
        process(value)
    }
    Err(e) => {
        eprintln!("Error: {}", e);
        handle_error(e)
    }
}
```

**Why**: Explicit, clear intent, handles all branches.

## Pattern 3: if-let (When Ok Matters)

When you only care about success:

```rust
if let Ok(value) = operation() {
    process(value);
} else {
    // Implicitly ignore error
}
```

**Why**: Concise when error path is unimportant.

## Pattern 4: Combinators (Functional)

Chain operations with combinators:

```rust
operation()
    .map(|v| v * 2)                    // Transform on success
    .and_then(validate)                // Chain fallible ops
    .unwrap_or_else(|e| {              // Fallback on error
        eprintln!("Error: {}", e);
        default_value()
    })
```

| Combinator | Use | Returns |
|------------|-----|---------|
| `map` | Transform value | `Result<U, E>` |
| `and_then` | Chain operations | `Result<U, E>` |
| `or` | Provide alt Result | `Result<T, E>` |
| `or_else` | Compute alt | `Result<T, E>` |
| `unwrap_or` | Provide default | `T` |
| `unwrap_or_else` | Compute default | `T` |
| `map_err` | Transform error | `Result<T, F>` |

## Pattern 5: Custom Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, MyError>;

// Usage
fn operation() -> Result<String> {
    Err(MyError::InvalidInput("test".into()))
}
```

**Why**: Strongly typed errors, easy to match on, good error messages.

## Pattern 6: Error Context

Add context to errors:

```rust
fn load_config(path: &str) -> Result<Config> {
    std::fs::read_to_string(path)
        .map_err(|e| Error::Io(format!("reading {}: {}", path, e)))?
        .parse::<Config>()
        .map_err(|e| Error::InvalidJson(format!("parsing config: {}", e)))
}
```

**Why**: Users understand which step failed and why.

## Pattern 7: Early Return

Return immediately on error:

```rust
fn validate_input(input: &str) -> Result<ValidInput> {
    if input.is_empty() {
        return Err(Error::Empty);
    }

    if input.len() > 1000 {
        return Err(Error::TooLong);
    }

    Ok(ValidInput { data: input.to_string() })
}
```

**Why**: Clear validation logic, obvious error paths.

## Pattern 8: Collect Errors (try_collect)

Collect results or fail on first error:

```rust
// Fail on first error
let values: Result<Vec<i32>> = vec!["1", "2", "3"]
    .into_iter()
    .map(|s| s.parse::<i32>().map_err(Error::ParseError))
    .collect();

// Or use try_fold to accumulate
vec!["1", "2", "3"]
    .into_iter()
    .try_fold(Vec::new(), |mut acc, s| {
        acc.push(s.parse::<i32>()?);
        Ok(acc)
    })
```

**Why**: Collect multiple results with error short-circuiting.

## Pattern 9: Filter with Error

```rust
let result = values
    .into_iter()
    .try_fold(Vec::new(), |mut acc, v| {
        if v > 0 {
            acc.push(v);
        }
        Ok::<_, Error>(acc)
    })?;
```

**Why**: Filter with fallible predicate.

## Pattern 10: Option to Result

Convert Option to Result:

```rust
let required = maybe_value
    .ok_or(Error::NotFound("value required".into()))?;

let or_default = maybe_value
    .ok_or_else(|| Error::NotFound("using default".into()))
    .unwrap_or(default);
```

**Why**: Integrate Option-returning APIs with Result-based code.

## Avoiding Common Mistakes

### ❌ Wrong: Using panic!

```rust
let value = maybe_value.unwrap();  // COMPILE ERROR
if maybe_value.is_some() {
    let v = maybe_value.unwrap();  // ❌ COMPILE ERROR (even though safe!)
}
```

### ✅ Right: Using pattern matching

```rust
let value = match maybe_value {
    Some(v) => v,
    None => return Err(Error::NotFound),
};

if let Some(v) = maybe_value {
    use_value(v);
}
```

### ❌ Wrong: Ignoring errors

```rust
operation()?;  // ❌ Value unused warning
```

### ✅ Right: Handling results

```rust
operation()?;  // ✅ If operation() returns Result<T, E>

let _ = operation();  // ✅ Explicit ignore
operation().ok();      // ✅ Convert to Option, ignore
```

## Error Type Guidelines

### Keep errors simple

```rust
// ✅ Good - specific, typed errors
#[derive(Error, Debug)]
pub enum Error {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("not found: {0}")]
    NotFound(String),
}

// ❌ Bad - stringly-typed
type Result<T> = std::result::Result<T, String>;
```

### Provide context

```rust
// ✅ Good - error with context
.map_err(|e| Error::Database(format!("finding user {}: {}", user_id, e)))?

// ❌ Bad - no context
.map_err(|_| Error::Database)?
```

### From implementations

```rust
#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(#[from] serde_json::Error),
}

// Now these work automatically:
std::fs::read_to_string("file")?;  // io::Error -> Error
serde_json::from_str(json)?;       // serde_json::Error -> Error
```

## Testing

Always test error paths:

```rust
#[test]
fn test_operation_success() {
    let result = operation("valid");
    assert!(result.is_ok());
}

#[test]
fn test_operation_error() {
    let result = operation("invalid");
    assert!(result.is_err());

    match result {
        Err(Error::Validation(_)) => {}, // ✓ Expected error
        other => panic!("unexpected: {:?}", other),
    }
}
```

## Real-World Example

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("missing required field: {0}")]
    MissingField(String),
}

pub type ConfigResult<T> = Result<T, ConfigError>;

pub fn load_config(path: &str) -> ConfigResult<Config> {
    let content = std::fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&content)?;

    let name = value["name"]
        .as_str()
        .ok_or(ConfigError::MissingField("name".into()))?;

    let port = value["port"]
        .as_u64()
        .ok_or(ConfigError::MissingField("port".into()))? as u16;

    Ok(Config {
        name: name.to_string(),
        port,
    })
}
```

## The Principle

> "Every error is recoverable information. Capture it, propagate it, handle it."

Never throw away error information with `unwrap()`. Never panic. Always return `Result`.

---

**Next**: [Building with Moon](02_MOON_BUILD.md)
