# Kill an invocation

Source: https://docs.restate.dev/admin-api/invocation/kill-an-invocation

schemas/openapi-admin.json patch /invocations/{invocation_id}/kill
Forcefully terminates an invocation. **Warning**: This operation does not guarantee consistency for virtual object instance state,
in-flight invocations to other services, or other side effects. Use with caution.
For more information, see the [cancellation documentation](https://docs.restate.dev/services/invocation/managing-invocations#kill).