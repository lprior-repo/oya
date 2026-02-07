# Supervisor Restart Latency Performance Testing

This document describes the performance testing infrastructure for supervisor restart latency, including flamegraph profiling and benchmark execution.

## Requirements

The performance test validates that supervisor restart latency meets the **p99 < 1s** requirement under various load conditions.

## Test Scenarios

### Load Levels

- **Idle**: No concurrent operations (baseline latency)
- **Light**: 10 concurrent operations (simulates normal load)
- **Heavy**: 100 concurrent operations (stress test)

### Metrics Collected

- **p50**: Median restart latency
- **p95**: 95th percentile latency
- **p99**: 99th percentile latency (must be < 1s)
- **min**: Fastest restart
- **max**: Slowest restart
- **mean**: Average restart latency

## Running Benchmarks

### Quick Validation

Run the validation tests (fast, no criterion benchmarks):

```bash
# Run all validation tests
cargo test -p orchestrator --test restart_latency_validation -- --nocapture

# Run specific test
cargo test -p orchestrator --test restart_latency_validation test_restart_latency_idle -- --nocapture
```

### Full Criterion Benchmarks

```bash
# Run all benchmarks (generates HTML report)
cargo bench -p orchestrator --bench restart_latency

# Run specific benchmark group
cargo bench -p orchestrator --bench restart_latency -- restart_latency_idle
cargo bench -p orchestrator --bench restart_latency -- worker_restart

# View HTML report
firefox target/criterion/restart_latency_idle/report/index.html
```

## Flamegraph Profiling

### Prerequisites

Install flamegraph tools:

```bash
# Install flamegraph crate
cargo install flamegraph

# Or use inferno (recommended for better SVG)
cargo install inferno
```

### Basic Flamegraph

Generate a flamegraph for restart operations:

```bash
# Profile the validation test under load
cargo flamegraph -p orchestrator --test restart_latency_validation test_restart_latency_under_load

# View the generated flamegraph
firefox flamegraph.svg
```

### Advanced Profiling

For more detailed profiling:

```bash
# Profile with specific frequency
cargo flamegraph -p orchestrator --bench restart_latency --freq 997

# Profile with custom output
cargo flamegraph -p orchestrator --test restart_latency_validation --output restart_flamegraph.svg

# Profile specific test
cargo flamegraph -p orchestrator --test restart_latency_validation test_restart_latency_heavy_load
```

### Interpreting Flamegraphs

Key areas to examine in the flamegraph:

1. **Actor Spawn Time**: Time to create new actor process
2. **Backoff Delay**: Exponential backoff duration
3. **Message Passing**: Communication overhead
4. **State Restoration**: Time to restore actor state
5. **Lock Contention**: Time spent waiting on locks under load

Look for:
- Wide bars: Functions consuming significant time
- Deep stacks: Complex call chains
- Hot paths: Repeated operations

## Performance Expectations

### Target Latencies

| Load Level | Expected p50 | Expected p95 | Expected p99 |
|------------|--------------|--------------|--------------|
| Idle       | < 350ms      | < 400ms      | < 500ms      |
| Light      | < 550ms      | < 750ms      | < 1000ms     |
| Heavy      | < 650ms      | < 900ms      | < 1000ms     |

### Failure Thresholds

- **p99 >= 1s**: FAIL - exceeds SLO
- **p95 >= 900ms**: WARNING - approaching limit
- **mean >= 500ms**: WARNING - degradation detected

## CI/CD Integration

Add to CI pipeline:

```yaml
performance_test:
  script:
    - cargo test -p orchestrator --test restart_latency_validation
    - cargo bench -p orchestrator --bench restart_latency
  artifacts:
    paths:
      - target/criterion/
    reports:
      performance: target/criterion/report/index.html
```

## Troubleshooting

### High p99 Latency

**Symptoms**: p99 consistently exceeds 1s

**Possible Causes**:
- Exponential backoff too aggressive
- Lock contention in supervisor state
- Slow actor spawn time
- Resource exhaustion (CPU, memory)

**Mitigation**:
- Reduce `base_backoff_ms` in `SupervisorConfig`
- Increase `max_backoff_ms` to cap backoff
- Profile with flamegraph to identify bottlenecks
- Reduce concurrent operations under test

### Inconsistent Results

**Symptoms**: High variance between runs

**Possible Causes**:
- System noise (other processes)
- CPU frequency scaling
- Insufficient warmup

**Mitigation**:
- Run tests multiple times and average
- Disable CPU frequency scaling: `sudo cpupower frequency-set -g performance`
- Increase warmup iterations in benchmark
- Use dedicated testing environment

## Benchmark Comparison

Compare before/after changes:

```bash
# Save baseline
cargo bench -p orchestrator --bench restart_latency -- --save-baseline main

# Make changes...

# Compare against baseline
cargo bench -p orchestrator --bench restart_latency -- --baseline main
```

## References

- [Criterion User Guide](https://bheisler.github.io/criterion.rs/book/)
- [Flamegraph Documentation](https://github.com/flamegraph-rs/flamegraph)
- [Ractor Performance](https://github.com/slawlor/ractor)
- [Supervision Patterns](https://www.erlang.org/doc/man/supervisor.html)
