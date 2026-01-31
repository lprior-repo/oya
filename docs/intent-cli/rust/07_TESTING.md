# Testing Strategy

Comprehensive testing without panics, unwraps, or unsafe code.

## Core Principle

Test both success AND failure paths. Use `Result` in tests.

## Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_success() {
        let result = operation("valid_input");
        assert!(result.is_ok());
    }

    #[test]
    fn test_operation_failure() {
        let result = operation("invalid_input");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_type() {
        match operation("invalid") {
            Err(Error::Validation(_)) => {}, // ✓ Expected
            other => panic!("unexpected: {:?}", other),
        }
    }
}
```

## Testing Results

### Test Success

```rust
#[test]
fn test_parsing() {
    let result = parse_json(r#"{"key": "value"}"#);
    assert!(result.is_ok());

    // Or assert on value
    let value = result.unwrap_or_default();
    assert_eq!(value["key"], "value");
}
```

### Test Failure

```rust
#[test]
fn test_invalid_json() {
    let result = parse_json("{ invalid }");
    assert!(result.is_err());
}

#[test]
fn test_error_message() {
    let result = operation("invalid");
    match result {
        Err(Error::Validation(msg)) => {
            assert!(msg.contains("required"));
        }
        other => panic!("expected Validation error, got: {:?}", other),
    }
}
```

## Pattern Matching Tests

```rust
#[test]
fn test_specific_error() {
    let result = validate_input("");

    // Explicit match
    match result {
        Ok(_) => panic!("should have failed"),
        Err(Error::ValidationError(msg)) => {
            assert_eq!(msg, "input cannot be empty");
        }
        Err(other) => panic!("unexpected error: {:?}", other),
    }
}
```

## Testing Options

```rust
#[test]
fn test_find_item() {
    let items = vec![1, 2, 3, 4, 5];
    let result = items.iter().find(|x| x == &3);

    assert!(result.is_some());
    assert_eq!(result, Some(&3));
}

#[test]
fn test_find_missing() {
    let items = vec![1, 2, 3, 4, 5];
    let result = items.iter().find(|x| x == &99);

    assert!(result.is_none());
}
```

## Property-Based Testing

```rust
#[cfg(test)]
mod property_tests {
    use proptest::proptest;

    proptest! {
        #[test]
        fn test_parser_never_panics(s in ".*") {
            let _ = parse(&s);  // Should never panic
        }

        #[test]
        fn test_roundtrip(data in vec!(any::<i32>(), 1..100)) {
            let serialized = serialize(&data).unwrap();
            let deserialized = deserialize(&serialized).unwrap();
            prop_assert_eq!(data, deserialized);
        }
    }
}
```

## Integration Tests

```rust
#[test]
fn test_full_pipeline() {
    let input = "test_data";
    let parsed = parse(input).expect("parsing failed");
    let validated = validate(&parsed).expect("validation failed");
    let output = transform(&validated).expect("transform failed");

    assert_eq!(output, expected_output);
}
```

## Mocking and Testing Results

```rust
#[test]
fn test_with_fallible_dependency() {
    // Dependency returns error
    let mock_fn = |_| Err::<String, _>(Error::NotFound);

    let result = operation_using_dependency(&mock_fn);

    assert!(result.is_err());
}
```

## Async Testing

```rust
#[tokio::test]
async fn test_async_operation() {
    let result = async_operation("input").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_async_error() {
    let result = async_operation("invalid").await;
    assert!(result.is_err());
}
```

## Doc Tests

```rust
/// Parses JSON string into configuration.
///
/// # Errors
///
/// Returns error if JSON is invalid.
///
/// # Examples
///
/// ```ignore
/// let config = parse_config(r#"{"name": "app"}"#)?;
/// assert_eq!(config.name, "app");
/// ```
pub fn parse_config(json: &str) -> Result<Config> {
    // implementation
}
```

## Test Organization

```rust
#[cfg(test)]
mod tests {
    use super::*;

    mod success_cases {
        use super::*;

        #[test]
        fn test_valid_input() {
            // ...
        }
    }

    mod error_cases {
        use super::*;

        #[test]
        fn test_invalid_input() {
            // ...
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn test_empty_input() {
            // ...
        }
    }
}
```

## Building & Running Tests

```bash
# Run all tests
moon run :test

# Run specific test
cargo test --lib test_name

# Run with output
cargo test -- --nocapture

# Run single-threaded
cargo test -- --test-threads=1
```

## Common Test Patterns

### Testing Collections

```rust
#[test]
fn test_collect_results() {
    let results = vec!["1", "2", "3"]
        .iter()
        .map(|s| s.parse::<i32>())
        .collect::<Result<Vec<_>>>();

    assert!(results.is_ok());
    assert_eq!(results.unwrap_or_default(), vec![1, 2, 3]);
}
```

### Testing Error Propagation

```rust
#[test]
fn test_error_propagates() {
    let result = operation1()
        .and_then(operation2)
        .and_then(operation3);

    // Should be error from operation2
    assert!(result.is_err());
}
```

### Testing Combinators

```rust
#[test]
fn test_map_transforms_value() {
    let result = Ok(5)
        .map(|x| x * 2)
        .map(|x| x + 1);

    assert_eq!(result, Ok(11));
}
```

## Test Performance

```bash
# Run with timing
cargo test -- --test-threads=1 --nocapture

# Profile tests
cargo test --release
```

## Benchmarking

```rust
#[bench]
fn bench_operation(b: &mut Bencher) {
    b.iter(|| operation("input"))
}
```

Run with:
```bash
cargo bench --features unstable
```

## The Philosophy

> "Test both success and failure. Use Result throughout. Never panic in tests (unless testing panic behavior)."

Each test should:
1. ✓ Test one thing
2. ✓ Have clear intent
3. ✓ Not panic
4. ✓ Handle all Result/Option cases
5. ✓ Be isolated (no dependencies between tests)

---

**Next**: [Beads Issue Tracking](08_BEADS.md)
