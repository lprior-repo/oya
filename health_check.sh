#!/bin/bash
# Health check script - validate subagents are running

LOG_FILE="/tmp/subagent_health_$(date +%Y%m%d).log"
MAX_STALL_TIME=300 # 5 minutes

echo "=== Health Check Started: $(date) ===" >>"$LOG_FILE"

check_subagent() {
	local task_id=$1
	local last_update=$2

	if [ -z "$last_update" ]; then
		echo "[$(date)] ERROR: No update time for $task_id" >>"$LOG_FILE"
		return 1
	fi

	local current_time=$(date +%s)
	local time_diff=$((current_time - last_update))

	if [ $time_diff -gt $MAX_STALL_TIME ]; then
		echo "[$(date)] WARNING: $task_id has stalled for ${time_diff}s" >>"$LOG_FILE"
		return 1
	fi

	echo "[$(date)] OK: $task_id last update ${time_diff}s ago" >>"$LOG_FILE"
	return 0
}

echo "Checking task: ses_3af4c6444ffeHX9FkM7db8lG8i (bug hunt)" >>"$LOG_FILE"
echo "Checking task: ses_3af3a352fffeiYgrpl61A7p0uh (clippy)" >>"$LOG_FILE"

echo "=== Health Check Complete: $(date) ===" >>"$LOG_FILE"

# Run continuously
while true; do
	echo "--- $(date) ---" >>"$LOG_FILE"

	# Check if opencode session is still active
	if ! pgrep -f "opencode" >/dev/null; then
		echo "[$(date)] WARNING: opencode not running" >>"$LOG_FILE"
	fi

	sleep 60
done
