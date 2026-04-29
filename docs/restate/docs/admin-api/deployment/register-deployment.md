# Register deployment

Source: https://docs.restate.dev/admin-api/deployment/register-deployment

schemas/openapi-admin.json post /deployments
Registers a new deployment (HTTP or Lambda). Restate will invoke the endpoint to discover available services and handlers,
and make them available for invocation. For more information, see the [deployment documentation](https://docs.restate.dev/services/versioning#registering-a-deployment).