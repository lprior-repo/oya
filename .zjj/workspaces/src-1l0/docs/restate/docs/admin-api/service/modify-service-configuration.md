# Modify service configuration

Source: https://docs.restate.dev/admin-api/service/modify-service-configuration

schemas/openapi-admin.json patch /services/{service}
Updates the configuration of a registered service, such as public visibility, retention policies, and timeout settings.
Note: Service re-discovery will update these settings based on the service endpoint configuration.