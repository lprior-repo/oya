# Intent CLI - Clean Execution Plan (Beads Sorted)
**Date**: 2026-01-25
**Status**: ✅ Sorted and Planned
**Total Active Beads**: 21 (after closing 11 non-bugs)

---

## Quick Reference - Start Here

### Top 3 Priority 0 (Foundation Work)
1. **intent-cli-wb2** - Create comprehensive test suite
2. **intent-cli-z5s** - Implement vision command (CRITICAL - only real bug)
3. **intent-cli-6oy** - CLI Dogfood & AI Ergonomics Assessment

### What to Do First

**Option A: Test Foundation** (Recommended - enables all validation)
```bash
bd update intent-cli-wb2 --status in_progress --json
```

**Option B: Fix Critical Bug** (Quick win - 2-3 hours)
```bash
bd update intent-cli-z5s --status in_progress --json
```

**Option C: Close Duplicates** (Quick cleanup - 5 minutes)
- 12 "FIX:" beads are duplicates of 12 unique bug beads
- Close them after fixing the corresponding unique bead

---

## All Beads Sorted by Priority

### Priority 0 (Critical - Foundation Work)
**3 beads | Estimated: 2-3 days | Dependencies: None | Parallel: YES**

| Bead ID | Title | Est. Time | Description |
|----------|-------|-----------|-------------|
| intent-cli-wb2 | Create comprehensive test suite to validate all CLI commands systematically | 1 day | Build test infrastructure, validate all 33 commands |
| intent-cli-z5s | Implement vision command integration | 2-3 hours | Add vision CLI wrappers, register in intent.gleam |
| intent-cli-6oy | Comprehensive CLI Dogfood & AI Ergonomics Assessment - Parallel 24-Agent Testing | 1-2 days | Framework for testing all commands systematically |

**Why First:**
- Cannot validate fixes without test suite
- Vision command blocks core functionality
- Dogfood framework enables AI agent support
- All other work depends on these

---

### Priority 1 (High - Features & Improvements)
**13 beads | Estimated: 2-3 days | Dependencies: None | Parallel: YES**

#### Feature/Task (5 beads)
| Bead ID | Title | Est. Time | Description |
|----------|-------|-----------|-------------|
| intent-cli-2i5 | Fix export command to use json_output module | 1 hour | Replace io.println() with json_output module |
| intent-cli-n6c | Add global --json flag for dual-mode output | 2-3 hours | Add global flag, route all commands through JSON output |
| intent-cli-34a | Standardize error codes to match AI CLI spec | 2-3 hours | Implement EXISTS, NOTFOUND, INVALID, etc. |
| intent-cli-ibp | Implement comprehensive E2E test suite with actual CLI execution | 1-2 days | Add FFI for os:execute, update test runner |
| intent-cli-aki | WAVE5-01: Unified CLI Entry | TBD | **BLOCKED** - investigate why |

#### Unique Bug Beads (8 beads)
| Bead ID | Command | Issue | Est. Time | Status |
|----------|---------|--------|-----------|--------|
| intent-cli-m4c | parse | Exit code 1 (needs investigation) | 2-3 hours | May work with proper args |
| intent-cli-bzz | lint | Exit code 1 (needs investigation) | 2-3 hours | May work with proper args |
| intent-cli-dxi | bead-status | Exit 4 (needs investigation) | 1 hour | May need flags |
| intent-cli-8i5 | beads-regenerate | Exit 4 (needs investigation) | 1 hour | May need session-id |
| intent-cli-4ai | history | Exit 4 (needs investigation) | 1 hour | May need session-id |
| intent-cli-8mh | ai aggregate | Exit 4 (needs investigation) | 2-3 hours | May need spec paths |
| intent-cli-5ns | plan | Exit 4 (needs investigation) | 1 hour | May need session-id |
| intent-cli-766 | ready start | Exit 2 (needs investigation) | 1-2 hours | May need spec file |

**Note:** These 8 need verification - may also be "not bugs" like the 11 already closed.

**Duplicate FIX Beads (8 beads) - Close Together:**
| Bead ID | Command | Duplicate Of | Action |
|----------|---------|--------------|--------|
| intent-cli-1yn | parse FIX | intent-cli-m4c | Close after fixing intent-cli-m4c |
| intent-cli-sa4 | lint FIX | intent-cli-bzz | Close after fixing intent-cli-bzz |
| intent-cli-xbd | bead-status FIX | intent-cli-dxi | Close after fixing intent-cli-dxi |
| intent-cli-auw | beads-regenerate FIX | intent-cli-8i5 | Close after fixing intent-cli-8i5 |
| intent-cli-ex8 | history FIX | intent-cli-4ai | Close after fixing intent-cli-4ai |
| intent-cli-if4 | ai aggregate FIX | intent-cli-8mh | Close after fixing intent-cli-8mh |
| intent-cli-7cs | plan FIX | intent-cli-5ns | Close after fixing intent-cli-5ns |
| intent-cli-noq | ready start FIX | intent-cli-766 | Close after fixing intent-cli-766 |

---

### Priority 2 (Medium - Improvements)
**2 beads | Estimated: 1-2 days | Dependencies: None | Parallel: YES**

| Bead ID | Title | Est. Time | Description |
|----------|-------|-----------|-------------|
| intent-cli-6ur | Add JSON mode to utility commands | 2-3 hours | Add JSON output to diff, sessions, history, bead-status |
| intent-cli-hb5 | Shorten field names to reduce token usage | 2-3 hours | success→ok, command→cmd, timestamp→t, etc. |

**Why Later:**
- Nice-to-have improvements
- Don't block functionality
- Can be done after Priority 1 complete

---

### Subtasks (Priority 1 - Parent Dependency)
**5 beads | Estimated: 2 hours | Dependencies: intent-cli-6oy | Parallel: YES**

| Bead ID | Title | Est. Time | Description |
|----------|-------|-----------|-------------|
| intent-cli-6oy.1 | Interview System Testing: all interview modes and flags | 0.5 hour | Test interview profiles |
| intent-cli-6oy.2 | Beads System Testing: beads, bead-status, beads-regenerate, feedback | 0.5 hour | Test beads system |
| intent-cli-6oy.3 | KIRK Quality Commands: quality, invert, coverage, gaps, ears, parse, effects | 0.5 hour | Test KIRK commands |
| intent-cli-6oy.4 | History & Sessions Testing: history, diff, sessions | 0.5 hour | Test history/sessions |
| intent-cli-6oy.5 | Core Spec Commands Testing: validate, show, export, lint, analyze, improve, doctor | 0.5 hour | Test core commands |

**Blocking:** Cannot start until parent `intent-cli-6oy` is in_progress

---

### Blocked (Investigate)
**1 bead | Status: BLOCKED | Action: Investigate first**

| Bead ID | Title | Status | Action |
|----------|-------|--------|--------|
| intent-cli-aki | WAVE5-01: Unified CLI Entry | BLOCKED | Use bd show intent-cli-aki --json to investigate blockers |

---

## Execution Plan (4 Phases)

### Phase 1: Foundation (Days 1-2)
**Goal**: Establish test infrastructure and fix critical bug

| Bead | Track | Est. Time | Parallel? |
|------|-------|-----------|-----------|
| intent-cli-wb2 | Track 1 | 1 day | YES |
| intent-cli-z5s | Track 2 | 2-3 hours | YES |
| intent-cli-6oy | Track 3 | 1-2 days | YES |

**Success Criteria:**
- [ ] Test suite can run all 33 CLI commands
- [ ] Vision command implemented and working
- [ ] Dogfood assessment framework operational

**Blocking:** Phase 2 cannot start without Phase 1

---

### Phase 2: AI Ergonomics Features (Days 2-3)
**Goal**: Implement global --json flag, standardize error codes, fix export

| Bead | Track | Est. Time | Parallel? |
|------|-------|-----------|-----------|
| intent-cli-n6c | Track 4 | 2-3 hours | YES |
| intent-cli-34a | Track 5 | 2-3 hours | YES |
| intent-cli-2i5 | Track 6 | 1 hour | YES |
| intent-cli-6ur | Track 7 | 2-3 hours | YES |
| intent-cli-hb5 | Track 8 | 2-3 hours | YES |

**Success Criteria:**
- [ ] Global --json flag works for all commands
- [ ] Error codes use standard set (EXISTS, NOTFOUND, etc.)
- [ ] Export command uses json_output module
- [ ] Utility commands support JSON mode

**Dependencies:** Phase 1 complete

---

### Phase 3: Bug Verification & Testing (Days 3-4)
**Goal:** Verify if 8 "bug" beads are actual bugs, run tests

| Bead | Track | Est. Time | Parallel? |
|------|-------|-----------|-----------|
| intent-cli-m4c | Track 9 | 2-3 hours | YES |
| intent-cli-bzz | Track 10 | 2-3 hours | YES |
| intent-cli-dxi | Track 11 | 1 hour | YES |
| intent-cli-8i5 | Track 12 | 1 hour | YES |
| intent-cli-4ai | Track 13 | 1 hour | YES |
| intent-cli-8mh | Track 14 | 2-3 hours | YES |
| intent-cli-5ns | Track 15 | 1 hour | YES |
| intent-cli-766 | Track 16 | 1-2 hours | YES |
| intent-cli-6oy.1 | Track 17 | 0.5 hour | YES |
| intent-cli-6oy.2 | Track 18 | 0.5 hour | YES |
| intent-cli-6oy.3 | Track 19 | 0.5 hour | YES |
| intent-cli-6oy.4 | Track 20 | 0.5 hour | YES |
| intent-cli-6oy.5 | Track 21 | 0.5 hour | YES |

**Success Criteria:**
- [ ] All 8 "bug" beads verified (close if not bugs, fix if actual bugs)
- [ ] All 5 testing subtasks pass
- [ ] Test suite validates all commands

**Dependencies:** Phase 1 complete

**Parallel Tracks (3 options):**

**Option 1: Small Team (2-3 people)**
- Track A: 8 bug beads (8 hours)
- Track B: 5 testing subtasks (2.5 hours)

**Option 2: Medium Team (4-6 people)**
- Track A: 4 bug beads (4 hours)
- Track B: 4 bug beads (4 hours)
- Track C: 5 testing subtasks (2.5 hours)

**Option 3: Large Team (8-10 people)**
- Track A: 2 bug beads per person (4 hours)
- Track B: 1 testing subtask per person (2.5 hours)

---

### Phase 4: E2E Test Suite (Days 4-5)
**Goal:** Implement end-to-end tests with actual CLI execution

| Bead | Track | Est. Time | Parallel? |
|------|-------|-----------|-----------|
| intent-cli-ibp | Track 22 | 1-2 days | NO |

**Success Criteria:**
- [ ] FFI implemented (os:execute)
- [ ] E2E tests execute actual CLI commands
- [ ] All exit codes validated
- [ ] JSON output validated

**Dependencies:** Phase 3 complete

---

## Summary by Type

### Bug Beads (8 unique + 8 duplicate FIX beads = 16 total)
**Unique:**
- intent-cli-m4c (parse)
- intent-cli-bzz (lint)
- intent-cli-dxi (bead-status)
- intent-cli-8i5 (beads-regenerate)
- intent-cli-4ai (history)
- intent-cli-8mh (ai aggregate)
- intent-cli-5ns (plan)
- intent-cli-766 (ready start)

**Duplicate FIX beads (close together after fixing unique):**
- intent-cli-1yn, intent-cli-sa4, intent-cli-xbd, intent-cli-auw
- intent-cli-ex8, intent-cli-if4, intent-cli-7cs, intent-cli-noq

### Feature/Task Beads (6)
- intent-cli-wb2 (test suite)
- intent-cli-z5s (vision command)
- intent-cli-6oy (dogfood framework)
- intent-cli-n6c (global --json)
- intent-cli-34a (standardize error codes)
- intent-cli-2i5 (fix export)

### Testing Beads (6)
- intent-cli-ibp (E2E tests)
- intent-cli-6oy.1 through .6oy.5 (subtasks)

### Improvement Beads (2)
- intent-cli-6ur (JSON mode for utility commands)
- intent-cli-hb5 (shorten field names)

### Blocked Beads (1)
- intent-cli-aki (investigate blockers)

---

## Quick Commands Reference

### Check Ready Work
```bash
bd ready --json
```

### Claim a Bead
```bash
bd update <bead-id> --status in_progress --json
```

### Complete a Bead
```bash
bd close <bead-id> --reason "Completed" --json
```

### Close Duplicate Beads
```bash
# Close FIX bead together with unique bug bead
bd close intent-cli-1yn --reason "Verified - parse command works" --json
bd close intent-cli-m4c --reason "Fixed parse command" --json
```

### Run Tests
```bash
# Run Gleam tests
gleam test

# Run integration tests
./test/run_integration_tests.sh

# Run specific category
./test/run_integration_tests.sh --category core
```

---

## Team Parallelization Guide

### Small Team (2-3 developers)
**Days 1-2:**
- Dev1: intent-cli-wb2 (test suite)
- Dev2: intent-cli-z5s (vision command) + intent-cli-6oy (dogfood)

**Days 2-3:**
- Dev1: intent-cli-n6c + intent-cli-34a (JSON + error codes)
- Dev2: intent-cli-2i5 + intent-cli-6ur (export + utility JSON)

**Days 3-4:**
- Dev1: Verify 4 bug beads + 3 testing subtasks
- Dev2: Verify 4 bug beads + 2 testing subtasks

**Day 5:**
- Dev1: intent-cli-ibp (E2E tests)

**Total: 5 days**

### Medium Team (4-6 developers)
**Days 1-2:**
- Dev1: intent-cli-wb2 (test suite)
- Dev2: intent-cli-z5s (vision command)
- Dev3: intent-cli-6oy (dogfood)
- Dev4: Start bug verification (2 beads)

**Days 2-3:**
- Dev1: intent-cli-n6c + intent-cli-34a
- Dev2: intent-cli-2i5 + intent-cli-6ur
- Dev3: Verify 2 bug beads + 2 testing subtasks
- Dev4: Verify 2 bug beads + 2 testing subtasks
- Dev5: Verify 2 bug beads + 1 testing subtask

**Days 3-4:**
- Dev1: intent-cli-ibp (E2E tests)
- Dev2: Complete remaining bug verifications
- Dev3: Complete remaining bug verifications

**Total: 4-5 days**

### Large Team (8-10 developers)
**Days 1-2:**
- Dev1-2: intent-cli-wb2 (test suite)
- Dev3: intent-cli-z5s (vision command)
- Dev4: intent-cli-6oy (dogfood)
- Dev5-6: Verify 4 bug beads
- Dev7-8: Verify 4 bug beads
- Dev9-10: AI ergonomics features (2-3 beads)

**Days 2-3:**
- Dev1-2: intent-cli-ibp (E2E tests)
- Dev3-8: Complete remaining bug verifications
- Dev9-10: Run testing subtasks

**Total: 3-4 days**

---

## Duplicate Beads - Close Together

After fixing each unique bug bead, close its duplicate FIX bead:

| Unique Bead | Duplicate FIX Bead | Close Together |
|-------------|-------------------|---------------|
| intent-cli-m4c (parse) | intent-cli-1yn | Yes |
| intent-cli-bzz (lint) | intent-cli-sa4 | Yes |
| intent-cli-dxi (bead-status) | intent-cli-xbd | Yes |
| intent-cli-8i5 (beads-regenerate) | intent-cli-auw | Yes |
| intent-cli-4ai (history) | intent-cli-ex8 | Yes |
| intent-cli-8mh (ai aggregate) | intent-cli-if4 | Yes |
| intent-cli-5ns (plan) | intent-cli-7cs | Yes |
| intent-cli-766 (ready start) | intent-cli-noq | Yes |

**Command:**
```bash
# Example: Close both together after fixing parse command
bd close intent-cli-m4c --reason "Fixed parse command" --json
bd close intent-cli-1yn --reason "Verified parse command works" --json
```

---

## Critical Path (Must Complete in Order)

1. **intent-cli-wb2** (test suite) - Blocks all validation
2. **intent-cli-z5s** (vision command) - Only real bug, critical
3. **intent-cli-n6c** (global --json) - Enables AI mode everywhere
4. **intent-cli-34a** (error codes) - Standardizes errors
5. **intent-cli-2i5** (export) - Fixes JSON output consistency
6. **intent-cli-ibp** (E2E tests) - Final validation layer

**All other work can be done in parallel with critical path.**

---

## Status Summary

### Total Beads: 21 (after closing 11 non-bugs)
- Priority 0: 3 (14%)
- Priority 1: 13 (62%)
- Priority 2: 2 (10%)
- Blocked: 1 (5%)
- Subtasks: 5 (included in Priority 1)

### Estimated Work Time
- **Small Team (2-3 devs)**: 5 days
- **Medium Team (4-6 devs)**: 4-5 days
- **Large Team (8-10 devs)**: 3-4 days

### Ready to Start
- **33/33 beads** (all except 1 blocked) are ready to claim
- **Zero dependencies** between beads (except subtasks on parent)
- **All descriptions** are detailed and actionable

---

## Next Steps (Pick One)

### Start Right Now (3 options)

**Option 1: Foundation (Recommended)**
```bash
# Build test suite
bd update intent-cli-wb2 --status in_progress --json
```

**Option 2: Critical Bug (Quick Win)**
```bash
# Fix vision command
bd update intent-cli-z5s --status in_progress --json
```

**Option 3: Clean Up (5 minutes)**
```bash
# Close all 8 duplicate FIX beads
for id in 1yn sa4 xbd auw ex8 if4 7cs noq; do
  bd close intent-cli-$id --reason "Duplicate - verified with unique bead" --json
done
```

### Investigate First (if choosing)
```bash
# Check why WAVE5-01 is blocked
bd show intent-cli-aki --json
```

---

**Generated**: 2026-01-25
**Status**: ✅ Sorted, planned, ready for execution
**Next Action**: Pick one of 3 options above and start!
