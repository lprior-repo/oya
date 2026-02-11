#!/usr/bin/env bash
# Functional Rust test script - Zero unwraps, pure shell logic
set -e

# Functional core - pure data extraction
extract_package_name() {
	local cargo_toml="$1"
	grep '^name = ' "$cargo_toml" | sed 's/name = "\([^"]*\)".*/\1/'
}

# Pure validation functions
validate_crate_exists() {
	local path="$1"
	[ -d "$path" ]
}

validate_cargo_toml() {
	local path="$1"
	[ -f "$path/Cargo.toml" ]
}

# ==============================================================================
# RED TESTS (these should FAIL initially)
# ==============================================================================

echo "=== RED PHASE: Tests should fail ==="

# Test 1: Verify current wrong naming
echo "Test 1: Verify backwards naming exists"
if validate_crate_exists "crates/oya-ui" && validate_crate_exists "crates/oya-zellij"; then
	echo "✗ FAIL: Old crate names still exist (expected after refactor)"
	exit 1
else
	echo "✓ PASS: Old crates renamed"
fi

# Test 2: Verify new crate names exist
echo "Test 2: Verify new crate names exist"
if ! validate_crate_exists "crates/oya-zellij-plugin"; then
	echo "✗ FAIL: oya-zellij-plugin does not exist"
	exit 1
fi

if ! validate_crate_exists "crates/oya-ui-components"; then
	echo "✗ FAIL: oya-ui-components does not exist"
	exit 1
fi

# Test 3: Verify package names match directory names
echo "Test 3: Verify package names match directories"
plugin_name=$(extract_package_name "crates/oya-zellij-plugin/Cargo.toml")
if [ "$plugin_name" != "oya-zellij-plugin" ]; then
	echo "✗ FAIL: Plugin package name is '$plugin_name', expected 'oya-zellij-plugin'"
	exit 1
fi

ui_name=$(extract_package_name "crates/oya-ui-components/Cargo.toml")
if [ "$ui_name" != "oya-ui-components" ]; then
	echo "✗ FAIL: UI components package name is '$ui_name', expected 'oya-ui-components'"
	exit 1
fi

# Test 4: Verify workspace includes new crates
echo "Test 4: Verify workspace Cargo.toml includes new crates"
if ! grep -q 'oya-zellij-plugin' Cargo.toml; then
	echo "✗ FAIL: workspace Cargo.toml missing oya-zellij-plugin"
	exit 1
fi

if ! grep -q 'oya-ui-components' Cargo.toml; then
	echo "✗ FAIL: workspace Cargo.toml missing oya-ui-components"
	exit 1
fi

# Test 5: Verify old crates removed from workspace
echo "Test 5: Verify old crates removed from workspace"
if grep -q '"oya-ui"' Cargo.toml || grep -q '"oya-zellij"' Cargo.toml; then
	echo "✗ FAIL: Old crate names still in workspace"
	exit 1
fi

# Test 6: Verify oya.kll references correct crate
echo "Test 6: Verify oya.kll manifest"
if ! grep -q 'oya_zellij_plugin' crates/oya-zellij-plugin/oya.kll; then
	echo "✗ FAIL: oya.kll does not reference oya_zellij_plugin.wasm"
	exit 1
fi

# Test 7: Verify WASM configuration preserved
echo "Test 7: Verify WASM configuration preserved"
if ! grep -q 'cdylib' crates/oya-zellij-plugin/Cargo.toml; then
	echo "✗ FAIL: WASM cdylib target not preserved"
	exit 1
fi

# Test 8: Verify optimization flags preserved
echo "Test 8: Verify optimization flags preserved"
if ! grep -q 'opt-level = "z"' crates/oya-zellij-plugin/Cargo.toml; then
	echo "✗ FAIL: Optimization flags not preserved"
	exit 1
fi

echo ""
echo "=== ALL TESTS PASSED ==="
echo "✓ All postconditions met"
echo "✓ All invariants preserved"
