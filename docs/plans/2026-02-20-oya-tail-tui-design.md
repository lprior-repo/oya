# Oya Tail TUI Design

## Overview

A Ratatui-based TUI for watching Restate pipeline invocations in real-time. Designed for clarity and focus - one invocation dominates the screen, with clear visual hierarchy and minimal chrome.

## User Needs

1. **At a glance**: Is it working? Stuck? Failed?
2. **Current action**: What's happening right now
3. **Problems**: What needs attention

## Design Principles

- Content first, chrome last
- One thing dominates the screen
- Whitespace is a feature
- Color used sparingly and meaningfully
- Information hierarchy: most important → least important

## UI Layout

### Single Invocation View (Primary)

```
                    fix-20260219-211035

                           RUNNING

                        gptreview 1/3

                     ━━━━━━━━━━━━━━━━━━

            ✅ check        ✅ test

                 ⏳ clippy 35s

                     ━━━━━━━━━━━━━━━━━━

    oya:clippy | Checking oya v0.1.0
    oya:clippy |     src/executor.rs:69:49
    oya:clippy |     warning: unused import

                     ━━━━━━━━━━━━━━━━━━

              running for 2m 33s · est. 4m
```

### Failed State

```
                    fix-20260219-211035

                           FAILED

                        shipgate 3/3

                     ━━━━━━━━━━━━━━━━━━

            ✅ check   ✅ test   ✅ clippy
            ✅ fmt     ❌ jj:sync

                     ━━━━━━━━━━━━━━━━━━

    jj sync failed: uncommitted changes

    M src/pipeline/executor.rs
    M src/pipeline/mod.rs

                     ━━━━━━━━━━━━━━━━━━

              failed after 7m · retry? [r]
```

### Multiple Invocations

When multiple invocations exist, show a compact list:

```
    fix-20260219-211035    RUNNING     gptreview 1/3    2m ago
    feat-20260219-180022   COMPLETED   shipped          1h ago
    fix-20260218-143055    FAILED      shipgate 3/3     3h ago

    [enter] focus   [↑↓] navigate   [q] quit
```

## Components

### Header Block
- Run ID (centered, prominent)
- Status badge (RUNNING/COMPLETED/FAILED with color)
- Stage name + attempt count

### Progress Section
- Gates as icons with status (✅ passed, ⏳ running, ❌ failed)
- Current gate shows elapsed time

### Output Section
- Live scroll of command output
- Truncate intelligently (show last N lines)
- Preserve important lines (errors, warnings)

### Footer
- Total elapsed time
- Estimated completion (if available)
- Keybindings hint

## Color Scheme

| Status | Color |
|--------|-------|
| RUNNING | Yellow/Amber |
| COMPLETED | Green |
| FAILED | Red |
| Default text | White/Default |
| Dimmed info | Gray |

## Interactions

| Key | Action |
|-----|--------|
| `q` | Quit |
| `↑`/`↓` | Navigate invocations (multi-view) |
| `Enter` | Focus on selected invocation |
| `Esc` | Return to list view |
| `r` | Retry failed invocation |
| `f` | Toggle follow mode (auto-scroll to newest) |

## Data Source

Query Restate SQL endpoint:

```
POST http://127.0.0.1:9070/query
{"query": "SELECT * FROM sys_invocation WHERE target_service_name = 'OyaOrchestrator' ORDER BY modified_at DESC"}
```

Parse nested JSON from:
- `completion_failure` - contains gate results, output, errors
- Orchestrator state (may need additional query)

## Refresh Strategy

- Poll every 2 seconds by default
- Configurable via `--interval` flag
- Smooth updates (no screen flash)

## CLI Interface

```bash
oya tail                    # Watch all invocations
oya tail --run-id fix-123   # Focus on specific run
oya tail --follow           # Auto-follow newest
oya tail --interval 5       # Refresh every 5s
```

## Dependencies

- `ratatui` - TUI framework
- `crossterm` - Terminal backend (cross-platform)
- `tokio` - Async runtime for polling
- `reqwest` - HTTP client for Restate queries
- `serde`/`serde_json` - JSON parsing

## File Structure

```
src/
  tail/
    mod.rs           # Entry point, App struct
    app.rs           # Application state
    ui.rs            # Ratatui rendering
    restate.rs       # Restate query client
    parser.rs        # Parse invocation JSON + nested data
main.rs              # Add `Tail` subcommand
```
