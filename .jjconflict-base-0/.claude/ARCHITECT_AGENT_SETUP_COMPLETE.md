# Architect Agent 2 - Setup Complete

## Status: ✅ Operational

Architect Agent 2 has been successfully configured and tested. The agent continuously processes development beads, generating Rust contracts and Martin Fowler test plans.

## What Was Done

### 1. Created Agent Scripts

Three scripts have been created in `/tmp/`:

- **`/tmp/architect_agent_2_loop.sh`** - Main continuous loop agent
- **`/tmp/architect_agent_run.sh`** - Single-run version for testing
- **`/tmp/label_ready_architect.sh`** - Helper to label beads

### 2. Successfully Processed Test Beads

The agent processed 3 beads successfully:

| Bead ID | Title | Contract | Test Plan |
|---------|-------|----------|-----------|
| src-3ax5 | guest: Create Zellij WASM plugin scaffold | `/tmp/rust-contract-src-3ax5.md` | `/tmp/martin-fowler-tests-src-3ax5.md` |
| src-3inn | guest: Implement IPC client in Zellij plugin | `/tmp/rust-contract-src-3inn.md` | `/tmp/martin-fowler-tests-src-3inn.md` |
| src-wkni | host: Implement IPC worker in oya-orchestrator | `/tmp/rust-contract-src-wkni.md` | `/tmp/martin-fowler-tests-src-wkni.md` |

### 3. Current System State

- **Ready-architect beads**: 0 (all processed)
- **Ready-for-builder beads**: 3 (awaiting implementation)
- **Generated contracts**: 6
- **Generated test plans**: 5

## How to Use

### Start the Continuous Agent

```bash
# Run in background
nohup /tmp/architect_agent_2_loop.sh > /tmp/architect_agent_2.log 2>&1 &

# Or run in terminal for testing
/tmp/architect_agent_2_loop.sh
```

### Label More Beads

```bash
# Label 5 more beads for processing
br list --status open --json | jq -r '.[0:5] | .[].id' | while read id; do
    br update "$id" --set-labels "stage:ready-architect" && echo "✓ Labeled $id"
done

# Or use the helper script
/tmp/label_ready_architect.sh 5
```

### Monitor Progress

```bash
# View agent logs
tail -f /tmp/architect_agent_2.log

# Check ready-architect beads
br list --status open --json | jq '[.[] | select(.labels and (.labels | index("stage:ready-architect")))] | .[] | {id, title}'

# Check ready-for-builder beads
br list --status open --json | jq '[.[] | select(.labels and (.labels | index("stage:ready-builder")))] | .[] | {id, title}'

# List generated artifacts
ls -lt /tmp/rust-contract-*.md | head -5
ls -lt /tmp/martin-fowler-tests-*.md | head -5
```

### View Generated Artifacts

```bash
# View contract for a specific bead
cat /tmp/rust-contract-src-3ax5.md

# View test plan for a specific bead
cat /tmp/martin-fowler-tests-src-3ax5.md
```

## Workflow Details

### 1. Triage Phase (Preparation)

Before the agent can process beads, they must be labeled:

```bash
# Manual labeling
br update <bead-id> --set-labels "stage:ready-architect"

# Batch labeling (top 10 open beads)
br list --status open --json | jq -r '.[0:10] | .[].id' | while read id; do
    br update "$id" --set-labels "stage:ready-architect"
done
```

### 2. Architect Phase (Processing)

The agent continuously:

1. Finds beads labeled `stage:ready-architect` with status `open`
2. Claims them:
   - Status: `open` → `in_progress`
   - Labels: `stage:ready-architect` → `stage:architecting`
   - Actor: `architect-2`
3. Generates artifacts:
   - Rust contract → `/tmp/rust-contract-{id}.md`
   - Martin Fowler test plan → `/tmp/martin-fowler-tests-{id}.md`
4. Hands off to builders:
   - Status: `in_progress` → `open`
   - Labels: `stage:architecting` → `stage:ready-builder,has-rust-contract,has-tests`

### 3. Builder Phase (Implementation)

Separate builder agents (TODO: implement) will:
- Find beads labeled `stage:ready-builder`
- Follow the Rust contract for API design
- Implement Martin Fowler test plan
- Mark as complete when done

## Generated Artifacts

### Rust Contract Template

Each contract includes:

1. **Overview** - Context from bead description
2. **Functional Requirements** - Detailed requirements
3. **API Contract**:
   - Types (structs, enums with proper derives)
   - Functions (all return Result<T, E>)
   - Zero unwrap, zero panic requirements
4. **Performance Constraints** - Latency, memory, throughput
5. **Testing Requirements** - Unit, integration, property-based
6. **Implementation Notes** - Functional patterns, ROP

### Martin Fowler Test Plan Template

Each test plan includes:

1. **Test Strategy** - Based on Martin Fowler's principles
2. **Test Pyramid** - 80% unit, 15% integration, 5% E2E
3. **Test Categories**:
   - Happy Path (normal operation)
   - Sad Path (error conditions)
   - Edge Cases (boundary values)
4. **Coverage Requirements** - >90% line, >85% branch
5. **Test Organization** - unit/, integration/, e2e/, fixtures/
6. **Mock Strategy** - mockall, fakes, test doubles
7. **CI Integration** - Parallel execution, fail fast

## Future Enhancements

### 1. Skill Integration

Currently using templates. Future versions should integrate:

- **rust-contract skill** - Generate exhaustive KIRK contracts
- **planner skill** - Generate Martin Fowler test plans
- These skills would be called from the agent instead of using templates

### 2. Parallel Processing

Process multiple beads concurrently:

```bash
# Spawn multiple workers
for i in {1..3}; do
    /tmp/architect_agent_2_loop.sh &
done
```

### 3. Quality Gates

Add validation before marking as ready:

- Check bead description quality
- Validate contract completeness
- Ensure test plan coverage
- Reject if description is insufficient

### 4. Metrics

Track performance:

- Processing time per bead
- Artifact quality scores
- Builder feedback loop
- Auto-improve templates

### 5. Auto-Labeling

Automatically label beads that need architectural work:

```bash
# Find beads with "Create" or "Implement" in title
br list --status open --json | jq -r '
    [.[] | select(.title | test("Create|Implement|Add"; "i")) |
    .id] | .[0:10][]' | while read id; do
    br update "$id" --set-labels "stage:ready-architect"
done
```

## Troubleshooting

### Agent Not Finding Beads

**Problem**: "No ready beads found" continuously

**Solution**: Label more beads

```bash
br list --status open --json | jq -r '.[0:5] | .[].id' | while read id; do
    br update "$id" --set-labels "stage:ready-architect"
done
```

### Failed to Claim Bead

**Problem**: "Failed to claim $bead_id"

**Solution**: Another agent may have claimed it. The agent will continue to the next bead.

### No Description Found

**Problem**: Bead has no description

**Solution**: The agent marks it as ready-for-builder anyway (contract/test plan will be minimal)

### Artifacts Not Generated

**Problem**: No .md files in /tmp/

**Solution**: Check permissions and disk space:

```bash
ls -ld /tmp
df -h /tmp
```

## Integration with Overall Workflow

This architect agent fits into the larger parallel agent workflow:

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Triage (bv --robot-triage)                               │
│    ↓                                                        │
│ 2. Claim (br update --status in_progress)                   │
│    ↓                                                        │
│ 3. Isolate (zjj add <session>)                              │
│    ↓                                                        │
│ 4. Architect (THIS AGENT)                                   │
│    - Generate Rust contract                                 │
│    - Generate Martin Fowler tests                           │
│    ↓                                                        │
│ 5. Build (TODO: builder agent)                              │
│    - Follow contract                                        │
│    - Implement tests                                        │
│    ↓                                                        │
│ 6. Review (red-queen skill)                                 │
│    ↓                                                        │
│ 7. Land (land skill)                                        │
│    - Moon quick check                                       │
│    - jj commit                                              │
│    - br sync --flush-only                                   │
│    - jj git push                                            │
│    ↓                                                        │
│ 8. Merge (zjj done)                                         │
└─────────────────────────────────────────────────────────────┘
```

## Next Steps

1. **Start the continuous agent** in background
2. **Label more beads** for processing
3. **Implement builder agent** to pick up ready-for-builder beads
4. **Integrate skills** (rust-contract, planner) for better artifact generation
5. **Add monitoring** and metrics
6. **Create more agents** (architect-1, architect-3) for parallel processing

## Files Reference

| File | Purpose |
|------|---------|
| `/tmp/architect_agent_2_loop.sh` | Continuous loop agent (main) |
| `/tmp/architect_agent_run.sh` | Single-run version (testing) |
| `/tmp/label_ready_architect.sh` | Label beads helper |
| `/tmp/rust-contract-*.md` | Generated Rust contracts |
| `/tmp/martin-fowler-tests-*.md` | Generated test plans |
| `/tmp/architect_agent_2.log` | Agent log file (if running in background) |

## Quick Commands Reference

```bash
# Start agent
/tmp/architect_agent_2_loop.sh

# Label beads (batch of 5)
br list --status open --json | jq -r '.[0:5] | .[].id' | while read id; do
    br update "$id" --set-labels "stage:ready-architect" && echo "✓ $id"
done

# Check status
echo "Ready-architect: $(br list --status open --json | jq '[.[] | select(.labels and (.labels | index("stage:ready-architect")))] | length')"
echo "Ready-for-builder: $(br list --status open --json | jq '[.[] | select(.labels and (.labels | index("stage:ready-builder")))] | length')"

# View artifacts
ls -lt /tmp/rust-contract-*.md | head -3
cat /tmp/rust-contract-src-3ax5.md
```

---

**Status**: Ready for production use
**Last Updated**: 2026-02-07 23:06:00
**Maintained By**: architect-2
