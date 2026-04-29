# Cancel an invocation

Source: https://docs.restate.dev/admin-api/invocation/cancel-an-invocation

schemas/openapi-admin.json patch /invocations/{invocation_id}/cancel
Gracefully cancels an invocation. The invocation is terminated, but its progress is persisted, allowing consistency guarantees to be maintained.
For more information, see the [cancellation documentation](https://docs.restate.dev/services/invocation/managing-invocations#cancel).