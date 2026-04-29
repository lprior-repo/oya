# OYA Restate Hourly Bead Plan

Planning session: `oya-restate-cd-hourly`

Execution rule for every bead:
1. SCOUT
2. ATDD (hidden tests)
3. RED (minimal failing stub)
4. IMPLEMENT
5. REVIEW (`moon run :quick`, `moon run :test`, `moon run :check`)
6. COMMIT

## Lane A - Core Reliability (start here)

1. `src-1si` - lifecycle state machine contract
2. `src-2f0` - durable event ledger and tail visibility
3. `src-2s0` - single-writer lease + idempotency
4. `src-1gw` - stage/run watchdog fail-fast
5. `src-31a` - provider rotation all stages
6. `src-2ey` - revision-pinned moon evidence
7. `src-1l0` - cleanup reconciler + cleanup_pending
8. `src-198` - release-readiness soak gate

## Lane B - VCS + Saga + Stack/Train Parity

9. `src-394` - dual logical change identity (`jj change_id` + git fallback)
10. `src-2bi` - provision saga compensation
11. `src-1n5` - land saga checkpoint + cleanup handoff
12. `src-16d` - stack model (parent/child blocked/ready)
13. `src-1ma` - auto child refresh/rebase on parent/main movement
14. `src-2zb` - dependency-aware merge train scheduling

## Lane C - Operations and Guardrails

15. `src-3m1` - main drift monitor and land blocking
16. `src-14t` - minimal operator commands (`status`, `tail`, `doctor`)
17. `src-1do` - doctor filters (`stuck`, `failed`, `cleanup_pending`)
18. `src-3hu` - provider pool health/cooldown/exhausted handling

## Policy Notes

- Fail-forward only: no `git revert`, no `jj revert` in orchestration flows.
- Main must remain releasable: gate all land actions on green moon checks.
- Internal queue/sync/rebase/merge/cleanup are workflow steps, not user-facing commands.
