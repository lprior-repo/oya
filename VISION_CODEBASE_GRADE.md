# Vision + Codebase Gradecard (Hostile Review)

## Executive Verdict
You have a **powerful narrative** and a **high-ambition architecture**, but the current repository behaves like a research lab with production branding. The gap between stated discipline and enforceable discipline is still large.

**Overall grade: B- (6.4/10)**

- Vision clarity: **A-**
- Architectural coherence: **B**
- Delivery realism: **C+**
- Functional-Rust adherence (as enforced): **C**
- Testing discipline (Fowler-style behavioral confidence): **B-**
- Operational/tooling reliability: **D+**

---

## What is strong (and worth keeping)

1. **Vision is crisp and differentiated.** The transformation + gatekeeping metaphor is specific, memorable, and maps to engineering principles like strict quality gates and explicit error handling.
2. **You explicitly define non-negotiables.** The workspace lints encode a zero-panic/zero-unwrap posture at policy level.
3. **You already think in behavior-first testing language.** The Fowler test-plan docs emphasize Given-When-Then and failure-path coverage.
4. **There is significant test investment.** The codebase contains a very high number of tests and broad scenario coverage scaffolding.

---

## Where the story and reality conflict

### 1) Vision/architecture drift
Your public docs describe a 9-part architecture and a unified CLI story, but the workspace membership and crate reality do not line up cleanly. Several components are still TODO, and `oya-cli` is referenced via workspace dependency while `crates/oya` lacks its own Cargo manifest.

**Why this matters:** strategy drift is liability. Every mismatch between narrative and executable structure increases onboarding, planning, and maintenance cost.

### 2) Enforcement is not trustworthy yet
You claim hard workflow requirements (`swarm`, `bv`, `moon`, `jj`), but in this environment those primary commands are unavailable. If your process cannot execute in a fresh environment, then your process is documentation theater.

**Why this matters:** process that cannot run cannot protect quality.

### 3) Functional purity posture is inconsistently evidenced
You have strict lint policy declarations, but repo-wide static scan still shows substantial `unwrap`/`expect`/`panic` surface area (many are in tests, some mixed in non-test trees and examples).

**Why this matters:** if you want a zero-panic brand, every exception should be intentional and mechanically scoped.

### 4) Test volume is high; behavioral signal quality is uneven
You have thousands of tests, but there is evidence of plan-heavy and report-heavy testing artifacts mixed with code. That can become “test theater” unless tied directly to executable, stable behavioral contracts and mutation kill-rates.

**Why this matters:** more tests do not equal more confidence; precise behavioral tests do.

---

## What must change (priority order)

## P0 — Stop lying to yourselves (process hardening)
1. **Create a bootstrap check script** (`scripts/doctor-ci.sh`) that validates availability and versions of mandatory tools (`swarm`, `bv`, `moon`, `jj`, `zjj`) and fails fast.
2. **Gate docs claims on executable checks.** If `moon`/`jj` is required, CI must fail when they are absent.
3. **Add a “minimum reproducible dev env” doc with one command install path** and smoke test command.

## P1 — Make architecture true
1. **Reconcile architecture docs with actual workspace members.** If components are aspirational, mark them explicitly as roadmap, not current architecture.
2. **Fix `oya-cli` packaging mismatch** (either add `crates/oya/Cargo.toml` and workspace membership, or remove stale dependency references).
3. **Publish one canonical system map** (single source of truth) and delete duplicate/contradictory architecture markdown.

## P2 — Make functional claims enforceable
1. **Separate production vs test lint posture clearly and universally.** Keep strict production lints, allow scoped test-only exceptions with explicit module boundaries.
2. **Add automated trend reporting** for unwrap/expect/panic counts by path category (`src`, `tests`, `examples`, `benches`) so direction is measurable.
3. **Eliminate remaining production-scope panic pathways first** (prioritize orchestrator/core/events critical paths).

## P3 — Upgrade testing to Fowler-grade behavioral confidence
1. **Create a test taxonomy by intent:**
   - Characterization tests (legacy behavior lock)
   - Business behavior tests (Given-When-Then)
   - Contract tests (module/API boundaries)
   - Property tests (invariants)
   - Mutation tests (assertion quality)
2. **Add requirement-to-test traceability**: every critical behavior has a stable ID mapped to executable tests.
3. **Define confidence KPIs** beyond coverage:
   - Mutation score threshold (per critical crate)
   - Flake-rate SLO
   - Mean test runtime budget
   - Escaped defect rate by layer

## P4 — Code is liability: reduce surface area
1. **Delete stale test scripts/reports not used by CI.** Archive out of repo or regenerate on demand.
2. **Consolidate duplicated docs** (multiple QA reports and plans should collapse to current + changelog).
3. **Add module-level “why this exists” docs** and delete dead prototypes/examples that do not serve current product goals.

---

## Hostile reviewer findings you should act on this week

1. **Toolchain commands in your own process docs are not runnable in this environment** (`swarm`, `bv`, `moon`, `jj` missing).
2. **Workspace dependency graph includes `oya-cli` path while crate manifest is missing.**
3. **Policy says zero panic/unwrap, but static scan still reports substantial occurrences repo-wide.**
4. **You have many tests, but no visible single confidence dashboard tying behavior requirements to mutation quality and flake/risk.**

---

## 30-day remediation plan

### Week 1: Truth alignment
- Implement bootstrap doctor script + CI gate.
- Resolve `oya-cli` manifest/dependency mismatch.
- Publish one canonical architecture status page (current vs planned).

### Week 2: Functional integrity
- Add panic/unwrap budget report by crate and path class.
- Burn down production-path violations in top 3 critical crates.

### Week 3: Fowler testing systemization
- Introduce behavior IDs + trace matrix.
- Add mutation testing thresholds for `oya-core`, `orchestrator`, `oya-events`.

### Week 4: Liability burn-down
- Delete or archive stale reports/scripts not in CI.
- Enforce doc freshness checks (last-verified metadata).

---

## Bottom line
The vision is strong enough to build a category-defining engineering system. But right now the repository still carries too much entropy and too many unenforced claims. 

If you want “storm + gatekeeper” to be real, convert every major promise into **machine-checked truth** and delete everything that does not increase behavioral confidence.
