# Build Pipeline: Moon

## Absolute Verification
To ensure no cached success masks a subtle regression, always run:
```bash
moon run :ci --force
```

## Quality Gates

`moon run :ci --force` runs the following gates in order:

| Gate | Command | What it checks |
|------|---------|----------------|
| **secrets** | `./scripts/scan-secrets.sh` | Accidental API keys, tokens, passwords |
| **fmt** | `cargo fmt --check --quiet` | Code formatting (rustfmt) |
| **check** | `cargo check` | Compilation (no codegen) |
| **test** | `cargo test` | Unit tests, integration tests, doc tests |
| **clippy** | `cargo clippy -- -D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic` | Linting (zero-panic policy) |
| **e2e** | `node_modules/.bin/playwright test` | Browser E2E tests (Playwright) |

### Individual Gates

Run any gate independently:
```bash
moon run :fmt          # Format check only
moon run :check        # Compilation only
moon run :test         # Tests only
moon run :clippy       # Lint only
moon run :build-web    # WASM production build
moon run :coverage     # Code coverage report
```

### Zero-Panic Policy

Clippy enforces `deny(unwrap_used)`, `deny(expect_used)`, and `deny(panic)` in CI.
Production code must use `Result<T, E>` with combinators. Test code lifts these with:
```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
```

### Not Yet in CI

| Gate | Status | Notes |
|------|--------|-------|
| Fuzz testing | Not configured | Would need `cargo-fuzz` + targets |
| Mutation testing | Not configured | Would need `cargo-mutants` setup |
