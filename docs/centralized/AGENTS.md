# Agent Instructions

This project uses **bd** (beads) for issue tracking, **zjj** for workspace isolation, and **Moon** for ALL build operations.

## Quick Reference

### Issue Tracking (bd - Beads)
```bash
bd ready              # Find available work (no blockers)
bd list --status=open # All open issues
bd show <id>          # View issue details with dependencies
bd update <id> --status in_progress  # Claim work
bd close <id>         # Complete work
bd close <id1> <id2> ...  # Close multiple at once
bd close <id> --reason="explanation"  # Close with reason
bd sync               # Sync with git remote
bd sync --status      # Check sync status without syncing

# Dependencies
bd dep add <issue> <depends-on>  # Add dependency
bd blocked            # Show all blocked issues

# Project Health
bd stats              # Project statistics
bd doctor             # Check for issues
```

**Priority Levels:** 0-4 or P0-P4 (0=critical, 2=medium, 4=backlog). NOT "high"/"medium"/"low".

### Workspace Isolation (zjj)
```bash
zjj init <bead-id>    # Create isolated workspace for a bead
zjj list              # List all workspaces
zjj status            # Show workspace status
zjj focus <name>      # Attach to workspace
zjj sync <name>       # Sync workspace changes
zjj diff <name>       # Show workspace diff
zjj done <name>       # Complete and land workspace
zjj remove <name>     # Remove workspace

# Advanced
zjj spawn <bead-id>   # Create with full dev environment
zjj dashboard         # Interactive workspace view
zjj clean             # Clean up completed workspaces
zjj doctor            # Diagnose workspace issues
zjj query             # Query workspaces with filters
```

### Development (Moon - ALWAYS)
```bash
moon run :quick       # Fast checks (fmt + clippy in parallel)
moon run :ci          # Full pipeline (all checks in parallel)
moon run :fmt-fix     # Auto-fix formatting
moon run :build       # Release build
moon run :install     # Install to ~/.local/bin
moon run :quality     # All quality gates
moon run :test        # Run tests
moon run :check       # Fast type check
moon run :clean       # Clean build artifacts
```

## MOON ONLY - Build System Rule

**CRITICAL: NEVER use raw cargo commands. ALWAYS use Moon.**

```bash
# ✅ CORRECT - Always use Moon
moon run :build    # Build (cached, fast, workspace-aware)
moon run :test     # Test (workspace-wide, cached)
moon run :check    # Type check (cached)
moon run :clean    # Clean artifacts

# ❌ WRONG - NEVER do this
cargo build        # NO - no caching, slow
cargo test         # NO - no workspace awareness
cargo check        # NO - no caching
cargo clean        # NO - use moon run :clean
```

**Why Moon Only?**
- Persistent caching across sessions (cached tasks run in milliseconds)
- Parallel task execution (runs fmt + clippy + test simultaneously)
- Workspace-wide operations (all crates built together)
- Dependency-aware rebuilds (only rebuilds what changed)

## Development Workflow

**1. Quick Iteration Loop:**
```bash
# Edit code...
moon run :quick  # Parallel fmt + clippy check (~ms with cache)
```

**2. Before Committing:**
```bash
moon run :fmt-fix  # Auto-fix formatting
moon run :ci       # Full pipeline (all checks in parallel)
```

**3. Build and Install:**
```bash
moon run :build    # Release build (cached)
moon run :install  # Install to ~/.local/bin
```

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed):
   ```bash
   moon run :quick  # Fast check (fmt + clippy)
   # OR for full validation:
   moon run :ci     # Complete pipeline
   ```
3. **Update issue status** - Close finished work, update in-progress items
4. **COMMIT AND PUSH** - This is MANDATORY:
   ```bash
   git add <files>
   git commit -m "description"
   bd sync  # Sync beads
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
- **ALWAYS use Moon for builds - NEVER use cargo directly**
