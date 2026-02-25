# Functional Rust - The Holy Trinity Stack

This document defines the strictly functional Rust approach for the OYA project, built on three core libraries that enable pure functional programming without fighting Rust's defaults.

## The Problem

Writing pure functional code with standard Rust collections (`Vec`, `HashMap`) tanks performance because cloning copies entire memory blocks. You're forced to choose between:
- **Slow but pure**: Clone everything, sacrifice performance
- **Fast but imperative**: Use `&mut`, sacrifice functional purity

## The Solution: The Holy Trinity

Instead of fighting Rust's defaults, these libraries replace standard containers and imperative flow with functional equivalents.

| **Goal** | **The Problem in Standard Rust** | **The Library Solution** |
| --- | --- | --- |
| **Immutability** | `Vec` and `HashMap` are slow to clone. You are tempted to use `&mut` for performance. | **`im`** (Immutable RS). Provides structural sharing. Cloning a huge vector is O(1) and instant. You never need `mut`. |
| **Piping (Gleam style)** | You have to nest functions: `f(g(x))` or allow temp variables. | **`tap`**. Adds `.pipe()`, `.tap()`, and `.pipe_ref()` to *everything*. |
| **Railway / Monads** | `Result` is built-in, but handling enumerations of logic (Effects) is boilerplate-heavy. | **`strum`**. Makes Enums (your "Effects") powerful. Auto-generates matching logic, iteration, and string conversion. |
| **Logic Loops** | `for` loops are imperative and mutable. | **`itertools`**. The missing functional adaptors (`interleave`, `unique`, `group_by`) that let you stay in Iterator-land forever. |
| **Typed Errors** | String errors crash; `Box<dyn Error>` is lazy. | **`thiserror`**. Forces you to define strict error types for the Railway tracks. |

---

## 1. Dependencies in `Cargo.toml`

Add these to your project. This is your functional toolkit.

```toml
[dependencies]
# The Functional Data Structures (Critical for performance without mut)
im = "15.1"

# The Pipe Operator
tap = "1.0"

# Enum Superpowers (For Effects/Commands)
strum = { version = "0.26", features = ["derive"] }

# The Standard Functional Tools
itertools = "0.13"
thiserror = "1.0"
```

---

## 2. Why `im` is the Game Changer

If you try to write "Pure Functional" code with standard Rust `Vec`, your performance will tank because you are copying memory constantly to avoid mutation.

**`im`** uses "Structural Sharing" (like React's state or Haskell's lists).

- **Standard Rust:** `let list2 = list1.clone();` → Copies 1,000 items. (Slow)
- **`im` Crate:** `let list2 = list1.clone();` → Copies 1 pointer. (Instant)

### Performance Characteristics

```rust
use im::Vector;

// O(1) clone - shares structure
let v1 = Vector::from(vec![1, 2, 3, 4, 5]);
let v2 = v1.clone(); // Instant, shares memory

// O(1) push - creates new version without mutating original
let v3 = v2.push_back(6); // v2 unchanged, v3 has new element
```

---

## 3. Complete Example: User Registration with the Holy Trinity

Here is a complete "User Registration" flow using `im` for data, `tap` for piping, and `strum` for effects.

**Notice:**
- We use `im::Vector` instead of `Vec`
- We use `tap` to pipe
- We use `strum` to manage our Effect enum
- Zero `unwrap()`, zero `panic!`, zero `mut`

```rust
// --- THE STRICT HEADER ---
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::arithmetic_side_effects)]

use im::Vector; // <--- The Immutable List (Fast cloning)
use tap::Pipe;  // <--- The Pipe
use thiserror::Error;
use strum::EnumDiscriminants; // <--- Helps matching Enums

// --- 1. DOMAIN (Immutable Data) ---

// "Effect as Data" Pattern
#[derive(Debug, EnumDiscriminants)]
#[strum_discriminants(name(EffectType))] // Auto-generates EffectType enum
pub enum Effect {
    Log { msg: String },
    SaveToDb { json: String },
    SendEmail { address: String },
}

#[derive(Debug, Clone)]
pub struct User {
    username: String,
    email: String,
}

#[derive(Debug, Error)]
pub enum RegError {
    #[error("username empty")]
    EmptyUsername,
}

// --- 2. PURE CORE (Monadic Flow) ---

fn validate(u: User) -> Result<User, RegError> {
    if u.username.is_empty() {
        Err(RegError::EmptyUsername)
    } else {
        Ok(u)
    }
}

// Returns a Tuple: (New State, List of Side Effects)
fn register_logic(u: User) -> (User, Vector<Effect>) {
    let effects = Vector::from(vec![
        Effect::Log { msg: format!("Registering {}", u.username) },
        Effect::SaveToDb { json: "{}".into() },
        Effect::SendEmail { address: u.email.clone() }
    ]);

    (u, effects)
}

// The Pipeline
pub fn handle_registration(username: String, email: String) -> Result<Vector<Effect>, RegError> {
    User { username, email }
        .pipe(validate)?             // Railway track check
        .pipe(register_logic)        // Pure logic
        .pipe(|(_user, effects)| effects) // We only care about effects for the shell
        .pipe(Ok)
}
```

---

## 4. The Pattern: Core + Shell Architecture

The functional approach separates your code into two layers:

### Core (Pure Functions)
- **No I/O**: Never touches filesystem, network, or database
- **Returns Effects**: Pure functions return `Vector<Effect>` describing what *should* happen
- **100% Testable**: All logic can be tested without mocks or side effects
- **Uses `im` types**: All data structures use `im::Vector`, `im::HashMap`, etc.

```rust
// Pure - returns instructions, doesn't execute them
fn process_payment(amount: u64) -> Vector<Effect> {
    Vector::from(vec![
        Effect::Log { msg: format!("Processing ${}", amount) },
        Effect::ChargeCard { amount },
        Effect::SendReceipt { amount },
    ])
}
```

### Shell (Effect Interpreter)
- **Executes Effects**: Interprets the `Vector<Effect>` and performs I/O
- **Thin layer**: No business logic, just executes instructions
- **Can use async**: Effect handlers can be async since they're at the boundary

```rust
async fn execute_effects(effects: Vector<Effect>) -> Result<(), Error> {
    for effect in effects {
        match effect {
            Effect::Log { msg } => log::info!("{}", msg),
            Effect::SaveToDb { json } => db.save(&json).await?,
            Effect::SendEmail { address } => email::send(&address).await?,
        }
    }
    Ok(())
}
```

---

## 5. Mandatory Usage Rules

### Always Use These Libraries

1. **DATA STRUCTURES (`im` crate):**
   - Never use `std::vec::Vec` or `std::collections::HashMap` for data passing
   - ALWAYS use `im::Vector` and `im::HashMap`
   - Explain that this allows "Cheap O(1) Cloning" and persistent data structures

2. **CONTROL FLOW (`tap` crate):**
   - Never assign intermediate variables
   - Use `.pipe()` for transformations
   - Use `.tap()` for side-effect-free inspection (like debug logging)

3. **ITERATION (`itertools` crate):**
   - Never use `for` loops in core logic
   - Use `fold`, `map`, `intersperse`, `group_by`

4. **EFFECTS & ERRORS (`strum` + `thiserror`):**
   - Use `thiserror` for all failure states
   - Use `strum` macros if Enums become complex
   - Pure functions return `im::Vector<Effect>` instructions, never perform I/O

### Safety Constraints

Start every file with:

```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(clippy::arithmetic_side_effects)]
```

- Zero `unwrap()` and `expect()` - use `?` operator or explicit error handling
- Zero `panic!`, `todo!`, `unimplemented!` - all code paths must be complete
- Use checked arithmetic only - no raw `+`, `-`, `*`, `/` on integers that could overflow

---

## 6. Common Patterns

### Railway-Oriented Programming

```rust
pub fn process_order(data: OrderData) -> Result<Vector<Effect>, OrderError> {
    data
        .pipe(validate_order)?           // First validation gate
        .pipe(check_inventory)?          // Second validation gate
        .pipe(calculate_total)?          // Transform data
        .pipe(apply_discounts)?          // Another transform
        .pipe(generate_effects)          // Pure logic returns effects
        .pipe(Ok)                        // Wrap in Ok
}
```

### Effect as Data

```rust
#[derive(Debug)]
pub enum Effect {
    CreateFile { path: String, content: String },
    DeleteFile { path: String },
    Log { level: LogLevel, msg: String },
    HttpRequest { url: String, method: Method },
}

// Pure function - just returns what should happen
fn handle_upload(file: FileData) -> Vector<Effect> {
    Vector::from(vec![
        Effect::Log {
            level: LogLevel::Info,
            msg: format!("Uploading {}", file.name)
        },
        Effect::CreateFile {
            path: format!("/uploads/{}", file.name),
            content: file.data,
        },
    ])
}
```

### Functional Loops

```rust
use itertools::Itertools;

// Instead of for loop with mutation:
// let mut results = Vec::new();
// for item in items {
//     results.push(process(item));
// }

// Use functional iteration:
let results: Vector<_> = items
    .into_iter()
    .map(process)
    .collect();

// Complex grouping:
let grouped = items
    .into_iter()
    .group_by(|item| item.category)
    .into_iter()
    .map(|(key, group)| {
        let items: Vector<_> = group.collect();
        (key, items)
    })
    .collect::<im::HashMap<_, _>>();
```

---

## 7. Integration with OYA Project

For the OYA project specifically:

1. **Task operations** return `Vector<Effect>` describing file changes, git operations, etc.
2. **Stage execution** is pure - returns effects for tool execution
3. **Workspace isolation** commands are effects to be interpreted by zjj/jj shell
4. **Pipeline stages** compose via `.pipe()` chains

Example:

```rust
pub fn execute_stage(task: Task, stage: Stage) -> Result<Vector<Effect>, StageError> {
    task
        .pipe(|t| validate_task_ready(t, &stage))?
        .pipe(|t| prepare_workspace(t, &stage))
        .pipe(|(_task, effects)| effects)
        .pipe(Ok)
}

fn prepare_workspace(task: Task, stage: &Stage) -> (Task, Vector<Effect>) {
    let effects = Vector::from(vec![
        Effect::ZjjCommand {
            cmd: format!("zjj add {}", task.slug)
        },
        Effect::Log {
            msg: format!("Workspace ready for {}", stage.name)
        },
    ]);
    (task, effects)
}
```

---

## 8. AI Prompt for Generating Functional Rust

When asking Claude Code or other AI to write Rust code for this project, use this prompt:

```markdown
You are a Functional Rust Expert. You reject imperative Rust patterns in favor of the "Holy Trinity" of functional crates: `im`, `tap`, and `itertools`.

**MANDATORY LIBRARY USAGE:**

1. **DATA STRUCTURES (`im` crate):**
   * Never use `std::vec::Vec` or `std::collections::HashMap` for data passing.
   * ALWAYS use `im::Vector` and `im::HashMap`.
   * Explain that this allows "Cheap O(1) Cloning" and persistent data structures.

2. **CONTROL FLOW (`tap` crate):**
   * Never assign intermediate variables.
   * Use `.pipe()` for transformations.
   * Use `.tap()` for side-effect-free inspection (like debug logging).

3. **ITERATION (`itertools` crate):**
   * Never use `for` loops.
   * Use `fold`, `map`, `intersperse`, `group_by`.

4. **EFFECTS & ERRORS (`strum` + `thiserror`):**
   * Use `thiserror` for all failure states.
   * Use `strum` macros if Enums become complex.
   * Pure functions return `im::Vector<Effect>` instructions, never perform I/O.

**SAFETY CONSTRAINTS:**
* Start every file with `#![deny(clippy::unwrap_used)]`, `#![deny(clippy::panic)]`, `#![deny(clippy::arithmetic_side_effects)]`.
* Enforce checked math only.

**RESPONSE FORMAT:**
Write the code assuming these libraries are present in `Cargo.toml`.
```

---

## References

- [`im` crate documentation](https://docs.rs/im/)
- [`tap` crate documentation](https://docs.rs/tap/)
- [`itertools` crate documentation](https://docs.rs/itertools/)
- [`strum` crate documentation](https://docs.rs/strum/)
- [`thiserror` crate documentation](https://docs.rs/thiserror/)
