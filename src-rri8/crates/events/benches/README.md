# Event Sourcing Benchmarks

This directory contains Criterion benchmarks for the oya-events crate.

## Overview

These benchmarks measure the performance overhead of fsync operations in the event-sourcing system.

### Benchmarks

1. **fsync_overhead** - Measures fsync latency for single and batch event appends
2. **event_throughput** - Measures sustained throughput under continuous load

## Running Benchmarks

### Run all benchmarks
```bash
cargo bench
```

### Run specific benchmark
```bash
cargo bench --bench fsync_overhead
cargo bench --bench event_throughput
```

### Generate HTML reports
```bash
cargo bench -- --save-baseline main
```

Results are saved to `target/criterion/` and can be viewed as HTML reports.

## Performance Targets

### Fsync Overhead Benchmarks
- **Append with fsync**: p99 < 3ms
- **Append without fsync** (baseline): p99 < 0.5ms
- **Batch append**: Cost per event amortizes with batch size

### Throughput Benchmarks
- **Read 1000 events**: < 50ms
- **Replay 1000 events**: < 5s
- **Sustained append throughput**: > 100 events/sec

## Implementation Notes

All benchmarks follow functional Rust patterns:
- Zero panics (no `panic!`, `unwrap`, `expect`)
- Zero unwraps
- Railway-oriented error handling with `Result<T, E>`
- Early returns on setup failures with `eprintln!` logging

## Contract and Test Plan

See:
- `CONTRACT_SPEC.md` - Design by contract specification
- `MARTIN_FOWLER_TESTS.md` - Martin Fowler test plan with Given-When-Then scenarios
