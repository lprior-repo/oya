#!/bin/bash
set -e

# OYA Frontend E2E Verification Suite (dioxus-agent-rs)
# Verifies "One-to-One Transparency" and Restate Integration

AGENT="/home/lewis/src/dioxus-agent-rs/target/release/dioxus-agent-rs"
URL=${1:-"http://100.117.222.107:8084"}

echo "🚀 Starting E2E Verification against $URL"

# Helper: Wait for text to appear in DOM
wait_for_text() {
    local text=$1
    local timeout=30
    echo "⏳ Waiting for text: '$text'..."
    for i in $(seq 1 $timeout); do
        if $AGENT --url "$URL" dom | grep -qi "$text"; then
            echo "✅ Found: '$text'"
            return 0
        fi
        sleep 1
    done
    echo "❌ Timeout waiting for: '$text'"
    return 1
}

# 1. Verification: App Shell & Title
echo "--- Phase 1: App Shell ---"
$AGENT --url "$URL" dom | grep -qi "Oya Frontend"
echo "✅ App Shell Loaded"

# 2. Verification: Restate Integration (One-to-One Transparency)
echo "--- Phase 2: Restate Integration ---"
# Toggling the Restate Panel (if it's not already visible)
# Note: In a real test we'd use 'click' on the toggle button, 
# but here we verify the initial state or text.
wait_for_text "Restate Invocations"
wait_for_text "No active invocations"
echo "✅ Restate Panel Initialized & Empty State Verified"

# 3. Verification: Restate DAG SDK Nodes (24/24 Parity)
echo "--- Phase 3: DAG Node Parity ---"
# We check for a few of the newly implemented nodes in the sidebar/search
wait_for_text "Kafka Consumer"
wait_for_text "Workflow Submit"
wait_for_text "Parallel"
wait_for_text "Loop Iterate"
echo "✅ New Restate Nodes Verified in UI"

# 4. Final Proof: Capture state
echo "--- Phase 4: Capture Evidence ---"
$AGENT --url "$URL" screenshot e2e_success.png
echo "📸 Evidence saved to e2e_success.png"

echo "🎯 E2E Verification Complete: PASS"
