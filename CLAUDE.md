# CLAUDE.md - Project Instructions for Claude Code

## Critical Rules

### NEVER Touch Clippy/Lint Configuration
**ABSOLUTE RULE: DO NOT MODIFY clippy or linting configuration files. EVER.**

This includes but is not limited to:
- `.clippy.toml`
- `clippy.toml`
- Any `#![allow(...)]` or `#![deny(...)]` attributes in `lib.rs` or `main.rs`
- Clippy-related sections in `Cargo.toml`
- Any lint configuration in `moon.yml` or build scripts

If clippy reports warnings or errors, fix the **code**, not the lint rules.
The user has explicitly configured these rules. Do not second-guess them.

### Build System: Moon Only
**NEVER use raw cargo commands.** Always use Moon for all build operations:

```bash
# Correct
moon run :quick      # Format + lint check
moon run :test       # Run tests
moon run :build      # Release build
moon run :ci         # Full pipeline
moon run :fmt-fix    # Auto-fix formatting
moon run :check      # Fast type check

# WRONG - Never do this
cargo fmt            # NO
cargo clippy         # NO
cargo test           # NO
cargo build          # NO
```

### Code Quality
- Zero unwraps: `unwrap()` and `expect()` are forbidden
- Zero panics: `panic!`, `todo!`, `unimplemented!` are forbidden
- All errors must use `Result<T, Error>` with proper propagation
- Use functional patterns: `map`, `and_then`, `?` operator

### Project Structure
```
crates/
  factory-core/   # Core library (error handling, types, functional utils)
  factory/        # CLI binary (new, stage, approve, show, list)
```

### MVP Commands
1. `factory new -s <slug>` - Create task with JJ workspace
2. `factory stage -s <slug> --stage <name>` - Run pipeline stage
3. `factory approve -s <slug>` - Mark task for integration
4. `factory show -s <slug>` - Show task details
5. `factory list` - List all tasks

### Key Decisions
- **Workspace isolation**: JJ workspaces (`jj workspace add`)
- **Task storage**: `.factory/tasks.json`
- **Beads**: Hard requirement, always integrate with `.beads/`
- **Stages**: implement, unit-test, coverage, lint, static, integration, security, review, accept

### Dependencies
- JJ (Jujutsu) for workspace management and version control
- Beads for issue tracking integration
- Language-specific tooling per stage

### Version Control: Jujutsu (jj) Only
**NEVER use git commands directly.** This project uses Jujutsu (jj) for all version control:

```bash
# Correct - Use jj
jj status              # Show working copy status
jj commit -m "msg"     # Create commit
jj git fetch           # Fetch from remote (auto-rebases)
jj git push            # Push to remote
jj log                 # View commit history
jj diff                # Show changes

# WRONG - Never use git directly
git status             # NO
git commit             # NO
git pull               # NO
git push               # NO
```

**Why jj**: Automatic conflict resolution, first-class workspace isolation, better UX.

---

## Quick Reference

### Issue Tracking (Beads)
```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --status in_progress  # Claim work
bd close <id>         # Complete work
bd sync               # Sync with jj (Jujutsu)
```

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

**Why**: Moon provides:
- Persistent remote caching (survives `moon clean`)
- Parallel task execution
- Dependency-aware rebuilds
- 98.5% faster with cache hits

---

## Using bv as an AI Sidecar

bv is a graph-aware triage engine for Beads projects (.beads/beads.jsonl). Instead of parsing JSONL or hallucinating graph traversal, use robot flags for deterministic, dependency-aware outputs with precomputed metrics (PageRank, betweenness, critical path, cycles, HITS, eigenvector, k-core).

**CRITICAL: Use ONLY `--robot-*` flags. Bare `bv` launches an interactive TUI that blocks your session.**

### The Workflow: Start With Triage

**`bv --robot-triage` is your single entry point.** It returns everything you need in one call:
- `quick_ref`: at-a-glance counts + top 3 picks
- `recommendations`: ranked actionable items with scores, reasons, unblock info
- `quick_wins`: low-effort high-impact items
- `blockers_to_clear`: items that unblock the most downstream work
- `project_health`: status/type/priority distributions, graph metrics
- `commands`: copy-paste shell commands for next steps

```bash
bv --robot-triage        # THE MEGA-COMMAND: start here
bv --robot-next          # Minimal: just the single top pick + claim command
```

---

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
bd update <bead-id> --status in_progress

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
# - bd sync
# - jj git push (MANDATORY - work not done until pushed)

# Step 7: MERGE - Reintegrate to main
# Use zjj skill to merge workspace back to main
# This handles: jj rebase -d main, cleanup, tab switching
```

---

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
   bd sync                     # Sync beads with jj
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
