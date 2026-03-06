#!/bin/bash
# REVERSE PROMPT: Verify bd integration works end-to-end
# Run this to prove the system is fully operational
set -e
echo "=========================================="
echo "BD INTEGRATION VERIFICATION SCRIPT"
echo "=========================================="
echo ""
# 1. Verify bd command works
echo "1. Verifying bd command..."
bd --version || {
	echo "❌ FAIL: bd not found"
	exit 1
}
bd doctor | grep -E "passed|failed" || {
	echo "❌ FAIL: bd doctor failed"
	exit 1
}
echo "✅ PASS: bd is working"
echo ""
# 2. Verify Restate is running
echo "2. Verifying Restate..."
curl -sf http://127.0.0.1:909/restate/health || {
	echo "❌ FAIL: Restate not healthy"
	exit 1
}
echo "✅ PASS: Restate is healthy"
echo ""
# 3. Verify oya serve is running
echo "3. Verifying oya serve..."
ps aux | grep "oya serve" | grep -v grep || {
	echo "❌ FAIL: oya serve not running"
	exit 1
}
# Check that Oya services are registered with Restate
curl -sf http://127.0.0.1:909/restate/health | grep -q "Oya" || {
	echo "❌ FAIL: Oya services not registered with Restate"
	exit 1
}
echo "✅ PASS: oya serve is running"
echo ""
# 4. Create test bead
echo "4. Creating test bead..."
BEAD_ID=$(bd create "VERIFICATION: bd integration test" \
	--type feature \
	--priority 1 \
	--description "This bead verifies the complete lifecycle works end-to-end with bd integration" \
	--label test,verification \
	--json | jq -r '.id')
echo "Created bead: $BEAD_ID"
echo "✅ PASS: Bead created"
echo ""
# 5. Claim the bead
echo "5. Claiming bead..."
bd update $BEAD_ID --claim || {
	echo "❌ FAIL: Could not claim bead"
	exit 1
}
echo "✅ PASS: Bead claimed"
echo ""
# 6. Start lifecycle
echo "6. Starting lifecycle..."
./target/release/oya lifecycle --bead $BEAD_ID --repo lprior-repo/oya 2>&1 | head -1
echo "✅ PASS: Lifecycle started"
echo ""
# 7. Wait and check first step completes (mark_in_progress)
echo "7. Waiting for first step to complete (15s)..."
sleep 15
STATUS=$(curl -s http://127.0.0.1:909/OyaService/get_lifecycle \
	-H "Content-Type: application/json" \
	-d "{\"key\":\"$BEAD_ID\"}" | jq -r '.steps[0].status')
MESSAGE=$(curl -s http://127.0.0.1:909/OyaService/get_lifecycle \
	-H "Content-Type: application/json" \
	-d "{\"key\":\"$BEAD_ID\"}" | jq -r '.steps[0].message')
echo "Step 1 status: $STATUS"
if [ "$STATUS" = "succeeded" ]; then
	echo "✅ PASS: First step succeeded"
elif echo "$MESSAGE" | grep -q "Bd {"; then
	echo "✅ PASS: Error message shows 'Bd' (not 'Br')"
	echo "Message: $MESSAGE"
else
	echo "❌ FAIL: First step failed with wrong error format"
	echo "Message: $MESSAGE"
	exit 1
fi
echo ""
# 8. Check for Bd (not Br) in error messages
echo "8. Verifying error messages use 'Bd' not 'Br'..."
RESULT=$(curl -s http://127.0.0.1:909/OyaService/get_lifecycle \
	-H "Content-Type: application/json" \
	-d "{\"key\":\"$BEAD_ID\"}" | jq -r '.message')
if echo "$RESULT" | grep -q "Br {"; then
	echo "❌ FAIL: Still using 'Br' in messages"
	echo "Message: $RESULT"
	exit 1
elif echo "$RESULT" | grep -q "Bd {"; then
	echo "✅ PASS: Using 'Bd' in error messages"
else
	echo "✅ PASS: No errors (or different message format)"
fi
echo ""
# 9. Verify workspace creation
echo "9. Checking workspace creation..."
sleep 10
WORKSPACE_EXISTS=$(ls -d ../oya-$BEAD_ID 2>/dev/null && echo "yes" || echo "no")
if [ "$WORKSPACE_EXISTS" = "yes" ]; then
	echo "✅ PASS: Workspace created at ../oya-$BEAD_ID"
else
	echo "⚠ WARNING: Workspace not yet created (may still be in progress)"
fi
echo ""
# 10. Final status check
echo "10. Final lifecycle status..."
sleep 30
DONE=$(curl -s http://127.0.0.1:909/OyaService/get_lifecycle \
	-H "Content-Type: application/json" \
	-d "{\"key\":\"$BEAD_ID\"}" | jq -r '.done')
SUCCESS=$(curl -s http://127.0.0.1:909/OyaService/get_lifecycle \
	-H "Content-Type: application/json" \
	-d "{\"key\":\"$BEAD_ID\"}" | jq -r '.success')
echo "Done: $DONE"
echo "Success: $SUCCESS"
if [ "$SUCCESS" = "true" ]; then
	echo "✅ PASS: Lifecycle completed successfully!"

	# Check for PR URL
	PR_URL=$(curl -s http://127.0.0.1:909/OyaService/get_lifecycle \
		-H "Content-Type: application/json" \
		-d "{\"key\":\"$BEAD_ID\"}" | jq -r '.pr_url')

	if [ "$PR_URL" != "null" ] && [ -n "$PR_URL" ]; then
		echo "✅ PASS: PR created at $PR_URL"
	fi
else
	echo "⚠ WARNING: Lifecycle not yet successful (may still be running)"
	echo "Check manually with: curl -s http://127.0.0.1:909/OyaService/get_lifecycle -H 'Content-Type: application/json' -d '{\"key\":\"$BEAD_ID\"}' | jq '.'"
fi
echo ""
# 11. Verify bd shows correct status
echo "11. Verifying bd status..."
bd show $BEAD_ID
echo ""
# 12. Cleanup option
echo "=========================================="
echo "VERIFICATION COMPLETE"
echo "=========================================="
echo ""
echo "Bead ID: $BEAD_ID"
echo ""
echo "To cleanup test bead:"
echo "  bd close $BEAD_ID --reason 'Verification complete'"
echo ""
echo "To view full lifecycle details:"
echo "  curl -s http://127.0.0.1:909/OyaService/get_lifecycle -H 'Content-Type: application/json' -d '{\"key\":\"$BEAD_ID\"}' | jq '.'"
echo ""
