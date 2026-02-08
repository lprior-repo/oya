#!/usr/bin/env bash
set -euo pipefail

# OYA Zellij Plugin Installer
# Automates WASM build and installation to Zellij

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CARGO_TARGET_DIR="${PROJECT_ROOT}/target"
WASM_TARGET_DIR="${CARGO_TARGET_DIR}/wasm32-wasip1/release"
ZELLIJ_PLUGIN_DIR="${HOME}/.local/share/zellij/plugins"
ZELLIJ_LAYOUT_DIR="${HOME}/.config/zellij/layouts"

# Files
WASM_SOURCE="${WASM_TARGET_DIR}/oya_zellij.wasm"
WASM_DEST="${ZELLIJ_PLUGIN_DIR}/oya_zellij.wasm"
LAYOUT_SOURCE="${PROJECT_ROOT}/crates/oya-zellij/assets/layout.kdl"
LAYOUT_DEST="${ZELLIJ_LAYOUT_DIR}/oya.kdl"

# Version info
PLUGIN_NAME="oya-zellij"
VERSION="${CARGO_PKG_VERSION:-dev}"

log_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

log_success() {
    echo -e "${GREEN}✓${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
}

log_error() {
    echo -e "${RED}✗${NC} $1"
}

header() {
    echo -e "${BLUE}"
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║     OYA Zellij Plugin - Automated Release Installer       ║"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

check_dependencies() {
    log_info "Checking dependencies..."

    if ! command -v cargo &> /dev/null; then
        log_error "cargo not found. Please install Rust toolchain."
        exit 1
    fi

    if ! command -v zellij &> /dev/null; then
        log_warn "zellij not found. Plugin will be installed but can't be tested."
    fi

    log_success "Dependencies check passed"
}

build_wasm() {
    log_info "Building ${PLUGIN_NAME} WASM plugin (release mode)..."

    cd "$PROJECT_ROOT"

    if cargo build --release --target wasm32-wasip1 -p oya-zellij; then
        log_success "WASM build completed"
    else
        log_error "WASM build failed"
        exit 1
    fi

    if [ ! -f "$WASM_SOURCE" ]; then
        log_error "WASM file not found at ${WASM_SOURCE}"
        exit 1
    fi

    local size
    size=$(du -h "$WASM_SOURCE" | cut -f1)
    log_success "WASM binary size: ${size}"
}

install_plugin() {
    log_info "Installing WASM plugin to Zellij..."

    mkdir -p "$ZELLIJ_PLUGIN_DIR"

    if cp "$WASM_SOURCE" "$WASM_DEST"; then
        chmod +r "$WASM_DEST"
        log_success "Plugin installed to: ${WASM_DEST}"
    else
        log_error "Failed to copy plugin file"
        exit 1
    fi
}

install_layout() {
    log_info "Installing Zellij layout..."

    if [ ! -f "$LAYOUT_SOURCE" ]; then
        log_warn "Layout file not found at ${LAYOUT_SOURCE}"
        return
    fi

    mkdir -p "$ZELLIJ_LAYOUT_DIR"

    if cp "$LAYOUT_SOURCE" "$LAYOUT_DEST"; then
        log_success "Layout installed to: ${LAYOUT_DEST}"
    else
        log_error "Failed to copy layout file"
        exit 1
    fi
}

verify_installation() {
    log_info "Verifying installation..."

    if [ ! -f "$WASM_DEST" ]; then
        log_error "Plugin verification failed: file not found"
        exit 1
    fi

    if [ ! -f "$LAYOUT_DEST" ]; then
        log_warn "Layout file not found (non-critical)"
    fi

    log_success "Installation verified"
}

print_summary() {
    local size
    size=$(du -h "$WASM_DEST" | cut -f1)

    echo ""
    echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                    Installation Complete!                  ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${BLUE}Plugin Details:${NC}"
    echo "  Name:        ${PLUGIN_NAME}"
    echo "  Version:     ${VERSION}"
    echo "  Size:        ${size}"
    echo "  Location:    ${WASM_DEST}"
    echo ""
    echo -e "${BLUE}Layout File:${NC}"
    echo "  Location:    ${LAYOUT_DEST}"
    echo ""
    echo -e "${BLUE}To launch OYA in Zellij:${NC}"
    echo "  1. Start the API server:"
    echo "     ${YELLOW}cargo run -p oya-web${NC}"
    echo ""
    echo "  2. Launch Zellij with OYA plugin:"
    echo "     ${YELLOW}zellij --layout oya${NC}"
    echo ""
    echo -e "${BLUE}Keyboard Shortcuts:${NC}"
    echo "  1 - Bead List     2 - Bead Detail    3 - Pipeline"
    echo "  4 - Agents        5 - Graph          6 - Health"
    echo "  7 - Logs         j/k - Navigate      r - Refresh"
    echo "  g/G - Top/Bottom q - Quit"
    echo ""
}

main() {
    header
    check_dependencies
    build_wasm
    install_plugin
    install_layout
    verify_installation
    print_summary
}

# Run main function
main "$@"
