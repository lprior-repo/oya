# Resume an invocation

Source: https://docs.restate.dev/admin-api/invocation/resume-an-invocation

schemas/openapi-admin.json patch /invocations/{invocation_id}/resume
Resumes a paused or suspended invocation. If the invocation is backing off due to a retry, this will immediately trigger the retry.
Optionally, you can change the deployment ID that will be used when the invocation resumes. For more information see [resume documentation](https://docs.restate.dev/services/invocation/managing-invocations#resume)