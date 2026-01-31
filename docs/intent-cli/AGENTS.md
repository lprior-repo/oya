# Intent CLI - AI Agent Instructions

**Documentation:** See `.beads/AGENTS.jsonl` for complete reference (JSONL format for token efficiency).

**Tagline:** Human-writes, AI-verifies, AI-implements.

**Quick start:**
```bash
bd ready                    # Find unblocked work
bv --robot-triage          # AI triage (everything in one call)
bv --robot-next            # Single top pick

# Workspace isolation
zjj add <bead-id>          # Create isolated workspace
zjj done <workspace>       # Complete and merge

# Session completion (MANDATORY)
bd sync --from-main  # Use bd sync, not git (ephemeral branch workflow)
```

**Tech Stack:** Rust (functional, zero-unwrap), Moon build system, CUE for specifications
