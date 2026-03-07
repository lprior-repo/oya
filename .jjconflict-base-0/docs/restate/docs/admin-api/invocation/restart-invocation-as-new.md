# Restart invocation as new

Source: https://docs.restate.dev/admin-api/invocation/restart-invocation-as-new

schemas/openapi-admin.json patch /invocations/{invocation_id}/restart-as-new
Creates a new invocation from a completed invocation, optionally copying partial progress from the original invocation's journal.
The new invocation will have a different invocation ID. Use the `from` parameter to specify how much of the original journal to preserve.