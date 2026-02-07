# OYA Zellij Plugin - Automated Release

This directory contains the Zellij plugin for the OYA Pipeline Orchestration system.

## Quick Start

### One-Line Installation

```bash
bash scripts/install-zellij-plugin.sh
```

This will:
1. Build the WASM plugin in release mode
2. Install it to `~/.local/share/zellij/plugins/`
3. Install the Zellij layout to `~/.config/zellij/layouts/`
4. Print instructions for launching

### Using Moon Tasks

```bash
# Build and install in one command
moon run :oya-zellij:install

# Quick rebuild during development
moon run :oya-zellij:update

# Full release workflow (clean + build + install)
moon run :oya-zellij:release

# Check WASM binary size
moon run :oya-zellij:size
```

## Development Workflow

### Initial Setup

```bash
# 1. Install the plugin (one-time)
bash scripts/install-zellij-plugin.sh

# 2. Start the OYA API server
cargo run -p oya-web

# 3. In another terminal, launch Zellij with OYA
zellij --layout oya
```

### Iterating on Changes

```bash
# Make changes to the plugin code...

# Quick update (fast rebuild)
moon run :oya-zellij:update

# Or use the script directly
bash scripts/update-zellij-plugin.sh

# Restart Zellij to see changes
zellij --layout oya
```

### Full Release

```bash
# Clean build + install
moon run :oya-zellij:release
```

## File Locations

After installation:

- **WASM Plugin**: `~/.local/share/zellij/plugins/oya_zellij.wasm`
- **Layout File**: `~/.config/zellij/layouts/oya.kdl`
- **Source Code**: `crates/oya-zellij/src/lib.rs`

## Plugin Features

The OYA Zellij plugin provides 7 view modes:

1. **Bead List** (press `1`) - List all beads with status
2. **Bead Detail** (press `2`) - Detailed view of selected bead
3. **Pipeline View** (press `3`) - Pipeline stages with progress
4. **Agent Dashboard** (press `4`) - Agent status and health
5. **Graph View** (press `5`) - Dependency graph with critical path
6. **System Health** (press `6`) - System metrics (coming soon)
7. **Log Aggregator** (press `7`) - Multi-source logs (coming soon)

## Keyboard Navigation

- `j` / `↓` - Move down
- `k` / `↑` - Move up
- `g` - Jump to top
- `G` - Jump to bottom
- `Ctrl+d` - Page down
- `Ctrl+u` - Page up
- `1-7` - Switch view modes
- `r` - Refresh data
- `Enter` - Execute action (rerun stage in Pipeline view)
- `q` / `Esc` - Quit plugin

## Architecture

```
crates/oya-zellij/
├── src/
│   ├── lib.rs              # Main plugin (1875 lines)
│   ├── command_pane.rs     # Command pane lifecycle tracking
│   └── log_stream.rs       # Log streaming with backpressure
├── assets/
│   └── layout.kdl          # Zellij layout configuration
├── scripts/
│   ├── install-zellij-plugin.sh  # Full installer
│   └── update-zellij-plugin.sh   # Quick updater
└── moon.yml                # Moon task definitions
```

## Build Output

- **Target**: `wasm32-wasip1` (WASI for Zellij)
- **Size**: ~1.2MB (release mode)
- **Optimization**: Release builds use `--release` flag
- **Warnings**: 20 dead code warnings (non-blocking, from unimplemented features)

## Troubleshooting

### Plugin not loading

1. Check the plugin file exists:
   ```bash
   ls -lh ~/.local/share/zellij/plugins/oya_zellij.wasm
   ```

2. Verify Zellij version (requires 0.39.0+):
   ```bash
   zellij --version
   ```

3. Check Zellij logs:
   ```bash
   zellij --layout oya --debug
   ```

### API connection failed

1. Ensure the API server is running:
   ```bash
   cargo run -p oya-web
   ```

2. Check the server URL (default: `http://localhost:3000`)

3. Verify network connectivity:
   ```bash
   curl http://localhost:3000/api/beads
   ```

### Layout not found

1. Check the layout file exists:
   ```bash
   ls -lh ~/.config/zellij/layouts/oya.kdl
   ```

2. Reinstall the plugin:
   ```bash
   bash scripts/install-zellij-plugin.sh
   ```

## Performance

- **Build time**: ~5s (cached) to ~30s (clean build)
- **WASM size**: 1.2MB (optimized release build)
- **Startup time**: <100ms to load and initialize
- **Memory usage**: ~5-10MB baseline

## Contributing

When modifying the plugin:

1. Follow the functional Rust patterns (zero panics, zero unwraps)
2. Test locally with `moon run :oya-zellij:update`
3. Verify WASM builds: `moon run :oya-zellij:build`
4. Update this README if adding new features
5. Commit with clear message: `fix(zellij): description`

## License

MIT License - See project root for details
