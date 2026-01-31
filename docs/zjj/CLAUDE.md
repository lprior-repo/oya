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
  zjj-core/     # Core library (error handling, types, functional utils)
  zjj/          # CLI binary (MVP: init, add, list, remove, focus)
```

### MVP Commands
1. `jjz init` - Initialize jjz in a JJ repository
2. `jjz add <name>` - Create session with JJ workspace + Zellij tab
3. `jjz list` - Show all sessions
4. `jjz remove <name>` - Cleanup session and workspace
5. `jjz focus <name>` - Switch to session's Zellij tab

### Key Decisions
- **Sync strategy**: Rebase (`jj rebase -d main`)
- **Zellij tab naming**: `jjz:<session-name>`
- **Beads**: Hard requirement, always integrate with `.beads/beads.db`
- **jjz runs inside Zellij**: Tab switching via `zellij action go-to-tab-name`

### Dependencies
- JJ (Jujutsu) for workspace management
- Zellij for terminal multiplexing
- Beads for issue tracking integration
- SQLite for session state persistence
