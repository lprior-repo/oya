# Update deployment

Source: https://docs.restate.dev/admin-api/deployment/update-deployment

schemas/openapi-admin.json patch /deployments/{deployment}
Updates an existing deployment configuration, such as the endpoint address or invocation headers.
By default, service schemas are not re-discovered. Set `overwrite: true` to trigger re-discovery.