# Erlang FFI Side Effects Documentation

This document catalogs all impure FFI functions and their hidden side effects. It serves as a critical reference for understanding type honesty violations in the Intent CLI codebase.

## Table of Contents
1. [Critical Type Honesty Violations](#critical-type-honesty-violations)
2. [All FFI Functions Catalog](#all-ffi-functions-catalog)
3. [Safe Usage Guidelines](#safe-usage-guidelines)
4. [Testing Strategies](#testing-strategies)
5. [Future Refactoring Roadmap](#future-refactoring-roadmap)

---

## Critical Type Honesty Violations

### 1. ETS Cache (intent_checker.erl)

**Function:** `get_or_compile_regex/1`

**Location:** `/home/lewis/src/intent-cli/src/intent_checker.erl`

**Signature Claims:** Pure (same input → same output)
```gleam
@external(erlang, "intent_checker", "get_or_compile_regex")
fn get_or_compile_regex(pattern: String) -> Result(Regex, Nil)
```

**Reality:** Mutates global ETS table

**Side Effects:**
- Creates named ETS table `intent_regex_cache` on first use
- Reads from global cache (concurrent reads via `read_concurrency`)
- Writes compiled regex patterns to cache
- Evicts oldest entries when cache exceeds 1000 patterns (DoS protection)
- FIFO eviction policy modifies cache state

**Impact:**
- Non-deterministic behavior in multi-threaded environments
- Memory accumulation up to 1000 compiled patterns
- First call initializes global state
- Race conditions possible during initialization (though unlikely due to ETS atomicity)

**Used By:**
- `src/intent/checker/rules.gleam:601`

**Recommendation:**
- Add explicit `Cache` effect type: `fn get_or_compile_regex(pattern: String) -> Cache(Result(Regex, Nil))`
- Or pass cache handle as explicit parameter: `fn get_or_compile_regex(cache: RegexCache, pattern: String) -> Result(Regex, Nil)`
- Document cache limits and eviction policy in type signature or comments

---

### 2. UUID Generation (intent_ffi.erl)

**Function:** `generate_uuid/0`

**Location:** `/home/lewis/src/intent-cli/src/intent_ffi.erl:31-34`

**Signature Claims:** Pure constant function
```gleam
@external(erlang, "intent_ffi", "generate_uuid")
fn generate_uuid() -> String
```

**Reality:** Non-deterministic, advances PRNG state

**Side Effects:**
- Calls `crypto:strong_rand_bytes(16)` which consumes system entropy
- Advances cryptographically secure PRNG state
- May block if entropy pool exhausted (rare on modern systems)
- Each call produces different output

**Implementation:**
```erlang
generate_uuid() ->
    <<A:32, B:16, C:16, D:16, E:48>> = crypto:strong_rand_bytes(16),
    Parts = [to_hex(A, 8), "-", to_hex(B, 4), "-", to_hex(C, 4), "-", to_hex(D, 4), "-", to_hex(E, 12)],
    list_to_binary(Parts).
```

**Used By:**
- `src/intent.gleam:3495`

**Recommendation:**
- Wrap in `Random` or `IO` effect type: `fn generate_uuid() -> Random(String)`
- Accept seed parameter for deterministic testing: `fn generate_uuid_seeded(seed: BitArray) -> String`
- For testing, provide mock implementation that returns predictable values

---

### 3. Time Functions (intent_ffi.erl)

#### 3a. now_ms/0

**Function:** `now_ms/0`

**Location:** `/home/lewis/src/intent-cli/src/intent_ffi.erl:4-5`

**Signature Claims:** Pure constant function
```gleam
@external(erlang, "intent_ffi", "now_ms")
fn now_ms() -> Int
```

**Reality:** Reads global system clock

**Side Effects:**
- Calls `erlang:system_time(millisecond)` which reads OS clock
- Non-deterministic: returns different value on each call
- Subject to clock skew, NTP adjustments, leap seconds
- May go backwards if system clock adjusted

**Implementation:**
```erlang
now_ms() ->
    erlang:system_time(millisecond).
```

**Used By:**
- `src/intent/http_client.gleam:442`

**Recommendation:**
- Wrap in `Clock` or `IO` effect type: `fn now_ms() -> Clock(Int)`
- Pass clock instance for dependency injection
- For testing, provide mock clock with fixed time

---

#### 3b. current_timestamp/0

**Function:** `current_timestamp/0`

**Location:** `/home/lewis/src/intent-cli/src/intent_ffi.erl:40-43`

**Signature Claims:** Pure constant function
```gleam
@external(erlang, "intent_ffi", "current_timestamp")
fn current_timestamp() -> String
```

**Reality:** Reads global system clock and formats to RFC3339

**Side Effects:**
- Same as `now_ms/0` plus string formatting
- Returns ISO 8601 / RFC3339 formatted timestamp
- Non-deterministic output

**Implementation:**
```erlang
current_timestamp() ->
    Now = erlang:system_time(millisecond),
    calendar:system_time_to_rfc3339(Now, [{unit, millisecond}]).
```

**Used By:**
- `src/intent/bead_feedback.gleam:409`
- `src/intent/json_output.gleam:88`
- `src/intent.gleam:3498`

**Recommendation:**
- Same as `now_ms/0`: wrap in `Clock` effect type
- Consider consolidating with `now_ms/0` using a `Clock` module

---

#### 3c. current_iso8601_timestamp/0 (MISSING)

**Function:** `current_iso8601_timestamp/0`

**Referenced By:**
- `src/intent/plan_mode.gleam:581`
- `src/intent.gleam:2197`

**Status:** ⚠️ **UNDEFINED** - Referenced but not implemented in `intent_ffi.erl`

**Likely Intent:** Alias for `current_timestamp/0` or similar time formatting

**Recommendation:**
- Either implement in `intent_ffi.erl` or remove references
- If implemented, same guidelines as `current_timestamp/0`

---

### 4. Environment Variables (intent_ffi.erl)

**Function:** `get_env/1`

**Location:** `/home/lewis/src/intent-cli/src/intent_ffi.erl:49-54`

**Signature Claims:** Pure lookup function
```gleam
@external(erlang, "intent_ffi", "get_env")
fn get_env(name: String) -> Result(String, Nil)
```

**Reality:** Reads mutable global state

**Side Effects:**
- Calls `os:getenv/1` which reads OS environment variables
- Mutable: environment can change between calls (though rare in practice)
- Platform-dependent behavior
- Security implications: leaks environment into application

**Implementation:**
```erlang
get_env(Name) when is_binary(Name) ->
    case os:getenv(binary_to_list(Name)) of
        false -> {error, nil};
        Value -> {ok, list_to_binary(Value)}
    end.
```

**Used By:**
- `src/intent.gleam:3501`
- `is_localhost_allowed_by_env/0` function

**Recommendation:**
- Pass environment as immutable snapshot: `fn get_env(env: Environment, name: String) -> Result(String, Nil)`
- Load environment once at startup into `Dict(String, String)`
- For testing, inject test environment without OS dependency
- Document security implications of environment access

---

### 5. IO Operations (intent_ffi_stdin.erl)

#### 5a. read_line/0

**Function:** `read_line/0`

**Location:** `/home/lewis/src/intent-cli/src/intent_ffi_stdin.erl:9-14`

**Signature Claims:** Pure constant function
```gleam
@external(erlang, "intent_ffi_stdin", "read_line")
fn read_line() -> Result(String, String)
```

**Reality:** IO operation, mutates stdin stream position

**Side Effects:**
- Blocks waiting for user input
- Consumes bytes from stdin stream (destructive read)
- Returns different value on each call
- Can return EOF
- Process-blocking operation

**Implementation:**
```erlang
read_line() ->
  case io:get_line('') of
    eof -> {error, <<"EOF">>};
    {error, Reason} -> {error, atom_to_binary(Reason, utf8)};
    Line -> {ok, Line}
  end.
```

**Used By:**
- `src/intent/stdin.gleam:3`

**Recommendation:**
- Wrap in `IO` effect type: `fn read_line() -> IO(Result(String, String))`
- For testing, provide mock stdin with predefined input
- Consider streaming abstractions for testability

---

#### 5b. read_line_trimmed/0

**Function:** `read_line_trimmed/0`

**Location:** `/home/lewis/src/intent-cli/src/intent_ffi_stdin.erl:17-25`

**Signature Claims:** Pure constant function
```gleam
@external(erlang, "intent_ffi_stdin", "read_line_trimmed")
fn read_line_trimmed() -> Result(String, String)
```

**Reality:** Same as `read_line/0` plus whitespace trimming

**Side Effects:**
- Same as `read_line/0`
- Additionally strips trailing newline/carriage return

**Implementation:**
```erlang
read_line_trimmed() ->
  case io:get_line('') of
    eof -> {error, <<"EOF">>};
    {error, Reason} -> {error, atom_to_binary(Reason, utf8)};
    Line ->
      Trimmed = string:trim(Line, trailing, "\n\r"),
      {ok, Trimmed}
  end.
```

**Used By:**
- `src/intent/stdin.gleam:6`

**Recommendation:**
- Same as `read_line/0`

---

### 6. Process Termination (intent_ffi.erl)

**Function:** `halt/1`

**Location:** `/home/lewis/src/intent-cli/src/intent_ffi.erl:7-8`

**Signature Claims:** Returns Nil
```gleam
@external(erlang, "intent_ffi", "halt")
fn halt(code: Int) -> Nil
```

**Reality:** Terminates entire BEAM VM immediately

**Side Effects:**
- Calls `erlang:halt/1` which **immediately terminates the VM**
- No cleanup, no finalizers, no graceful shutdown
- Destructive operation that affects entire process tree
- Exit code affects shell $? variable

**Implementation:**
```erlang
halt(Code) ->
    erlang:halt(Code).
```

**Used By:**
- `src/intent/ai_errors.gleam:417`
- `src/intent.gleam:3492`

**Type Honesty Issue:** Function signature claims it returns `Nil`, but it never returns at all!

**Recommendation:**
- Change return type to reflect reality: `fn halt(code: Int) -> Never`
- Or wrap in `IO` effect and document termination: `fn halt(code: Int) -> IO(Never)`
- Consider alternatives: return error codes instead of terminating
- Document that this is a last-resort escape hatch

---

### 7. Pure FFI Functions (For Contrast)

These FFI functions are actually pure and have no side effects:

#### 7a. base64_url_decode/1

**Function:** `base64_url_decode/1`

**Location:** `/home/lewis/src/intent-cli/src/intent_ffi.erl:11-28`

**Side Effects:** None - pure transformation

**Implementation:**
```erlang
base64_url_decode(Input) when is_binary(Input) ->
    Standard = << <<(case C of
        $- -> $+;
        $_ -> $/;
        _ -> C
    end)>> || <<C>> <= Input >>,
    Padded = case byte_size(Standard) rem 4 of
        0 -> Standard;
        2 -> <<Standard/binary, "==">>;
        3 -> <<Standard/binary, "=">>
    end,
    try
        {ok, base64:decode(Padded)}
    catch
        _:_ -> {error, invalid_base64}
    end.
```

**Used By:**
- `src/intent/checker/rules.gleam:416`

**Status:** ✅ Type honest - truly pure function

---

#### 7b. int_to_float/1

**Function:** `int_to_float/1`

**Location:** `/home/lewis/src/intent-cli/src/intent_ffi.erl:45-47`

**Side Effects:** None - pure conversion

**Implementation:**
```erlang
int_to_float(I) when is_integer(I) ->
    float(I).
```

**Status:** ✅ Type honest - truly pure function

---

## All FFI Functions Catalog

| Function | Module | Pure? | Side Effects | Risk Level |
|----------|--------|-------|--------------|-----------|
| `get_or_compile_regex/1` | intent_checker | ❌ No | ETS cache mutation | 🟡 Medium |
| `generate_uuid/0` | intent_ffi | ❌ No | PRNG state, entropy | 🟢 Low |
| `now_ms/0` | intent_ffi | ❌ No | System clock read | 🟢 Low |
| `current_timestamp/0` | intent_ffi | ❌ No | System clock read | 🟢 Low |
| `get_env/1` | intent_ffi | ❌ No | Environment read | 🟡 Medium |
| `read_line/0` | intent_ffi_stdin | ❌ No | Stdin consumption | 🟡 Medium |
| `read_line_trimmed/0` | intent_ffi_stdin | ❌ No | Stdin consumption | 🟡 Medium |
| `halt/1` | intent_ffi | ❌ No | VM termination | 🔴 High |
| `base64_url_decode/1` | intent_ffi | ✅ Yes | None | 🟢 Low |
| `int_to_float/1` | intent_ffi | ✅ Yes | None | 🟢 Low |

**Risk Levels:**
- 🔴 **High**: Destructive operations (halt, file deletion, etc.)
- 🟡 **Medium**: Stateful operations (caching, I/O, environment)
- 🟢 **Low**: Side effects exist but safe in practice

---

## Safe Usage Guidelines

### General Principles

1. **Acknowledge Impurity**: Recognize that these functions have side effects
2. **Isolate at Boundaries**: Keep impure code at application edges
3. **Document Usage**: Comment when and why impure functions are called
4. **Test Carefully**: Use dependency injection or mocking strategies

### Specific Patterns

#### Pattern 1: Lazy Initialization (ETS Cache)

**Current (Type-Dishonest):**
```gleam
// Appears pure but mutates global cache
let regex = get_or_compile_regex(pattern)
```

**Better (Explicit State):**
```gleam
// Make cache explicit
pub type RegexCache {
  RegexCache(table: ets.Table)
}

pub fn new_cache() -> RegexCache {
  let table = ets.new([Named("intent_regex_cache"), Public])
  RegexCache(table)
}

pub fn get_or_compile(cache: RegexCache, pattern: String) -> Result(Regex, Nil) {
  // Now it's clear this function needs a cache
}
```

#### Pattern 2: Dependency Injection (Time)

**Current (Type-Dishonest):**
```gleam
// Appears pure but reads system clock
let timestamp = current_timestamp()
```

**Better (Clock Injection):**
```gleam
pub type Clock {
  Clock(now: fn() -> Int, format: fn(Int) -> String)
}

pub fn system_clock() -> Clock {
  Clock(now: now_ms, format: format_timestamp)
}

pub fn fixed_clock(time: Int) -> Clock {
  Clock(now: fn() { time }, format: format_timestamp)
}

// Use in application code
pub fn do_work(clock: Clock) {
  let timestamp = (clock.format)((clock.now)())
  // ...
}
```

#### Pattern 3: Environment as Value

**Current (Type-Dishonest):**
```gleam
// Appears pure but reads OS environment
case get_env("INTENT_ALLOW_LOCALHOST") {
  Ok("true") -> True
  _ -> False
}
```

**Better (Environment Snapshot):**
```gleam
pub type Environment {
  Environment(vars: Dict(String, String))
}

pub fn load_environment() -> Environment {
  // Load once at startup
  Environment(vars: load_all_env_vars())
}

pub fn get(env: Environment, key: String) -> Result(String, Nil) {
  dict.get(env.vars, key)
}

// Use in application code
pub fn main() {
  let env = load_environment()
  run_app(env)
}
```

#### Pattern 4: IO as Explicit Type

**Current (Type-Dishonest):**
```gleam
// Appears pure but performs I/O
let line = read_line()
```

**Better (IO Monad/Type):**
```gleam
pub type IO(a) {
  IO(run: fn() -> a)
}

pub fn read_line() -> IO(Result(String, String)) {
  IO(run: fn() { ffi_read_line() })
}

pub fn run_io(io: IO(a)) -> a {
  (io.run)()
}

// Use in application code
pub fn main() {
  let io_action = read_line()
  let result = run_io(io_action)  // Explicit execution point
}
```

---

## Testing Strategies

### Strategy 1: Mock FFI Functions

Create test doubles for FFI functions:

```gleam
// test/test_doubles.gleam
pub fn mock_uuid() -> String {
  "00000000-0000-0000-0000-000000000000"
}

pub fn mock_timestamp() -> String {
  "2026-01-17T00:00:00.000Z"
}

pub fn mock_env(vars: Dict(String, String)) -> fn(String) -> Result(String, Nil) {
  fn(key) { dict.get(vars, key) }
}
```

### Strategy 2: Dependency Injection

Pass dependencies as parameters:

```gleam
// Production code
pub type Dependencies {
  Dependencies(
    uuid_gen: fn() -> String,
    clock: fn() -> String,
    env: fn(String) -> Result(String, Nil),
  )
}

pub fn production_deps() -> Dependencies {
  Dependencies(
    uuid_gen: generate_uuid,
    clock: current_timestamp,
    env: get_env,
  )
}

pub fn do_work(deps: Dependencies, input: Input) -> Result(Output, Error) {
  let id = (deps.uuid_gen)()
  let timestamp = (deps.clock)()
  // ... rest of logic
}

// Test code
pub fn test_do_work() {
  let test_deps = Dependencies(
    uuid_gen: fn() { "test-uuid" },
    clock: fn() { "2026-01-17T00:00:00Z" },
    env: fn(_) { Error(Nil) },
  )

  let result = do_work(test_deps, test_input)
  // Assert on deterministic result
}
```

### Strategy 3: Capture-and-Replay

For stdin testing:

```gleam
pub type StdinReader {
  StdinReader(read: fn() -> Result(String, String))
}

pub fn live_stdin() -> StdinReader {
  StdinReader(read: read_line_trimmed)
}

pub fn mock_stdin(lines: List(String)) -> StdinReader {
  let mut index = 0
  StdinReader(read: fn() {
    case list.at(lines, index) {
      Ok(line) -> {
        index = index + 1
        Ok(line)
      }
      Error(_) -> Error("EOF")
    }
  })
}
```

### Strategy 4: Property-Based Testing

For caching behavior:

```gleam
// Test that cache returns same result for same input
pub fn cache_consistency_test() {
  let pattern = "test.*pattern"
  let result1 = get_or_compile_regex(pattern)
  let result2 = get_or_compile_regex(pattern)

  should.equal(result1, result2)
}

// Test that cache doesn't affect correctness
pub fn cache_correctness_test() {
  let pattern = "test.*pattern"
  let cached = get_or_compile_regex(pattern)
  let fresh = compile_regex_no_cache(pattern)

  should.equal(cached, fresh)
}
```

---

## Future Refactoring Roadmap

### Phase 1: Documentation (CURRENT)
- ✅ Document all FFI side effects in this file
- ✅ Add inline comments to FFI call sites
- ✅ Update CLAUDE.md with FFI guidelines

### Phase 2: Type System Enhancement
- [ ] Introduce `IO` effect type
- [ ] Introduce `Clock` effect type
- [ ] Introduce `Random` effect type
- [ ] Introduce `Cache` effect type
- [ ] Update function signatures to reflect effects

### Phase 3: Dependency Injection
- [ ] Create `Dependencies` type for application context
- [ ] Refactor main functions to accept dependencies
- [ ] Create production and test dependency constructors
- [ ] Update all FFI call sites to use dependencies

### Phase 4: Stateful Abstractions
- [ ] Implement `Environment` snapshot type
- [ ] Implement `RegexCache` explicit type
- [ ] Implement `StdinReader` abstraction
- [ ] Replace global state with explicit parameters

### Phase 5: Testing Infrastructure
- [ ] Create test doubles module
- [ ] Create mock constructors for all FFI functions
- [ ] Update test suite to use mocks
- [ ] Add property-based tests for cache behavior

### Phase 6: Effect System (Future)
Consider full effect system like:
- Koka-style effect handlers
- Algebraic effects
- Monad transformers (IO, Reader, State)

---

## Related Documentation

- [Gleam Type System](https://gleam.run/book/tour/type-system.html)
- [BEAM Erlang Interop](https://gleam.run/book/tour/erlang-interop.html)
- [Functional Core, Imperative Shell](https://www.destroyallsoftware.com/screencasts/catalog/functional-core-imperative-shell)
- [Effect Systems Research](https://www.microsoft.com/en-us/research/project/koka/)

---

## Questions or Issues?

If you find additional FFI functions with undocumented side effects, please:

1. Add them to this document
2. Update the catalog table
3. Provide usage examples and recommendations
4. Consider creating a bead for refactoring

---

**Last Updated:** 2026-01-17
**Maintainer:** Intent CLI Team
**Status:** Living Document
