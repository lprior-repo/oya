# CLAUDE.md - Project Instructions for Claude Code

## Critical Rules

### ALWAYS Use Moon - NEVER Use Cargo Directly
**ABSOLUTE RULE: ALL build operations MUST go through Moon. NEVER use raw cargo commands.**

```bash
# ✅ CORRECT - Always use Moon
moon run :quick      # Format + lint check (parallel, cached)
moon run :clippy     # Lint with strict clippy rules
moon run :fmt        # Check formatting
moon run :fmt-fix    # Auto-fix formatting
moon run :test       # Run all tests (workspace-wide)
moon run :test-doc   # Run documentation tests
moon run :build      # Release build (cached)
moon run :check      # Fast type check
moon run :ci         # Full pipeline (all checks in parallel)
moon run :quality    # All quality gates
moon run :install    # Install binaries to ~/.local/bin

# ❌ WRONG - NEVER do this
cargo fmt            # NO
cargo clippy         # NO
cargo test           # NO
cargo build          # NO
cargo check          # NO
cargo clean          # NO - use `moon run :clean` instead
```

**Why Moon?**
- Persistent caching across sessions (runs in ms when cached)
- Parallel task execution
- Workspace-wide operations (all crates together)
- Dependency-aware rebuilds

### NEVER Touch Clippy/Lint Configuration
**ABSOLUTE RULE: DO NOT MODIFY clippy or linting configuration files. EVER.**

This includes but is not limited to:
- `.clippy.toml`
- `clippy.toml`
- Any `#![allow(...)]` or `#![deny(...)]` attributes in `lib.rs` or `main.rs`
- Clippy-related sections in `Cargo.toml`
- Any lint configuration in `.moon` or build scripts

If clippy reports warnings or errors, fix the **code**, not the lint rules.
The user has explicitly configured these rules. Do not second-guess them.

### Code Quality: Pure Functional Rust
**Zero panics in our code, explicit error handling:**
- `unwrap()` and `expect()` are **ABSOLUTELY FORBIDDEN** - everywhere, always
- `panic!`, `todo!`, `unimplemented!` are **ABSOLUTELY FORBIDDEN** - everywhere, always
- All errors must use `Result<T, Error>` with proper propagation
- Use functional patterns: `map`, `and_then`, `?` operator
- For arithmetic: use `saturating_add`, `saturating_sub`, `checked_*` methods

**Note:** Dependencies (e.g., spider-rs) may have panics; we validate inputs to avoid triggering them.

**NO EXCEPTIONS** - Not in tests, not in examples, not in ANY code. Tests should use:
- `assert!(matches!(result, Ok(value)))` for Ok results
- `assert!(matches!(result, Err(e)))` for errors
- `assert_eq!(result, Ok(expected))` for equality checks
- Proper match patterns for Option/Result extraction

### Project Structure
```
centralized-docs/
├── doc_transformer/       # Main library (indexing, search)
├── contextual-chunker/    # Semantic document chunking
├── llms-txt-parser/       # llms.txt file format parsing
└── Cargo.toml             # Workspace configuration
```

### Key Architectural Decisions
- **HNSW Index**: O(log n) similarity search with cosine distance
- **Tantivy**: Full-text search with BM25 ranking
- **Knowledge Graph**: DAG-based document relationships (petgraph)
- **Semantic Chunking**: Context-aware token-based splits

### Dependencies
- `hnsw_rs`: Approximate nearest neighbor search
- `tantivy`: Full-text search engine
- `petgraph`: Graph algorithms for document relationships
- `pulldown-cmark`: CommonMark parsing with AST
- `spider`: Web scraping with content extraction

### Quality Gates
All code must pass (using Moon):
1. `moon run :fmt` - Code formatting check
2. `moon run :clippy` - Zero warnings (strict mode)
3. `moon run :test` - All tests pass
4. `moon run :test-doc` - Documentation tests pass

Or simply run: `moon run :ci` for everything in parallel.

### Issue Tracking (bd) and Workspace Isolation (zjj)

This project uses **bd** (beads) for issue tracking and **zjj** for workspace isolation.

#### bd (Beads Issue Tracker)

```bash
# Finding Work
bd ready              # Show issues ready to work (no blockers)
bd list --status=open # All open issues
bd show <id>          # Detailed issue view with dependencies

# Creating & Updating
bd create --title="..." --type=task|bug|feature --priority=2  # New issue
bd update <id> --status=in_progress                           # Claim work
bd close <id>         # Mark complete
bd close <id1> <id2> ...  # Close multiple issues at once
bd close <id> --reason="explanation"  # Close with reason

# Dependencies & Blocking
bd dep add <issue> <depends-on>  # Add dependency
bd blocked            # Show all blocked issues

# Sync & Collaboration
bd sync               # Sync with git remote (run at session end)
bd sync --status      # Check sync status without syncing

# Project Health
bd stats              # Project statistics
bd doctor             # Check for issues
```

**Priority Levels:** 0-4 or P0-P4 (0=critical, 2=medium, 4=backlog). NOT "high"/"medium"/"low".

#### zjj (Workspace Isolation)

**zjj provides isolated jj workspaces with Zellij session integration for parallel development.**

```bash
# Workspace Management
zjj init <bead-id>    # Create new isolated workspace for a bead
zjj list              # List all workspaces
zjj status            # Show workspace status
zjj add <bead-id>     # Add bead to existing workspace

# Workspace Operations
zjj focus <name>      # Focus/attach to workspace
zjj sync <name>       # Sync workspace changes
zjj diff <name>       # Show workspace diff
zjj done <name>       # Complete and land workspace
zjj remove <name>     # Remove workspace

# Advanced Operations
zjj spawn <bead-id>   # Create workspace with full dev environment
zjj dashboard         # Show all workspaces in interactive view
zjj clean             # Clean up completed/abandoned workspaces
zjj doctor            # Diagnose workspace issues
zjj query             # Query workspaces with filters
zjj attach <name>     # Attach to existing Zellij session
```

**Common Workflow:**
```bash
# Start work on a bead
zjj init beads-123    # Create isolated workspace
# ... work in isolation ...
zjj sync beads-123    # Sync changes
zjj done beads-123    # Land and clean up
```

**Why zjj?** Enables true parallel development with jj's change isolation and Zellij's session management.
