# OYA Zellij Plugin

Real-time pipeline orchestration dashboard for Zellij.

## Features

- **Bead List View**: See all active beads, their status, and current pipeline stage
- **Detail View**: Deep dive into individual bead execution
- **Pipeline View**: Visualize stage progression
- **Live Updates**: Real-time status from oya-web API
- **Vim Keybindings**: Navigate with j/k, switch views with 1/2/3

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

- `1` - Bead list view
- `2` - Bead detail view
- `3` - Pipeline view
- `j/↓` - Move down
- `k/↑` - Move up
- `r` - Refresh from API
- `q` - Quit plugin

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

- [ ] Live API integration with oya-web
- [ ] Pipeline stage visualization (ASCII art)
- [ ] Bead dependency graph view
- [ ] Interactive stage triggering
- [ ] Logs streaming view
- [ ] Color themes
- [ ] Configurable refresh interval
