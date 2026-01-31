# Intent CLI - Bead Execution Plan

**Date**: 2026-01-25
**Total Beads**: 33 (23 bugs, 7 tasks, 1 feature, 2 subtasks blocked by parent)

---

## Overview

All beads are from today (2026-01-25). The work falls into three main categories:

1. **Critical Priority 0**: Test infrastructure foundation
2. **High Priority 1**: 23 bugs + 4 testing tasks + 1 feature (blocked)
3. **No current dependencies**: All beads are independent and can be tackled in parallel

---

## Priority Classification

### Priority 0 (Critical - Foundation)
Blocks everything else. Test infrastructure must exist before we can validate fixes.

- `intent-cli-wb2` - Create comprehensive test suite
- `intent-cli-6oy` - CLI Dogfood & AI Ergonomics Assessment (parent to subtasks)

### Priority 1 (High - Bugs & Features)
23 bugs across 12 unique commands + 4 testing subtasks + 1 feature (blocked)

#### Bug Categories by Command:
- **Exit Code 4 Errors** (12 bugs): feedback, prompt, diff, plan, ai aggregate, vision start, beads-regenerate, history, bead-status
- **Exit Code 1 Errors** (2 bugs): parse, lint
- **Exit Code 2 Error** (1 bug): ready start
- **Fix Tasks** (8 duplicates): Investigative fix tasks for above bugs

#### Testing Subtasks (intent-cli-6oy.x):
- `intent-cli-6oy.1` - Interview System Testing
- `intent-cli-6oy.2` - Beads System Testing (depends on parent)
- `intent-cli-6oy.3` - KIRK Quality Commands Testing
- `intent-cli-6oy.4` - History & Sessions Testing
- `intent-cli-6oy.5` - Core Spec Commands Testing (depends on parent)

#### Features:
- `intent-cli-aki` - WAVE5-01: Unified CLI Entry (status: blocked)

---

## Dependency Graph

```
Phase 1 (P0): Foundation
├── intent-cli-wb2 [Create test suite] ────────────────────────────────┐
└── intent-cli-6oy [CLI Dogfood] ───┬─> intent-cli-6oy.1 (interview)  │
                                    ├─> intent-cli-6oy.2 (beads)     │
                                    ├─> intent-cli-6oy.3 (KIRK)      │
                                    ├─> intent-cli-6oy.4 (history)   │
                                    └─> intent-cli-6oy.5 (spec cmds) │

Phase 2 (P1): Bug Fixes (all parallel)
├── intent-cli-irr [vision start - exit 4]
├── intent-cli-766 [ready start - exit 2]
├── intent-cli-m4c [parse - exit 1]
├── intent-cli-bzz [lint - exit 1]
├── intent-cli-92o [feedback - exit 4]
├── intent-cli-o7m [prompt - exit 4]
├── intent-cli-nlh [diff - exit 4]
├── intent-cli-5ns [plan - exit 4]
├── intent-cli-8mh [ai aggregate - exit 4]
├── intent-cli-8i5 [beads-regenerate - exit 4]
├── intent-cli-4ai [history - exit 4]
├── intent-cli-dxi [bead-status - exit 4]
└── intent-cli-98f, noq, 7cs, if4, ex8, auw, xbd, 1yn, sa4 [FIX duplicates]

Phase 3 (P1): E2E Testing
└── intent-cli-ibp [E2E test suite]

Note: intent-cli-aki [Unified CLI Entry] is BLOCKED
```

---

## Phased Execution Plan

### Phase 1: Test Infrastructure Foundation (Days 1-2)
**Goal**: Establish testing capability to validate all future work

| Bead | Title | Est. Time | Dependencies | Parallel? |
|------|-------|-----------|--------------|-----------|
| `intent-cli-wb2` | Create comprehensive test suite | 1 day | None | Yes |
| `intent-cli-6oy` | CLI Dogfood & AI Ergonomics | 2 days | None | Yes |

**Success Criteria**:
- Test suite can run all CLI commands
- Dogfood assessment framework operational
- Exit code validation tests exist

**Blocking**: All Phase 2 bug fixes depend on having a test suite to verify fixes

---

### Phase 2: Critical Bug Fixes (Days 2-4)
**Goal**: Fix all 12 unique command bugs (can be done in parallel)

#### Track A: Vision & Ready (1 day)
| Bead | Command | Error | Est. Time | Parallel? |
|------|---------|-------|-----------|-----------|
| `intent-cli-irr` | vision start | Exit 4 | 2-3 hours | Yes |
| `intent-cli-766` | ready start | Exit 2 | 2-3 hours | Yes |

#### Track B: Core Commands (1 day)
| Bead | Command | Error | Est. Time | Parallel? |
|------|---------|-------|-----------|-----------|
| `intent-cli-m4c` | parse | Exit 1 | 1-2 hours | Yes |
| `intent-cli-bzz` | lint | Exit 1 | 1-2 hours | Yes |

#### Track C: Utility Commands (1.5 days)
| Bead | Command | Error | Est. Time | Parallel? |
|------|---------|-------|-----------|-----------|
| `intent-cli-92o` | feedback | Exit 4 | 1 hour | Yes |
| `intent-cli-o7m` | prompt | Exit 4 | 1 hour | Yes |
| `intent-cli-nlh` | diff | Exit 4 | 1 hour | Yes |
| `intent-cli-5ns` | plan | Exit 4 | 1 hour | Yes |
| `intent-cli-4ai` | history | Exit 4 | 1 hour | Yes |

#### Track D: Beads & AI (1 day)
| Bead | Command | Error | Est. Time | Parallel? |
|------|---------|-------|-----------|-----------|
| `intent-cli-8mh` | ai aggregate | Exit 4 | 2-3 hours | Yes |
| `intent-cli-8i5` | beads-regenerate | Exit 4 | 1 hour | Yes |
| `intent-cli-dxi` | bead-status | Exit 4 | 1 hour | Yes |

**Duplicate FIX Tasks** (can be closed when corresponding bug is fixed):
- `intent-cli-98f` (vision), `intent-cli-noq` (ready), `intent-cli-1yn` (parse)
- `intent-cli-sa4` (lint), `intent-cli-ffj` (prompt), `intent-cli-5bs` (feedback)
- `intent-cli-tm9` (diff), `intent-cli-7cs` (plan), `intent-cli-if4` (ai aggregate)
- `intent-cli-ex8` (history), `intent-cli-auw` (beads-regenerate), `intent-cli-xbd` (bead-status)

**Success Criteria**:
- All commands return exit code 0 on success
- All commands return exit code 1 on error
- No exit code 2 (invalid) or 4 (internal error) in normal operation

---

### Phase 3: Testing Validation (Days 4-5)
**Goal**: Validate all fixes with comprehensive tests

| Bead | Title | Est. Time | Dependencies | Parallel? |
|------|-------|-----------|--------------|-----------|
| `intent-cli-6oy.1` | Interview System Testing | 0.5 day | intent-cli-6oy | Yes |
| `intent-cli-6oy.2` | Beads System Testing | 0.5 day | intent-cli-6oy | Yes |
| `intent-cli-6oy.3` | KIRK Quality Commands | 0.5 day | intent-cli-6oy | Yes |
| `intent-cli-6oy.4` | History & Sessions | 0.5 day | intent-cli-6oy | Yes |
| `intent-cli-6oy.5` | Core Spec Commands | 0.5 day | intent-cli-6oy | Yes |

**Blocking**: Requires Phase 2 bugs to be fixed first

---

### Phase 4: E2E Test Suite (Day 5-6)
**Goal**: Implement end-to-end tests with actual CLI execution

| Bead | Title | Est. Time | Dependencies | Parallel? |
|------|-------|-----------|--------------|-----------|
| `intent-cli-ibp` | E2E test suite | 1-2 days | All bugs fixed | No |

**Blocking**: Requires all Phase 2 bug fixes to be complete

---

### Blocked Work (Investigate First)

| Bead | Title | Status | Action Needed |
|------|-------|--------|---------------|
| `intent-cli-aki` | Unified CLI Entry | Blocked | Investigate why - check dependencies or blockers |

---

## Parallel Execution Tracks

### Option 1: Small Team (2-3 developers)
**Suggested Parallel Tracks**:

- **Track 1**: Test Infrastructure (2 beads, 3 days)
  - intent-cli-wb2 + intent-cli-6oy
  - Critical for all subsequent work

- **Track 2**: Vision & Ready Bugs (2 beads, 1 day)
  - intent-cli-irr + intent-cli-766
  - Start after test suite is ready

- **Track 3**: Core Commands (2 beads, 1 day)
  - intent-cli-m4c + intent-cli-bzz
  - Start after test suite is ready

### Option 2: Medium Team (4-6 developers)
**Suggested Parallel Tracks**:

- **Track 1**: Test Infrastructure (2 beads, 3 days)
- **Track 2**: Vision & Ready (2 beads, 1 day) → then Testing Subtasks
- **Track 3**: Core Commands (2 beads, 1 day) → then Testing Subtasks
- **Track 4**: Utility Commands (5 beads, 1.5 days)
- **Track 5**: Beads & AI (3 beads, 1 day)

### Option 3: Large Team (8-10 developers)
**Suggested Parallel Tracks**:

- **Track 1**: Test Infrastructure (2 beads, 3 days)
- **Track 2**: Vision & Ready (2 beads, 1 day)
- **Track 3**: Core Commands (2 beads, 1 day)
- **Track 4**: Utility Commands (5 beads, 1.5 days)
- **Track 5**: Beads & AI (3 beads, 1 day)
- **Track 6**: Testing Subtasks (5 beads, 2.5 days) - can start after Phase 2
- **Track 7**: E2E Test Suite (1 bead, 2 days) - start after Phase 2 complete

---

## Quick Reference Guide

### Pick One of These 3 to Start (Foundation)
1. **`intent-cli-wb2`** - Create comprehensive test suite
2. **`intent-cli-6oy`** - CLI Dogfood & AI Ergonomics Assessment
3. **Investigate `intent-cli-aki`** - Find out why it's blocked

### After Completing Test Suite (intent-cli-wb2), Do These First
**Highest Impact Bugs** (most used commands):
1. **`intent-cli-m4c`** - parse command (core functionality)
2. **`intent-cli-bzz`** - lint command (core functionality)
3. **`intent-cli-irr`** - vision start (AI integration)

### These 12 Can Be Done in Parallel (All Phase 2 Bugs)
```
Track A: intent-cli-irr, intent-cli-766 (vision & ready)
Track B: intent-cli-m4c, intent-cli-bzz (parse & lint)
Track C: intent-cli-92o, intent-cli-o7m, intent-cli-nlh, intent-cli-5ns, intent-cli-4ai (feedback, prompt, diff, plan, history)
Track D: intent-cli-8mh, intent-cli-8i5, intent-cli-dxi (ai aggregate, beads-regenerate, bead-status)
```

### After All Bugs Fixed
Run these 5 testing subtasks in parallel:
- `intent-cli-6oy.1` (interview)
- `intent-cli-6oy.2` (beads)
- `intent-cli-6oy.3` (KIRK)
- `intent-cli-6oy.4` (history)
- `intent-cli-6oy.5` (spec commands)

### Final Step
- `intent-cli-ibp` - E2E test suite implementation

---

## Priority Justification

### Why Test Infrastructure First (P0)?
- **Cannot validate fixes without tests**: 23 bug fixes need automated validation
- **Prevents regression**: Test suite ensures new bugs aren't introduced
- **AI agent support**: Enables automated verification of all commands
- **Foundation for E2E**: E2E tests (intent-cli-ibp) depend on unit tests existing

### Why Fix Bugs Before Testing Subtasks?
- **Can't test broken commands**: Testing subtasks require commands to work
- **False negatives**: Tests would fail on broken commands, wasting investigation time
- **Exit code validation**: Phase 2 fixes establish correct exit code patterns

### Why Focus on Exit Codes?
- **Critical for CI/CD**: Exit codes determine build success/failure
- **AI agent compatibility**: Agents rely on correct exit codes for decision-making
- **User experience**: Exit code 4 (internal error) is confusing vs 1 (expected error)

### Why Parallel Execution?
- **Zero dependencies**: All 23 bugs are independent
- **Quick wins**: 8 bugs are 1-hour fixes each
- **Fast feedback**: Parallel work reduces total time from 12 days to 3-4 days

---

## Success Metrics

### Phase 1 Complete When:
- [ ] `intent-cli-wb2` closed - Test suite exists and passes
- [ ] `intent-cli-6oy` closed - Dogfood assessment framework operational

### Phase 2 Complete When:
- [ ] All 12 unique bugs fixed (exit codes 0/1 only)
- [ ] Test suite passes for all commands
- [ ] No exit code 2 or 4 in normal operation

### Phase 3 Complete When:
- [ ] All 5 testing subtasks pass
- [ ] All commands validated across all test scenarios

### Phase 4 Complete When:
- [ ] `intent-cli-ibp` closed - E2E test suite operational
- [ ] Full CLI workflow tested end-to-end

---

## Command Reference

### Bead Operations
```bash
# List all beads
bd list

# Check ready work
bd ready

# Claim a bead
bd update <id> --status in_progress

# Complete a bead
bd close <id> --reason "Completed"

# View dependencies
bd graph --format=json
```

### Testing Commands
```bash
# Run test suite
gleam test

# Run specific test
gleam run -m intent_test

# Validate exit codes
intent <command> && echo "Exit 0" || echo "Exit $?"
```

---

## Notes

1. **Duplicate FIX Tasks**: The 8 "FIX:" beads (intent-cli-ffj, -5bs, etc.) appear to be duplicates of the exit code bugs. When fixing a bug, close both the bug bead and its corresponding FIX bead together.

2. **Subtask Dependencies**: The `intent-cli-6oy.x` subtasks depend on parent `intent-cli-6oy`. Parent must be claimed/in_progress before subtasks can start.

3. **Blocked Feature**: `intent-cli-aki` is blocked - investigate before attempting work. May require unblocking by owner or resolving external dependencies.

4. **Exit Code Standards**:
   - 0 = Success
   - 1 = Expected error (e.g., invalid input, file not found)
   - 2 = Invalid arguments or state
   - 4 = Internal error (should never happen in production)

5. **Git Workflow**: Always commit `.beads/issues.jsonl` together with code changes to keep issue state in sync with code state.

---

**Generated**: 2026-01-25
**Total Estimated Time**: 5-6 days (small team), 3-4 days (medium team), 2-3 days (large team)
**Critical Path**: intent-cli-wb2 → Phase 2 Bugs → intent-cli-ibp
