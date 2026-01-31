# Building & Testing with Moon

Moon is the build orchestration tool. It caches tasks, parallelizes execution, and ensures consistency.

## The Rule

**ALWAYS use Moon. NEVER use cargo directly.**

```bash
✅ moon run :ci      # Correct
✅ moon run :test    # Correct
❌ cargo build       # Wrong
❌ cargo test        # Wrong
```

## Common Commands

```bash
# Full CI pipeline
moon run :ci

# Fast lint only
moon run :quick

# Tests only
moon run :test

# Build release binaries
moon run :build

# Deploy (CI + Windmill push)
moon run :deploy
```

## What Each Command Does

### `moon run :ci` (Full Pipeline)

```
fmt check       ← Code formatting check
clippy          ← Lints (-D warnings)
validate        ← YAML validation
test            ← Tests (parallel with nextest)
build           ← Release build
copy            ← Copy binaries to bin/
```

**Duration**: ~60-120 seconds (first run slower due to compilation)

### `moon run :quick` (Fast Check)

```
fmt check       ← Code formatting
clippy          ← Lints
```

**Duration**: ~10-15 seconds

**Use**: Before pushing or committing

### `moon run :test`

```
test            ← Run all tests (parallel)
```

**Duration**: ~30-45 seconds

### `moon run :build`

```
build           ← Release build
copy            ← Copy binaries
```

**Duration**: ~45-90 seconds

## Caching & Speed

Moon caches based on **input fingerprints**:
- First build: **slow** (full compilation)
- After change to a file: **fast** (only recompile affected)
- No changes: **instant** (cached)

**Speedups via**:
- mold (fast linker)
- sccache (compiler cache)
- nextest (parallel test runner)
- incremental builds

## Project Structure

```
.moon/
├── workspace.yml       # Workspace config
├── toolchain.yml       # Rust version (nightly)
└── bin/                # Moon binaries

moon.yml                 # Task definitions
Cargo.toml               # Workspace root
rust-toolchain.toml      # Rust version
```

## Workflow Integration

### Before Pushing

```bash
# Local validation
moon run :quick         # Fast lint check (~10s)

# If changes to logic
moon run :test          # Run tests (~30s)

# Final validation
moon run :ci            # Full pipeline (~2min)

# Push
jj git push
```

### In CI/CD

```bash
# CI runs full pipeline
moon run :ci

# All checks pass before merge
```

## Troubleshooting

### "moon not found"

Ensure Moon is installed:
```bash
# Check if in PATH
which moon

# Or use brew
brew install moonrepo/tools/moon
```

### "sccache not found"

Compiler cache not installed:
```bash
# Install via mise
mise install

# Or manually
cargo install sccache
```

### "Task failed"

Check which task:
```bash
# Run with debug logging
moon run :ci --log debug

# Run single task
moon run :test

# Check task definition
cat moon.yml
```

### "Cache not working"

Check inputs are correct:
```bash
# View task definition
moon dump :ci

# Check last run
ls -la ~/.moon/cache
```

## Performance Metrics

### Typical Times

| Command | First Run | Cached |
|---------|-----------|--------|
| `:quick` | 15s | 5s |
| `:test` | 45s | 25s |
| `:build` | 90s | 45s |
| `:ci` | 120s | 60s |

### Optimization

- Change minimal files → faster rebuild
- Use `:quick` for frequent checks
- Run `:test` before `:build`
- Let caching work (reuse outputs)

## Configuration

### `.moon/workspace.yml`

```yaml
workspace:
  version: "1.20"
  generator:
    templates:
      - .moon/templates
```

### `.moon/toolchain.yml`

```yaml
rustup:
  version: "nightly"
```

### `moon.yml`

Task definitions. Never edit directly unless you know what you're doing.

## Build Profiles

### Release (moon run :build)

- Optimization level: 3 (maximum)
- Debug info: stripped
- Panic: abort (smaller binary)
- LTO: enabled
- Code gen units: 1

### Debug (development)

- Optimization level: 0
- Debug info: included
- Panic: unwind
- Incremental: enabled

### Test

- Optimization level: 1
- Debug assertions: enabled
- Panic: unwind

## Binaries

All binaries in `crates/*/src/bin/` are built:

```
crates/zjj-core/src/bin/
├── example1.rs
├── example2.rs
└── ...
```

Built to: `target/release/`

## Integration with CI

Moon runs in CI with:
```bash
moon ci :build
```

This uses cached outputs when available and checks all dependencies.

## Common Patterns

### Before Committing

```bash
# Quick lint
moon run :quick

# If satisfied, commit
jj describe -m "feat: description"
```

### Making Changes

```bash
# Change a file
vim crates/zjj-core/src/lib.rs

# Test
moon run :test

# If green, continue. If red, fix.
```

### Before Pushing

```bash
# Full pipeline
moon run :ci

# If all pass
jj git push

# If any fail, fix and retry
moon run :ci
```

## Exit Codes

- `0` - All tasks passed
- `1` - At least one task failed

Check `moon.yml` for task definitions and dependencies.

## Advanced

### Run specific task

```bash
moon run :test --scope zjj-core
```

### Watch mode (experimental)

```bash
moon watch :test
```

### Dry run

```bash
moon run :test --dry-run
```

## The Philosophy

Moon exists to:
1. **Cache** - Skip unchanged work
2. **Parallelize** - Run independent tasks simultaneously
3. **Consistency** - Same commands work locally and in CI
4. **Transparency** - Know what's being built and why

Use it religiously. Never bypass it with direct cargo commands.

---

**Next**: [Daily Workflow](03_WORKFLOW.md)
