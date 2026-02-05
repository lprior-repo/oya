# Caching Architecture Guide

This document explains how bazel-remote, sccache, and Moon work together for hyper-fast builds.

## Cache Layers (3-Tier Architecture)

### Tier 1: sccache (Rust Compilation Cache)
**What it caches:** Individual rustc invocations
**Location:** `~/.cache/sccache`
**Size:** 200GB (configured)
**Hit rate:** ~79%

```toml
# ~/.cargo/config.toml
[build]
rustc-wrapper = "/home/lewis/.cargo/bin/sccache"
jobs = 3
```

**How it works:**
1. Wrapper intercepts `cargo build` calls
2. Computes hash of: source code, compiler flags, environment
3. Checks cache for matching hash
4. If hit: returns cached `.rlib`/`.rmeta` instantly
5. If miss: runs rustc, caches result

**What's cached:**
- Compiled crates (`.rlib`, `.rmeta`, `.a` files)
- Individual compilation units per crate
- ~20GB currently (9,735 compile requests, 7,683 hits)

---

### Tier 2: Moon Task Cache (bazel-remote)
**What it caches:** Moon task outputs and hashes
**Location:** `~/.cache/bazel-remote`
**Size:** 500GB (configured)
**Protocol:** gRPC (localhost:9092)
**Hit rate:** Cold (177MB used - needs warmup)

```yaml
# .moon/workspace.yml
unstable_remote:
  host: 'grpc://localhost:9092'
  cache:
    compression: 'zstd'
```

**How it works:**
1. Moon hashes task inputs (files, env, command)
2. Queries bazel-remote for hash
3. If hit: Restores cached output to `target/`
4. If miss: Runs task, uploads output to bazel-remote

**What's cached:**
- Entire task outputs (directories, files)
- Task hashes for fast invalidation
- Content-addressable storage (CAS) and action cache (AC)
- Compressed with zstd for storage efficiency

**Cache keys:**
- CAS: SHA256 of file contents
- AC: SHA256 of action (command + inputs)

---

### Tier 3: Cargo Registry Cache
**What it caches:** Downloaded crates
**Location:** `~/.cargo/registry`
**Size:** Automatic (manages itself)
**Hit rate:** 100% for repeated builds

**How it works:**
1. `cargo build` requests dependency
2. Checks local `~/.cargo/registry`
3. Downloads if missing
4. Never re-downloads same version

---

## Cache Hierarchy

```
Cargo build request
    ↓
[1] sccache check
    ├─ Hit: Return cached .rlib INSTANT
    └─ Miss:
        ↓
        [2] Moon task check (bazel-remote)
            ├─ Hit: Restore entire task output
            └─ Miss:
                ↓
                [3] Compile with rustc
                    ↓
                    Cache to both sccache AND bazel-remote
```

**Why both tiers?**
- **sccache:** Granular (per-file) - good for incremental changes
- **Moon:** Coarse (per-task) - good for full rebuilds, CI/CD
- **Complementary:** sccache handles code changes, Moon handles build artifacts

---

## Storage Architecture

### bazel-remote Disk Layout
```
~/.cache/bazel-remote/
├── ac.v2/           # Action cache (metadata about actions)
├── cas.v2/          # Content-addressable storage (actual files)
└── raw.v2/          # Uncompressed blobs (when storage_mode=uncompressed)
```

**Directory structure:**
```
cas.v2/
└── 62/              # First 2 hex digits
    └── 626f62.../  # SHA256 hash (full 64 chars)
        └── blob       # Actual content (compressed with zstd)
```

**Eviction policy:**
- LRU (Least Recently Used)
- Automatic when hitting `max_size` (500GB)
- No manual cleanup needed

**Storage modes:**
- `zstd` (default): Compressed storage, ~3-5x smaller
- `uncompressed`: Faster reads/writes, more disk used

---

## Performance Metrics

### Current Cache State
```bash
# sccache
Compile requests: 11,839
Cache hits: 7,683 (79.12%)
Cache size: 20GB

# bazel-remote
CurrSize: 177MB (cold - needs warmup)
MaxSize: 100GB (config)
NumFiles: 6,331
```

### Expected Performance After Warmup

**With 40 agents, 10 concurrent:**

| Scenario | Time | RAM Usage |
|----------|-------|-----------|
| Cold build (no cache) | 30-60 min | 60GB peak |
| Warm build (sccache 79%) | 5-10 min | 12GB avg |
| Hot build (Moon 95%+ hit) | 30-60 sec | 4GB avg |

**Cache hit progression:**
```
First agent run: 0% Moon, 20% sccache
After 5 agents: 60% Moon, 70% sccache
After 20 agents: 95% Moon, 85% sccache
Steady state: 98%+ Moon, 90%+ sccache
```

---

## Optimization Strategies

### 1. Warm Up Cache Before Agent Spikes

```bash
#!/bin/bash
# Run once to populate both caches
cd /home/lewis/src/oya

# Warm Moon cache
moon run :ci --force

# Warm sccache (full build)
cargo build --release --workspace --all-features
cargo test --workspace --all-features
```

### 2. Parallelism Tuning

**Formula for optimal concurrency:**
```
MAX_AGENTS = floor(AVAILABLE_RAM / (jobs × rustc_ram × maxTasks))

With 126GB RAM, jobs=3, rustc_ram=2GB, maxTasks=1:
MAX_AGENTS = floor(126 / (3 × 2 × 1)) = floor(126 / 6) = 21
```

**Recommended settings:**
- Single agent: `jobs=8-12`, `maxTasks=2-4`
- 2-3 agents: `jobs=4-6`, `maxTasks=2`
- 10+ agents: `jobs=2-4`, `maxTasks=1`
- 40+ agents: `jobs=1-3`, `maxTasks=1`

### 3. Increase Cache Hit Rates

**sccache optimization:**
- Use `--target-dir` to separate debug/release
- Avoid `cargo clean` (clears sccache's incremental tracking)
- Build with `--all-features` to cache all combinations

**Moon optimization:**
- Always use same command flags (reproducible hashes)
- Set `inputs:` in tasks.yml correctly
- Use `cache: true` for deterministic tasks

### 4. Monitor Cache Health

```bash
# sccache stats
sccache --show-stats | grep "Cache hits rate"

# bazel-remote stats
curl -s http://localhost:9090/status | jq '{size_gb: (.CurrSize/1024/1024/1024|floor), files: .NumFiles}'

# Cache hit breakdown
watch -n 5 '
  echo "=== sccache ==="
  sccache --show-stats | grep -E "hits|Cache hits rate"
  echo ""
  echo "=== bazel-remote ==="
  curl -s http://localhost:9090/status | jq ".CurrSize | . / 1024/1024/1024 | \"GB: \(.)\""
'
```

---

## Troubleshooting

### Low Moon Cache Hit Rate

**Symptoms:** bazel-remote stays at 100-500MB
**Cause:** Moon hashes include environment variables or non-deterministic inputs

**Fix:**
1. Check `.moon/tasks.yml` `inputs:` paths
2. Avoid `date: true` or random values in tasks
3. Use `~/.config/moon/config.toml` for consistent settings

### sccache Showing 0% Hits

**Symptoms:** `Cache hits rate: 0.00%`
**Cause:** Wrapper not configured or incremental builds disabled

**Fix:**
```bash
# Verify sccache is being used
cat ~/.cargo/config.toml  # Should have rustc-wrapper

# Check sccache is running
ps aux | grep sccache

# Clear and retry
sccache --stop-server
sccache --start-server
```

### bazel-remote Not Responding

**Symptoms:** Moon errors: "failed to connect to remote cache"

**Fix:**
```bash
# Check status
systemctl --user status bazel-remote

# Restart if needed
systemctl --user restart bazel-remote

# Verify listening
lsof -i :9092  # Should show bazel-remote
```

### Cache Size Growing Too Large

**Symptoms:** Disk filling up
**Cause:** `max_size` not enforced

**Fix:**
```bash
# Current size
du -sh ~/.cache/bazel-remote

# Restart bazel-remote (triggers cleanup)
systemctl --user restart bazel-remote

# Lower max_size in systemd service
# Edit ~/.config/systemd/user/bazel-remote.service
# Change --max_size 500 to lower value
```

---

## Advanced Configuration

### bazel-remote with Metrics

```toml
# ~/.config/systemd/user/bazel-remote.service
ExecStart=%h/.local/bin/bazel-remote \
    --dir %h/.cache/bazel-remote \
    --max_size 500 \
    --storage_mode zstd \
    --grpc_address 127.0.0.1:9092 \
    --http_address 127.0.0.1:9090 \
    --enable_endpoint_metrics
```

**Access metrics:**
```bash
curl http://localhost:9090/metrics
```

### Moon Cache Modes

```bash
# Read-only cache (for CI)
MOON_CACHE=read-only moon run :ci

# Disable cache (force rebuild)
MOON_CACHE=off moon run :ci

# Default (read-write)
MOON_CACHE=read-write moon run :ci
```

### sccache with S3/GCS Backend

```toml
# ~/.config/sccache/config
[cache.s3]
bucket = "my-sccache-bucket"
endpoint = "s3.amazonaws.com"
region = "us-east-1"
key = "AWS_ACCESS_KEY_ID"
secret = "AWS_SECRET_ACCESS_KEY"
```

---

## Summary

| Component | What | Where | Size | Hit Rate |
|-----------|-------|--------|-------|----------|
| sccache | Rust compilation | ~/.cache/sccache | 200GB | 79% |
| bazel-remote | Moon task outputs | ~/.cache/bazel-remote | 500GB | Cold → Hot |
| Cargo registry | Dependencies | ~/.cargo/registry | Auto | 100% |

**Combined effect:**
- Cold builds: 30-60 min
- Warm builds (79% sccache): 5-10 min
- Hot builds (95% Moon): 30-60 sec

**For 40 agents:**
- 10 concurrent active (agent-pool)
- 30 queued waiting
- 6-9GB RAM usage (after warmup)
- 95%+ cache hits after 20 agents complete
