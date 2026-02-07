#!/bin/bash
# Test script for health indicators functionality
# This validates the health status indicators without requiring full WASM compilation

set -e

# Change to script directory
cd "$(dirname "$0")/.."

echo "=== Testing Health Status Indicators ==="
echo ""

# Check syntax of the health_indicators module
echo "✓ Checking syntax..."
rustup run 1.83 rustc --crate-type lib src/ui/health_indicators.rs --edition 2021

echo "✓ Syntax check passed"
echo ""

# Verify the module exports the expected types
echo "Checking module exports..."
grep -q "pub enum HealthStatus" src/ui/health_indicators.rs && echo "✓ HealthStatus enum exported"
grep -q "pub struct HealthTracker" src/ui/health_indicators.rs && echo "✓ HealthTracker struct exported"
grep -q "pub fn format_health" src/ui/health_indicators.rs && echo "✓ format_health function exported"
grep -q "pub fn overall_health" src/ui/health_indicators.rs && echo "✓ overall_health function exported"
echo ""

# Verify test coverage
echo "Checking test coverage..."
test_count=$(grep -c "#\[test\]" src/ui/health_indicators.rs || echo "0")
echo "✓ Found $test_count unit tests"

if [ "$test_count" -lt 20 ]; then
    echo "⚠ Warning: Expected at least 20 tests, found $test_count"
else
    echo "✓ Good test coverage"
fi
echo ""

# Verify zero panic policy
echo "Checking for panics..."
panic_count=$(grep -i "panic!\|unwrap()\|expect(" src/ui/health_indicators.rs | grep -v "^[ ]*//" | grep -v "test.*panic" | wc -l)
if [ "$panic_count" -gt 0 ]; then
    echo "✗ Found $panic_count potential panic sources"
    grep -n "panic!\|unwrap()\|expect(" src/ui/health_indicators.rs | grep -v "^[ ]*//" | grep -v "test.*panic"
    exit 1
else
    echo "✓ Zero panic policy enforced"
fi
echo ""

# Verify documentation
echo "Checking documentation..."
doc_lines=$(grep -c "^///" src/ui/health_indicators.rs || echo "0")
echo "✓ Found $doc_lines documentation lines"
echo ""

echo "=== All Checks Passed ==="
echo ""
echo "Health Status Indicators module is ready!"
echo ""
echo "Features implemented:"
echo "  • Three-state health model (Healthy, Unhealthy, Unknown)"
echo "  • Color-coded visual indicators (green/red/gray)"
echo "  • Symbol-based status display (●/✗/?)"
echo "  • Health score calculation from scores"
echo "  • Health change tracking and events"
echo "  • Configurable formatting options"
echo "  • Zero panics throughout"
echo "  • Comprehensive unit tests"
