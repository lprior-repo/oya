# Martin Fowler Test Plan: Batch Append Benchmark

## Overview

This test plan specifies **benchmark behavior as executable specification**. Each test is a self-documenting scenario that verifies performance characteristics, not correctness.

**Benchmark Purpose**:
- Measure batch append throughput vs single append
- Verify fsync amortization benefits
- Establish performance regression baseline

**Test Philosophy**:
- Benchmarks ARE specifications (expressive names)
- Given-When-Then structure for clarity
- Happy path only (performance focus)
- Statistical significance (10s measurement, 100 samples)

## Happy Path Tests

### Test: `bench_batch_append_throughput_measures_events_per_second`

**Given**: A configured Criterion benchmark group
- Warm-up time: 3 seconds
- Measurement time: 10 seconds
- Sample size: 100 iterations
- Event payload: 1KB per event

**When**: Benchmarking batch append with varying sizes
- Batch sizes: [1, 10, 50, 100, 500, 1000]
- Fresh DurableEventStore per iteration
- Fresh Tokio runtime per iteration

**Then**:
- Throughput is calculated (events/sec)
- Statistical distribution is generated (mean, median, p90, p95, p99)
- Report includes batch size labels for comparison
- No panics or unwrap violations occur

### Test: `bench_single_append_baseline_establishes_floor`

**Given**: A configured Criterion benchmark group
- Single event append (baseline for comparison)
- Event payload: 1KB
- Fresh DurableEventStore per iteration

**When**: Benchmarking single event append
- 100 iterations for statistical significance
- Isolated environment per iteration

**Then**:
- Baseline latency is measured (expected: ~1-3ms per event)
- Throughput is calculated (expected: ~300-1000 events/sec)
- Report provides comparison baseline for batch append
- No errors or panics occur

### Test: `bench_single_vs_batch_comparison_shows_amortization`

**Given**: Two benchmark scenarios in same group
- Scenario A: 100 single appends (100 fsyncs)
- Scenario B: 1 batch append of 100 events (1 fsync)
- Equal total events: 100
- Event payload: 1KB per event

**When**: Running comparison benchmark
- Same measurement time for both (10s)
- Same sample size (100 iterations)

**Then**:
- Batch append is 10x+ faster than single append
- Fsync amortization benefit is demonstrated
- Report shows side-by-side comparison
- Performance improvement is visible in Criterion output

### Test: `bench_fsync_amortization_verifies_single_fsync_per_batch`

**Given**: Batch append benchmark with varying sizes
- Batch sizes: [10, 50, 100, 500, 1000]
- Event payload: 1KB per event

**When**: Benchmarking batch append throughput
- Each batch size is benchmarked separately
- Fresh DurableEventStore per iteration

**Then**:
- All batch sizes complete successfully
- Throughput scales with batch size (amortization benefit)
- Report shows linear or super-linear scaling
- Single fsync per batch is verified (via append_batch contract)

## Error Path Tests

### Test: `bench_handles_temp_dir_failure_gracefully`

**Given**: Benchmark initialization
- Tempfile creation fails (e.g., disk full)

**When**: Running benchmark setup

**Then**:
- Error is logged to stderr
- Benchmark iteration is skipped (not panics)
- Other benchmark groups continue running
- Exit code is 0 (graceful degradation)

### Test: `bench_handles_database_init_failure_gracefully`

**Given**: Benchmark initialization
- SurrealDB initialization fails (e.g., port conflict)

**When**: Running benchmark setup

**Then**:
- Error is logged with context
- Benchmark iteration is skipped
- Other sizes in group continue if possible
- No panic or unwrap violation

### Test: `bench_handles_store_creation_failure_gracefully`

**Given**: Database initialized successfully
- DurableEventStore creation fails (e.g., WAL permission error)

**When**: Running benchmark iteration

**Then**:
- Error is logged with reason
- Iteration is skipped
- Benchmark continues with next size
- No crash or hang

## Edge Case Tests

### Test: `bench_handles_batch_size_1_correctly`

**Given**: Batch size of 1 (edge case)
- Smallest possible batch
- Tests batch overhead without amortization benefit

**When**: Benchmarking batch append with size=1

**Then**:
- Benchmark completes successfully
- Performance is similar to single append
- Throughput is measured correctly
- No errors occur

### Test: `bench_handles_max_batch_size_1000_correctly`

**Given**: Batch size of 1000 (maximum allowed)
- Tests precondition boundary
- Maximum amortization benefit

**When**: Benchmarking batch append with size=1000

**Then**:
- Benchmark completes successfully
- Best throughput (maximum amortization)
- No BatchTooLarge error
- Memory usage is reasonable

### Test: `bench_handles_large_payload_10kb_correctly`

**Given**: Event payload size of 10KB
- Tests serialization overhead
- Total batch size: 1MB to 10MB

**When**: Benchmarking with 10KB events

**Then**:
- Benchmark completes successfully
- Throughput scales with payload size
- Serialization time is included in measurement
- No memory issues

## Contract Verification Tests

### Test: `verify_precondition_batch_size_between_1_and_1000`

**Given**: Batch append benchmark
- Batch sizes tested: [1, 10, 50, 100, 500, 1000]

**When**: Running benchmarks

**Then**:
- All batch sizes are within valid range
- No EmptyBatch error occurs
- No BatchTooLarge error occurs
- Precondition is satisfied for all iterations

### Test: `verify_postcondition_single_fsync_per_batch`

**Given**: Batch append implementation
- Method: DurableEventStore::append_batch()

**When**: Benchmarking batch append

**Then**:
- Exactly one fsync is performed per batch
- Fsync happens after WAL write
- All events are flushed before fsync returns
- SurrealDB write happens after fsync

### Test: `verify_invariant_event_order_preserved`

**Given**: Batch of events with known order
- Events: [E1, E2, E3, ..., E100]
- Each has unique BeadId

**When**: Calling append_batch()

**Then**:
- Returned EventIds are in same order as input
- E1 → EventId[0], E2 → EventId[1], etc.
- No reordering occurs
- Invariant is verified by implementation

### Test: `verify_invariant_atomic_batch_operation`

**Given**: Batch append operation
- 100 events in batch

**When**: append_batch() executes

**Then**:
- All events are persisted or none are
- WAL write is atomic (single write syscall)
- SurrealDB insert is atomic (batch operation)
- No partial persistence occurs

## Given-When-Then Scenarios

### Scenario 1: Warm-up Allows JIT Optimization

**Given**: Cold benchmark start
- No previous iterations
- Criterion warm-up configured: 3 seconds

**When**: Running first 3 seconds of benchmarks

**Then**:
- JIT compilation occurs during warm-up
- Warm-up iterations are not measured
- Measurement starts after warm-up completes
- Stable performance during measurement phase

### Scenario 2: Statistical Significance Achieved

**Given**: Benchmark configuration
- Measurement time: 10 seconds
- Sample size: 100 iterations
- Batch size: 100 events

**When**: Running full benchmark

**Then**:
- 100 data points are collected
- Mean and standard deviation are calculated
- Percentiles are computed (p50, p90, p95, p99)
- Confidence interval is available
- Results are statistically significant

### Scenario 3: Throughput Calculation Is Correct

**Given**: Completed benchmark iteration
- Duration: 100ms for batch
- Batch size: 100 events

**When**: Calculating throughput

**Then**:
- Throughput = events / duration
- Throughput = 100 / 0.1s = 1000 events/sec
- Formula is applied consistently
- Units are clearly labeled (events/sec)

### Scenario 4: Comparison Shows Amortization Benefit

**Given**: Two benchmark results
- Single append: 500 events/sec (2ms per event)
- Batch append (100 events): 5000 events/sec (0.2ms per event amortized)

**When**: Comparing results in Criterion report

**Then**:
- Batch append is 10x faster (5000 / 500 = 10x)
- Fsync amortization is visible (1 vs 100 fsyncs)
- Improvement is clearly shown in output
- Comparison is fair (same total events)

### Scenario 5: Memory Usage Is Bounded

**Given**: Large batch benchmark
- Batch size: 1000 events
- Event payload: 1KB each
- Total batch size: 1MB

**When**: Running benchmark iteration

**Then**:
- Memory usage is ~1MB + overhead
- No memory leak occurs
- Memory is freed after each iteration
- RAII fixtures cleanup correctly
- Benchmark is repeatable

## Performance Regression Tests

### Test: `bench_regression_batch_append_throughput_not_degraded`

**Given**: Established baseline
- Batch append (100 events): 5000 events/sec
- Measured on commit ABC1234

**When**: Running benchmark on new code

**Then**:
- Throughput is within 10% of baseline
- If slower: regression detected
- Benchmark fails if regression > 10%
- Criterion comparison highlights difference

### Test: `bench_regression_single_vs_batch_ratio_maintained`

**Given**: Established baseline
- Batch vs single ratio: 10x improvement
- Measured on commit ABC1234

**When**: Running comparison benchmark

**Then**:
- Ratio is still 10x or better
- If ratio drops: fsync amortization broken
- Benchmark fails if ratio < 8x (20% degradation)
- Root cause must be investigated

## Test Organization

### File Structure

```
crates/events/benches/
├── batch_append.rs          # Main benchmark file
├── batch_append_tests.md    # This file
└── batch_append_contract.md # Contract specification
```

### Benchmark Groups

1. **batch_append_throughput**: Primary benchmark
   - Varies batch size: 1, 10, 50, 100, 500, 1000
   - Measures events/sec
   - 10s measurement, 100 samples

2. **single_vs_batch_comparison**: Comparison benchmark
   - Single append: 100 events sequentially
   - Batch append: 100 events in one batch
   - Demonstrates fsync amortization

3. **fsync_amortization**: Verification benchmark
   - Batch sizes: 10, 50, 100, 500, 1000
   - Verifies single fsync per batch
   - Shows scaling behavior

### Running the Benchmarks

```bash
# Run all batch append benchmarks
cargo bench --bench batch_append

# Run specific benchmark group
cargo bench --bench batch_append -- batch_append_throughput

# Save baseline for regression testing
cargo bench --bench batch_append -- --save-baseline main

# Compare against baseline
cargo bench --bench batch_append -- --baseline main
```

### Expected Output

```
batch_append_throughput/100
                        time:   [20.123 ms 20.456 ms 20.789 ms]
                        throughput: 4.8134 thousand events/s
                        change: [-2.3% -1.8% -1.2%] (p = 0.01 < 0.05)
                        Performance has improved.

single_vs_batch_comparison/batch_append_100_events
                        time:   [20.123 ms 20.456 ms 20.789 ms]
                        throughput: 4.8134 thousand events/s

single_vs_batch_comparison/single_append_100_events
                        time:   [201.23 ms 205.67 ms 210.12 ms]
                        throughput: 487.2 events/s

Comparison:
  batch_append_100_events: 20.456 ms
  single_append_100_events: 205.67 ms
  Speedup: 10.05x
```

## Test Coverage Matrix

| Test Category | Tests | Coverage |
|--------------|-------|----------|
| Happy Path | 4 | All success scenarios |
| Error Path | 3 | Graceful degradation |
| Edge Cases | 3 | Boundaries and extremes |
| Contract Verification | 4 | All pre/post/invariants |
| Performance Regression | 2 | Baseline comparison |
| **Total** | **16** | **Complete specification** |

## Exit Criteria

All tests must pass before benchmark is considered complete:

- [ ] All benchmarks run without panics
- [ ] Zero unwrap/expect/panic violations
- [ ] Throughput measurements are generated
- [ ] 10x improvement is demonstrated
- [ ] Fsync amortization is verified
- [ ] Statistical significance is achieved
- [ ] Comparison report is generated
- [ ] No memory leaks or crashes
- [ ] Regression tests pass (if baseline exists)
