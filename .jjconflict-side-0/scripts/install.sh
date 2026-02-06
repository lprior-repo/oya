#!/usr/bin/env bash
# Install intent CLI globally using gleescript

set -euo pipefail

INSTALL_DIR="${HOME}/.local/bin"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "Installing intent CLI..."

# Build the escript using gleescript
cd "${PROJECT_ROOT}"
echo "Building escript with gleescript..."
gleam run -m gleescript -- --out=./build > /dev/null 2>&1 || {
    echo "Build failed!"
    exit 1
}

# Install the escript
mkdir -p "${INSTALL_DIR}"
cp "${PROJECT_ROOT}/build/intent" "${INSTALL_DIR}/intent"
chmod +x "${INSTALL_DIR}/intent"

echo "✓ Installed intent to ${INSTALL_DIR}/intent"
echo ""
echo "You can now run 'intent' from anywhere."
echo ""
