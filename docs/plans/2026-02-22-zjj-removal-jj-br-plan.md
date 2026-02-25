# JJ Removal + JJ/BR 12-Agent Coordination Plan

## Goal

Remove all runtime reliance on `jj` while preserving deterministic orchestration with `jj` workspaces and `br` lifecycle commands. Keep pipeline replay-safe under Restate and prevent re-run loops.

## Scope

- In scope:
  - Remove `jj` command execution from runtime stages, landing, workspace prep, and gate logic.
  - Replace with explicit `jj`/`br` command contracts.
  - Keep merge queue semantics via internal queue schema + single-worker enforcement.
  - Add property tests and contract tests for queue ordering, freshness, lock TTL, and idempotency.
- Out of scope:
  - Re-adding `jj` compatibility.
  - UI-heavy dashboard (minimal CLI status only).

## Queue Schema (CUE-first contract)

Create `cue/queue_schema.cue` and validate any queue artifact before execution.

```cue
#QueueItem: {
  id: string & =~"^[a-z0-9][a-z0-9._-]{1,63}$"
  bead_id: string & =~"^[a-z0-9][a-z0-9._-]{1,63}$"
  workspace: string & =~"^[a-z0-9][a-z0-9._-]{1,63}$"
  priority: >=1 & <=10
  created_at: string
  freshness_base_rev: string & =~"^[a-f0-9]{40}$"
  deps: [...string]
  state: "queued" | "claimed" | "merging" | "done" | "failed"
}

#Lock: {
  lock_id: string
  owner: string
  resource: string
  ttl_seconds: >=5 & <=900
  acquired_at: string
  expires_at: string
}

#ConflictRecord: {
  run_id: string
  workspace: string
  bead_id: string
  strategy: "manual" | "ours" | "theirs" | "abort"
  resolved_by: string
  resolved_at: string
  notes?: string
}
```

## Type Contracts (Rust)

- `QueueItem`: invariant `priority` in `1..=10`, `freshness_base_rev` is full SHA, `deps` acyclic.
- `SessionLock`: invariant `expires_at > acquired_at`; expired lock is reclaimable and must be ignored.
- `MergeDecision`: illegal states unrepresentable:
  - `Ready(QueueItem)`
  - `Blocked { item, missing_deps }`
  - `Stale { item, head_rev, base_rev }`
  - `Conflict { item, files }`
  - `Merged { item, commit }`
- `FreshnessCheck`: result enum only (`Fresh`, `StaleNeedsRebase`, `MissingBase`).

## Property Tests to Add

1. Queue ordering is stable: for equal priority, FIFO by `created_at` then `id`.
2. Dependency safety: item cannot be selected until all deps are in `done`.
3. Rebase freshness: if `main` moves, stale item is marked `StaleNeedsRebase` before merge.
4. Lock exclusivity: two agents cannot claim same queue item concurrently.
5. Lock TTL: expired lock is reclaimable exactly once; no double-merge.
6. Merge worker single-flight: at most one `merging` item at any time.
7. Conflict trail append-only: every conflict resolution creates immutable audit record.
8. Replay idempotency: replaying same stage input produces identical queue transition output.

## Acceptance Tests (Dan North style)

- Given a queued item with no dependencies, When merge worker polls, Then item transitions to `merging` and executes `moon run :ci` before `br close`.
- Given an item blocked by dependency, When worker polls, Then it remains queued with `missing_deps` diagnostics.
- Given stale base revision, When freshness guard runs, Then item is rebased via `jj` and revalidated before merge.
- Given conflict on merge, When resolver applies strategy, Then conflict record is persisted and queue state updates deterministically.
- Given lock expiry during agent crash, When another agent polls, Then lock is reclaimed and work continues once.

## Implementation Phases

1. Remove jj branches and env flags from runtime path.
2. Introduce `jj` workspace adapter and queue domain types.
3. Implement deterministic merge worker + lock manager.
4. Wire freshness guard and conflict recorder into landing path.
5. Add CUE schema validation + property/acceptance test suites.
6. Validate with `moon run :test` and `moon run :ci`.

## Files Expected to Change

- `src/main.rs`
- `src/pipeline/mod.rs`
- `src/pipeline/executor.rs`
- `src/runtime_tools/workspace.rs`
- `src/runtime_tools/gates.rs`
- `src/types/pipeline.rs`
- `src/lib.rs`
- `src/lib_tests.rs`
- `src/main/tests.rs`
- `tests/gates.rs`
- `tests/contract_verify.rs`
- `scripts/run_bead_pipeline.rs`
- `cue/queue_schema.cue` (new)
