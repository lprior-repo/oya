# Functional Programming Patterns

Idiomatic Rust using functional techniques and nightly features.

## Core Philosophy

- **Immutability First** - Values don't change, we create new ones
- **Composition** - Build from small, pure functions
- **Lazy Evaluation** - Compute only when needed
- **Type Safety** - Let the compiler prevent errors

## Iterator Combinators (Fundamental)

```rust
// Map: transform each element
vec![1, 2, 3]
    .iter()
    .map(|x| x * 2)
    .collect::<Vec<_>>()  // [2, 4, 6]

// Filter: keep matching elements
vec![1, 2, 3, 4, 5]
    .iter()
    .filter(|x| x % 2 == 0)
    .collect::<Vec<_>>()  // [2, 4]

// Fold: accumulate into single value
vec![1, 2, 3, 4, 5]
    .iter()
    .fold(0, |acc, x| acc + x)  // 15

// Chain: combine operations
vec![1, 2, 3, 4, 5]
    .iter()
    .filter(|x| x % 2 == 0)
    .map(|x| x * 2)
    .fold(0, |acc, x| acc + x)  // 12
```

## Higher-Order Functions

Functions that take or return functions:

```rust
// Function as parameter
fn apply<T, U, F: Fn(T) -> U>(value: T, f: F) -> U {
    f(value)
}

let double = |x: i32| x * 2;
apply(5, double)  // 10

// Return a function
fn make_adder(n: i32) -> impl Fn(i32) -> i32 {
    move |x| x + n
}

let add_five = make_adder(5);
add_five(3)  // 8

// Function composition
fn compose<T, U, V, F, G>(f: F, g: G) -> impl Fn(T) -> V
where
    F: Fn(T) -> U,
    G: Fn(U) -> V,
{
    move |x| g(f(x))
}

let add_one = |x: i32| x + 1;
let double = |x: i32| x * 2;
let composed = compose(add_one, double);
composed(5)  // (5 + 1) * 2 = 12
```

## Lazy Evaluation (Important for Performance)

```rust
// ❌ Eager - creates intermediate Vec
let result = items
    .iter()
    .filter(|x| x > 5)
    .map(|x| x * 2)
    .collect::<Vec<_>>()  // Materializes here!
    .iter()
    .sum::<i32>();

// ✅ Lazy - never materializes
let result: i32 = items
    .iter()
    .filter(|x| x > 5)
    .map(|x| x * 2)
    .sum();  // Computed lazily

// ✅ Lazy with complex operations
items
    .iter()
    .chunks(5)        // Lazily group by 5
    .into_iter()
    .map(|chunk| chunk.sum::<i32>())
    .filter(|sum| sum > 10)
    .collect::<Vec<_>>()
```

## Option as Functor

```rust
// Map: transform Some value
let maybe = Some(5);
maybe.map(|x| x * 2)  // Some(10)

// and_then: chain Optional operations
let maybe = Some(5);
maybe
    .and_then(|x| if x > 0 { Some(x * 2) } else { None })
    .map(|x| x + 1)
    // Some(11)

// unwrap_or: provide default
maybe.unwrap_or(0)  // 5 (or 0 if None)

// Convert to Result
let required = maybe.ok_or(Error::NotFound)?;
```

## Result as Functor

```rust
// Chain fallible operations
read_file("config.json")
    .and_then(|content| parse_json(&content))
    .and_then(|config| validate_config(&config))
    .map(|config| config.name)
    .unwrap_or_default()

// Or with ? operator
fn operation() -> Result<String> {
    let content = read_file("config.json")?;
    let config = parse_json(&content)?;
    validate_config(&config)?;
    Ok(config.name)
}
```

## Partition (Split Collection)

```rust
let (even, odd): (Vec<_>, Vec<_>) = (1..=5)
    .partition(|x| x % 2 == 0);
// even = [2, 4]
// odd = [1, 3, 5]
```

## Group By

```rust
use itertools::Itertools;

let grouped = vec!["apple", "apricot", "banana", "blueberry"]
    .into_iter()
    .group_by(|s| s.chars().next().unwrap())
    .into_iter()
    .map(|(letter, words)| (letter, words.collect::<Vec<_>>()))
    .collect::<Vec<_>>();
// [('a', ["apple", "apricot"]), ('b', ["banana", "blueberry"])]
```

## Fold/Try-Fold (Accumulation)

```rust
// Simple fold
let sum = (1..=5)
    .fold(0, |acc, x| acc + x);  // 15

// Fold with Result (error short-circuits)
let result = (1..=5)
    .try_fold(0, |acc, x| {
        if x > 3 {
            Err(Error::TooLarge)
        } else {
            Ok(acc + x)
        }
    });
    // Err(Error::TooLarge)

// Complex accumulation
let mut map = HashMap::new();
(1..=5).for_each(|x| {
    *map.entry(x % 2).or_insert(0) += x;
});
// {0: 6, 1: 9}
```

## Immutable Data Structures

```rust
use im::HashMap;

// Create immutable map
let map1 = HashMap::new();

// "Mutate" by creating new map
let map2 = map1.update("key1", "value1");
let map3 = map2.update("key2", "value2");

// All three exist independently
// with shared structural sharing
assert_eq!(map1.len(), 0);
assert_eq!(map2.len(), 1);
assert_eq!(map3.len(), 2);
```

## Async with Combinators

```rust
use futures::stream::StreamExt;

async fn process() -> Result<Vec<String>> {
    futures::stream::iter(vec![1, 2, 3, 4, 5])
        .then(|x| async move {
            fetch_data(x).await
        })
        .filter(|result| futures::future::ready(result.is_ok()))
        .map(|result| result.map(|data| data.to_uppercase()))
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>>>()
}
```

## Pattern Matching

```rust
// Exhaustive matching enforces all cases
match value {
    Some(x) if x > 0 => process_positive(x),
    Some(x) => process_negative(x),
    None => handle_missing(),
}

// Destructuring
let (a, b, c) = (1, 2, 3);

// Match with guards
match items {
    (a, b) if a > b => ("first larger", a - b),
    (a, b) => ("second larger", b - a),
}
```

## Closures

```rust
// Simple
let double = |x| x * 2;

// With type annotations
let add = |x: i32, y: i32| -> i32 { x + y };

// Capturing environment
let multiplier = 2;
let multiply = |x| x * multiplier;

// Move semantics
let text = String::from("hello");
let take_ownership = move || println!("{}", text);
```

## Functional Error Handling

```rust
// Combine multiple validations
let validators = vec![
    |s: &str| if s.is_empty() { Err("empty") } else { Ok(()) },
    |s: &str| if s.len() > 100 { Err("too long") } else { Ok(()) },
    |s: &str| if s.contains(' ') { Err("no spaces") } else { Ok(()) },
];

fn validate_all(input: &str, validators: &[Box<dyn Fn(&str) -> Result<()>>]) -> Result<()> {
    validators
        .iter()
        .try_fold((), |_, v| v(input))
}

// Or with Either for branching
use either::{Either, Left, Right};

fn process(value: i32) -> Either<String, i32> {
    if value < 0 {
        Left(format!("Error: {}", value))
    } else {
        Right(value * 2)
    }
}
```

## Real-World Example

```rust
use itertools::Itertools;

fn analyze_logs(lines: Vec<String>) -> Result<LogAnalysis> {
    lines
        .into_iter()
        .filter(|line| !line.is_empty())
        .map(|line| parse_log_entry(&line))
        .collect::<Result<Vec<_>>>()?  // Error short-circuits
        .into_iter()
        .group_by(|entry| entry.level.clone())
        .into_iter()
        .map(|(level, entries)| {
            let count = entries.count();
            (level, count)
        })
        .collect::<HashMap<_, _>>()
        .into_iter()
        .try_fold(LogAnalysis::default(), |mut analysis, (level, count)| {
            analysis.add_level(level, count)?;
            Ok(analysis)
        })
}
```

## Libraries

- **itertools** - `Itertools` trait with 40+ combinator methods
- **futures** - Future and stream combinators
- **either** - Left/Right sum types
- **im** - Immutable collections
- **tokio** - Async runtime with functional patterns

## Performance Notes

- **Zero-cost abstractions** - Iterator chains compile to equivalent loops
- **Lazy evaluation** - Avoid materializing intermediates
- **Move semantics** - Rust's ownership prevents unnecessary copies
- **Inline** - Closures often inlined by compiler

## The Principle

> "Functional code is easier to reason about, test, parallelize, and optimize."

Rust's type system and zero-cost abstractions make functional programming both ergonomic AND performant.

---

## Command Implementation Patterns

zjj follows a consistent pattern for command implementation:

### 1. Command Structure

**Args + Options Pattern** (for commands with flags):
```rust
// CLI arguments (from clap::ArgMatches)
pub struct Args {
    pub bead_id: String,
    pub format: String,  // Raw string from clap
}

// Internal options (for business logic)
pub struct Options {
    pub bead_id: String,
    pub format: OutputFormat,  // Enum for code clarity
}

// Conversion: Args → Options
impl Args {
    pub fn to_options(&self) -> Options {
        Options {
            bead_id: self.bead_id.clone(),
            format: if self.format == "json" {
                OutputFormat::Json
            } else {
                OutputFormat::Human
            },
        }
    }
}
```

**Options-Only Pattern** (for commands without conversion):
```rust
// Commands like query that are always JSON use only Options
pub struct Options {
    pub query_type: String,
}
```

### 2. Error Handling Pattern

**Business Logic Errors**: Use `zjj_core::Error` at command boundaries
```rust
use zjj_core::Error;

pub fn run(options: &Options) -> Result<()> {
    let result = execute_business_logic(options)
        .map_err(|e| anyhow::Error::new(e))?;

    output_result(&result, options.format)
}
```

**System Errors**: Use `anyhow::Error` with `.context()` for system operations
```rust
pub fn run(options: &Options) -> Result<()> {
    let file = read_file(path)
        .context("Failed to read configuration")?;
    Ok(())
}
```

### 3. JSON Output Pattern

All commands use `SchemaEnvelope` for JSON output:
```rust
use zjj_core::json::SchemaEnvelope;

pub struct Output {
    pub name: String,
    pub status: String,
}

fn output_json(data: &Output) -> Result<()> {
    let envelope = SchemaEnvelope::new("command-response", "single", data);
    println!("{}", serde_json::to_string_pretty(&envelope)?);
}
```

### 4. Module Boundaries

- **zjj-core**: Pure library (no CLI, database, or side effects)
- **crates/zjj**: CLI + commands + database + Zellij integration
- **zjj_core::Error**: Defined in core for semantic exit codes

**Next**: [Rust Standards](05_RUST_STANDARDS.md)
