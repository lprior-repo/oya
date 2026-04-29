# Purge a completed invocation

Source: https://docs.restate.dev/admin-api/invocation/purge-a-completed-invocation

schemas/openapi-admin.json patch /invocations/{invocation_id}/purge
Deletes all state associated with a completed invocation, including its journal and metadata.
This operation only applies to invocations that have already completed. For more information,
see the [purging documentation](https://docs.restate.dev/services/invocation/managing-invocations#purge).