# Quick Reference - Bead Selection Guide

## Pick One of These 3 to Start

```bash
# Foundation work - do this first!
bd update intent-cli-wb2 --status in_progress --json
# or
bd update intent-cli-6oy --status in_progress --json

# Or investigate the blocked feature first
bd list intent-cli-aki --json
```

## After Completing Test Suite, Do These First

```bash
# Core functionality bugs (highest impact)
bd update intent-cli-m4c --status in_progress --json  # parse command
bd update intent-cli-bzz --status in_progress --json  # lint command
bd update intent-cli-irr --status in_progress --json  # vision start
```

## These 12 Can Be Done in Parallel (All Bug Fixes)

```bash
# Track A: Vision & Ready (2 beads)
bd update intent-cli-irr --status in_progress --json  # vision start - exit 4
bd update intent-cli-766 --status in_progress --json  # ready start - exit 2

# Track B: Core Commands (2 beads)
bd update intent-cli-m4c --status in_progress --json  # parse - exit 1
bd update intent-cli-bzz --status in_progress --json  # lint - exit 1

# Track C: Utility Commands (5 beads)
bd update intent-cli-92o --status in_progress --json  # feedback - exit 4
bd update intent-cli-o7m --status in_progress --json  # prompt - exit 4
bd update intent-cli-nlh --status in_progress --json  # diff - exit 4
bd update intent-cli-5ns --status in_progress --json  # plan - exit 4
bd update intent-cli-4ai --status in_progress --json  # history - exit 4

# Track D: Beads & AI (3 beads)
bd update intent-cli-8mh --status in_progress --json  # ai aggregate - exit 4
bd update intent-cli-8i5 --status in_progress --json  # beads-regenerate - exit 4
bd update intent-cli-dxi --status in_progress --json  # bead-status - exit 4
```

## After All Bugs Fixed - These 5 Can Be Done in Parallel

```bash
# Testing subtasks (all depend on intent-cli-6oy parent)
bd update intent-cli-6oy.1 --status in_progress --json  # Interview System Testing
bd update intent-cli-6oy.2 --status in_progress --json  # Beads System Testing
bd update intent-cli-6oy.3 --status in_progress --json  # KIRK Quality Commands
bd update intent-cli-6oy.4 --status in_progress --json  # History & Sessions
bd update intent-cli-6oy.5 --status in_progress --json  # Core Spec Commands
```

## After Testing Complete

```bash
# E2E test suite (final step)
bd update intent-cli-ibp --status in_progress --json
```

## After Completing Any Work

```bash
# Close the bead
bd close <bead-id> --reason "Completed" --json

# Check what's ready next
bd ready --json
```

## Duplicate FIX Tasks (Close Together)

When fixing a bug, also close its corresponding FIX bead:

| Bug Bead | FIX Bead |
|----------|----------|
| intent-cli-o7m (prompt - exit 4) | intent-cli-ffj |
| intent-cli-92o (feedback - exit 4) | intent-cli-5bs |
| intent-cli-nlh (diff - exit 4) | intent-cli-tm9 |
| intent-cli-irr (vision start - exit 4) | intent-cli-98f |
| intent-cli-766 (ready start - exit 2) | intent-cli-noq |
| intent-cli-5ns (plan - exit 4) | intent-cli-7cs |
| intent-cli-8mh (ai aggregate - exit 4) | intent-cli-if4 |
| intent-cli-4ai (history - exit 4) | intent-cli-ex8 |
| intent-cli-8i5 (beads-regenerate - exit 4) | intent-cli-auw |
| intent-cli-dxi (bead-status - exit 4) | intent-cli-xbd |
| intent-cli-m4c (parse - exit 1) | intent-cli-1yn |
| intent-cli-bzz (lint - exit 1) | intent-cli-sa4 |

```bash
# Example: Close both together after fixing prompt command
bd close intent-cli-o7m --reason "Fixed prompt command exit code" --json
bd close intent-cli-ffj --reason "Verified prompt command logic" --json
```

## Be the First to Claim These (Start Here!)

**3 Best Starting Points**:
1. **intent-cli-wb2** - Build test suite (enables all validation)
2. **intent-cli-6oy** - Set up dogfood assessment (AI agent support)
3. **intent-cli-aki** - Investigate why blocked (might unblock quickly)

---

## Quick Commands

```bash
# What's ready to work on?
bd ready

# Claim a bead
bd update <bead-id> --status in_progress

# Complete a bead
bd close <bead-id> --reason "Done"

# Check bead details
bd show <bead-id>

# View all open beads
bd list --status open
```
