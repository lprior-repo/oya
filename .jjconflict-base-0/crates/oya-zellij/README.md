# OYA Zellij Plugin

Real-time pipeline orchestration dashboard for Zellij.

## Features

- **Three View Modes**:
  - **Bead List**: Overview of all beads with status, stage, and progress bars
  - **Bead Detail**: Complete info on selected bead + quick actions (zjj spawn, oya stage)
  - **Pipeline View**: Visual pipeline flow with stage status and timing

- **Rich UI**:
  - Color-coded status (pending/in-progress/completed/failed)
  - Unicode symbols for pipeline stages (○ pending, ◐ running, ● passed, ✗ failed)
  - Progress bars with percentage
  - Summary stats (total/completed/in-progress/failed)

- **Smart Navigation**:
  - Vim keybindings (j/k, g/G for top/bottom)
  - Number keys (1/2/3) to switch views
  - Enter to cycle through views
  - Auto-refresh every 2 seconds

- **Live Updates**: Real-time status from oya-web API (mock data for now, ready for integration)

## Building

```bash
# Build the plugin
cargo build --release --target wasm32-wasip1 -p oya-zellij

# Or use moon
moon run build-zellij

# Output: target/wasm32-wasip1/release/oya_zellij.wasm
```

## Installation

```bash
# Easy way: use moon
moon run install-zellij

# Manual way:
mkdir -p ~/.config/zellij/plugins
cp target/wasm32-wasip1/release/oya_zellij.wasm ~/.config/zellij/plugins/
cp plugin.yaml ~/.config/zellij/plugins/oya.yaml
```

## Usage

### Load in Zellij Layout

Add to your Zellij layout file (e.g., `~/.config/zellij/layouts/oya.kdl`):

```kdl
layout {
    pane size=1 borderless=true {
        plugin location="file:~/.config/zellij/plugins/oya-zellij.wasm"
    }
    pane
}
```

### Launch with Layout

```bash
zellij --layout oya
```

### Load in Running Session

```
Ctrl-o + w  # Open plugin manager
# Then select oya plugin
```

## Keybindings

**View Switching:**
- `1` - Bead list view
- `2` - Bead detail view
- `3` - Pipeline view
- `Enter` - Cycle through views (List → Detail → Pipeline → List)

**Navigation:**
- `j` / `↓` - Move down
- `k` / `↑` - Move up
- `g` - Jump to top
- `G` - Jump to bottom

**Actions:**
- `r` - Manual refresh (auto-refreshes every 2s anyway)
- `q` / `Esc` - Quit plugin

## Configuration

Edit `~/.config/zellij/plugins/oya.yaml`:

```yaml
configuration:
  server_url:
    type: string
    default: "http://localhost:3000"
```

## Architecture

```
┌─────────────────┐
│  Zellij Plugin  │
│   (oya-zellij)  │
└────────┬────────┘
         │ HTTP
         ▼
┌─────────────────┐
│   oya-web API   │
│  (localhost:3k) │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Pipeline Engine │
│  (orchestrator) │
└─────────────────┘
```

## Development

### Prerequisites

```bash
# Install WASM target
rustup target add wasm32-wasip1
```

### Hot Reload

```bash
# Watch for changes and rebuild
cargo watch -x 'build --release --target wasm32-wasip1 -p oya-zellij'

# In another terminal, reload Zellij
# Ctrl-o + d (detach) and reattach to see changes
```

### Testing

Currently no automated tests for Zellij plugins (limitation of WASM environment).
Test manually by loading in Zellij.

## Roadmap

- [x] Three view modes (List, Detail, Pipeline)
- [x] Vim-style navigation
- [x] Color-coded status and progress bars
- [x] Pipeline stage visualization with Unicode symbols
- [x] Auto-refresh timer
- [ ] **Live API integration with oya-web** (replace mock data)
- [ ] Bead dependency graph view (DAG visualization)
- [ ] Interactive stage triggering (press 's' to run stage)
- [ ] Logs streaming view for selected bead
- [ ] Filter beads by status/stage
- [ ] Sort beads (by status, progress, ID)
- [ ] Configurable refresh interval in plugin.yaml
- [ ] Color themes
