# Rust Standards

- **Safety first**: `#![forbid(unsafe_code)]`, `#![deny(clippy::unwrap_used)]`, `#![deny(clippy::expect_used)]`, `#![deny(clippy::panic)]`.
- **Typed boundaries**: parse/validate external input at boundaries (`RepoSlug`, `Model`, request payloads).
- **Error discipline**: represent failures with explicit domain types (`LifecycleError`, `FailureCategory`, `FailureClass`).
- **Workflow determinism**: validate lifecycle DAGs before executing effects.
- **Verification**: run checks through moon tasks (`moon run :test`, `moon run :ci`).
