#!/usr/bin/env bash
# scan-secrets.sh - Scan for accidentally committed secrets
#
# Usage:
#   ./scripts/scan-secrets.sh           # Scan all tracked files
#   ./scripts/scan-secrets.sh --staged  # Scan only staged files (pre-commit mode)
#
# Exit code: 0 = clean, 1 = secrets found
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

# Patterns that indicate secrets (case-insensitive)
PATTERNS=(
    'password\s*=\s*["\x27][^"\x27]{4,}'
    'api[_-]?key\s*=\s*["\x27][^"\x27]{8,}'
    'secret[_-]?key\s*=\s*["\x27][^"\x27]{8,}'
    'auth[_-]?token\s*=\s*["\x27][^"\x27]{8,}'
    'access[_-]?token\s*=\s*["\x27][^"\x27]{8,}'
    'private[_-]?key\s*=\s*["\x27][^"\x27]{20,}'
    'Bearer\s+[A-Za-z0-9\-._~+/]+=*'
    '-----BEGIN\s+(RSA\s+)?PRIVATE\s+KEY-----'
    'sk-[a-zA-Z0-9]{20,}'             # OpenAI-style keys
    'ghp_[a-zA-Z0-9]{36}'             # GitHub PATs
    'gho_[a-zA-Z0-9]{36}'             # GitHub OAuth
    'ghu_[a-zA-Z0-9]{36}'             # GitHub user-to-server
    'ghs_[a-zA-Z0-9]{36}'             # GitHub server-to-server
    'AKIA[0-9A-Z]{16}'                # AWS access key IDs
    'AIza[0-9A-Za-z\-_]{35}'          # Google API keys
    'xox[bposa]-[0-9]{10,}'           # Slack tokens
)

# File extensions to skip
SKIP_EXTS='\.lock$|\.snap$|\.png$|\.jpg$|\.jpeg$|\.gif$|\.ico$|\.woff2?$|\.ttf$|\.eot$|\.map$|\.bin$'
# Directories to skip
SKIP_DIRS='(^|/)target/(debug|release)/|(^|/)node_modules/|(^|/)\.git/|(^|/)mutants\.out'

found=0

# Build the grep pattern
combined_pattern=""
for i in "${!PATTERNS[@]}"; do
    if [ "$i" -gt 0 ]; then
        combined_pattern+="|"
    fi
    combined_pattern+="(${PATTERNS[$i]})"
done

if [ "${1:-}" = "--staged" ]; then
    # Pre-commit mode: only scan staged files
    files=$(git diff --cached --name-only --diff-filter=ACM 2>/dev/null || echo "")
else
    # Full scan: all tracked files
    files=$(git ls-files 2>/dev/null || echo "")
fi

if [ -z "$files" ]; then
    echo -e "${GREEN}No files to scan${NC}"
    exit 0
fi

# Filter files
scan_files=""
while IFS= read -r file; do
    # Skip binary/extensions
    if echo "$file" | grep -qE "$SKIP_EXTS"; then
        continue
    fi
    # Skip directories
    if echo "$file" | grep -qE "$SKIP_DIRS"; then
        continue
    fi
    # Skip this script itself
    if [ "$file" = "scripts/scan-secrets.sh" ]; then
        continue
    fi
    # Only scan existing files
    if [ -f "$file" ]; then
        scan_files="$scan_files $file"
    fi
done <<< "$files"

if [ -z "$scan_files" ]; then
    echo -e "${GREEN}No scannable files found${NC}"
    exit 0
fi

# Scan
for file in $scan_files; do
    matches=$(grep -nEi "$combined_pattern" "$file" 2>/dev/null || true)
    if [ -n "$matches" ]; then
        while IFS= read -r match; do
            echo -e "${RED}SECRET FOUND: ${file}:${match}${NC}"
            found=$((found + 1))
        done <<< "$matches"
    fi
done

if [ "$found" -gt 0 ]; then
    echo ""
    echo -e "${RED}Found ${found} potential secret(s). Review and remove before committing.${NC}"
    echo -e "${YELLOW}If this is a false positive, add an inline comment: # nosecret${NC}"
    exit 1
fi

echo -e "${GREEN}No secrets detected${NC}"
exit 0
