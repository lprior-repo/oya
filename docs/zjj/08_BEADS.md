# Beads: Issue Tracking & Triage

Issue tracking and intelligent triage using Beads (graph-aware dependency system).

## Core Concept

Beads stores issues in `.beads/beads.jsonl`. Each bead (issue) is a node in a dependency graph. Use `bv` (Beads triage engine) to understand scope, dependencies, and prioritization.

## Creating Issues

### Basic Issue

```bash
bd add --title "Feature: implement X"

# With description
bd add --title "Bug: Y fails on Z" \
  --description "Steps to reproduce:
1. Do X
2. Observe Y
3. Should see Z instead"

# With priority and labels
bd add --title "Feature: add validation" \
  --priority high \
  --label feature \
  --label "p0"
```

### Issue Templates

**Feature**:
```bash
bd add --title "Feature: description" --label feature --priority high
```

**Bug**:
```bash
bd add --title "Bug: what fails" --label bug --priority high \
  --description "Steps: 1. ... 2. ... Expected: ... Actual: ..."
```

**Chore**:
```bash
bd add --title "Chore: refactor X" --label chore --priority medium
```

## Managing Issues

### List & Filter

```bash
bd list                              # All open issues
bd list --filter "status:open"       # Only open
bd list --filter "assigned:me"       # My issues
bd list --filter "label:feature"     # By label
bd list --filter "priority:high"     # By priority
```

### Claiming Issues

```bash
bd claim BD-123          # Start working
bd claim BD-123 --show   # See details
```

### Updating Status

```bash
bd resolve BD-123        # Mark ready for review
bd complete BD-123       # Mark done
bd unresolved BD-123     # Reopen
```

### Adding Dependencies

```bash
bd link BD-123 BD-124    # BD-123 blocks BD-124
bd unlink BD-123 BD-124  # Remove dependency
```

## Using `bv` for Triage

**`bv` is your triage engine.** It computes graph metrics (PageRank, critical path, cycles) and provides intelligent recommendations.

### Start Here: `bv --robot-triage`

```bash
bv --robot-triage
```

Returns in one call:
- `quick_ref` - At-a-glance summary (counts, top 3 picks)
- `recommendations` - Ranked actionable items with reasons
- `quick_wins` - Low-effort, high-impact tasks
- `blockers_to_clear` - Tasks that unblock most work
- `project_health` - Status/type distributions, graph metrics
- `commands` - Copy-paste commands for next steps

### Quick Next Steps

```bash
bv --robot-next    # Just the single top pick + claim command
```

### Planning & Parallel Work

```bash
bv --robot-plan              # Parallel execution tracks with unblock dependencies
bv --robot-plan --label core # Scope to "core" label subgraph
```

### Graph Analysis

```bash
bv --robot-insights  # Full metrics:
                     # - PageRank (importance)
                     # - Betweenness (bottleneck)
                     # - Critical path (minimum time to completion)
                     # - Cycles (circular dependencies - must fix!)
                     # - K-core (dense subgroups)
                     # - Eigenvector (authority)

bv --robot-insights | jq '.Cycles'  # Find cycles
```

### Label & Flow Analysis

```bash
bv --robot-label-health           # Health by label: velocity, staleness, block count
bv --robot-label-flow             # Cross-label dependencies and bottlenecks
bv --robot-label-attention        # What needs attention most (PageRank × staleness)
```

### History & Change Tracking

```bash
bv --robot-history                        # Bead-to-commit correlations
bv --robot-diff --diff-since HEAD~10     # What changed (new/closed/modified)
```

### Forecasting & Burndown

```bash
bv --robot-forecast BD-123  # ETA prediction with deps
bv --robot-burndown sprint  # Sprint burndown tracking
```

### Alerts & Hygiene

```bash
bv --robot-alerts   # Stale issues, blocking cascades, priority mismatches
bv --robot-suggest  # Hygiene: duplicates, missing deps, cycle breaks
```

### Export & Visualization

```bash
bv --robot-graph --graph-format json   # JSON dependency graph
bv --robot-graph --graph-format mermaid # Mermaid diagram
bv --export-graph graph.html           # Interactive visualization
```

## Filtering with Recipes

```bash
bv --recipe actionable --robot-plan     # Only ready-to-work (no blockers)
bv --recipe high-impact --robot-triage  # Only high PageRank
```

## Scoping by Dimension

```bash
bv --robot-plan --label backend         # Just backend work
bv --robot-insights --as-of HEAD~30     # Historical snapshot
bv --robot-triage --robot-triage-by-label  # Grouped by domain
bv --robot-triage --robot-triage-by-track  # Grouped by parallel tracks
```

## Understanding Output

Every `bv` response includes:
- `data_hash` - Fingerprint of beads.jsonl (verify consistency)
- `status` - Metric readiness: `computed|approx|timeout|skipped`
- `as_of_commit` - When using `--as-of`

### Two Phases of Analysis

**Phase 1 (instant)**: degree, topo sort, density
**Phase 2 (async, 500ms timeout)**: PageRank, betweenness, HITS, eigenvector, cycles

For large graphs (>500 nodes), some metrics may be approximated. Always check `status`.

## jq Cheatsheet

```bash
bv --robot-triage | jq '.quick_ref'                     # Summary
bv --robot-triage | jq '.recommendations[0]'            # Top pick
bv --robot-plan | jq '.plan.summary.highest_impact'     # Best unblock target
bv --robot-insights | jq '.Cycles'                      # Circular deps
bv --robot-insights | jq '.status'                      # Metric status
bv --robot-label-health | jq '.results.labels[] | select(.health_level == "critical")'
```

## Workflow Integration

### Morning: Triage

```bash
# Get recommendations
bv --robot-triage

# Pick top item
bv --robot-next

# Claim it
bd claim BD-123
```

### During Work: Track Progress

```bash
# See where we are
bv --robot-plan --label core

# Update as you finish
bd complete BD-123
```

### End of Day: Health Check

```bash
# Any blockers?
bv --robot-alerts

# Cycles introduced?
bv --robot-insights | jq '.Cycles'

# Health by area?
bv --robot-label-health
```

## Label Standards

Use consistent labels:

```
epic        - Large feature (multiple issues)
feature     - New functionality
bug         - Something broken
chore       - Maintenance, refactoring, tooling
p0, p1, p2  - Priority (0=highest)
core        - Core functionality
testing     - Test-related
docs        - Documentation
```

## Dependency Management

### Creating Dependencies

Good reasons to link issues:
- "BD-124 can't start until BD-123 is done"
- "BD-124 is a subtask of BD-123"
- "BD-124 requires output from BD-123"

```bash
bd link BD-123 BD-124  # BD-123 blocks BD-124
```

### Checking for Cycles

```bash
bv --robot-insights | jq '.Cycles | length'

# If > 0, you have circular dependencies
# Break them before continuing
```

### Critical Path

```bash
bv --robot-insights | jq '.CriticalPath'  # Longest dependency chain
```

## Graph Metrics Explained

| Metric | Meaning | Use |
|--------|---------|-----|
| PageRank | Importance in graph | Higher = more critical |
| Betweenness | Bottleneck potential | High = unblock many tasks |
| Critical Path | Min time to done | Shows deadline pressure |
| Cycles | Circular deps | Must eliminate |
| Eigenvector | Authority/hubness | Like PageRank but iterative |
| K-core | Dense subgroups | Tightly coupled work |

## Best Practices

1. **Create issues early** - Don't wait until starting work
2. **Link dependencies** - Especially blockers
3. **Use labels** - For grouping and filtering
4. **Run `bv --robot-triage` daily** - Catch issues early
5. **Break cycles immediately** - Never ignore circular deps
6. **Estimate (optional)** - For forecast accuracy

## Common Workflows

### Feature Development

```bash
# Create epic
bd add --title "Epic: feature X" --label epic --priority high

# Break into tasks
bd add --title "Feature: part 1" --label feature --priority high
bd add --title "Feature: part 2" --label feature --priority high
bd add --title "Tests: feature X" --label testing

# Link to epic
bd link BD-epic BD-part1
bd link BD-epic BD-part2
bd link BD-epic BD-tests

# Triage
bv --robot-plan

# Work
bd claim BD-part1
# ... implement ...
bd complete BD-part1
```

### Bug Triage

```bash
# Report bug
bd add --title "Bug: X fails" --label bug --priority high

# Find impact
bv --robot-insights | jq '.PageRank[] | select(.id == "BD-123")'

# Estimate effort
bd claim BD-123
# ... investigate ...
bd resolve BD-123  # Ready for review
```

## Performance Notes

- **Phase 1** (instant): degree, topo, density
- **Phase 2** (async): PageRank, betweenness, HITS, eigenvector, cycles (500ms timeout)
- **Prefer `--robot-plan`** over `--robot-insights` when speed matters
- **Results cached** by data hash (no redundant computation)

## Integration with Development

1. **Create issue** → `bd add ...`
2. **Claim issue** → `bd claim BD-123`
3. **Make branch** → `jj bookmark set feature/...` (implicit in ZJJ)
4. **Work** → Edit files, commit with `jj describe`
5. **Push** → `jj git push`
6. **Close** → `bd complete BD-123`

All connected through Beads dependency graph and tracked by `bv`.

---

**Next**: [Version Control with Jujutsu](09_JUJUTSU.md)
