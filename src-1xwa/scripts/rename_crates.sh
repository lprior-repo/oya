#!/usr/bin/env bash
# Functional Rust rename implementation - Zero unwraps, pure + imperative shell
set -e

# ==============================================================================
# ERROR TYPES (thiserror-style enum in shell)
# ==============================================================================

declare -A ERROR_MESSAGES=(
	[DIR_EXISTS]="Target directory already exists"
	[GIT_CONFLICT]="Git operation failed"
	[CARGO_VALIDATE]="Cargo.toml validation failed"
	[WORKSPACE_UPDATE]="Workspace update failed"
	[BUILD_FAILED]="Build validation failed"
)

fail() {
	local error_type="$1"
	local message="${2:-${ERROR_MESSAGES[$error_type]}}"
	echo "✗ FAIL: $message" >&2
	exit 1
}

# ==============================================================================
# VALIDATION (preconditions)
# ==============================================================================

validate_preconditions() {
	# Check source directories exist
	if [ ! -d "crates/oya-ui" ]; then
		fail "CARGO_VALIDATE" "Source directory crates/oya-ui does not exist"
	fi

	if [ ! -d "crates/oya-zellij" ]; then
		fail "CARGO_VALIDATE" "Source directory crates/oya-zellij does not exist"
	fi

	# Check target directories don't exist
	if [ -d "crates/oya-zellij-plugin" ]; then
		fail "DIR_EXISTS" "Target directory crates/oya-zellij-plugin already exists"
	fi

	if [ -d "crates/oya-ui-components" ]; then
		fail "DIR_EXISTS" "Target directory crates/oya-ui-components already exists"
	fi

	# Check Cargo.toml files exist
	if [ ! -f "crates/oya-ui/Cargo.toml" ]; then
		fail "CARGO_VALIDATE" "crates/oya-ui/Cargo.toml not found"
	fi

	if [ ! -f "crates/oya-zellij/Cargo.toml" ]; then
		fail "CARGO_VALIDATE" "crates/oya-zellij/Cargo.toml not found"
	fi

	echo "✓ Preconditions validated"
}

# ==============================================================================
# CORE FUNCTION: Update package name in Cargo.toml
# ==============================================================================

update_package_name() {
	local cargo_file="$1"
	local old_name="$2"
	local new_name="$3"

	# Functional approach: read-transform-write
	local content
	content=$(cat "$cargo_file")

	# Replace name field
	local new_content
	new_content=$(echo "$content" | sed "s/^name = \"$old_name\"/name = \"$new_name\"/")

	if [ "$new_content" = "$content" ]; then
		fail "CARGO_VALIDATE" "Failed to update name in $cargo_file"
	fi

	# Write back atomically
	echo "$new_content" >"$cargo_file.tmp" && mv "$cargo_file.tmp" "$cargo_file"
	echo "  ✓ Updated package name: $old_name → $new_name"
}

# ==============================================================================
# CORE FUNCTION: Update oya.kll manifest
# ==============================================================================

update_oya_kll() {
	local kll_file="$1"
	local old_name="$2"
	local new_name="$3"

	if [ -f "$kll_file" ]; then
		local content
		content=$(cat "$kll_file")

		local new_content
		new_content=$(echo "$content" | sed "s/$old_name/$new_name/g")

		if [ "$new_content" != "$content" ]; then
			echo "$new_content" >"$kll_file.tmp" && mv "$kll_file.tmp" "$kll_file"
			echo "  ✓ Updated oya.kll: $old_name → $new_name"
		fi
	fi
}

# ==============================================================================
# CORE FUNCTION: Update workspace Cargo.toml
# ==============================================================================

update_workspace() {
	local workspace_file="Cargo.toml"

	# Read workspace content
	local content
	content=$(cat "$workspace_file")

	# Update members section (add new crates)
	local new_content
	new_content=$(echo "$content" |
		sed 's|crates/zellij-frontend,|crates/zellij-frontend,\n    "crates/oya-zellij-plugin",\n    "crates/oya-ui-components",|')

	# Update dependencies section (add new crate references)
	new_content=$(echo "$new_content" |
		sed '/^oya-web = { path = "crates\/oya-web" }$/a\
oya-zellij-plugin = { path = "crates/oya-zellij-plugin" }\
oya-ui-components = { path = "crates/oya-ui-components" }')

	# Remove old references (if they exist)
	new_content=$(echo "$new_content" | grep -v '^oya-ui = ' || true)
	new_content=$(echo "$new_content" | grep -v '^oya-zellij = ' || true)

	# Write back
	echo "$new_content" >"$workspace_file.tmp" && mv "$workspace_file.tmp" "$workspace_file"
	echo "  ✓ Updated workspace Cargo.toml"
}

# ==============================================================================
# CORE FUNCTION: Rename directory with git mv
# ==============================================================================

rename_crate() {
	local old_path="$1"
	local new_path="$2"

	echo "Renaming: $old_path → $new_path"

	# Check if in git repo
	if git rev-parse --git-dir >/dev/null 2>&1; then
		# Use git mv to preserve history
		git mv "$old_path" "$new_path" 2>/dev/null || fail "GIT_CONFLICT" "git mv failed"
		echo "  ✓ Git mv completed"
	else
		# Fallback to mv
		mv "$old_path" "$new_path"
		echo "  ✓ mv completed"
	fi
}

# ==============================================================================
# VALIDATION (postconditions)
# ==============================================================================

validate_postconditions() {
	echo "Validating postconditions..."

	# Check old directories removed
	if [ -d "crates/oya-ui" ]; then
		fail "BUILD_FAILED" "Old directory crates/oya-ui still exists"
	fi

	if [ -d "crates/oya-zellij" ]; then
		fail "BUILD_FAILED" "Old directory crates/oya-zellij still exists"
	fi

	# Check new directories exist
	if [ ! -d "crates/oya-zellij-plugin" ]; then
		fail "BUILD_FAILED" "New directory crates/oya-zellij-plugin missing"
	fi

	if [ ! -d "crates/oya-ui-components" ]; then
		fail "BUILD_FAILED" "New directory crates/oya-ui-components missing"
	fi

	# Check Cargo.toml package names match
	local plugin_name
	plugin_name=$(grep '^name = ' crates/oya-zellij-plugin/Cargo.toml | sed 's/name = "\([^"]*\)".*/\1/')
	if [ "$plugin_name" != "oya-zellij-plugin" ]; then
		fail "BUILD_FAILED" "Plugin package name is '$plugin_name', expected 'oya-zellij-plugin'"
	fi

	local ui_name
	ui_name=$(grep '^name = ' crates/oya-ui-components/Cargo.toml | sed 's/name = "\([^"]*\)".*/\1/')
	if [ "$ui_name" != "oya-ui-components" ]; then
		fail "BUILD_FAILED" "UI components package name is '$ui_name', expected 'oya-ui-components'"
	fi

	# Check workspace includes new crates
	if ! grep -q 'oya-zellij-plugin' Cargo.toml; then
		fail "BUILD_FAILED" "workspace Cargo.toml missing oya-zellij-plugin"
	fi

	if ! grep -q 'oya-ui-components' Cargo.toml; then
		fail "BUILD_FAILED" "workspace Cargo.toml missing oya-ui-components"
	fi

	# Check oya.kll references correct crate
	if ! grep -q 'oya-zellij-plugin' crates/oya-zellij-plugin/oya.kll; then
		fail "BUILD_FAILED" "oya.kll does not reference oya-zellij-plugin"
	fi

	# Check WASM config preserved
	if ! grep -q 'cdylib' crates/oya-zellij-plugin/Cargo.toml; then
		fail "BUILD_FAILED" "WASM cdylib target not preserved"
	fi

	# Check optimization flags preserved
	if ! grep -q 'opt-level = "z"' crates/oya-zellij-plugin/Cargo.toml; then
		fail "BUILD_FAILED" "Optimization flags not preserved"
	fi

	echo "✓ All postconditions validated"
}

# ==============================================================================
# MAIN EXECUTION
# ==============================================================================

main() {
	echo "=== Starting crate rename ==="
	echo ""

	# Step 1: Validate preconditions
	validate_preconditions
	echo ""

	# Step 2: Rename oya-ui → oya-zellij-plugin
	echo "Step 1: Rename oya-ui → oya-zellij-plugin"
	rename_crate "crates/oya-ui" "crates/oya-zellij-plugin"
	update_package_name "crates/oya-zellij-plugin/Cargo.toml" "oya-ui" "oya-zellij-plugin"
	update_oya_kll "crates/oya-zellij-plugin/oya.kll" "oya-ui" "oya-zellij-plugin"
	echo ""

	# Step 3: Rename oya-zellij → oya-ui-components
	echo "Step 2: Rename oya-zellij → oya-ui-components"
	rename_crate "crates/oya-zellij" "crates/oya-ui-components"
	update_package_name "crates/oya-ui-components/Cargo.toml" "oya-zellij" "oya-ui-components"
	echo ""

	# Step 4: Update workspace Cargo.toml
	echo "Step 3: Update workspace Cargo.toml"
	update_workspace
	echo ""

	# Step 5: Validate postconditions
	validate_postconditions
	echo ""

	echo "=== Rename complete ==="
	echo "✓ All invariants preserved"
	echo "✓ Ready for build validation"
}

# Execute main
main "$@"
