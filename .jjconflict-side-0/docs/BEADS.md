# Beads (br) - Complete Reference Guide

> Agent-first issue tracking system for oya project
> Version: 0.1.12 | Repository: https://github.com/Dicklesworthstone/beads_rust

---

## Quick Reference

```bash
# Start working
br ready                           # Find actionable work
br show <id>                      # View full details
br update <id> --status in_progress  # Claim work

# Complete work
br close <id> --reason "Done"      # Close with reason
br sync --flush-only                 # Export to JSONL
git add .beads/ && git commit -m "..."  # Commit

# Find issues
br list --status open --priority 0-1  # High priority open issues
br search "authentication"            # Full-text search
br blocked                          # Show blocked issues
```

---

## Table of Contents

1. [Installation & Setup](#installation--setup)
2. [Core Workflow](#core-workflow)
3. [Command Reference](#command-reference)
4. [Issue Types & Priority](#issue-types--priority)
5. [Dependencies](#dependencies)
6. [Labels](#labels)
7. [Searching & Filtering](#searching--filtering)
8. [Epics](#epics)
9. [Comments](#comments)
10. [Sync & Git Integration](#sync--git-integration)
11. [Saved Queries](#saved-queries)
12. [Visualization](#visualization)
13. [Configuration](#configuration)
14. [Output Modes](#output-modes)
15. [Agent Integration](#agent-integration)
16. [Best Practices](#best-practices)
17. [Common Patterns](#common-patterns)
18. [Troubleshooting](#troubleshooting)

---

## Installation & Setup

```bash
# Install from releases (recommended)
curl -fsSL "https://raw.githubusercontent.com/Dicklesworthstone/beads_rust/main/install.sh?$(date +%s)" | bash

# Or via cargo
cargo install beads_rust

# Verify installation
br --version
# br 0.1.12 (rustc 1.85.0-nightly)
```

### Initialize in Your Project

```bash
cd /path/to/oya
br init
# Initialized beads workspace in .beads/
```

**Directory Structure:**
```
.beads/
├── beads.db           # SQLite database (primary storage)
├── issues.jsonl       # Git-friendly export (for sync)
├── config.yaml        # Project configuration
└── metadata.json      # Workspace metadata
```

### Add AGENTS.md Instructions

```bash
# Add beads workflow instructions to AGENTS.md
br agents --add

# Check if already added
br agents --check

# Update to latest version
br agents --update

# Remove instructions
br agents --remove
```

---

## Core Workflow

### The Complete Issue Lifecycle

```bash
# 1. CREATE - Add new issue
br create --title="Fix login timeout" --type=bug --priority=1 --description="Users report login times out after 30s"
# Created: src-abc123

# 2. READY - Find actionable work
br ready
# Shows: src-abc123 P1 bug Fix login timeout

# 3. CLAIM - Start working
br update src-abc123 --status in_progress

# 4. ADD DEPENDENCY - If blocked by another issue
br dep add src-abc123 src-def456

# 5. WORK - Implement the fix
# ... (your code changes) ...

# 6. COMPLETE - Close when done
br close src-abc123 --reason="Increased timeout to 60s, added retry logic"

# 7. SYNC - Export for git
br sync --flush-only
git add .beads/
git commit -m "Fix: login timeout (src-abc123)"
```

### Quick Capture (Minimalist)

```bash
# Create issue quickly, get ID only
br q --title="Quick task"
# src-xyz789
```

### Interactive Creation

```bash
# Fill in form interactively
br create-form

# Prompts for:
# - Title
# - Description
# - Type (task, bug, feature, etc.)
# - Priority (0-4)
# - Assignee (optional)
# - Labels (optional)
```

---

## Command Reference

### Issue Lifecycle Commands

#### `br init` - Initialize Workspace

```bash
br init [--prefix PREFIX]
```

**Options:**
- `--prefix PREFIX` - Set issue ID prefix (default: "bd", oya uses "src")

**Example:**
```bash
br init --prefix src
# Creates .beads/ with prefix configuration
```

---

#### `br create` - Create New Issue

```bash
br create [OPTIONS] --title "TITLE"
```

**Options:**
- `--title "TITLE"` - Issue title (required)
- `--description "DESC"` - Detailed description
- `--design "DESIGN"` - Technical design notes
- `--acceptance-criteria "CRITERIA"` - Acceptance criteria
- `--notes "NOTES"` - Additional notes
- `--type TYPE` - Issue type (task, bug, feature, epic, chore, docs, question)
- `--priority N` - Priority (0-4, default: 2)
- `--assignee EMAIL` - Assign to user
- `--external-ref REF` - External reference (e.g., JIRA-123)
- `--due-at DATE` - Due date (RFC3339)

**Examples:**
```bash
# Minimal
br create --title="Fix crash on startup"

# Full
br create --title="Fix login timeout" \
  --description="Users report timeouts after 30 seconds" \
  --type=bug --priority=1 \
  --acceptance-criteria="1. Timeout increased to 60s\n2. Retry logic added" \
  --assignee="team@example.com"

# JSON output (for agents)
br create --title="Test" --json
# {"id":"src-abc123","title":"Test",...}
```

---

#### `br q` - Quick Capture

```bash
br q --title "TITLE" [OPTIONS]
```

Same options as `br create`, but **outputs only the issue ID**.

**Example:**
```bash
br q --title "Quick thought to capture"
# src-xyz789
```

---

#### `br show` - Show Issue Details

```bash
br show <ID> [OPTIONS]
```

**Options:**
- `--comments` - Include comments
- `--events` - Include audit events
- `--json` - Output as JSON

**Example:**
```bash
# Basic
br show src-abc123

# Full details with comments and events
br show src-abc123 --comments --events

# JSON (for agents)
br show src-abc123 --json
# {"id":"src-abc123","title":"Fix login timeout",...}
```

**Output Sections:**
1. **Header** - ID, status, priority, type
2. **Title** - Issue title
3. **Description** - Full description
4. **Design** - Technical design notes (if any)
5. **Acceptance Criteria** - Completion criteria (if any)
6. **Notes** - Additional notes (if any)
7. **Metadata** - Created at, updated at, assignee, etc.
8. **Dependencies** - Blocked by / Blocks
9. **Labels** - Assigned labels
10. **Comments** - Threaded comments (if `--comments`)
11. **Events** - Audit log (if `--events`)

---

#### `br update` - Update Issue

```bash
br update <ID> [OPTIONS]
```

**Options:**
- `--title "TITLE"` - New title
- `--description "DESC"` - New description
- `--design "DESIGN"` - New design notes
- `--acceptance-criteria "CRITERIA"` - New acceptance criteria
- `--notes "NOTES"` - New notes
- `--status STATUS` - New status (open, in_progress, blocked, deferred, closed)
- `--priority N` - New priority (0-4)
- `--type TYPE` - New issue type
- `--assignee EMAIL` - New assignee (or empty to unassign)
- `--due-at DATE` - New due date
- `--pinned` - Pin issue
- `--unpinned` - Unpin issue

**Examples:**
```bash
# Update status
br update src-abc123 --status in_progress

# Update priority
br update src-abc123 --priority 0

# Change assignee
br update src-abc123 --assignee "alice@example.com"

# Clear assignee
br update src-abc123 --assignee ""

# Pin issue (keeps it in `br ready` even if blocked)
br update src-abc123 --pinned
```

---

#### `br close` - Close Issue

```bash
br close <ID>... [OPTIONS]
```

**Options:**
- `--reason "REASON"` - Closure reason
- `--status STATUS` - Close issues with this status (e.g., `--status open` to close all open)
- `--type TYPE` - Close issues of this type (e.g., `--type bug` to close all bugs)

**Examples:**
```bash
# Close single issue
br close src-abc123 --reason "Fixed by increasing timeout"

# Close multiple issues
br close src-abc123 src-def456 src-ghi789 --reason "All done"

# Close all open bugs
br close --status open --type bug --reason "Bug bash complete"

# Quiet mode (only outputs IDs)
br close src-abc123 --quiet
# src-abc123
```

---

#### `br reopen` - Reopen Closed Issue

```bash
br reopen <ID>
```

**Example:**
```bash
br reopen src-abc123
# Issue reopened, status set to open
```

---

#### `br delete` - Delete Issue (Creates Tombstone)

```bash
br delete <ID>... [OPTIONS]
```

**Options:**
- `--reason "REASON"` - Deletion reason
- `--actor ACTOR` - Actor performing deletion

**Example:**
```bash
br delete src-abc123 --reason "Duplicate of src-def456"

# Delete multiple
br delete src-abc123 src-def456
```

**Note:** Deletion creates a tombstone (soft delete). Issue is hidden from `br list` but preserved in JSONL for git history.

---

### Query & List Commands

#### `br list` - List Issues

```bash
br list [FILTER_OPTIONS] [OUTPUT_OPTIONS]
```

**Filter Options:**
- `-s, --status STATUS` - Filter by status (can repeat)
- `-t, --type TYPE` - Filter by type (can repeat)
- `--assignee EMAIL` - Filter by assignee
- `--unassigned` - Show unassigned issues only
- `--id ID` - Filter by specific ID(s) (can repeat)
- `-l, --label LABEL` - Filter by label (AND logic, can repeat)
- `--label-any LABEL_ANY` - Filter by label (OR logic, can repeat)
- `-p, --priority N` - Filter by priority (can repeat)
- `--priority-min N` - Minimum priority (0-4)
- `--priority-max N` - Maximum priority (0-4)
- `--title-contains TEXT` - Title contains substring
- `--desc-contains TEXT` - Description contains substring
- `--notes-contains TEXT` - Notes contains substring
- `-a, --all` - Include closed issues (default excludes closed)

**Output Options:**
- `--limit N` - Maximum results (0 = unlimited, default: 50)
- `--sort FIELD` - Sort by `priority`, `created_at`, `updated_at`, `title`
- `--json` - Output as JSON
- `--format {json|toon|plain}` - Output format

**Examples:**
```bash
# List all open issues
br list --status open

# List high priority issues (P0-P1)
br list --priority-min 0 --priority-max 1

# List unassigned bugs
br list --type bug --unassigned

# List issues with specific label
br list --label frontend

# List issues with any of multiple labels (OR logic)
br list --label-any frontend backend

# List issues with multiple labels (AND logic)
br list --label frontend --label urgent

# Sort by priority
br list --sort priority

# Limit to 10 results
br list --limit 10

# Include closed issues
br list --all

# Search in title/description
br list --title-contains "login"

# JSON output
br list --json | jq '.[] | {id, title, priority}'

# TOON format (token-optimized)
br list --format toon
```

**Status Icons:**
- ● Open
- ◐ In Progress
- ◐ Blocked
- ❄ Deferred
- ✔ Closed

**Priority Colors:**
- P0 - Red (critical)
- P1 - Yellow (high)
- P2 - Green (medium)
- P3 - Blue (low)
- P4 - Gray (backlog)

---

#### `br ready` - List Actionable Work

```bash
br ready [OPTIONS]
```

**Shows issues that are:**
- Status: `open` or `in_progress`
- NOT blocked by dependencies
- NOT deferred
- NOT pinned (pinned issues always show)

**Options:**
- `--json` - Output as JSON
- `--format {json|toon|plain}` - Output format

**Examples:**
```bash
br ready
# ● src-abc123  P1  bug     Fix login timeout
# ◐ src-def456  P2  task     Implement API endpoint

# JSON for agents
br ready --json | jq '.[0]'

# TOON format
br ready --format toon
```

**Note:** `br ready` is **cached** for performance. Cache is invalidated when dependencies change or issues close/reopen.

---

#### `br blocked` - List Blocked Issues

```bash
br blocked [OPTIONS]
```

**Shows issues that are blocked by:**
- Dependencies (not closed)
- Explicit `blocked` status

**Options:**
- `--json` - Output as JSON
- `--format {json|toon|plain}` - Output format

**Examples:**
```bash
br blocked
# ❄ src-xyz789  P2  feature  Implement OAuth
#   Blocked by: src-abc123 (open)

# JSON
br blocked --json
```

---

#### `br stale` - List Stale Issues

```bash
br stale [OPTIONS]
```

**Options:**
- `--days N` - Stale threshold in days (default: 30)
- `--status STATUS` - Filter by status (default: open,in_progress)
- `--json` - Output as JSON

**Examples:**
```bash
# Issues not updated in 30 days
br stale

# Issues not updated in 7 days
br stale --days 7

# In-progress issues stale for 14 days
br stale --status in_progress --days 14
```

---

#### `br search` - Full-Text Search

```bash
br search "QUERY" [OPTIONS]
```

**Options:**
- `--status STATUS` - Filter by status
- `--type TYPE` - Filter by type
- `--priority N` - Filter by priority

**Examples:**
```bash
# Search for keyword
br search "authentication"

# Search for bugs
br search "memory leak" --type bug

# Search high priority
br search "crash" --priority 0
```

**Note:** Search is full-text, searches across title, description, design, notes.

---

#### `br count` - Count Issues

```bash
br count [OPTIONS]
```

**Options:**
- `--by FIELD` - Group by field (`status`, `type`, `priority`, `assignee`)
- `--status STATUS` - Filter by status
- `--type TYPE` - Filter by type
- `--priority N` - Filter by priority

**Examples:**
```bash
# Total count
br count
# 42

# Count by status
br count --by status
# open: 15
# in_progress: 8
# closed: 19

# Count by type
br count --by type
# task: 20
# bug: 12
# feature: 10

# Count open bugs
br count --status open --type bug
# 8
```

---

#### `br stats` - Project Statistics

```bash
br stats [OPTIONS]
```

**Options:**
- `--json` - Output as JSON

**Shows:**
- Total issues
- Open/closed counts
- By type breakdown
- By priority breakdown
- Oldest open issue
- Most recently updated
- Blocked issues count
- Deferred issues count

**Example:**
```bash
br stats
# ╭─────────────────────────────────────────╮
# │  Project Statistics                      │
# ├─────────────────────────────────────────┤
# │  Total Issues: 42                    │
# │  Open: 15 | In Progress: 8          │
# │  Closed: 19                           │
# │                                         │
# │  By Type:                              │
# │  task: 20 | bug: 12 | feature: 10   │
# │                                         │
# │  By Priority:                           │
# │  P0: 3 | P1: 8 | P2: 18 | P3: 8   │
# │                                         │
# │  Oldest Open: src-abc123 (2024-12-01) │
# │  Most Recent: src-xyz789 (2026-01-31)  │
# ╰─────────────────────────────────────────╯
```

---

### Dependency Commands

#### `br dep add` - Add Dependency

```bash
br dep add <ISSUE_ID> <DEPENDS_ON_ID> [OPTIONS]
```

**Options:**
- `--type TYPE` - Dependency type (default: `blocks`)

**Dependency Types:**
- `blocks` - A depends on B (B must close before A)
- `parent-child` - Parent/child relationship
- `conditional-blocks` - A conditionally blocked by B
- `waits-for` - A waits for B
- `related` - Related issues
- `discovered-from` - Discovered from this issue
- `replies-to` - Reply thread
- `relates-to` - Related to
- `duplicates` - Duplicate of
- `supersedes` - Supersedes this issue
- `caused-by` - Caused by this issue

**Examples:**
```bash
# Simple blocking relationship
br dep add src-abc123 src-def456
# src-abc123 now depends on src-def456

# Explicit type
br dep add src-abc123 src-def456 --type blocks

# Related relationship (non-blocking)
br dep add src-abc123 src-def456 --type related
```

---

#### `br dep remove` - Remove Dependency

```bash
br dep remove <ISSUE_ID> <DEPENDS_ON_ID>
```

**Example:**
```bash
br dep remove src-abc123 src-def456
# Dependency removed, src-abc123 no longer blocked by src-def456
```

---

#### `br dep list` - List Dependencies

```bash
br dep list <ID> [OPTIONS]
```

**Options:**
- `--json` - Output as JSON

**Shows:**
- Blocked by (what this issue depends on)
- Blocks (what depends on this issue)

**Example:**
```bash
br dep list src-abc123
# ╭─────────────────────────────────────╮
# │  Dependencies for src-abc123        │
# ├─────────────────────────────────────┤
# │  Blocked by:                       │
# │  • src-def456 [blocks]          │
# │  • src-ghi789 [parent-child]    │
# │                                     │
# │  Blocks:                           │
# │  • src-xyz123 (waiting for this)  │
# ╰─────────────────────────────────────╯
```

---

#### `br dep tree` - Show Dependency Tree

```bash
br dep tree <ID> [OPTIONS]
```

**Options:**
- `--compact` - One line per issue
- `--json` - Output as JSON

**Example:**
```bash
br dep tree src-abc123
# src-abc123 (blocked by)
# ├── src-def456 (closed) ✔
# └── src-ghi789 (blocked by)
#     └── src-xyz123 (in_progress) ◐
```

---

#### `br dep cycles` - Detect Dependency Cycles

```bash
br dep cycles [OPTIONS]
```

**Options:**
- `--json` - Output as JSON

**Example:**
```bash
br dep cycles
# No cycles detected

# If cycles exist:
# ⚠ Cycle detected:
# src-abc123 → src-def456 → src-ghi789 → src-abc123
```

---

### Label Commands

#### `br label add` - Add Label

```bash
br label add <ID> <LABEL>...
```

**Examples:**
```bash
# Add single label
br label add src-abc123 frontend

# Add multiple labels
br label add src-abc123 frontend urgent
```

---

#### `br label remove` - Remove Label

```bash
br label remove <ID> <LABEL>...
```

**Examples:**
```bash
# Remove single label
br label remove src-abc123 urgent

# Remove multiple labels
br label remove src-abc123 frontend backend
```

---

#### `br label list` - List Labels

```bash
br label list <ID> [OPTIONS]
```

**Options:**
- `--json` - Output as JSON

**Example:**
```bash
br label list src-abc123
# frontend, backend, urgent
```

---

#### `br label list-all` - List All Labels in Project

```bash
br label list-all [OPTIONS]
```

**Options:**
- `--json` - Output as JSON

**Example:**
```bash
br label list-all
# frontend (12 issues)
# backend (8 issues)
# urgent (5 issues)
# bug (3 issues)
```

---

### Comment Commands

#### `br comments add` - Add Comment

```bash
br comments add <ID> "COMMENT"
```

**Example:**
```bash
br comments add src-abc123 "I found the root cause - it's a race condition in the auth service."
```

---

#### `br comments list` - List Comments

```bash
br comments list <ID> [OPTIONS]
```

**Options:**
- `--json` - Output as JSON

**Example:**
```bash
br comments list src-abc123
# ╭─────────────────────────────────────────╮
# │  Comments for src-abc123            │
# ├─────────────────────────────────────────┤
# │  alice@example.com (2026-02-01)     │
# │  Found root cause                   │
# │                                         │
# │  bob@example.com (2026-02-01)       │
# │  Proposed fix: add mutex           │
# ╰─────────────────────────────────────────╯
```

---

### Epic Commands

#### `br epic status` - Show Epic Progress

```bash
br epic status [OPTIONS]
```

**Options:**
- `--dry-run` - Check eligibility without closing
- `--json` - Output as JSON

**Shows:**
- All epics with child counts
- Children closed vs total
- Whether eligible to close

**Example:**
```bash
br epic status
# ╭─────────────────────────────────────────╮
# │  Epic Status                            │
# ├─────────────────────────────────────────┤
# │  src-epic-001                          │
# │  Implement User Auth                    │
# │  Progress: 7/10 children closed        │
# │  Eligible to close: NO                 │
# │                                         │
# │  src-epic-002                          │
# │  Implement OAuth                        │
# │  Progress: 5/5 children closed         │
# │  Eligible to close: YES ✅             │
# ╰─────────────────────────────────────────╯
```

---

#### `br epic close-eligible` - Close Eligible Epics

```bash
br epic close-eligible [OPTIONS]
```

**Options:**
- `--dry-run` - Preview without closing
- `--json` - Output as JSON

**Example:**
```bash
# Preview what would close
br epic close-eligible --dry-run

# Actually close eligible epics
br epic close-eligible
# Closing: src-epic-002 (5/5 children closed)
```

---

### Workflow Commands

#### `br defer` - Defer Issue

```bash
br defer <ID> [OPTIONS]
```

**Options:**
- `--until DATE` - Defer until specific date (RFC3339)

**Examples:**
```bash
# Defer indefinitely
br defer src-abc123

# Defer until next week
br defer src-abc123 --until "2026-02-08T00:00:00Z"
```

---

#### `br undefer` - Undefer Issue

```bash
br undefer <ID>
```

**Example:**
```bash
br undefer src-abc123
# Issue now ready again (if not blocked)
```

---

### System & Maintenance Commands

#### `br sync` - Sync Database with JSONL

```bash
br sync [MODE] [OPTIONS]
```

**Modes (one required):**
- `--flush-only` - Export DB to JSONL (DB → JSONL)
- `--import-only` - Import JSONL to DB (JSONL → DB)
- `--status` - Show sync status (read-only)

**Options:**
- `--verbose` / `-v` - Show safety guard decisions
- `--verbose` / `-vv` - Show DEBUG-level file operations

**Examples:**
```bash
# Export to JSONL (before git commit)
br sync --flush-only

# Import from JSONL (after git pull)
br sync --import-only

# Check sync status
br sync --status
# Database: Clean (up to date with JSONL)
# Database: Stale (3 issues in JSONL not in DB)

# Export with verbose logging
br sync --flush-only -v
# ✓ Exporting 12 dirty issues
# ✓ Writing to .beads/issues.jsonl
# ✓ Clearing dirty flags
```

**Safety Guards:**
- **Empty DB Guard:** Won't export empty DB over non-empty JSONL
- **Stale DB Guard:** Won't export if JSONL has issues missing from DB
- **Conflict Markers:** Won't import files with `<<<<<<<` markers

---

#### `br doctor` - Run Diagnostics

```bash
br doctor [OPTIONS]
```

**Options:**
- `--rebuild-cache` - Force rebuild of blocked issues cache
- `--vacuum` - Vacuum database to reclaim space
- `--json` - Output as JSON

**Checks:**
- Database integrity
- Index validity
- Cache consistency
- JSONL sync status

**Example:**
```bash
br doctor
# ╭─────────────────────────────────────────╮
# │  Diagnostics                             │
# ├─────────────────────────────────────────┤
# │  ✓ Database integrity: OK               │
# │  ✓ Indexes: Valid                      │
# │  ✓ Blocked cache: Consistent             │
# │  ✓ Sync status: Clean                   │
# ╰─────────────────────────────────────────╯
```

---

#### `br info` - Show Workspace Information

```bash
br info
```

**Shows:**
- Database path
- JSONL path
- Config path
- Issue count
- ID prefix
- Version

**Example:**
```bash
br info
# Database: /home/lewis/src/oya/.beads/beads.db
# JSONL: /home/lewis/src/oya/.beads/issues.jsonl
# Config: /home/lewis/src/oya/.beads/config.yaml
# Issues: 42
# Prefix: src
# Version: 0.1.12
```

---

#### `br where` - Show .beads Directory

```bash
br where
```

**Example:**
```bash
br where
# /home/lewis/src/oya/.beads
```

---

#### `br version` - Show Version

```bash
br version
# br 0.1.12 (rustc 1.85.0-nightly)
```

---

#### `br upgrade` - Self-Update

```bash
br upgrade
```

Downloads and installs the latest version from GitHub releases.

---

### Configuration Commands

#### `br config list` - List All Config

```bash
br config list
# Runtime settings:
#   auto_compact_enabled: false
#   compact_batch_size: 50
#   issue_prefix: src
#
# Startup settings:
#   json: false
#   no-auto-flush: false
#   no-auto-import: false
```

---

#### `br config get` - Get Config Value

```bash
br config get <KEY>
```

**Example:**
```bash
br config get issue_prefix
# src

br config get defaults.priority
# 2
```

---

#### `br config set` - Set Config Value

```bash
br config set <KEY> <VALUE>
```

**Example:**
```bash
br config set defaults.priority=1
br config set id.prefix=oya
```

---

#### `br config edit` - Open Config in Editor

```bash
br config edit
# Opens .beads/config.yaml in $EDITOR
```

---

#### `br config path` - Show Config Paths

```bash
br config path
# Project: /home/lewis/src/oya/.beads/config.yaml
# User: /home/lewis/.config/beads/config.yaml
```

---

### Schema & Tooling Commands

#### `br schema` - Emit JSON Schemas

```bash
br schema [TARGET] [OPTIONS]
```

**Targets:**
- `all` - Emit all schemas (default)
- `issue` - Core Issue object
- `issue-with-counts` - Issue + dependency/dependent counts
- `issue-details` - Issue + relations/comments/events
- `ready-issue` - Ready list row
- `stale-issue` - Stale list row
- `blocked-issue` - Blocked list row
- `tree-node` - Dependency tree node
- `statistics` - Stats output
- `error` - Structured error envelope

**Options:**
- `--format {text|json|toon}` - Output format

**Examples:**
```bash
# Get all schemas
br schema all --format json

# Get specific schema
br schema issue-details --format json

# TOON format (token-optimized)
br schema all --format toon

# Text documentation
br schema issue
```

---

### Advanced Commands

#### `br orphans` - List Orphan Issues

```bash
br orphans [OPTIONS]
```

**Orphans:** Issues referenced in commits but not closed in br.

**Example:**
```bash
br orphans
# ⚠ Orphan issues (referenced in commits):
# src-abc123 - mentioned in commit abc1234f
# src-def456 - mentioned in commit def5678b
```

---

#### `br changelog` - Generate Changelog

```bash
br changelog [OPTIONS]
```

**Generates changelog from closed issues.**

**Example:**
```bash
br changelog
# # Changelog
#
# ## [2026-02-01]
# - Fixed login timeout (src-abc123)
# - Implemented OAuth (src-def456)
#
# ## [2026-01-31]
# - Added user registration (src-ghi789)
```

---

#### `br graph` - Visualize Dependency Graph

```bash
br graph [ID] [OPTIONS]
```

**Options:**
- `--all` - Show graph for all open/in_progress/blocked issues
- `--compact` - One line per issue

**Example:**
```bash
# Graph for specific issue
br graph src-abc123

# Graph for all issues
br graph --all

# Compact format
br graph src-abc123 --compact
```

---

#### `br lint` - Check Issues

```bash
br lint [OPTIONS]
```

**Checks:**
- Missing template sections
- Incomplete fields

**Example:**
```bash
br lint
# ⚠ src-abc123: Missing acceptance_criteria
# ⚠ src-def456: Empty description
```

---

### Query Management

#### `br query save` - Save Filter Set

```bash
br query save <NAME>
```

**Saves current filter set as a named query.**

**Example:**
```bash
# Set up filters
br list --status open --priority 0-1 --assignee alice@example.com

# Save as named query
br query save alice-high-priority
# Saved query: alice-high-priority
```

---

#### `br query run` - Run Saved Query

```bash
br query run <NAME>
```

**Example:**
```bash
br query run alice-high-priority
# Runs with saved filters
```

---

#### `br query list` - List Saved Queries

```bash
br query list
```

---

#### `br query delete` - Delete Saved Query

```bash
br query delete <NAME>
```

---

### Agent Integration Commands

#### `br audit` - Record Agent Interactions

```bash
br audit <JSON_LINE>
```

**Appends agent interaction to audit trail.**

---

#### `br history` - Manage History Backups

```bash
br history [OPTIONS]
```

---

### Completion Commands

#### `br completions` - Generate Shell Completions

```bash
# Bash
br completions bash > ~/.local/share/bash-completion/completions/br
source ~/.local/share/bash-completion/completions/br

# Zsh
br completions zsh > ~/.zfunc/_br

# Fish
br completions fish > ~/.config/fish/completions/br.fish
```

---

## Issue Types & Priority

### Priority Levels

| Priority | Name | Color | Usage |
|----------|------|--------|---------|
| P0 | Critical | Red | Blocking release, production bugs |
| P1 | High | Yellow | Important features, severe bugs |
| P2 | Medium | Green | Normal work (default) |
| P3 | Low | Blue | Nice to have, minor bugs |
| P4 | Backlog | Gray | Future work, ideas |

**Setting Priority:**
```bash
br create --title="Fix crash" --priority=0
br update src-abc123 --priority=1
```

### Issue Types

| Type | Usage |
|------|--------|
| task | Standard work item (default) |
| bug | Bug fix, defect |
| feature | New feature, enhancement |
| epic | Large feature (contains child issues) |
| chore | Maintenance, cleanup, refactoring |
| docs | Documentation work |
| question | Clarification, investigation |

**Setting Type:**
```bash
br create --title="Fix login" --type=bug
br create --title="Add OAuth" --type=feature
br create --title="Update README" --type=docs
```

---

## Dependencies

### Understanding Dependency Types

**Blocking Types (affect `br ready`):**
- `blocks` - A must wait for B
- `parent-child` - A is child of B
- `conditional-blocks` - A blocked by B conditionally
- `waits-for` - A waits for B

**Non-Blocking Types:**
- `related` - Related issues
- `discovered-from` - Found during this work
- `replies-to` - Comment thread
- `relates-to` - Related
- `duplicates` - Duplicate issue
- `supersedes` - Replaces this issue
- `caused-by` - Root cause

### Dependency Patterns

**Sequential Work:**
```bash
br create --title="Design database schema" --priority=1
br create --title="Implement database" --priority=2
br dep add src-db-impl src-db-design
# src-db-impl blocked until src-db-design closes
```

**Parallel Work:**
```bash
# Both can work in parallel
br create --title="Frontend UI" --priority=1
br create --title="Backend API" --priority=1
# No dependencies between them
```

**Complex Dependencies:**
```bash
br dep add src-integration src-api --type waits-for
br dep add src-integration src-auth --type blocks
# Integration waits for API AND blocked by auth
```

### Pinned Issues

```bash
# Pin issue (shows in br ready even if blocked)
br update src-abc123 --pinned

# Unpin
br update src-abc123 --unpinned
```

**Use Case:** Work on blocked issue while waiting for blocker to resolve.

---

## Labels

### Label Patterns

**Component:**
```bash
br label add src-abc123 frontend
br label add src-abc123 backend
br label add src-abc123 database
```

**Priority:**
```bash
br label add src-abc123 urgent
br label add src-abc123 important
```

**Type:**
```bash
br label add src-abc123 bug
br label add src-abc123 feature-request
```

**Finding by Labels:**
```bash
# All frontend issues
br list --label frontend

# Frontend AND urgent
br list --label frontend --label urgent

# Frontend OR backend
br list --label-any frontend backend
```

---

## Searching & Filtering

### Common Filter Patterns

**By Status:**
```bash
br list --status open
br list --status in_progress
br list --status closed
```

**By Priority Range:**
```bash
br list --priority-min 0 --priority-max 1  # P0-P1
br list --priority-min 0                   # P0+
br list --priority 0 --priority 1          # P0 or P1
```

**By Type:**
```bash
br list --type bug
br list --type feature
br list --type epic
```

**By Assignee:**
```bash
br list --assignee alice@example.com
br list --unassigned
```

**By Text Search:**
```bash
br list --title-contains "login"
br list --desc-contains "crash"
br list --notes-contains "todo"
```

**Combining Filters:**
```bash
# Open, high priority, assigned to Alice
br list --status open --priority-min 0 --priority-max 1 --assignee alice@example.com

# Bugs, unassigned, containing "crash"
br list --type bug --unassigned --title-contains "crash"
```

---

## Epics

### Creating Epic Workflows

```bash
# 1. Create epic
br create --title="Implement User Authentication" --type=epic --priority=1

# 2. Create child tasks
br create --title="Design auth flow" --priority=1
br create --title="Implement login" --priority=1
br create --title="Implement registration" --priority=1
br create --title="Add OAuth" --priority=2

# 3. Link children to epic
br dep add src-design src-epic-001 --type parent-child
br dep add src-login src-epic-001 --type parent-child
br dep add src-registration src-epic-001 --type parent-child
br dep add src-oauth src-epic-001 --type parent-child
```

### Tracking Epic Progress

```bash
# Check epic status
br epic status
# src-epic-001 Implement User Authentication
# Progress: 2/4 children closed
# Eligible to close: NO

# When all children are done
br epic close-eligible
# Closing: src-epic-001 (4/4 children closed)
```

---

## Sync & Git Integration

### The Complete Sync Workflow

```bash
# 1. After making changes to issues
br sync --flush-only
# ✓ Exporting 12 dirty issues to .beads/issues.jsonl

# 2. Stage and commit
git add .beads/
git commit -m "Update issues: closed login timeout, added OAuth epic"

# 3. Push to remote
git push
```

### After Pulling Changes

```bash
# 1. Pull from remote
git pull

# 2. Check for merge conflicts
git status .beads/
# If conflicts in issues.jsonl, resolve manually

# 3. Import into database
br sync --import-only
# ✓ Imported 3 new issues from .beads/issues.jsonl
# ✓ Updated 5 issues from .beads/issues.jsonl
```

### Sync Status

```bash
br sync --status
# Database: Clean (up to date with JSONL)

# After someone else pushed changes:
br sync --status
# Database: Stale (3 issues in JSONL not in DB)
# Action needed: Run `br sync --import-only`
```

### Safety Guards

**Empty DB Guard:**
```bash
# Won't overwrite non-empty JSONL with empty DB
br sync --flush-only
# ⚠ Refusing to export: database is empty but .beads/issues.jsonl has 42 issues
# Use --force to override (not recommended)
```

**Stale DB Guard:**
```bash
# Won't export if JSONL has issues missing from DB
br sync --flush-only
# ⚠ Refusing to export: .beads/issues.jsonl has 3 issues not in database
# Run `br sync --import-only` first to import those issues
```

**Conflict Markers:**
```bash
# Won't import files with git merge conflicts
br sync --import-only
# ✗ Error: .beads/issues.jsonl contains git merge conflict markers
# Resolve conflicts manually, then retry
```

---

## Saved Queries

### Creating and Using Queries

```bash
# 1. Set up filters interactively
br list --status open --priority-min 0 --priority-max 1 --assignee alice

# 2. Save as named query
br query save alice-high-priority
# Saved query: alice-high-priority
# Filters: status=[open], priority=[0-1], assignee=[alice]

# 3. Run query anytime
br query run alice-high-priority

# 4. List all queries
br query list
# alice-high-priority
# frontend-issues
# open-bugs

# 5. Delete query
br query delete alice-high-priority
```

### Common Saved Queries

```bash
# High priority work
br list --priority-min 0 --priority-max 1
br query save high-priority

# Unassigned bugs
br list --type bug --unassigned
br query save unassigned-bugs

# My work
br list --assignee $(git config user.email)
br query save my-work
```

---

## Visualization

### Dependency Graphs

```bash
# Graph for specific issue
br graph src-abc123

# Graph for all open issues
br graph --all

# Compact format (good for large graphs)
br graph --all --compact
```

### Dependency Trees

```bash
# Visual tree
br dep tree src-abc123
# src-abc123 (blocked by)
#   ├── src-def456 (closed) ✔
#   └── src-ghi789 (blocked by)
#       └── src-xyz123 (in_progress) ◐

# Compact format
br dep tree src-abc123 --compact
```

---

## Configuration

### Layered Config (highest to lowest priority)

1. **CLI flags** (e.g., `--db`, `--actor`)
2. **Environment variables** (e.g., `BEADS_DB`, `RUST_LOG`)
3. **Project config** (`.beads/config.yaml`)
4. **User config** (`~/.config/beads/config.yaml`)
5. **Defaults** (hardcoded)

### Common Config Settings

```yaml
# .beads/config.yaml

# Issue ID prefix
id:
  prefix: "src"  # Default: "bd"

# Default values for new issues
defaults:
  priority: 2  # Default: 2 (medium)
  type: "task"  # Default: "task"
  assignee: ""  # Default: none

# Output formatting
output:
  color: true  # Default: true
  date_format: "%Y-%m-%d"  # Default: "%Y-%m-%d"

# Sync behavior
sync:
  auto_import: false  # Default: false
  auto_flush: false  # Default: false
```

### Environment Variables

| Variable | Purpose |
|----------|---------|
| `BEADS_DB` | Override database path |
| `BEADS_JSONL` | Override JSONL path (requires `--allow-external-jsonl`) |
| `RUST_LOG` | Logging level (`debug`, `info`, `warn`, `error`) |
| `NO_COLOR` | Disable colored output |
| `BR_OUTPUT_FORMAT` | Default output format (`json`, `toon`, `plain`) |

---

## Output Modes

### Mode Detection

```bash
# 1. --json or --robot flag
br list --json  # JSON mode

# 2. --quiet flag
br list --quiet  # Quiet mode

# 3. NO_COLOR env var or --no-color
NO_COLOR=1 br list  # Plain mode

# 4. Piped output (non-TTY)
br list | cat  # Plain mode

# 5. Interactive terminal with colors
br list  # Rich mode (default)
```

### JSON Mode (for Agents)

```bash
# Stable, machine-readable output
br list --json
br ready --json
br show src-abc123 --json

# Parse with jq
br list --json | jq '.[] | select(.priority <= 1)'
br ready --json | jq '.[0].id'
```

### TOON Mode (Token-Optimized)

```bash
# Binary format, ~40% smaller than JSON
br list --format toon
br ready --format toon

# Decode with tru tool (if available)
br list --format toon | tru --decode
```

### Quiet Mode

```bash
# Minimal output (usually just IDs)
br close src-abc123 --quiet
# src-abc123

br list --quiet
# src-abc123
# src-def456
# src-ghi789
```

---

## Agent Integration

### Agent Workflow Pattern

```bash
# 1. TRIAGE - Find work
bv --robot-triage          # Get top pick from beads_viewer
# OR
br ready --json | jq '.[0]'  # Get first ready issue

# 2. CLAIM - Reserve issue
br update src-2tg --status in_progress

# 3. WORK - Implement
# ... (code changes) ...

# 4. REVIEW - Test
# ... (run tests) ...

# 5. COMPLETE - Close issue
br close src-2tg --reason "Implemented fix for type mismatch in json/error.rs"

# 6. SYNC - Export for git
br sync --flush-only
git add .beads/
git commit -m "Fix: type mismatch errors (src-2tg)"
```

### Schemas for Agents

```bash
# Get all schemas
br schema all --format json

# Get specific schema
br schema issue-details --format json
br schema statistics --format json
br schema error --format json

# TOON schemas (token-optimized)
br schema all --format toon
```

### Error Handling for Agents

```bash
# Structured error envelope on stderr
br show nonexistent --json 2>&1 | jq .
# {
#   "error": {
#     "type": "issue_not_found",
#     "message": "Issue 'nonexistent' not found"
#   }
# }
```

---

## Best Practices

### Daily Workflow

```bash
# Start of day
br ready                    # See what's actionable
# Pick an issue and work...

# During work
br update <id> --status in_progress  # Claim when start
# ... work ...
br comments add <id> "Found root cause"  # Document findings

# End of day
br sync --flush-only    # Export changes
git add .beads/
git commit -m "Daily update"
```

### Issue Hygiene

```bash
# Always use descriptive titles
br create --title="Fix race condition in auth service timeout handler"  # ✓ Good
br create --title="Fix bug"  # ✗ Too vague

# Always close with reasons
br close <id> --reason "Increased timeout from 30s to 60s, added retry logic"  # ✓ Good
br close <id> --reason="Done"  # ✗ Useless

# Set appropriate priorities
br create --title="Production crash" --priority=0  # ✓ Critical
br create --title="Typos in README" --priority=4  # ✓ Backlog
```

### Dependency Hygiene

```bash
# Only add blocking dependencies
br dep add src-abc123 src-def456 --type blocks  # ✓ Clear relationship

# Avoid circular dependencies
br dep cycles  # Check before adding dependencies

# Use parent-child for hierarchical work
br dep add src-task src-epic --type parent-child
```

### Sync Hygiene

```bash
# ALWAYS sync before committing
br sync --flush-only  # ← Don't forget this!
git add .beads/
git commit -m "..."

# ALWAYS import after pulling
git pull
br sync --import-only  # ← Don't forget this!
```

---

## Common Patterns

### Triage Pattern

```bash
# Process inbox items into actionable tasks
# 1. List all open issues
br list --status open --all

# 2. For each unassigned issue:
#    - Assign if relevant to you
#    - Set appropriate priority
#    - Add dependencies if needed

# 3. Create dependencies for blocked work
br dep add src-blocked src-blocker

# 4. Sync when done
br sync --flush-only
```

### Bug Fix Pattern

```bash
# 1. Create bug issue
br create --title="Crash on login after timeout" \
  --type=bug --priority=1 \
  --description="When login takes > 30s, app crashes with segfault"

# 2. Add reproduction steps as comments
br comments add src-abc123 "Reproduction:
# 1. Enter invalid credentials
# 2. Wait 35 seconds
# 3. App crashes"

# 3. Work on fix...

# 4. When fixed, add test results as comments
br comments add src-abc123 "Verified fix: login completes successfully even after 60s timeout"

# 5. Close with reason
br close src-abc123 --reason "Fixed: Changed timeout from 30s to 60s, added proper error handling"
```

### Feature Pattern

```bash
# 1. Create epic
br create --title="Implement User Authentication" --type=epic --priority=1

# 2. Break down into tasks
br create --title="Design auth schema" --priority=1
br create --title="Implement login endpoint" --priority=1
br create --title="Implement registration" --priority=1
br create --title="Add OAuth support" --priority=2

# 3. Link tasks to epic
br dep add src-design src-epic --type parent-child
br dep add src-login src-epic --type parent-child
# ...

# 4. Track progress
br epic status
# Check when all children are done

# 5. Close epic
br epic close-eligible
```

### Code Review Pattern

```bash
# 1. Create review issues for each PR
br create --title="PR #123: Refactor auth module" \
  --type=chore --priority=2 \
  --external-ref="PR-123"

# 2. Link to original issue
br dep add src-pr123 src-original --type supersedes

# 3. When PR merged, close review issue
br close src-pr123 --reason="Merged, supersedes src-original"
```

### Spike/Investigation Pattern

```bash
# 1. Create investigation issue
br create --title="Investigate memory leak in auth service" \
  --type=bug --priority=1 \
  --description="Auth service memory grows unbounded over time"

# 2. Create follow-up tasks for findings
br comments add src-investigation "Finding: Leak in session cache - need to implement expiration"

# 3. When done, create actual fix issue
br create --title="Implement session cache expiration" --type=bug --priority=1
br dep add src-fix src-investigation --type discovered-from

# 4. Close investigation
br close src-investigation --reason "Investigation complete, created fix issue src-fix"
```

---

## Troubleshooting

### Database Locked

```bash
br list
# Error: Database is locked

# Check for other br processes
pgrep -f "br "

# Kill other process (if safe)
pkill -f "br "

# Or use lock timeout
br --lock-timeout 5000 list  # 5 second timeout
```

### Issue Not Found

```bash
br show src-abc123
# Error: Issue 'src-abc123' not found

# Check for similar IDs
br list | grep abc

# List all issues
br list --all
```

### Sync Issues

```bash
# JSONL has conflicts
br sync --import-only
# Error: .beads/issues.jsonl contains git merge conflict markers

# Resolve conflicts in editor
vim .beads/issues.jsonl

# Remove conflict markers manually, then retry
br sync --import-only
```

### Stale Database

```bash
br sync --flush-only
# Error: Database is stale (3 issues in JSONL not in DB)

# Import first
br sync --import-only

# Then export
br sync --flush-only
```

### Empty DB Guard Triggered

```bash
br sync --flush-only
# Error: Refusing to export: database is empty but .beads/issues.jsonl has issues

# Check if you're in right directory
br where
# /correct/path/to/oya/.beads

# Import existing JSONL if needed
br sync --import-only
```

### Performance Issues

```bash
# br ready is slow
# Rebuild blocked cache
br doctor --rebuild-cache

# Vacuum database
br doctor --vacuum

# Check for many issues
br count
# If > 1000 issues, consider using filters
br list --limit 50
```

### Corrupted Database

```bash
br list
# Error: Database disk image is malformed

# Restore from JSONL backup
cp .beads/issues.jsonl .beads/issues.jsonl.backup

# Reimport
br sync --import-only
```

---

## Global Options

These options can be used with any command:

| Option | Description |
|--------|-------------|
| `--db <PATH>` | Override database path (auto-discovers .beads/*.db) |
| `--actor <NAME>` | Set actor name for audit trail |
| `--json` | Output as JSON (machine-readable) |
| `--format {json|toon|plain}` | Output format |
| `--quiet` / `-q` | Suppress output (except errors) |
| `--verbose` / `-v` | Increase logging (use `-vv` for DEBUG) |
| `--no-color` | Disable colored output |
| `--no-daemon` | Force direct mode (no-op in br v1) |
| `--no-auto-flush` | Skip auto JSONL export |
| `--no-auto-import` | Skip auto import check |
| `--allow-stale` | Allow stale DB (bypass warning) |
| `--lock-timeout <MS>` | SQLite busy timeout in milliseconds |

---

## Integration with Other Tools

### beads_viewer (bv) - TUI & Triage

```bash
# Interactive TUI
bv

# Robot mode for agents
bv --robot-triage
bv --robot-next
```

### Git Hooks

```bash
# Pre-commit hook: Check if beads synced
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/sh
br sync --status | grep -q "Clean" || {
  echo "Error: Beads database not synced"
  echo "Run: br sync --flush-only && git add .beads/"
  exit 1
}
EOF
chmod +x .git/hooks/pre-commit
```

### Editor Integration (VS Code)

```bash
# Create task from editor selection
# Add to settings.json:
{
  "tasks": [
    {
      "label": "br: Create Issue",
      "type": "shell",
      "command": "br q --title '${selectedText}'"
    }
  ]
}
```

### Shell Aliases

```bash
# Add to .bashrc or .zshrc
alias brs='br ready'              # Show ready issues
alias brl='br list'              # List all
alias brc='br close'             # Close issue
alias bru='br update'            # Update issue
alias brn='br create'            # Create new

# Use
brs  # instead of br ready
brl  # instead of br list
```

---

## Quick Reference Card

```bash
# Workflow
br ready                     # Find work
br show <id>                # View details
br update <id> --status in_progress  # Claim
br close <id> --reason "..."  # Complete
br sync --flush-only         # Export for git

# Queries
br list --status open --priority 0-1
br search "keyword"
br blocked
br stale --days 30

# Dependencies
br dep add <id> <depends-on>
br dep tree <id>
br dep cycles

# Epics
br epic status
br epic close-eligible

# System
br doctor
br info
br where
br stats
```

---

**Last Updated:** 2026-02-01
**br Version:** 0.1.12
**For oya Project:** /home/lewis/src/oya
