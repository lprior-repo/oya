# Stream D: Process Pool + Workspace Isolation

## Goal

Run OpenCode reliably as subprocess workers and execute bead work in isolated zjj workspaces.

## Canonical Decisions

- OpenCode is invoked as a subprocess adapter per stage attempt.
- Worker health is explicit and reconciled automatically.
- Workspace isolation is zjj-first and retained.
- Sticky assignment metadata persists in Sled-backed runtime state.

## Bead Set (6)

| Task | Title | Bead ID | Effort | Priority |
|---|---|---|---|---|
| task-001 | process-pool: Implement ProcessPoolActor with subprocess management | `intent-cli-20260201014712-70yhgsj1` | 4hr | 1 |
| task-002 | opencode: Implement subprocess wrapper and stream handling | `intent-cli-20260201014712-nirdcff7` | 4hr | 1 |
| task-003 | process-pool: Implement heartbeat monitoring for dead worker detection | `intent-cli-20260201014713-y9wal8p7` | 2hr | 1 |
| task-004 | zjj: Implement WorkspaceManager for isolated workspaces | `intent-cli-20260201014713-6wwfbzye` | 4hr | 1 |
| task-005 | worker: Integrate WorkspaceManager with BeadWorkerActor | `intent-cli-20260201014713-mgcop1nx` | 2hr | 1 |
| task-006 | sticky: Implement sticky worker assignment with soft/hard modes | `intent-cli-20260201014713-j3tktekl` | 4hr | 2 |

## Operational Flow

1. Claim worker from pool.
2. Create zjj workspace for bead attempt.
3. Invoke OpenCode subprocess with stage context packet.
4. Parse output and persist attempt result.
5. Release worker and close workspace by policy.

## Quality and Validation

- Worker lifecycle tests include crash and timeout behavior.
- Workspace cleanup is validated on success and failure paths.
- Run with `moon run :quick` and `moon run :ci`.

## Success Criteria

- No zombie subprocesses after shutdown.
- Workspace isolation remains conflict-free.
- Retry and sticky assignment behavior remains deterministic.
