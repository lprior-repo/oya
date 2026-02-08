# OYA Swarm - 13-Agent Continuous Assembly Line

## Overview

OYA Swarm orchestrates 13 parallel agents (**4 Test Writers**, 4 Implementers, 4 Reviewers, 1 Planner) in a continuous assembly line to complete beads using **contract-first development** with **continuous-deployment principles**.

## Key Principles

### Contract-First Development
1. **Test Writers** create test contracts BEFORE implementation using `rust-contract` skill
2. Contracts define: exhaustive error variants, preconditions, postconditions, invariants
3. Martin Fowler test philosophy: Given-When-Then structure, expressive test names
4. Tests serve as documentation and specification

### Continuous-Deployment (ABSOLUTE LAW)
Continuous-deployment is the foundation that ensures:
- **Velocity**: Fast, small batches through the pipeline
- **One-Piece Flow**: Single bead at a time per agent
- **Moon Gates**: All quality gates must pass (`moon run :ci`, `moon run :quick`)
- **Functional Rust**: Zero unwrap/expect/panic enforced
- **TDD15**: Test-driven development (RED → GREEN → REFACTOR)
- **Shift-Left**: Quality enforced early, not as afterthought

**Critical**: `--continuous-deployment` is always ON and CANNOT be disabled.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    OrchestratorActor (ractor)                   │
│  - Maintains shared work queue                                  │
│  - Spawns/replaces crashed agents (one-for-one supervision)     │
│  - Tracks completion count (target: 25 beads)                   │
│  - Monitors handoff files in /tmp/                              │
└─────────────────────────────────────────────────────────────────┘
           │              │              │              │
           ▼              ▼              ▼              ▼
    ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐
    │   Test   │   │Implement │   │ Reviewer │   │ Planner  │
    │  Writer  │   │ Actor    │   │ Actor    │   │ Actor    │
    │  (x4)    │   │  (x4)    │   │  (x4)    │   │  (x1)    │
    └──────────┘   └──────────┘   └──────────┘   └──────────┘
         │              │              │              │
         └──────────────┴──────────────┴──────────────┘
                   │
           File-based Handoffs in /tmp/
```

## Agent Responsibilities

### Test Writer Agents (4)
**Skills**: rust-contract, functional-rust, continuous-deployment

1. Run `bv --robot-triage` to get next bead
2. Claim via `br update <bead-id> --status in_progress`
3. Use `rust-contract` skill to define:
   - Exhaustive error variants (all possible failure modes)
   - Preconditions (what inputs are valid)
   - Postconditions (what promises the code makes)
   - Invariants (what must always be true)
   - Break analysis (edge cases, what could go wrong)
4. Write Martin Fowler style tests (Given-When-Then, expressive names)
5. Create `/tmp/bead-contracts-<id>.json` with contract and test code
6. Create `/tmp/bead-ready-to-implement-<id>.json`
7. Print: `CONTRACT_READY:<id>` and `READY_TO_IMPLEMENT:<id>`
8. On failure: Report to orchestrator, mark for retry

### Implementer Agents (4)
**Primary Skill**: continuous-deployment (ABSOLUTE LAW)

1. Poll `/tmp/bead-ready-to-implement-*.json` for work
2. Read `/tmp/bead-contracts-<id>.json` (test plan from Test Writer)
3. Spawn isolated workspace: `zjj spawn <bead-id> --session implement-<bead-id>`
4. Follow continuous-deployment workflow:
   - **Velocity**: Small batch, single bead focus
   - **TDD15**: RED → GREEN → REFACTOR (using test contracts)
   - **Functional Rust**: Use functional-rust-generator (ZERO unwraps)
   - **Moon Gates**: `moon run :ci` MUST pass (all tests, zero warnings)
   - **Shift-Left**: Quality enforced at every step
5. Create `/tmp/bead-ready-review-<id>.json` with test results
6. On moon ci failure: Fix immediately or report for re-evaluation
7. On failure: Preserve workspace for inspection, report bead for re-evaluation

### Reviewer Agents (4)
1. Poll `/tmp/bead-ready-review-*.json` for work
2. Apply `/red-queen` skill for adversarial QA
3. Verify zero clippy warnings: `moon run :quick`
4. Apply `/landing` skill for commit + sync + push
5. Verify git push succeeded with `jj log`
6. Clean up workspace: `zjj done <workspace>` (auto-merges to main, removes session)
7. Create `/tmp/bead-complete-<id>.json` on success
8. On QA failure: Send bead back to Implementer queue, clean up workspace
9. On landing failure: Preserve workspace for inspection, report for retry

### Planner Agent (1)
**Skills**: rust-contract, planner

1. Review bead requirements from handoff files
2. Use `rust-contract` to define contracts
3. Design Martin Fowler test philosophy tests
4. Create contract specifications
5. Coordinate contract workflow between Test Writers and Implementers

## File-Based Handoff Mechanism

Agents communicate via atomic file operations in `/tmp/`:

- `/tmp/bead-contracts-<id>.json` - Test contracts from Test Writers
- `/tmp/bead-ready-to-implement-<id>.json` - Ready for implementation
- `/tmp/bead-implementation-in-progress-<id>.json` - Implementer claimed
- `/tmp/bead-implementation-complete-<id>.json` - Implementation done
- `/tmp/bead-ready-review-<id>.json` - Ready for review
- `/tmp/bead-reviewing-<id>.json` - Reviewer claimed
- `/tmp/bead-complete-<id>.json` - Bead landed

**State transitions** via `mv` commands (atomic filesystem operations)

## Quality Gates (Non-Negotiable)

### Foundation: continuous-deployment skill principles
1. **Velocity Gate**: Small batch size, single bead per agent
2. **One-Piece Flow Gate**: No batching, sequential flow only
3. **Moon CI Gate**: `moon run :ci` MUST pass (all tests, zero warnings)
4. **Moon Quick Gate**: `moon run :quick` MUST pass (zero clippy warnings)
5. **Functional Rust Gate**: ZERO unwrap/expect/panic violations
6. **TDD15 Gate**: RED → GREEN → REFACTOR workflow followed

### Per-stage gates
1. **Test Writer Gate**: `br update --status in_progress` succeeds
2. **Implementer Gate**: `moon run :ci` passes (all tests, zero warnings)
3. **Reviewer Gate**: `moon run :quick` passes (zero clippy), red-queen QA passes
4. **Landing Gate**: `jj git push` succeeds, main remains clean

**On Failure**:
- Implementer failure: Preserve workspace, mark bead for re-evaluation
- Reviewer failure: Send bead back to Implementer queue, clean workspace
- Agent crash: Spawn replacement immediately, re-queue in-progress bead

## CLI Usage

```bash
# Basic usage (25 beads, 13 agents)
oya swarm

# Dry run preview
oya swarm --dry-run

# Small scale test (3 beads, 1 agent each type)
oya swarm --target 3 --test-writers 1 --implementers 1 --reviewers 1

# Full scale with custom configuration
oya swarm --target 50 --test-writers 8 --implementers 8 --reviewers 8

# Resume interrupted session
oya swarm --resume session-abc123

# JSON output for automation
oya swarm --format json | jq '.landed_beads'
```

### CLI Options

- `--target <N>`: Target bead count [default: 25]
- `--test-writers <N>`: Test Writer agents [default: 4]
- `--implementers <N>`: Implementer agents [default: 4]
- `--reviewers <N>`: Reviewer agents [default: 4]
- `--planner`: Enable Planner agent [default: true]
- `--continuous-deployment`: Enforce CD principles [default: true, CANNOT DISABLE]
- `--dry-run`: Preview without execution
- `--resume <session-id>`: Continue from previous session
- `--format <text|json>`: Output format [default: text]

## Configuration File

Configuration is loaded from `.oya/swarm.toml`:

```toml
[defaults]
target_beads = 25
test_writers = 4
implementers = 4
reviewers = 4
planner = true

[continuous-deployment]
enabled = true  # CANNOT be disabled

[quality-gates]
moon_ci = true
moon_quick = true
zero_panic = true
red_queen = true
git_push = true

[handoff]
dir = "/tmp"
```

## Integration with Tools

| Tool | Purpose | Agent Type |
|------|---------|------------|
| `/continuous-deployment` skill | DRIVES EVERYTHING | ALL |
| `/rust-contract` skill | Define contracts | Test Writer, Planner |
| `/functional-rust` skill | Implementation (ZERO unwraps) | Test Writer, Implementer |
| `/tdd15` skill | TDD workflow | Implementer |
| `bv --robot-triage` | Get next bead | Test Writer |
| `br update --status` | Claim bead | Test Writer |
| `moon run :ci` | Quality gates | Implementer |
| `/red-queen` skill | Adversarial QA | Reviewer |
| `moon run :quick` | Clippy check | Reviewer |
| `/landing` skill | Commit+push | Reviewer |
| `zjj spawn` | Create workspace | Implementer |
| `zjj done` | Clean workspace | Reviewer |

## Stop Conditions

1. **Success**: `landed_beads >= target_beads` (default 25)
2. **User Interrupt**: Ctrl+C triggers graceful shutdown
3. **Timeout**: 1 hour maximum execution
4. **Critical Failure**: 10+ consecutive bead failures

## Troubleshooting

### Swarm won't start
- Check that `.oya/swarm.toml` exists and is valid
- Verify `continuous_deployment = true` (cannot be disabled)
- Ensure required skills are in `~/.claude/skills/`

### Agents failing to claim beads
- Check `bv --robot-triage` returns valid beads
- Verify `br` CLI is working
- Check /tmp directory permissions

### Moon gates failing
- Run `moon run :ci` manually to see errors
- Check for clippy warnings: `moon run :quick`
- Verify zero unwrap/expect/panic violations

### Workspace issues
- List active workspaces: `zjj list`
- Recover orphans: `zjj recover --diagnose`
- Clean up manually: `zjj abort -w <name>`

### Handoff file errors
- Check /tmp directory permissions
- Verify file cleanup: `ls /tmp/bead-*.json`
- Manually clean stuck files: `rm /tmp/bead-*<id>*.json`

## Monitoring and Status

Swarm prints progress every 1 second:

```
=== Swarm Mode Started ===

Configuration:
  Target beads: 25
  Total agents: 13

Agent Distribution:
  Test Writers: 4
  Implementers: 4
  Reviewers: 4
  Planner: 1

Progress:
  Landed: 5/25 (20%)
  In Progress: 4
  Pending: 16
  Failed: 0

Active Agents:
  test-writer-1: Working on src-abc
  implementer-2: Working on src-def
  reviewer-3: QA testing src-ghi
  ...
```

## Example Workflow

1. **Start swarm**: `oya swarm --target 5`
2. **Test Writer 1** gets bead from `bv --robot-triage`
3. **Test Writer 1** writes contract to `/tmp/bead-contracts-src-abc.json`
4. **Implementer 1** claims `/tmp/bead-ready-to-implement-src-abc.json`
5. **Implementer 1** spawns workspace: `zjj spawn src-abc`
6. **Implementer 1** implements following contract, runs `moon run :ci`
7. **Implementer 1** creates `/tmp/bead-ready-review-src-abc.json`
8. **Reviewer 1** claims, runs `/red-queen` QA, `moon run :quick`
9. **Reviewer 1** lands bead: `/landing`, `zjj done src-abc`
10. **Reviewer 1** creates `/tmp/bead-complete-src-abc.json`
11. **Orchestrator** increments landed count
12. Repeat until target reached

## Continuous-Deployment Skill Requirements

The continuous-deployment skill is the absolute law of this system. All agents MUST:

1. **Follow Velocity**: One bead at a time, no batching
2. **Maintain Flow**: Sequential progress through pipeline
3. **Pass Moon Gates**: `moon run :ci` and `moon run :quick` must pass
4. **Use Functional Rust**: Zero unwrap/expect/panic enforced
5. **Follow TDD15**: RED → GREEN → REFACTOR workflow
6. **Enforce Quality Early**: Shift-left quality principles

**Main branch is ALWAYS releasable** - This is non-negotiable.
