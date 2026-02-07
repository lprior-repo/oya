# Functional State Transformation - OYA Zellij Plugin

## Summary

Successfully transformed the Zellij UI state management from imperative to functional pattern while maintaining compatibility with the Zellij trait's `&mut self` requirement.

## Architecture Pattern: Exterior Mutability

### Why This Pattern?

The `zellij_tile::Plugin` trait requires methods with `&mut self` signatures:
```rust
fn update(&mut self, event: Event) -> bool;
fn render(&mut self, rows: usize, cols: usize);
```

To maintain functional purity internally while satisfying this interface, we use the **exterior mut pattern**:

```rust
// Pure functional handler - returns new state
fn handle_event(self, event: Event) -> (Self, bool) {
    match event {
        Event::Timer(_) => self.handle_timer_event(),
        // ...
    }
}

// Zellij trait implementation uses exterior mut
impl zellij_tile::Plugin for State {
    fn update(&mut self, event: Event) -> bool {
        let (new_state, should_render) = std::mem::replace(self, State::default())
            .handle_event(event);
        *self = new_state;
        should_render
    }
}
```

## Key Transformations

### 1. Load Methods: Imperative → Functional

#### BEFORE: Imperative with `&mut self`
```rust
fn load_beads(&mut self) {
    if let Some((cached_beads, timestamp)) = &self.beads_cache {
        if timestamp.elapsed() < CACHE_TTL {
            self.beads = cached_beads.clone();
            return;
        }
    }

    let url = format!("{}/api/beads", self.server_url);
    let mut context = BTreeMap::new();
    context.insert(CTX_REQUEST_TYPE.to_string(), CTX_BEADS_LIST.to_string());
    self.pending_requests = self.pending_requests.saturating_add(1);
    self.last_request_sent = Some(Instant::now());
    web_request(&url, HttpVerb::Get, BTreeMap::new(), vec![], context);
}
```

#### AFTER: Functional with `self` by value
```rust
fn load_beads(mut self) -> (Self, Result<()>) {
    if let Some((cached_beads, timestamp)) = &self.beads_cache {
        if timestamp.elapsed() < CACHE_TTL {
            self.beads = cached_beads.clone();
            return (self, Ok(()));
        }
    }

    let url = format!("{}/api/beads", self.server_url);
    let mut context = BTreeMap::new();
    context.insert(CTX_REQUEST_TYPE.to_string(), CTX_BEADS_LIST.to_string());
    self.pending_requests = self.pending_requests.saturating_add(1);
    self.last_request_sent = Some(Instant::now());
    web_request(&url, HttpVerb::Get, BTreeMap::new(), vec![], context);
    (self, Ok(()))
}
```

**Key Changes:**
- Takes `self` by value instead of `&mut self`
- Returns `(Self, Result<()>)` tuple for explicit state transformation
- Early returns use `return (self, Ok(()))` pattern
- `mut self` binding allows local mutations before returning

### 2. Trigger Methods: Chaining Support

#### BEFORE: Consuming but not returning
```rust
fn trigger_beads_load(mut self) {
    self.load_beads();  // BUG: Consumes self, doesn't return it!
}
```

#### AFTER: Proper functional chaining
```rust
fn trigger_beads_load(self) -> Self {
    self.load_beads().0
}
```

**Key Changes:**
- Returns `Self` for method chaining
- Unwraps the tuple from `load_beads()`
- Enables call-site chaining: `self = self.trigger_beads_load();`

### 3. Event Handlers: Explicit State Chaining

#### BEFORE: Broken state management
```rust
fn handle_timer_event(mut self) -> (Self, bool) {
    // ... timeout logic ...

    self.trigger_beads_load();  // MOVES self!

    if self.mode == ViewMode::AgentView {
        self.trigger_agents_load();  // ERROR: use of moved value
    }

    set_timeout(2.0);
    (self, true)  // COMPILER ERROR: self was moved
}
```

#### AFTER: Explicit state transformations
```rust
fn handle_timer_event(mut self) -> (Self, bool) {
    // ... timeout logic ...

    // Each trigger returns the updated state
    self = self.trigger_beads_load();

    if self.mode == ViewMode::AgentView {
        self = self.trigger_agents_load();
    }

    set_timeout(2.0);
    (self, true)  // OK: self is now the final state
}
```

**Key Changes:**
- Explicit `self = ` at each transformation step
- No hidden mutations - all state changes are visible
- Compiler verifies no use-after-move bugs

## Benefits

### 1. Explicit State Transformations
Every state change is visible in the function signature:
```rust
fn load_beads(self) -> (Self, Result<()>)  // Returns new state explicitly
```

### 2. No Hidden Mutations
All transformations consume old state and produce new state:
```rust
self = self.trigger_beads_load();  // Clear ownership transfer
```

### 3. Easy to Test
Pure functions are trivial to unit test:
```rust
#[test]
fn test_load_beads_with_valid_cache() {
    let state = State {
        beads_cache: Some((Vector::new(), Instant::now())),
        ..State::default()
    };

    let (new_state, result) = state.load_beads();
    assert!(result.is_ok());
    // Assert new_state has expected properties
}
```

### 4. Structural Sharing
`im::Vector` and `im::HashMap` provide efficient cloning:
```rust
self.beads = cached_beads.clone();  // O(1) structural sharing
```

## Side Effects Note

The `web_request()` calls are I/O side effects. While we can't make these pure, we structure the code to separate state transformation from I/O:

```rust
fn load_beads(mut self) -> (Self, Result<()>) {
    // 1. Pure: Check cache, return early if valid
    if let Some((cached, _)) = &self.beads_cache {
        if timestamp.elapsed() < CACHE_TTL {
            return (self, Ok(()));  // Pure transformation
        }
    }

    // 2. Side effect: Perform web request
    web_request(&url, HttpVerb::Get, ...);

    // 3. Pure: Return new state
    (self, Ok(()))
}
```

This makes the state transformation explicit even though I/O occurs.

## Complete Method Transformations

All load methods converted:
- `load_beads(&mut self)` → `load_beads(self) -> (Self, Result<()>)`
- `load_pipeline_for_selected(&mut self)` → `load_pipeline_for_selected(self) -> (Self, Result<()>)`
- `load_agents(&mut self)` → `load_agents(self) -> (Self, Result<()>)`
- `load_graph(&mut self)` → `load_graph(self) -> (Self, Result<()>)`
- `load_system_health(&mut self)` → `load_system_health(self) -> (Self, Result<()>)`
- `load_log_aggregator(&mut self)` → `load_log_aggregator(self) -> (Self, Result<()>)`

All trigger methods updated:
- `trigger_beads_load(mut self)` → `trigger_beads_load(self) -> Self`
- `trigger_pipeline_load(mut self)` → `trigger_pipeline_load(self) -> Self`
- `trigger_agents_load(mut self)` → `trigger_agents_load(self) -> Self`
- `trigger_graph_load(mut self)` → `trigger_graph_load(self) -> Self`
- `trigger_system_health_load(mut self)` → `trigger_system_health_load(self) -> Self`
- `trigger_log_aggregator_load(mut self)` → `trigger_log_aggregator_load(self) -> Self`

## Verification

```bash
cargo check -p oya-zellij --target wasm32-wasip1
```

Result: ✅ Compiles successfully with only warnings (21 warnings, 0 errors)

All warnings are for unused code (dead code analysis), not logic errors.

## Files Modified

- `/home/lewis/src/oya/crates/oya-zellij/src/lib.rs`
  - Added comprehensive documentation explaining exterior mut pattern
  - Converted 6 load methods from `&mut self` to `self -> (Self, Result<()>)`
  - Converted 6 trigger methods to return `Self`
  - Updated all call sites to use explicit state chaining
  - Fixed test bug (line 1961: `self` → `state`)

## Next Steps

The functional state transformation is complete. The code now:
- ✅ Uses pure functional state management internally
- ✅ Maintains Zellij trait compatibility via exterior mut pattern
- ✅ Provides explicit state transformations
- ✅ Eliminates hidden mutation bugs
- ✅ Compiles without errors

Future enhancements could:
- Add integration tests for state transformation chains
- Profile performance of im::Vector structural sharing
- Consider adding `#[must_use]` to trigger methods to prevent accidental state drops
