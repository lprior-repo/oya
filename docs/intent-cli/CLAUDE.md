# CLAUDE.md - Project Instructions for Claude Code

## Critical Rules

### ABSOLUTE PROHIBITION: NEVER TOUCH CLIPPY/LINT CONFIGURATION

**THIS IS A HARD BLOCK. THERE ARE NO EXCEPTIONS. EVER.**

**YOU ARE FORBIDDEN FROM:**
- Editing `.clippy.toml`
- Editing `clippy.toml`
- Editing ANY `#![allow(...)]` or `#![deny(...)]` attributes in `lib.rs` or `main.rs`
- Editing Clippy-related sections in `Cargo.toml`
- Editing ANY lint configuration in `moon.yml` or build scripts
- Creating workarounds, wrappers, or alternatives to avoid lint rules
- Suggesting changes to lint configuration

**IF CLIPPY REPORTS WARNINGS OR ERRORS:**
→ FIX THE CODE, NOT THE LINT RULES
→ The user has explicitly configured these rules
→ Do not second-guess them
→ Do not suggest "temporarily disabling" rules
→ Do not add `#[allow(...)]` attributes to individual functions

**VIOLATION OF THIS RULE WILL RESULT IN IMMEDIATE SESSION TERMINATION.**

---

## Version Control: JJ Only (NO Git Commands)

**NEVER use raw git commands.** Always use jj (Jujutsu) for all VCS operations:

```bash
# Correct - use jj
jj status              # See working copy changes
jj diff                # See changes
jj new                 # Create new change
jj commit -m "msg"     # Describe and close current change
jj describe -m "msg"   # Update current change description
jj log                 # View history
jj bookmark set main   # Set bookmark
jj git push            # Push to remote

# WRONG - Never do this
git status             # NO
git add                # NO
git commit             # NO
git push               # NO
```

---

## Build System: Moon Only (Local)

**NEVER use raw cargo commands.** Always use Moon for all build operations:

```bash
# Correct
moon run :quick      # Format + lint check
moon run :test       # Run tests
moon run :build      # Release build
moon run :ci         # Full pipeline
moon run :fmt-fix    # Auto-fix formatting
moon run :check      # Fast type check

# WRONG - Never do this
cargo fmt            # NO
cargo clippy         # NO
cargo test           # NO
cargo build          # NO
```

---

## Code Quality: Zero Unwrap Philosophy

- Zero unwraps: `unwrap()` and `expect()` are forbidden
- Zero panics: `panic!`, `todo!`, `unimplemented!` are forbidden
- All errors must use `Result<T, Error>` with proper propagation
- Use functional patterns: `map`, `and_then`, `?` operator

### Functional Programming Principles

1. **Immutability First**: Use `im` crate for persistent data structures
2. **Railway-Oriented Programming**: Chain operations with `map`, `and_then`, `?`
3. **No Side Effects in Pure Functions**: Isolate IO at the edges
4. **Type-Driven Design**: Make invalid states unrepresentable
5. **Exhaustive Pattern Matching**: Handle all cases explicitly
6. **Composition Over Inheritance**: Build complex from simple
7. **Higher-Order Functions**: Use closures and function parameters

### Error Handling Patterns

```rust
// GOOD - Railway-oriented
fn process(input: &str) -> Result<Output> {
    input
        .parse::<Config>()
        .map(|cfg| cfg.validate())
        .and_then(|cfg| cfg.process())
}

// BAD - Panic-prone
fn process(input: &str) -> Output {
    let cfg = input.parse::<Config>().unwrap();  // FORBIDDEN
    cfg.process()
}
```

---

## Project Structure

```
crates/
  intent-core/    # Core library (error handling, types, functional utils)
  intent/         # CLI binary
docs/
  rust/           # Rust documentation and patterns
```

---

## Quick Reference

### Issue Tracking (Beads)

**MANDATORY: All beads MUST use the Enhanced Template**

When creating beads, you MUST include ALL 10 sections from `.beads/BEAD_TEMPLATE.md`:

1. **EARS Requirements** - THE SYSTEM SHALL patterns
2. **KIRK Contracts** - Preconditions, postconditions, invariants
3. **Inversion Analysis** - What could go wrong + prevention
4. **ATDD Tests** - Real input/output, no mocks
5. **E2E Tests** - Full pipeline test with real data
6. **Implementation Tasks** - Phase 1: Tests first, Phase 2: Implementation
7. **Failure Modes** - Symptoms, causes, where to look
8. **Completion Checklist** - All checkboxes for done
9. **Context** - Related files, similar implementations
10. **AI Hints** - Do/don't patterns

**Validation**:
```bash
.beads/validate-bead.sh <bead-id>  # Validate a bead
```

**Creating Enhanced Beads**:
```bash
# Write description to file first
vim /tmp/bead-desc.md

# Create with validation
.beads/bd-create-enhanced.sh \
  --title "Component: Description" \
  --description-file /tmp/bead-desc.md \
  --type feature \
  --priority 2
```

**Quick Reference**:
```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --status in_progress  # Claim work
bd close <id>         # Complete work
bd sync               # Sync with git
```

### Development (Moon CI/CD - Local Only)
```bash
moon run :quick       # Fast checks (6-7ms with cache!)
moon run :ci          # Full pipeline (parallel)
moon run :fmt-fix     # Auto-fix formatting
moon run :build       # Release build
moon run :install     # Install to ~/.local/bin
```

### Version Control (JJ)
```bash
jj status             # Working copy status
jj diff               # See changes
jj new -m "message"   # New change with message
jj commit             # Close current change
jj git push           # Push to remote
```
