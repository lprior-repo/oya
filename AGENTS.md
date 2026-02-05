# Agent Instructions

This project uses **br** (beads_rust) for issue tracking, **Moon** for hyper-fast builds, and **zjj** for workspace isolation.

## Critical: Use zjj CLI for Workspace Management

**zjj wraps Jujutsu + Zellij for isolated development:**

```bash
# Create isolated workspace
zjj add <session-name>     # Manual work
zjj spawn <bead-id>        # Agent work

# Workflow
zjj status                 # Check session state
zjj sync                   # Sync with main
zjj done                   # Merge and cleanup

# For raw jj when needed
jj commit -m "msg"         # Create commit
jj git fetch               # Fetch from remote
jj git push                # Push to remote
```

## Quick Reference

### Issue Tracking (Beads)

**See [docs/BEADS.md](docs/BEADS.md) for complete br reference.**

### Development (Moon CI/CD)
```bash
moon run :quick       # Fast checks (6-7ms with cache!)
moon run :ci          # Full pipeline (parallel)
moon run :fmt-fix     # Auto-fix formatting
moon run :build       # Release build
moon run :install     # Install to ~/.local/bin
```

## Hyper-Fast CI/CD Pipeline

This project uses **Moon + bazel-remote** for 98.5% faster builds:

### Performance Characteristics
- **6-7ms** for cached tasks (vs ~450ms uncached)
- **Parallel execution** across all crates
- **100GB local cache** persists across sessions
- **Zero sudo** required (systemd user service)

### Development Workflow

**1. Quick Iteration Loop** (6-7ms with cache):
```bash
# Edit code...
moon run :quick  # Parallel fmt + clippy check
```

**2. Before Committing**:
```bash
moon run :fmt-fix  # Auto-fix formatting
moon run :ci       # Full pipeline (if tests pass)
```

**3. Cache Management**:
```bash
# View cache stats
curl http://localhost:9090/status | jq

# Restart cache if needed
systemctl --user restart bazel-remote
```

### Build System Rules

**ALWAYS use Moon, NEVER raw cargo:**
- `moon run :build` (cached, fast)
- `moon run :test` (parallel with nextest)
- `moon run :check` (quick type check)
- `cargo build` (no caching, slow)
- `cargo test` (no parallelism)

**Why**: Moon provides:
- Persistent remote caching (survives `moon clean`)
- Parallel task execution
- Dependency-aware rebuilds
- 98.5% faster with cache hits

## Using bv for AI Triage

bv is a graph-aware triage engine for Beads projects. Use robot flags for deterministic, dependency-aware outputs with precomputed metrics.

**CRITICAL: Use ONLY `--robot-*` flags. Bare `bv` launches an interactive TUI that blocks.**

```bash
# THE ENTRY POINT - Get everything in one call
bv --robot-triage

# Minimal: just the top pick + claim command
bv --robot-next

# Parallel execution tracks for multi-agent workflows
bv --robot-plan --robot-triage-by-track

# Token-optimized output
bv --robot-triage --format toon
```

**Key outputs from `--robot-triage`:**
- `quick_ref.top_picks` - Top 3 ranked issues
- `recommendations` - Full ranked list with scores, reasons
- `quick_wins` - Low-effort, high-impact items
- `blockers_to_clear` - High-impact unblock targets
- `commands` - Copy-paste shell commands for next steps

**jq examples:**
```bash
bv --robot-triage | jq '.quick_ref.top_picks[:3]'
bv --robot-triage | jq '.recommendations[0]'
bv --robot-plan | jq '.plan.summary.highest_impact'
```

Use bv instead of parsing beads.jsonl directly—it computes PageRank, critical paths, and parallel tracks deterministically.

## Parallel Agent Workflow (Orchestration Pattern)

For high-throughput parallel work, use this multi-agent workflow orchestrated through subagents:

### The Complete Pipeline

Each autonomous agent follows this workflow from triage to merge:

```bash
# Step 1: TRIAGE - Find what to work on
bv --robot-triage --robot-triage-by-track  # Get parallel execution tracks
# OR for single issue:
bv --robot-next  # Get top recommendation + claim command

# Step 2: CLAIM - Reserve the bead
br update <bead-id> --status in_progress

# Step 3: ISOLATE - Create isolated workspace
# Use zjj skill to spawn isolated JJ workspace + Zellij tab
zjj add <session-name>

# Step 4: IMPLEMENT - Build with functional patterns
# For Rust: functional-rust-generator skill
# Implements with: zero panics, zero unwraps, Railway-Oriented Programming

# Step 5: REVIEW - Adversarial QA
# Use red-queen skill for evolutionary testing
# Drives regression hunting and quality gates

# Step 6: LAND - Finalize and push
# Use land skill for mandatory quality gates:
# - Moon quick check (6-7ms cached)
# - jj commit with proper message
# - br sync --flush-only
# - git add .beads/ && git commit -m "sync beads"
# - jj git push (MANDATORY - work not done until pushed)

# Step 7: MERGE - Reintegrate to main
# Use zjj skill to merge workspace back to main
# This handles: jj rebase -d main, cleanup, tab switching
```

### Orchestrator Responsibilities

As orchestrator, your job is to:
1. **Keep context clean** - Delegate work to subagents, don't implement yourself
2. **Monitor progress** - Use `TaskOutput` to check agent status without loading full context
3. **Handle failures** - Spawn replacement agents if needed
4. **Track completion** - Verify each agent completes all 7 steps
5. **Report summary** - Provide final status of all beads completed

### Subagent Prompt Template

```markdown
You are a parallel autonomous agent. Complete this workflow:

**BEAD TO WORK ON**: <bead-id> - "<title>"

**WORKFLOW**:
1. CLAIM: `br update <bead-id> --status in_progress`
2. ISOLATE: Use the zjj skill to spawn an isolated workspace named "<session-name>"
3. IMPLEMENT: Use functional-rust-generator skill
   - Zero unwraps, zero panics
   - Railway-Oriented Programming
   - Functional patterns (map, and_then, ? operator)
4. REVIEW: Use red-queen skill for adversarial QA
5. LAND: Use land skill to finalize (quality gates, sync, push)
6. MERGE: Use zjj skill to merge back to main

**CRITICAL CONSTRAINTS**:
- Zero unwraps, zero panics
- Use jj for version control (NEVER raw git commands)
- Use Moon for builds (NEVER raw cargo commands)
- Work is NOT done until jj git push succeeds

Report your final status with the bead ID.
```

### Parallel Execution Example

```bash
# Run bv triage to get parallel tracks
bv --robot-triage --robot-triage-by-track

# Spawn 8 parallel agents using Task tool
# Each gets unique bead from different track
# All run simultaneously in isolated workspaces
# Orchestrator monitors from clean context
```

### Key Benefits

- **Isolation**: Each agent works in separate JJ workspace
- **Parallel**: 8x throughput with no conflicts
- **Deterministic**: bv precomputes dependencies and execution tracks
- **Quality**: Red-queen ensures adversarial testing on each change
- **Clean handoff**: land skill guarantees all work pushed before completion

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `jj git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed):
   ```bash
   moon run :quick  # Fast check (6-7ms)
   # OR for full validation:
   moon run :ci     # Complete pipeline
   ```
3. **Update issue status** - Close finished work, update in-progress items
4. **COMMIT AND PUSH** - This is MANDATORY:
   ```bash
   jj commit -m "description"  # jj auto-tracks changes, no 'add' needed
   br sync --flush-only        # Export beads to JSONL
   git add .beads/
   git commit -m "sync beads"
   jj git fetch                # Fetch from remote (auto-rebases)
   jj git push                 # Push to remote
   jj status                   # MUST show clean working copy
   ```
5. **Verify cache health**:
   ```bash
   systemctl --user is-active bazel-remote  # Should be "active"
   ```
6. **Clean up** - Clear abandoned workspaces
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `jj git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
- Always use jj for version control (NEVER raw git commands)
- Always use Moon for builds (NEVER raw cargo commands)
- YOU ARE TO NEVER TOUCH CLIPPY SETTINGS EVER


