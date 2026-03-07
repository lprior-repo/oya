# Delete deployment

Source: https://docs.restate.dev/admin-api/deployment/delete-deployment

schemas/openapi-admin.json delete /deployments/{deployment}
Delete a deployment. Currently, only forced deletions are supported.
**Use with caution**: forcing a deployment deletion can break in-flight invocations.