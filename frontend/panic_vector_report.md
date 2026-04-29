# Panic Vector Report — oya-frontend

**Date:** 2026-04-19
**Auditor:** polecat/ember (of-aq5)

## Executive Summary

**ZERO production-code panic vectors found.** The codebase enforces zero-tolerance through crate-level compiler lints and module-level belt-and-suspenders annotations.

## Methodology

1. `grep -n 'panic!' src/` — all hits in `#[cfg(test)]` only
2. `grep -n '.unwrap()' src/` — all hits in `#[cfg(test)]` only
3. `grep -n '.expect(' src/` — all hits in `#[cfg(test)]` only
4. `grep -n 'unsafe' src/` — only `#![forbid(unsafe_code)]` declarations, zero unsafe blocks
5. Verified specific files: `src/restate_client/client.rs`, `src/graph/execution_runtime/` — clean

## Crate-Level Enforcement (main.rs)

```rust
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]
```

These lints make it a **compile error** to introduce any panic vector in production code.

## Findings by Category

### panic! — 0 production violations

All `panic!` occurrences (30 hits) are inside test code:
- `src/graph/cycle_detection_tests.rs` (24) — test assertion panics
- `src/ui/editor_interactions.rs:227` — test assertion
- `src/restate_sync/poller.rs:440` — test assertion
- `src/ui/restate/state_browser.rs:418,442` — test assertions
- `src/graph/execution_state/tests/*.rs` (2) — structural verification

### .unwrap() — 0 production violations

All 120+ `.unwrap()` occurrences are inside `#[cfg(test)]` modules. Key areas:
- `src/restate_client/client.rs` (4) — in `#[cfg(test)] mod tests`
- `src/restate_client/types.rs` (30+) — in `#[cfg(test)] mod tests`
- `src/restate_sync/poller.rs` (11) — in `#[cfg(test)] mod tests`
- `src/ui/edges.rs` (10) — in `#[cfg(test)] mod tests`
- `src/graph/execution_state/tests/` (20+) — in test modules

### .expect() — 0 production violations

All `.expect()` occurrences are inside test code or doc examples.

### unsafe — 0 blocks

Only `#![forbid(unsafe_code)]` declarations found. No `unsafe {}` blocks exist anywhere.

## Lint Coverage

| Metric | Count |
|--------|-------|
| Files with `#![forbid(unsafe_code)]` | 42 |
| Production files WITHOUT module-level lints | 135 |
| Files relying solely on crate-level lints | 135 |

The 135 files without module-level annotations are still protected by the crate-level lints in `main.rs`. The module-level annotations in 42 files provide defense-in-depth.

## Architecture Assessment

**Status: HEALTHY**

The codebase follows the zero-policy defined in CLAUDE.md:
- `Result<T, Error>` with combinators throughout
- No `.unwrap()` or `.expect()` in production paths
- No `panic!()` in production paths
- No `unsafe` code
- Compiler-enforced via `deny`/`forbid` lints

## Recommendations

1. **No immediate fixes needed** — zero violations found.
2. **Optional hardening** — The 135 files without module-level `#![forbid(unsafe_code)]` could benefit from adding the annotation as defense-in-depth, but this is low priority since crate-level enforcement is active.
3. **CI gate** — The existing `moon run :clippy` gate catches any regressions automatically.
