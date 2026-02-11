#!/usr/bin/env bash
#
# Hostile verification that EVERYTHING is named OYA
# No tolerance for "factory" references
#

set -euo pipefail

echo "HOSTILE VERIFICATION MODE: OYA NAMING CONVENTION"
echo "================================================="
echo ""

ERRORS_FOUND=0

# Function to report error
report_error() {
	local file="$1"
	local line="$2"
	local match="$3"
	echo "❌ FOUND: $file:$line"
	echo "   >> $match"
	echo ""
	((ERRORS_FOUND++))
}

# Search for 'factory' (case-insensitive)
echo "🔍 Searching for 'factory' references (case-insensitive)..."
echo ""

while IFS= read -r line; do
	if [[ -n "$line" && ! "$line" =~ (target/|vendor/|node_modules/) ]]; then
		file="${line%%:*}"
		lineno="${line#*:}"
		lineno="${lineno%%:*}"
		match="${line#*:*:* }"
		report_error "$file" "$lineno" "$match"
	fi
done < <(grep -rn --include="*.rs" --include="*.md" --include="*.toml" --include="*.yaml" --include="*.yml" -i "factory" . 2>/dev/null || true)

# Search for 'Factory' (capitalized)
echo ""
echo "🔍 Searching for 'Factory' references..."
echo ""

while IFS= read -r line; do
	if [[ -n "$line" && ! "$line" =~ (target/|vendor/|node_modules/) ]]; then
		file="${line%%:*}"
		lineno="${line#*:}"
		lineno="${lineno%%:*}"
		match="${line#*:*:* }"
		report_error "$file" "$lineno" "$match"
	fi
done < <(grep -rn --include="*.rs" --include="*.md" --include="*.toml" --include="*.yaml" --include="*.yml" "Factory" . 2>/dev/null || true)

# Search for 'FACTORY' (all caps)
echo ""
echo "🔍 Searching for 'FACTORY' references..."
echo ""

while IFS= read -r line; do
	if [[ -n "$line" && ! "$line" =~ (target/|vendor/|node_modules/) ]]; then
		file="${line%%:*}"
		lineno="${line#*:}"
		lineno="${lineno%%:*}"
		match="${line#*:*:* }"
		report_error "$file" "$lineno" "$match"
	fi
done < <(grep -rn --include="*.rs" --include="*.md" --include="*.toml" --include="*.yaml" --include="*.yml" "FACTORY" . 2>/dev/null || true)

# Check for .factory directory
echo ""
echo "🔍 Checking for .factory directory..."
if [[ -d ".factory" ]]; then
	echo "❌ FOUND: .factory directory exists"
	echo "   >> Rename to .oya immediately"
	echo ""
	((ERRORS_FOUND++))
fi

# Check Cargo.toml descriptions
echo ""
echo "🔍 Checking Cargo.toml descriptions..."
if grep -q "factory" Cargo.toml crates/*/Cargo.toml 2>/dev/null; then
	echo "❌ FOUND: 'factory' in Cargo.toml files"
	grep -n "factory" Cargo.toml crates/*/Cargo.toml 2>/dev/null | grep -v node_modules | while read -r line; do
		echo "   >> $line"
	done
	echo ""
	((ERRORS_FOUND++))
fi

# Summary
echo ""
echo "================================================"
echo "HOSTILE VERIFICATION COMPLETE"
echo "================================================"
echo ""

if ((ERRORS_FOUND == 0)); then
	echo "✅ CLEAN: No 'factory' references found"
	echo "   All code properly named OYA"
	exit 0
else
	echo "❌ CONTAMINATED: $ERRORS_FOUND issues found"
	echo "   Fix them immediately"
	exit 1
fi
