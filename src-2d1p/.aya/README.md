# Architect Agent - Contract & Test Generation

## Overview

The Architect Agent is a continuous monitoring system that automatically generates Rust contracts and Martin Fowler test plans for beads in the Oya project. This agent replaces the previously completed agent `a9b95ff`.

## Purpose

- **Generate Rust Contracts**: Functional specifications with zero-panic, zero-unwrap requirements
- **Create Test Plans**: Martin Fowler-style comprehensive test strategies
- **Bridge Triage to Implementation**: Convert requirements into builder-ready artifacts

## Workflow

The agent follows this continuous loop:

1. **Discover**: Poll for beads needing architecture work
   - Open beads without `has-rust-contract` label

2. **Claim**: Reserve bead for architecture work
   - Update status to `in_progress`
   - Add label `stage:architecting`
   - Track actor as `architect-replacement`

3. **Generate**: Create architecture artifacts
   - Rust contract with functional requirements
   - Martin Fowler test plan with test pyramid
   - Output to `.aya/contracts/` and `.aya/test-plans/`

4. **Complete**: Mark ready for builders
   - Update status to `open`
   - Add labels: `stage:ready-builder`, `has-rust-contract`, `has-tests`
   - Contract revision marked as `draft`

5. **Loop**: Continue monitoring for new work

## Usage

### Start the Agent

```bash
# Run in foreground (with logging)
./.aya/architect-agent-v2.sh

# Run in background
nohup ./.aya/architect-agent-v2.sh > /dev/null 2>&1 &

# Check if running
pgrep -f architect-agent-v2.sh
```

### Monitor Progress

```bash
# Tail the log
tail -f /home/lewis/src/oya/.aya/architect-agent.log

# Check generated contracts
ls -la /home/lewis/src/oya/.aya/contracts/

# Check test plans
ls -la /home/lewis/src/oya/.aya/test-plans/
```

### Stop the Agent

```bash
pkill -f architect-agent-v2.sh
# Or press Ctrl+C if running in foreground
```

## Generated Artifacts

### Rust Contract Format

Each contract includes:

- **Overview**: Bead description and context
- **Functional Requirements**: API surface and behavior
- **Error Handling**: Comprehensive error types with `thiserror`
- **Performance Requirements**: Latency, throughput, memory targets
- **Integration Points**: Dependencies and consumers
- **Acceptance Criteria**: Checklist for completion

Example:
```bash
cat /home/lewis/src/oya/.aya/contracts/rust-contract-src-38tm.md
```

### Martin Fowler Test Plan Format

Each test plan includes:

- **Test Strategy**: Balanced pyramid (70% unit, 20% integration, 10% E2E)
- **Test Categories**: Detailed test descriptions with code examples
- **Test Organization**: Directory structure and naming conventions
- **Mock Strategy**: Guidelines for fakes vs mocks
- **Performance Tests**: Criterion benchmarks
- **Test Metrics**: Coverage and execution time targets

Example:
```bash
cat /home/lewis/src/oya/.aya/test-plans/martin-fowler-tests-src-38tm.md
```

## Integration with Oya Workflow

### Before Architect Agent

```bash
# Triaged by bv --robot-triage
br list --status open
```

### During Architect Agent

```bash
# Agent claims bead
br show src-38tm --json
# Status: in_progress
# Labels: stage:architecting,actor:architect-replacement
```

### After Architect Agent

```bash
# Ready for builders
br show src-38tm
# Status: open
# Labels: stage:ready-builder,has-rust-contract,has-tests

# Builder agent can now implement
# Use /rust-contract skill or functional-rust-generator
```

## Quality Gates

The agent enforces:

- **Zero Panics**: All contracts forbid `panic!`, `todo!`, `unimplemented!`
- **Zero Unwraps**: All contracts forbid `unwrap()`, `expect()`
- **Railway-Oriented Programming**: All functions use `Result<T, E>`
- **Comprehensive Testing**: Martin Fowler test pyramid with >90% coverage goal

## Customization

### Adjust Polling Interval

Edit `SLEEP_INTERVAL` in `architect-agent-v2.sh`:

```bash
SLEEP_INTERVAL=60  # Check every minute instead of 30 seconds
```

### Change Output Directories

Edit directory paths at the top of the script:

```bash
CONTRACT_DIR="/path/to/contracts"
TEST_PLAN_DIR="/path/to/test-plans"
```

### Modify Search Priority

Adjust `find_ready_beads()` function to change bead selection logic.

## Troubleshooting

### No beads found

```bash
# Check available beads
br list --status open

# Verify labels
br list --status open --json | jq -r '.[].labels'
```

### Permission errors

```bash
# Verify br CLI works
br list --status open

# Check bv CLI
bv --robot-triage
```

### Artifacts not generated

Check logs for errors:

```bash
grep ERROR /home/lewis/src/oya/.aya/architect-agent.log
```

## Performance

- **Memory**: Minimal (~10MB resident)
- **CPU**: Low (mostly sleeping)
- **I/O**: Read/write to local disk only
- **Network**: Minimal (beads JSON API calls)

## Next Steps

After the Architect Agent generates contracts:

1. **Review Contracts**: Human review of generated specs
2. **Refine Requirements**: Adjust technical constraints
3. **Assign Builders**: Use builder agent or manual implementation
4. **Track Progress**: Monitor `stage:ready-builder` beads

## Related Documentation

- [CLAUDE.md](/home/lewis/src/oya/CLAUDE.md) - Project instructions
- [BEADS.md](/home/lewis/src/oya/docs/BEADS.md) - Beads workflow reference
- [bv](https://github.com/dicklesworthstone/beads-viewer) - Triage engine

## History

- **a9b95ff**: Previous agent (completed 3 beads)
- **architect-replacement**: Current agent (v2 implementation)

## License

Same as parent Oya project.
