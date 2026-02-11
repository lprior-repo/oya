# Moon Remote Caching with bazel-remote

This document explains Moon's integration with bazel-remote and the REAPI (Remote Execution API).

## How Moon Uses bazel-remote

### Architecture

```
┌─────────────┐
│   Moon CLI   │
└──────┬──────┘
       │ gRPC client
       ▼
┌─────────────────┐
│  bazel-remote  │ (localhost:9092)
│  REAPI v2       │
└───────┬────────┘
        │
        ▼
┌─────────────────────┐
│  ~/.cache/bazel- │
│      remote       │ (500GB disk)
└───────────────────┘
```

### Protocol: REAPI (Remote Execution API v2)

Moon uses the **Bazel Remote Execution API** to communicate with bazel-remote. This is a standardized protocol for:

1. **Action Cache (AC)** - Stores metadata about build actions
2. **Content-Addressable Storage (CAS)** - Stores actual build artifacts

### Cache Flow

```
1. HASH
   Moon hashes task inputs:
   - Source files
   - Environment variables
   - Command line
   - Dependencies
   ↓
2. QUERY
   Moon sends hash to bazel-remote:
   gRPC: GetActionResult(hash)
   ↓
3. HIT OR MISS
   ├─ HIT: bazel-remote returns ActionResult
   │         Moon downloads CAS blobs
   │         Restores to target/
   │         Time: 10-100ms (on local SSD)
   │
   └─ MISS: Moon runs task locally
            Uploads ActionResult to bazel-remote
            Uploads output files to CAS
            Stores compressed with zstd
```

## Configuration

### Workspace Config (Required)

**File:** `.moon/workspace.yml`

```yaml
$schema: "https://moonrepo.dev/schemas/workspace.json"

projects:
  globs:
    - "crates/*"

vcs:
  manager: "git"
  defaultBranch: "main"

versionConstraint: ">=1.20.0"

# Remote cache configuration
unstable_remote:
  host: 'grpc://localhost:9092'  # gRPC endpoint
  cache:
    compression: 'zstd'  # Compress cache entries

# Performance tuning
hasher:
  optimization: 'performance'
```

**Key settings:**
- `host`: Must match bazel-remote's `--grpc_address`
- `compression: 'zstd'`: Reduces cache size 3-5x
- `hasher.optimization: 'performance'`: Faster hashing (uses more RAM)

### Moon Config (Optional)

**File:** `~/.config/moon/config.toml`

```toml
[runner]
maxTasks = 1  # Parallel tasks per agent

[telemetry]
enabled = false  # Disable for privacy
```

### bazel-remote Service

**File:** `~/.config/systemd/user/bazel-remote.service`

```ini
[Unit]
Description=bazel-remote cache (hyper-fast local caching)
After=network.target

[Service]
Type=simple
LimitNOFILE=65536
Restart=on-failure
RestartSec=5s
Environment=GOMAXPROCS=0

ExecStart=%h/.local/bin/bazel-remote \
    --dir %h/.cache/bazel-remote \
    --max_size 500 \
    --storage_mode zstd \
    --grpc_address 127.0.0.1:9092 \
    --http_address 127.0.0.1:9090 \
    --enable_endpoint_metrics

[Install]
WantedBy=default.target
```

**Flags explained:**
- `--dir`: Cache storage directory
- `--max_size`: Maximum cache size in GiB
- `--storage_mode`: `zstd` (compressed) or `uncompressed`
- `--grpc_address`: gRPC API endpoint (Moon uses this)
- `--http_address`: HTTP API endpoint (for monitoring/status)
- `--enable_endpoint_metrics`: Enable Prometheus metrics

## Cache Keys and Hashing

### What Moon Hashes

Moon creates a hash for each task based on:

1. **Input files**
   ```yaml
   # .moon/tasks.yml
   inputs:
     - "crates/**/*.rs"  # Source code
     - "Cargo.toml"       # Config files
     - "/Cargo.lock"        # Absolute path (workspace root)
   ```

2. **Environment variables**
   - Variables explicitly listed in `env:` section
   - `RUSTFLAGS`, `CARGO_TARGET_*` (automatically included)

3. **Command**
   - Executable path
   - Arguments
   - Working directory

4. **Dependencies**
   - Hashes of dependent tasks
   - Ensures invalidation propagates

### Hash Algorithm

- **Algorithm:** SHA-256
- **Mode:** Deterministic (same inputs = same hash)
- **Optimization:** `performance` (uses xxhash for internal operations)

### Cache Entry Structure

```
Action Cache Entry:
{
  "action_digest": {
    "hash": "a1b2c3d4e5f6...",
    "size_bytes": 1024
  },
  "output_files": [
    {
      "path": "target/release/binary",
      "digest": {
        "hash": "f6e5d4c3b2a1...",
        "size_bytes": 2048576
      }
    }
  ]
}

CAS Entry (Content-Addressable Storage):
~/.cache/bazel-remote/cas.v2/62/626f62e79a4b823d.../blob
                                          ↑
                                          First 2 hex digits (directory)
```

## Performance Characteristics

### Local Cache (bazel-remote on NVMe SSD)

| Operation | Time | Throughput |
|-----------|-------|------------|
| Cache query (GetActionResult) | 1-5ms | 200-1000 queries/sec |
| Download small blob (<1MB) | 10-30ms | 33-100 MB/sec |
| Download large blob (10MB) | 100-300ms | 33-100 MB/sec |
| Upload small blob | 10-50ms | 20-100 MB/sec |
| Upload large blob (10MB) | 150-400ms | 25-66 MB/sec |

**Why so fast?**
- Zero network latency (localhost)
- NVMe SSD: 14GB/s sequential reads
- zstd compression: 3-5x smaller transfers
- Direct file I/O (no filesystem overhead)

### Comparison: Moon vs. Direct Cargo

| Scenario | Cargo (no cache) | Cargo (sccache 79%) | Moon (cold) | Moon (hot 95%) |
|----------|------------------|----------------------|-------------|----------------|
| Fresh checkout | 30-60 min | 5-10 min | 2-5 min | 30-60 sec |
| Single file change | 2-5 min | 30-60 sec | 10-30 sec | 5-10 sec |
| Full rebuild | 30-60 min | 5-10 min | 2-5 min | 30-60 sec |

**Key differences:**
- **sccache:** Granular (per-file) - good for incremental
- **Moon:** Coarse (per-task) - good for full rebuilds
- **Together:** Best of both worlds

## Monitoring and Debugging

### Check Cache Status

```bash
# bazel-remote status
curl -s http://localhost:9090/status | jq

# Example output:
{
  "CurrSize": 203661312,      # 194MB
  "MaxSize": 107374182400,    # 100GB (config)
  "NumFiles": 6331,
  "ServerTime": 1770116947
}
```

### Moon Cache Logs

```bash
# Run with verbose logging
MOON_LOG=debug moon run :ci

# Look for these lines:
[INFO] Checking remote cache for hash a1b2c3d4...
[INFO] Cache HIT, restoring outputs
[INFO] Cache MISS, running task locally
[INFO] Uploading outputs to remote cache
```

### Metrics Endpoint

If `--enable_endpoint_metrics` is enabled:

```bash
# Prometheus metrics
curl http://localhost:9090/metrics

# Example metrics:
bazel_remote_disk_cache_size_bytes 203661312
bazel_remote_request_duration_seconds{endpoint="cas",method="get"} 0.005
bazel_remote_request_duration_seconds{endpoint="ac",method="get"} 0.002
```

### Troubleshooting

#### Moon not connecting to bazel-remote

```bash
# Check bazel-remote is running
systemctl --user status bazel-remote

# Check it's listening on the right port
lsof -i :9092  # Should show bazel-remote

# Verify Moon config
cat .moon/workspace.yml | grep -A3 unstable_remote

# Test connection manually
grpcurl -plaintext localhost:9092 build.bazel.remote.execution.v2.Capabilities/GetCapabilities
```

#### Low cache hit rate

**Symptoms:** Always showing "cache miss"

**Causes:**
1. Non-deterministic hashes
   - Random values in environment
   - Using `date: true` in tasks
   - Changing command flags

2. Input paths not configured
   ```yaml
   # Wrong (relative paths change hash)
   inputs:
     - "*.rs"
   
   # Correct (absolute paths)
   inputs:
     - "crates/**/*.rs"
   ```

3. Cache clearing between runs
   - Running `moon clean`
   - Changing `unstable_remote.host`
   - Deleting `~/.cache/bazel-remote`

#### Cache not persisting

**Symptoms:** Same task rebuilds every time

**Fix:**
```bash
# Check cache mode
MOON_CACHE=moon run :ci  # Should be read-write (default)

# Verify bazel-remote permissions
ls -la ~/.cache/bazel-remote  # Should be owned by you

# Check disk space
df -h ~/.cache/bazel-remote  # Should have space available
```

## Advanced Topics

### Compression Tuning

bazel-remote supports two compression modes:

**zstd (default):**
- Pros: 3-5x smaller cache, less disk I/O
- Cons: 10-20% CPU overhead during compress/decompress
- Best for: Limited disk space, high concurrency

**uncompressed:**
- Pros: Faster reads/writes, zero CPU overhead
- Cons: 3-5x larger cache
- Best for: Unlimited disk space, low CPU

**Switching:**
```bash
# Edit ~/.config/systemd/user/bazel-remote.service
# Change: --storage_mode zstd → --storage_mode uncompressed

systemctl --user daemon-reload
systemctl --user restart bazel-remote

# Warning: This invalidates existing cache
# Run moon run :ci --force to rebuild
```

### Distributed Caching

bazel-remote can proxy to cloud storage:

**S3 proxy:**
```bash
ExecStart=%h/.local/bin/bazel-remote \
    --dir %h/.cache/bazel-remote \
    --max_size 500 \
    --storage_mode zstd \
    --grpc_address 127.0.0.1:9092 \
    --http_address 127.0.0.1:9090 \
    --s3_proxy.endpoint s3.amazonaws.com \
    --s3_proxy.bucket my-bazel-cache \
    --s3_proxy.auth_method access_key \
    --s3_proxy.access_key_id KEY \
    --s3_proxy.secret_access_key SECRET
```

**GCS proxy:**
```bash
--gcs_proxy.bucket my-cache-bucket \
--gcs_proxy.json_credentials_file /path/to/creds.json
```

**Benefits:**
- Share cache across machines
- CI/CD uses same cache as dev
- Faster cold builds for new machines

### Authentication

bazel-remote supports multiple auth methods:

**htpasswd (basic auth):**
```bash
# Create password file
htpasswd -c /path/to/.htpasswd user1

# Configure bazel-remote
--htpasswd_file /path/to/.htpasswd

# Update Moon config
host: 'http://user1:password@localhost:9090'
```

**mTLS (mutual TLS):**
```bash
--tls_cert_file /path/to/server.cert \
--tls_key_file /path/to/server.key \
--tls_ca_file /path/to/ca.cert
```

## Summary

Moon + bazel-remote = Hyper-fast builds

| Component | Role | Protocol | Location |
|-----------|-------|-----------|----------|
| Moon CLI | Task orchestration | - | Local |
| bazel-remote | Remote cache server | REAPI v2 (gRPC) | Local |
| NVMe SSD | Storage | - | ~/.cache/bazel-remote |

**Performance:**
- Cache queries: 1-5ms
- Small blob transfers: 10-30ms
- Full task restore: 50-200ms
- Hit rate progression: 0% → 60% → 95%+

**For 40 agents:**
- 10 concurrent (via agent-pool)
- 30 queued
- 6-9GB RAM usage
- 95%+ hits after warmup
