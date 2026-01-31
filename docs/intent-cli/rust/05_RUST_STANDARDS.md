# Rust Standards: Zero Unwrap, Zero Panic

The sacred law: All code returns Result. No panics. No unsafe. Enforced by the compiler.

## The Three Laws (Compile Errors)

```rust
❌ .unwrap()        // forbid(clippy::unwrap_used)
❌ .expect()        // forbid(clippy::expect_used)
❌ panic!()         // forbid(clippy::panic)
❌ unsafe { }       // forbid(unsafe_code)
❌ unimplemented!() // forbid(clippy::unimplemented)
❌ todo!()          // forbid(clippy::todo)
```

**These will not compile.**

## Error Handling Required

Every fallible operation must return `Result<T, Error>`:

```rust
// ✅ Correct
fn operation(input: &str) -> Result<Output> {
    validate(input)?;
    Ok(transform(input))
}

// ❌ Wrong - doesn't return Result
fn operation(input: &str) -> Output {
    validate(input).unwrap();  // COMPILE ERROR
    transform(input)
}
```

## Required Patterns

### Pattern 1: `?` Operator
```rust
fn operation() -> Result<T> {
    let value = fallible()?;
    Ok(value)
}
```

### Pattern 2: Match
```rust
match operation() {
    Ok(v) => use_it(v),
    Err(e) => handle_error(e),
}
```

### Pattern 3: Combinators
```rust
operation()
    .map(transform)
    .and_then(validate)
    .unwrap_or_default()
```

### Pattern 4: if-let
```rust
if let Ok(value) = operation() {
    use_value(value);
}
```

## Option Handling

Never unwrap Options. Never:

```rust
❌ maybe.unwrap()              // COMPILE ERROR
❌ if maybe.is_some() {         // Don't do this either
     let v = maybe.unwrap();  // COMPILE ERROR
   }
```

Do this:

```rust
✅ if let Some(v) = maybe {
     use_value(v);
   }

✅ maybe.map(use_value).unwrap_or_else(default)

✅ match maybe {
     Some(v) => process(v),
     None => default_action(),
   }
```

## Custom Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, MyError>;
```

## Builder Pattern (Validation on Build)

```rust
pub struct ConfigBuilder {
    name: Option<String>,
}

impl ConfigBuilder {
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn build(self) -> Result<Config> {
        let name = self
            .name
            .ok_or(Error::InvalidConfig("name required".into()))?;

        if name.is_empty() {
            return Err(Error::InvalidConfig("name cannot be empty".into()));
        }

        Ok(Config { name })
    }
}
```

## Documentation Requirements

All public items must be documented:

```rust
/// Brief description.
///
/// Longer description if needed.
///
/// # Errors
///
/// Returns an error if [condition].
///
/// # Examples
///
/// ```ignore
/// let result = my_function(input)?;
/// ```
pub fn my_function(input: &str) -> Result<Output> {
    // implementation
}
```

## Testing Without Panics

```rust
#[test]
fn test_success() {
    let result = operation("valid");
    assert!(result.is_ok());
}

#[test]
fn test_error() {
    let result = operation("invalid");
    assert!(result.is_err());
}

#[test]
fn test_error_type() {
    match operation("invalid") {
        Err(Error::Validation(_)) => {}, // ✓ Expected
        other => panic!("unexpected: {:?}", other),
    }
}
```

## Clippy Rules (Auto-Enforced)

### Forbidden (Compile Errors)
- `unsafe_code` - No unsafe blocks
- `unwrap_used` - No unwrap()
- `expect_used` - No expect()
- `panic` - No panic!()
- `unimplemented` - No unimplemented!()
- `todo` - No todo!()

### Enforced (Warnings = Errors)
- `clippy::all` - All pedantic warnings
- `clippy::pedantic` - Best practices
- `clippy::correctness` - Likely bugs
- `clippy::suspicious` - Suspicious code

## Code Review Checklist

Before any PR:

- [ ] No `unwrap()` calls (compiler checks)
- [ ] No `expect()` calls (compiler checks)
- [ ] No `panic!()` calls (compiler checks)
- [ ] No `unsafe { }` (compiler checks)
- [ ] All `Err` paths handled
- [ ] All `None` paths handled
- [ ] All public items documented
- [ ] `Result` types for fallible operations
- [ ] Error types are descriptive
- [ ] Tests don't panic
- [ ] `moon run :ci` passes
- [ ] No clippy warnings

## Common Mistakes

### ❌ Returning bool for errors
```rust
fn validate(input: &str) -> bool {  // ❌ Wrong
    // returns true/false, no error info
}
```

### ✅ Return Result with error info
```rust
fn validate(input: &str) -> Result<()> {  // ✅ Right
    if invalid {
        Err(Error::Validation("reason".into()))
    } else {
        Ok(())
    }
}
```

### ❌ Ignoring errors
```rust
operation()?;  // ❌ Clippy warning - value unused
```

### ✅ Handle or explicitly ignore
```rust
operation()?;           // ✅ If function returns Result
let _ = operation();    // ✅ Explicit ignore
operation().ok();       // ✅ Convert to Option
```

## Error Context Pattern

```rust
fn load_config(path: &str) -> Result<Config> {
    std::fs::read_to_string(path)
        .map_err(|e| Error::Io(format!("reading {}: {}", path, e)))?
        .parse::<Config>()
        .map_err(|e| Error::Parse(format!("parsing config: {}", e)))
}
```

## Collecting Results

```rust
// Fail on first error
let values: Result<Vec<i32>> = vec!["1", "2", "3"]
    .into_iter()
    .map(|s| s.parse::<i32>().map_err(Error::ParseError))
    .collect();

// Or accumulate with error handling
vec!["1", "2", "3"]
    .into_iter()
    .try_fold(Vec::new(), |mut acc, s| {
        acc.push(s.parse::<i32>()?);
        Ok(acc)
    })
```

## Filter with Fallible Predicate

```rust
let valid = items
    .into_iter()
    .try_fold(Vec::new(), |mut acc, item| {
        if should_keep(&item)? {  // Fallible predicate
            acc.push(item);
        }
        Ok(acc)
    })?;
```

## The Principle

> "Write code that fails gracefully, not code that crashes."

Every operation that can fail must return `Result`. The compiler enforces this. Trust it.

## Performance Note

Zero-cost abstraction: All Result operations compile to identical machine code as direct computation. No runtime overhead.

---

**Next**: [Combinators](06_COMBINATORS.md)
