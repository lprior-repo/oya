# Heartbeat Instructions

Your agent should update the heartbeat file regularly to show it's alive.

## Heartbeat File Location
`.zjj/heartbeat`

## Update Interval
Every 30 seconds

## How to Update
```bash
echo $(date +%s) > .zjj/heartbeat
```

## Example (for long-running agents)
```bash
# In your agent loop, call:
update_heartbeat() {
    echo $(date +%s) > .zjj/heartbeat
}

# Call every 30 seconds
while true; do
    update_heartbeat
    # ... do your work ...
    sleep 30
done
```
