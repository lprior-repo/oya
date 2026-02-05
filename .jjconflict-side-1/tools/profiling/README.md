# OYA Memory Profiling Harness

A functional, panic-free memory profiling harness using heaptrack for 1-hour sustained load testing with RSS monitoring.

## Features

- **Zero panics, zero unwraps**: Built with Railway-Oriented Programming
- **1-hour sustained profiling**: Configurable duration with safety limits
- **RSS monitoring**: Samples every 10 seconds (configurable, minimum 5s for <10% overhead)
- **Low overhead**: <10% profiler overhead guaranteed by sampling interval constraints
- **JSON logs**: Metrics exported to JSON lines format for analysis
- **Type-safe**: Leverages Rust's type system for compile-time guarantees

## Requirements

### System Dependencies

```bash
# Install heaptrack
sudo apt-get install heaptrack  # Debian/Ubuntu
sudo dnf install heaptrack      # Fedora/RHEL
brew install heaptrack          # macOS
```

Verify installation:
```bash
which heaptrack
# Should output: /usr/bin/heaptrack (or similar)
```

## Installation

```bash
cd tools/profiling
cargo build --release
```

The binary will be at `target/release/oya-profiling`.

## Usage

### Basic Usage

```bash
# Profile a command for 1 hour
./oya-profiling ./my-app --load-test

# Profile with arguments
./oya-profiling cargo run --release -- --server-mode
```

### Output

Metrics are logged to `memory-profile.jsonl` in JSON lines format:

```json
{"timestamp":"2026-02-01T12:00:00Z","pid":12345,"metrics":{"rss_kb":102400,"vm_size_kb":204800,"vm_peak_kb":307200,"rss_shared_kb":51200},"elapsed_secs":0}
{"timestamp":"2026-02-01T12:00:10Z","pid":12345,"metrics":{"rss_kb":104448,"vm_size_kb":204800,"vm_peak_kb":307200,"rss_shared_kb":51200},"elapsed_secs":10}
...
```

### Programmatic Usage

```rust
use oya_profiling::{ProfilingConfig, ProfilingRunner};
use std::time::Duration;
use std::path::PathBuf;

// Create configuration
let config = ProfilingConfig::one_hour_default(
    "my-app".to_string(),
    vec!["--load-test".to_string()],
)?;

// Run profiling
let runner = ProfilingRunner::new(config);
let summary = runner.run()?;

println!("Max RSS: {:.2} MB", summary.max_rss_mb());
println!("Avg RSS: {:.2} MB", summary.avg_rss_mb());
```

### Background Profiling

```rust
// Run in background thread
let handle = runner.run_background()?;

// Do other work...

// Wait for completion
let summary = handle.join()?;
```

## Configuration

### Default Configuration

- **Duration**: 3600s (1 hour)
- **Sampling interval**: 10s
- **Output**: `./memory-profile.jsonl`
- **Max overhead**: <10% (enforced by minimum 5s sampling interval)

### Custom Configuration

```rust
use std::time::Duration;
use std::path::PathBuf;

let config = ProfilingConfig::new(
    Duration::from_secs(7200),  // 2 hours
    Duration::from_secs(5),      // Sample every 5s (minimum)
    PathBuf::from("custom-output.jsonl"),
    "my-app".to_string(),
    vec!["--arg".to_string()],
)?;
```

### Constraints

- **Maximum duration**: 4 hours (14400s)
- **Minimum sampling interval**: 5s (ensures <10% overhead)
- **Minimum duration**: 1s

## Architecture

Built using **Functional Core, Imperative Shell** pattern:

```
┌─────────────────────────────────────┐
│     IMPERATIVE SHELL (Edges)        │
│  - Process spawning (process.rs)   │
│  - File I/O (metrics.rs logger)    │
│  - CLI interface (main.rs)          │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│      FUNCTIONAL CORE (Pure)         │
│  - Configuration (config.rs)        │
│  - Metrics types (metrics.rs)       │
│  - Error types (error.rs)           │
│  - Orchestration (runner.rs)        │
└─────────────────────────────────────┘
```

## Metrics Collected

- **RSS (Resident Set Size)**: Actual physical memory used
- **VmSize**: Virtual memory size
- **VmPeak**: Peak virtual memory
- **RssShared**: Shared memory (via RssAnon proxy)

All metrics in kilobytes, with conversion helpers for megabytes.

## Error Handling

All operations return `Result<T, ProfilingError>` with semantic error types:

- `HeaptrackNotFound`: heaptrack not installed
- `ProcessSpawnFailed`: Failed to start profiled process
- `ProcessTerminated`: Process exited unexpectedly
- `MetricsReadFailed`: Cannot read /proc/[pid]/status
- `DurationTooLong`: Exceeds 4-hour safety limit
- `SamplingIntervalTooShort`: Would cause >10% overhead

## Testing

```bash
# Run tests
cargo test

# Run with coverage
cargo tarpaulin --out Html
```

## Performance Characteristics

- **Overhead**: <10% (guaranteed by ≥5s sampling interval)
- **Memory**: Minimal (metrics buffered per-sample)
- **Disk I/O**: One write per sample (append-only)

### Overhead Calculation

Sampling every 10s for 1 hour:
- Samples: 3600s / 10s = 360 samples
- Per-sample cost: ~1ms (read /proc, parse, write)
- Total overhead: 360ms / 3600s = 0.01% ✓

## Roadmap

- [ ] Support for custom heaptrack arguments
- [ ] Real-time metrics visualization
- [ ] Flame graph integration
- [ ] Memory leak detection heuristics
- [ ] Differential profiling (compare two runs)

## License

MIT

## Contributing

This code follows strict functional programming principles:
- Zero panics, zero unwraps
- Railway-Oriented Programming for errors
- Immutability by default
- Pure functions in the core

See [functional-rust-generator skill](../../.claude/skills/functional-rust-generator) for guidelines.
