# Oya CLI Enhancement Design

**Date**: 2026-02-18
**Status**: Approved
**Goal**: Single-binary CLI with bundled Restate, workspace isolation, and full development velocity tooling

## Overview

Transform Oya from a multi-dependency orchestration tool into a self-contained binary that bundles everything needed for autonomous development workflows. The CLI becomes the single entry point for session management, pipeline execution, workspace isolation, bead management, and quality gates.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    OYA BINARY (one file)                    │
├─────────────────────────────────────────────────────────────┤
│  CLI (clap)                                                 │
│    up/down/status/reset • run/watch • bead • workspace      │
│    check • session • serve                                  │
├─────────────────────────────────────────────────────────────┤
│  BUNDLED RESTATE BINARY                                     │
│    Extracted to ~/.oya/bin/ on first run                    │
│    Spawned as subprocess by `oya up`                        │
├─────────────────────────────────────────────────────────────┤
│  RESTATE SERVICES (via restate-sdk)                         │
│    Oya • Oya • Oya        │
│    OyaWorkspaceManager • OyaMergeQueue (NEW)                │
├─────────────────────────────────────────────────────────────┤
│  NATIVE RUST                                                │
│    Systemd (zbus) • HTTP (reqwest) • Config                 │
├─────────────────────────────────────────────────────────────┤
│  SUBPROCESS CALLS                                           │
│    jj • br • moon • opencode                                │
└─────────────────────────────────────────────────────────────┘
```

## CLI Surface

```
oya
│
├── SESSION MANAGEMENT (Priority 1)
│   ├── up                      # Start all (extract/spawn Restate, start Oya)
│   ├── down                    # Graceful shutdown
│   ├── status                  # Health check table
│   ├── logs [-f]               # Tail logs
│   └── reset                   # Clear all state, restart fresh
│
├── PIPELINE (Priority 4)
│   ├── run <bead_id>           # Execute workflow
│   └── watch <bead_id>         # TUI progress viewer
│
├── WORKSPACE (Priority 2 - new Restate services)
│   ├── list                    # List all workspaces
│   ├── create <name> [--bead]  # Create isolated workspace
│   ├── sync <name>             # Rebase onto main
│   ├── done <name>             # Merge to main, remove workspace
│   ├── abort <name>            # Abandon without merge
│   └── status [<name>]         # Show workspace/queue status
│
├── BEAD (Priority 2)
│   ├── list [--ready]          # List beads
│   ├── show <id>               # Bead details
│   ├── next                    # Next ready bead
│   └── create                  # Interactive creation
│
├── QUALITY GATES (Priority 3)
│   └── check [--quick|--ci|--test]
│
├── SESSION (OpenCode)
│   ├── status                  # Session state
│   └── clear                   # Clear pending items
│
└── serve                       # Start Restate services (internal)
```

## New Restate Services

### OyaWorkspaceManager (Virtual Object)

Per-workspace state and operations. Replaces jj subprocess calls with durable Restate handlers.

```rust
service OyaWorkspaceManager /{workspace_id}/ {
    handler create(req: CreateRequest) -> Result<Workspace, HandlerError>;
    handler sync() -> Result<SyncResult, HandlerError>;
    handler done() -> Result<MergeResult, HandlerError>;
    handler abort() -> Result<(), HandlerError>;
    handler status() -> WorkspaceStatus;
}
```

### OyaMergeQueue (Singleton)

Coordinates merge order across workspaces.

```rust
service OyaMergeQueue {
    handler add(req: QueueAddRequest) -> Result<QueueEntry, HandlerError>;
    handler process_next() -> Result<Option<QueueEntry>, HandlerError>;
    handler list() -> Vec<QueueEntry>;
    handler remove(entry_id: String) -> Result<(), HandlerError>;
}
```

## Types

```rust
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub path: PathBuf,
    pub bead_id: Option<BeadId>,
    pub status: WorkspaceStatus,
    pub created_at: DateTime<Utc>,
    pub branch: String,
}

pub enum WorkspaceStatus {
    Creating,
    Active,
    Syncing,
    Merging,
    Completed,
    Failed { reason: String },
}

pub struct QueueEntry {
    pub id: String,
    pub workspace_id: WorkspaceId,
    pub bead_id: Option<BeadId>,
    pub priority: u32,
    pub status: QueueStatus,
    pub created_at: DateTime<Utc>,
}
```

## Bundled Restate

### Build Integration

```rust
// build.rs
fn main() {
    // Download restate-server binary for target platform
    // Verify checksum
    // Tell cargo to include it
}

// src/binary.rs
const RESTATE_BINARY: &[u8] = include_bytes!("../bundled/restate-server");

pub fn extract_restate() -> Result<PathBuf, OyaError> {
    let dir = dirs::data_dir()
        .ok_or_else(|| OyaError::Config("Cannot determine data directory".into()))?
        .join(".oya")
        .join("bin");

    fs::create_dir_all(&dir)?;
    let binary_path = dir.join("restate-server");

    if !binary_path.exists() {
        fs::write(&binary_path, RESTATE_BINARY)?;
        set_permissions(&binary_path)?;
    }

    Ok(binary_path)
}
```

### Process Management

```rust
pub struct RestateProcess {
    child: Option<Child>,
    binary_path: PathBuf,
}

impl RestateProcess {
    pub fn start(&mut self) -> Result<(), OyaError> {
        let child = Command::new(&self.binary_path)
            .arg("run")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        self.child = Some(child);
        self.wait_for_health(Duration::from_secs(30))?;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), OyaError> {
        if let Some(mut child) = self.child.take() {
            child.kill()?;
            child.wait()?;
        }
        Ok(())
    }
}
```

## Session Management (oya up/down/status)

### `oya up`

1. Extract bundled restate-server to `~/.oya/bin/`
2. Spawn restate-server subprocess
3. Poll health endpoint until ready (30s timeout)
4. Start Oya services (register with Restate)
5. Write PID file for process management

### `oya down`

1. Unregister Oya services from Restate
2. Send SIGTERM to restate-server
3. Wait for graceful shutdown (10s timeout)
4. Kill if still running
5. Clean up PID file

### `oya status`

```
┌─────────────────────────────────────────────────────────┐
│ OYA STATUS                                    14:32:01  │
├─────────────────────────────────────────────────────────┤
│ Service          Status    Port     Uptime              │
├─────────────────────────────────────────────────────────┤
│ Restate          ● running  8080     2h 14m             │
│ Oya  ● running  9080     2h 14m             │
│ OpenCode         ● running  4097     4h 32m             │
│ Workspace Queue  ○ empty    -        -                  │
├─────────────────────────────────────────────────────────┤
│ Active Workspaces: 2                                    │
│   oya-run-abc123-tdd15-a0  (TDD15 stage, bead: src-f7) │
│   oya-run-def456-qa-a1     (QA stage, bead: src-e3)    │
└─────────────────────────────────────────────────────────┘
```

## Workspace Isolation (replaces jj)

Drops Zellij integration (agents run headless). Retains JJ subprocess for workspace operations.

### Workspace Lifecycle

```
create → [Creating] → Active → sync → [Syncing] → Active
                                    ↓
                              done → [Merging] → Completed → (deleted)
                                    ↓
                              abort → (deleted)
```

### Integration with Pipeline

Oya calls OyaWorkspaceManager instead of shelling to jj:

```rust
// Before (subprocess)
let workspace = format!("oya-{}-{}-a{}", run_id, stage, attempt);
Command::new("jj").args(["add", &workspace]).spawn()?;

// After (Restate call)
let workspace_id = WorkspaceId::new(&format!("oya-{}-{}-a{}", run_id, stage, attempt));
ctx.service_client()
    .service("OyaWorkspaceManager")
    .handler("create")
    .call(CreateRequest { workspace_id, bead_id })?;
```

## Quality Gates (oya check)

Thin wrapper around moon with structured output:

```rust
pub enum CheckMode {
    Quick,  // moon run :quick
    Ci,     // moon run :ci (default)
    Test,   // moon run :test
}

pub fn run_check(mode: CheckMode) -> Result<CheckResult, OyaError> {
    let output = Command::new("moon")
        .args(["run", &format!(":{}", mode.as_str())])
        .output()?;

    CheckResult::parse(output)
}
```

## Pipeline Visibility (oya watch)

TUI using ratatui crate:

```
┌─────────────────────────────────────────────────────────┐
│ OYA WATCH: src-abc123                        14:32:01  │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ✓ PLAN          2m 14s    Gates: Compiles ✓           │
│  ✓ CONTRACT      1m 32s    Gates: Compiles ✓           │
│  ◉ TDD15         4m 01s    Gates: Compiles ✓ Tests ◉   │
│  ○ QA            -         Waiting...                   │
│  ○ RED_QUEEN     -                                       │
│  ○ GPT_REVIEW    -                                       │
│  ○ SHIP_GATE     -                                       │
│                                                         │
├─────────────────────────────────────────────────────────┤
│ Logs (last 5):                                          │
│   [14:31:58] Running cargo test                         │
│   [14:32:01] test result: 23 passed, 0 failed           │
│   [14:32:01] Gate TestsPass: PASSED                     │
└─────────────────────────────────────────────────────────┘
```

## Dependencies

### New Crates

| Crate | Purpose |
|-------|---------|
| `ratatui` | TUI for `oya watch` |
| `crossterm` | Terminal handling |
| `zbus` | Systemd integration |
| `dirs` | XDG directory paths |

### Bundled

| Binary | Source |
|--------|--------|
| `restate-server` | Downloaded at build time, bundled via `include_bytes!` |

## Implementation Phases

### Phase 1: Session Management
- Bundle restate-server binary
- `oya up/down/status/reset` commands
- Process management (spawn, health check, shutdown)
- PID file handling

### Phase 2: Workspace Services
- `OyaWorkspaceManager` Restate service
- `OyaMergeQueue` Restate service
- `oya workspace` CLI commands
- Integration with Oya

### Phase 3: Bead & Quality Gates
- `oya bead` commands (br wrapper)
- `oya check` command (moon wrapper)
- Structured output parsing

### Phase 4: Pipeline Visibility
- `oya watch` TUI
- Real-time stage progress
- Log streaming
- Gate status display

## Migration Path

1. **Parallel run**: New CLI commands coexist with existing scripts
2. **Feature parity**: All current workflows work via CLI
3. **Deprecation**: Scripts become thin wrappers calling CLI
4. **Removal**: Scripts deleted, CLI is sole interface

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Restate binary size (~50MB) | Accept as cost of self-containment |
| Binary extraction permissions | Use user-writable dirs only |
| Restate version drift | Pin version, update deliberately |
| Process orphaning | PID files, cleanup on startup |

## Success Criteria

- [ ] `oya up` starts everything from single binary
- [ ] `oya status` shows all service health
- [ ] `oya workspace` creates isolated workspaces via Restate
- [ ] `oya check` runs quality gates
- [ ] `oya watch` shows live pipeline progress
- [ ] Zero external dependencies (except jj, br, moon, opencode)
