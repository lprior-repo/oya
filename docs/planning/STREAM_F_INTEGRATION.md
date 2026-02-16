# Stream F: Integration + Chaos Testing

## Goal

Prove production-readiness of the governed pipeline through integration, chaos, performance, and load validation.

## Scope

- Integrate Streams A, B, C, and D.
- Validate restart/recovery behavior under orchestrator and worker failures.
- Verify performance and memory stability targets.
- No UI dependencies in this stream.

## Bead Set (6)

| # | Bead ID | Title | Priority | Effort |
|---|---|---|---:|---|
| 1 | `intent-cli-20260201020059-jjbgksde` | integration: Implement orchestrator initialization and graceful shutdown | 1 | 4hr |
| 2 | `intent-cli-20260201020059-mahvrqrz` | integration: End-to-end bead execution integration tests | 1 | 4hr |
| 3 | `intent-cli-20260201020339-ykctqedr` | chaos: Implement chaos testing framework with 6 scenarios | 1 | 4hr |
| 4 | `intent-cli-20260201020339-tou8kwbh` | perf: Implement performance benchmarks with criterion | 2 | 2hr |
| 5 | `intent-cli-20260201020059-jonmp2v0` | perf: Implement load testing with 100 concurrent beads | 1 | 4hr |
| 6 | `intent-cli-20260201020339-fauedab9` | perf: Memory profiling and leak detection | 1 | 2hr |

## Key Validation Scenarios

- Worker crash mid-stage attempt and controlled retry.
- Orchestrator restart with checkpoint resume.
- Sled write failures and recovery behavior.
- zjj workspace cleanup guarantees across failure paths.

## Gates

- `moon run :quick` for local iteration.
- `moon run :ci` for stream completion checks.
- `moon run :ci --force` for release-confidence validation.

## Success Criteria

- End-to-end pipeline behavior is deterministic and auditable.
- Chaos scenarios recover without silent corruption.
- Load and perf targets are measured and recorded.
- No memory leak regression in sustained execution tests.
