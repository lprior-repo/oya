# Purge invocation journal

Source: https://docs.restate.dev/admin-api/invocation/purge-invocation-journal

schemas/openapi-admin.json patch /invocations/{invocation_id}/purge-journal
Deletes only the journal entries for a completed invocation, while retaining its metadata.
This operation only applies to invocations that have already completed.