package conflicts_gaps

import "github.com/intent-cli/intent/schema:intent"

// Example: Conflict and Gap Detection
// Demonstrates common requirement conflicts and specification gaps
// that Intent's interview system helps identify and resolve

spec: intent.#Spec & {
	name: "Multi-Tenant SaaS API"

	description: """
		A multi-tenant SaaS platform API that demonstrates common conflicts
		between different stakeholder perspectives and gaps in requirements.

		This spec documents RESOLVED conflicts and gaps as a reference for
		how Intent helps discover and address these issues during interviews.

		# Conflicts Found and Resolved

		1. TENANT ISOLATION (Security vs Performance)
		   - Security: Complete data isolation, separate databases
		   - Performance: Shared database with row-level security
		   - Resolution: Shared DB with encryption per tenant

		2. API RATE LIMITS (Business vs Engineering)
		   - Business: Unlimited for enterprise tier
		   - Engineering: Must have limits to protect infrastructure
		   - Resolution: High limits (10k/min) for enterprise, with burst

		3. DATA RETENTION (Legal vs Cost)
		   - Legal: Keep everything for 7 years
		   - Cost: Storage is expensive at scale
		   - Resolution: 7 years cold storage, 90 days hot

		4. ERROR VERBOSITY (Security vs Developer Experience)
		   - Security: Minimal error details to prevent info leaks
		   - DX: Detailed errors help debugging
		   - Resolution: Verbose in dev/staging, minimal in production

		# Gaps Identified and Filled

		1. Tenant onboarding flow (who creates first admin?)
		2. Cross-tenant data sharing (is it ever allowed?)
		3. Tenant deletion and data export requirements
		4. Audit log access (who can see what?)
		"""

	audience: """
		Primary: Enterprise customers managing their own tenant data
		Secondary: Tenant administrators configuring their organization
		Tertiary: Platform operators managing the multi-tenant system
		"""

	version: "2.0.0"

	success_criteria: [
		"Tenants are completely isolated from each other",
		"Each tenant can customize their configuration",
		"Platform admins can manage all tenants",
		"Audit logs track all sensitive operations",
		"Data export available for compliance",
	]

	config: {
		base_url:   "http://localhost:8080"
		timeout_ms: 5000
		headers: {
			"X-Tenant-ID": "${tenant_id}"
		}
	}

	features: [
		{
			name: "Tenant Isolation"

			description: """
				Core tenant isolation demonstrating the RESOLVED conflict
				between complete isolation (security) and shared resources
				(performance/cost).

				CONFLICT: Security wanted separate databases per tenant.
				          Operations said this doesn't scale past 100 tenants.

				RESOLUTION: Shared database with:
				- Row-level security (RLS) policies
				- Encrypted tenant data with per-tenant keys
				- Query-level tenant ID enforcement
				"""

			behaviors: [
				{
					name:   "tenant-data-isolation"
					intent: "User can only access data from their own tenant"

					notes: """
						CONFLICT RESOLUTION:
						- Every query includes tenant_id filter (auto-added by middleware)
						- RLS policies prevent cross-tenant access at DB level
						- Tenant ID in JWT is source of truth, not request headers
						"""

					request: {
						method: "GET"
						path:   "/users"
						headers: {
							"Authorization": "Bearer ${user_token}"
						}
					}

					response: {
						status: 200

						example: {
							users: [
								{
									id:        "usr_abc123"
									tenant_id: "tenant_acme"
									email:     "alice@acme.com"
									role:      "admin"
								},
								{
									id:        "usr_def456"
									tenant_id: "tenant_acme"
									email:     "bob@acme.com"
									role:      "member"
								},
							]
							meta: {
								tenant_id: "tenant_acme"
								total:     2
							}
						}

						checks: {
							"meta.tenant_id": {
								rule: "equals ${tenant_id}"
								why:  "Response confirms tenant context (from isolation conflict resolution)"
							}
							"users": {
								rule: "array where each matches .*tenant_acme.*"
								why:  "All returned users belong to requesting tenant"
							}
						}
					}
				},
				{
					name:   "cross-tenant-access-blocked"
					intent: "Attempting to access another tenant's data returns 404"

					notes: """
						GAP IDENTIFIED: Original spec didn't specify what happens when
						a user tries to access another tenant's resources.

						Options considered:
						1. Return 403 Forbidden - reveals resource exists
						2. Return 404 Not Found - no information leakage
						3. Return 400 Bad Request - confusing

						RESOLUTION: Return 404 to prevent tenant enumeration attacks.
						"""

					request: {
						method: "GET"
						path:   "/users/usr_other_tenant"
						headers: {
							"Authorization": "Bearer ${user_token}"
						}
					}

					response: {
						status: 404

						example: {
							error: {
								code:    "NOT_FOUND"
								message: "User not found"
							}
						}

						checks: {
							"error.code": {
								rule: "equals NOT_FOUND"
								why:  "Don't reveal the resource exists in another tenant"
							}
							"error.message": {
								rule: "string containing not found"
								why:  "Generic message, no tenant information leaked"
							}
						}
					}
				},
			]
		},
		{
			name: "Rate Limiting"

			description: """
				API rate limiting demonstrating the RESOLVED conflict between
				Business (wanting unlimited for enterprise) and Engineering
				(needing infrastructure protection).

				CONFLICT: Sales promised "unlimited API calls" to enterprise.
				          Platform team said unlimited will cause outages.

				RESOLUTION:
				- Free tier: 100 requests/minute
				- Pro tier: 1,000 requests/minute
				- Enterprise: 10,000 requests/minute with burst to 15,000
				- All tiers get clear headers showing usage
				"""

			behaviors: [
				{
					name:   "rate-limit-headers"
					intent: "Every response includes rate limit information"

					notes: """
						CONFLICT RESOLUTION: Instead of "unlimited", enterprise gets
						very high limits plus burst capacity. All tiers get transparent
						headers so clients can self-throttle.
						"""

					request: {
						method: "GET"
						path:   "/api/status"
						headers: {
							"Authorization": "Bearer ${user_token}"
						}
					}

					response: {
						status: 200

						headers: {
							"X-RateLimit-Limit":     "10000"
							"X-RateLimit-Remaining": "9995"
							"X-RateLimit-Reset":     "1705326000"
						}

						example: {
							status: "ok"
						}

						checks: {
							// Header checks would be done separately
							"status": {
								rule: "equals ok"
								why:  "Basic status check"
							}
						}
					}
				},
				{
					name:   "rate-limit-exceeded"
					intent: "Exceeding rate limit returns 429 with retry info"

					notes: """
						GAP IDENTIFIED: Original spec didn't define what happens
						when rate limit is exceeded.

						RESOLUTION: Return 429 with Retry-After header and helpful
						message. Log for abuse detection but don't immediately block.
						"""

					request: {
						method: "GET"
						path:   "/api/status"
						headers: {
							"Authorization": "Bearer ${user_token}"
						}
					}

					response: {
						status: 429

						headers: {
							"Retry-After":           "30"
							"X-RateLimit-Limit":     "10000"
							"X-RateLimit-Remaining": "0"
						}

						example: {
							error: {
								code:        "RATE_LIMITED"
								message:     "Rate limit exceeded. Please retry after 30 seconds."
								retry_after: 30
								limit:       10000
								tier:        "enterprise"
								upgrade_url: null
							}
						}

						checks: {
							"error.code": {
								rule: "equals RATE_LIMITED"
								why:  "Clear error code for programmatic handling"
							}
							"error.retry_after": {
								rule: "integer >= 1"
								why:  "Tells client when to retry"
							}
							"error.tier": {
								rule: "one of [\"free\", \"pro\", \"enterprise\"]"
								why:  "Shows current tier for upgrade prompts"
							}
						}
					}
				},
			]
		},
		{
			name: "Data Retention"

			description: """
				Data retention policies demonstrating RESOLVED conflict between
				Legal (keep everything) and Cost (minimize storage).

				CONFLICT: Legal required 7-year retention for audit.
				          Finance said storing 7 years of data is too expensive.

				RESOLUTION: Tiered storage
				- Hot: Last 90 days, fast queries
				- Warm: 90 days to 1 year, slower queries
				- Cold: 1-7 years, archived, retrieval takes hours
				- Deleted: After 7 years, permanently removed
				"""

			behaviors: [
				{
					name:   "query-recent-data"
					intent: "Recent data (hot tier) returns immediately"

					request: {
						method: "GET"
						path:   "/audit-logs"
						headers: {
							"Authorization": "Bearer ${admin_token}"
						}
						query: {
							from: "2024-01-01"
							to:   "2024-01-15"
						}
					}

					response: {
						status: 200

						example: {
							logs: [
								{
									id:         "log_abc123"
									timestamp:  "2024-01-15T10:30:00Z"
									actor_id:   "usr_admin"
									action:     "user.created"
									resource:   "usr_new123"
									tenant_id:  "tenant_acme"
									ip_address: "192.168.1.1"
								},
							]
							meta: {
								storage_tier: "hot"
								query_time:   "45ms"
								total:        1250
							}
						}

						checks: {
							"meta.storage_tier": {
								rule: "equals hot"
								why:  "Recent data served from hot storage"
							}
							"logs": {
								rule: "array"
								why:  "Returns array of audit logs"
							}
							"logs[0].tenant_id": {
								rule: "equals ${tenant_id}"
								why:  "Logs scoped to requesting tenant"
							}
						}
					}
				},
				{
					name:   "query-archived-data"
					intent: "Old data (cold tier) requires async retrieval"

					notes: """
						GAP IDENTIFIED: How do users access 5-year-old audit data?

						RESOLUTION: Async retrieval with job tracking
						1. User requests data range
						2. System returns job ID
						3. User polls for completion
						4. Data available for 24 hours once retrieved
						"""

					request: {
						method: "POST"
						path:   "/audit-logs/retrieve"
						headers: {
							"Authorization": "Bearer ${admin_token}"
						}
						body: {
							from: "2019-01-01"
							to:   "2019-12-31"
						}
					}

					response: {
						status: 202

						example: {
							job_id:              "job_archive_xyz789"
							status:              "pending"
							estimated_time:      "2-4 hours"
							storage_tier:        "cold"
							notification_email:  "admin@acme.com"
							expires_at:          "2024-01-17T10:30:00Z"
						}

						checks: {
							"status": {
								rule: "equals pending"
								why:  "Async job is queued"
							}
							"job_id": {
								rule: "string starting with job_"
								why:  "Job ID for polling"
							}
							"storage_tier": {
								rule: "equals cold"
								why:  "Indicates archived data retrieval"
							}
						}
					}
				},
			]
		},
		{
			name: "Error Handling"

			description: """
				Error response verbosity demonstrating RESOLVED conflict between
				Security (minimal info) and Developer Experience (detailed errors).

				CONFLICT: Security team wanted errors like "An error occurred"
				          Developers complained they can't debug integrations

				RESOLUTION: Environment-aware error responses
				- Production: Minimal errors with request ID for support
				- Staging/Dev: Detailed errors with stack traces
				- All environments: Structured error codes for programmatic handling
				"""

			behaviors: [
				{
					name:   "production-error-minimal"
					intent: "Production errors are minimal but trackable"

					notes: """
						CONFLICT RESOLUTION: Production errors include:
						- Structured error code (for programmatic handling)
						- Human message (generic but helpful)
						- Request ID (for support ticket correlation)
						- NO stack traces, internal details, or query info
						"""

					request: {
						method: "POST"
						path:   "/users"
						headers: {
							"Authorization": "Bearer ${admin_token}"
							"X-Environment":  "production"
						}
						body: {
							email: "invalid-email"
						}
					}

					response: {
						status: 400

						example: {
							error: {
								code:       "VALIDATION_ERROR"
								message:    "The request contains invalid data"
								request_id: "req_abc123xyz"
								docs_url:   "https://docs.example.com/errors/VALIDATION_ERROR"
							}
						}

						checks: {
							"error.code": {
								rule: "non-empty string"
								why:  "Error code for programmatic handling"
							}
							"error.request_id": {
								rule: "string starting with req_"
								why:  "Request ID for support correlation"
							}
							"error.stack": {
								rule: "absent"
								why:  "No stack traces in production (security)"
							}
							"error.query": {
								rule: "absent"
								why:  "No SQL queries exposed (security)"
							}
						}
					}
				},
				{
					name:   "development-error-verbose"
					intent: "Development errors include debugging details"

					notes: """
						For non-production environments, include:
						- Field-level validation errors
						- Internal error context
						- Suggested fixes
						Still no stack traces in API responses (use logs)
						"""

					request: {
						method: "POST"
						path:   "/users"
						headers: {
							"Authorization": "Bearer ${admin_token}"
							"X-Environment":  "development"
						}
						body: {
							email: "invalid-email"
						}
					}

					response: {
						status: 400

						example: {
							error: {
								code:       "VALIDATION_ERROR"
								message:    "The request contains invalid data"
								request_id: "req_dev123xyz"
								details: {
									fields: {
										email: {
											value:      "invalid-email"
											constraint: "Must be a valid email address"
											pattern:    "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
										}
									}
								}
								suggestion: "Check the email field format"
							}
						}

						checks: {
							"error.code": {
								rule: "equals VALIDATION_ERROR"
								why:  "Same error code as production"
							}
							"error.details": {
								rule: "present"
								why:  "Details included in development"
							}
							"error.details.fields.email": {
								rule: "present"
								why:  "Field-level error information"
							}
						}
					}
				},
			]
		},
		{
			name: "Tenant Lifecycle"

			description: """
				Tenant onboarding and offboarding - major GAPS identified
				during interview process.

				GAPS IDENTIFIED:
				1. Who creates the first admin user for a new tenant?
				2. What happens to data when a tenant is deleted?
				3. How does data export work for compliance?
				4. Can a deleted tenant be restored?
				"""

			behaviors: [
				{
					name:   "create-tenant"
					intent: "Platform admin creates new tenant with initial admin"

					notes: """
						GAP RESOLVED: "Who creates the first admin?"

						Answer: Platform admin creates tenant with initial admin email.
						System sends invitation to that email to set password.
						This solves chicken-and-egg of no users in empty tenant.
						"""

					request: {
						method: "POST"
						path:   "/platform/tenants"
						headers: {
							"Authorization": "Bearer ${platform_admin_token}"
						}
						body: {
							name:        "Acme Corp"
							slug:        "acme"
							tier:        "enterprise"
							admin_email: "admin@acme.com"
						}
					}

					response: {
						status: 201

						example: {
							id:          "tenant_acme"
							name:        "Acme Corp"
							slug:        "acme"
							tier:        "enterprise"
							status:      "provisioning"
							admin: {
								email:             "admin@acme.com"
								invitation_sent:   true
								invitation_expires: "2024-01-22T10:30:00Z"
							}
							created_at: "2024-01-15T10:30:00Z"
						}

						checks: {
							"id": {
								rule: "string starting with tenant_"
								why:  "Tenant ID prefix"
							}
							"status": {
								rule: "equals provisioning"
								why:  "Tenant starts in provisioning state"
							}
							"admin.invitation_sent": {
								rule: "equals true"
								why:  "Admin receives invitation email"
							}
						}
					}

					captures: {
						new_tenant_id: "response.body.id"
					}
				},
				{
					name:   "delete-tenant-request"
					intent: "Tenant deletion is a controlled process with data export"

					notes: """
						GAP RESOLVED: "What happens when a tenant is deleted?"

						Answer: Multi-step process:
						1. Request deletion (starts 30-day countdown)
						2. Data export generated automatically
						3. Tenant marked as "pending_deletion"
						4. Can cancel during 30-day grace period
						5. After 30 days, data permanently deleted
						"""

					request: {
						method: "POST"
						path:   "/tenants/${tenant_id}/delete"
						headers: {
							"Authorization": "Bearer ${tenant_admin_token}"
						}
						body: {
							confirm:     true
							reason:      "Switching to competitor"
							export_data: true
						}
					}

					response: {
						status: 202

						example: {
							tenant_id:      "tenant_acme"
							status:         "pending_deletion"
							deletion_date:  "2024-02-15T10:30:00Z"
							grace_period:   30
							can_cancel:     true
							export: {
								job_id:     "export_xyz789"
								status:     "generating"
								format:     "zip"
								includes:   ["users", "data", "audit_logs", "config"]
								expires_at: "2024-01-22T10:30:00Z"
							}
						}

						checks: {
							"status": {
								rule: "equals pending_deletion"
								why:  "Deletion is pending, not immediate"
							}
							"grace_period": {
								rule: "integer >= 30"
								why:  "Minimum 30-day grace period for compliance"
							}
							"can_cancel": {
								rule: "equals true"
								why:  "Can be cancelled during grace period"
							}
							"export.status": {
								rule: "one of [\"generating\", \"ready\", \"failed\"]"
								why:  "Data export is automatic"
							}
						}
					}
				},
				{
					name:   "cancel-deletion"
					intent: "Tenant can cancel deletion during grace period"

					notes: """
						GAP RESOLVED: "Can a deleted tenant be restored?"

						Answer: Yes, during the 30-day grace period.
						After that, data is permanently gone per retention policy.
						"""

					request: {
						method: "POST"
						path:   "/tenants/${tenant_id}/cancel-deletion"
						headers: {
							"Authorization": "Bearer ${tenant_admin_token}"
						}
					}

					response: {
						status: 200

						example: {
							tenant_id: "tenant_acme"
							status:    "active"
							message:   "Deletion cancelled. Tenant restored to active status."
						}

						checks: {
							"status": {
								rule: "equals active"
								why:  "Tenant is restored to active"
							}
						}
					}
				},
			]
		},
		{
			name: "Cross-Tenant Sharing"

			description: """
				Data sharing between tenants - GAP that revealed complex
				requirements during interview.

				GAP IDENTIFIED: "Is cross-tenant data sharing ever allowed?"

				Answer: Yes, in specific controlled scenarios:
				1. Tenant A explicitly shares a resource with Tenant B
				2. Sharing is read-only by default
				3. Both tenant admins must approve
				4. Audit log tracks all cross-tenant access
				5. Sharing can be revoked anytime
				"""

			behaviors: [
				{
					name:   "create-share-link"
					intent: "Tenant admin creates a sharing link for a resource"

					request: {
						method: "POST"
						path:   "/resources/res_abc123/share"
						headers: {
							"Authorization": "Bearer ${tenant_admin_token}"
						}
						body: {
							target_tenant: "tenant_partner"
							permission:    "read"
							expires_in:    "7d"
						}
					}

					response: {
						status: 201

						example: {
							share_id:       "share_xyz789"
							resource_id:    "res_abc123"
							source_tenant:  "tenant_acme"
							target_tenant:  "tenant_partner"
							permission:     "read"
							status:         "pending_acceptance"
							expires_at:     "2024-01-22T10:30:00Z"
							share_url:      "https://app.example.com/shared/share_xyz789"
						}

						checks: {
							"status": {
								rule: "equals pending_acceptance"
								why:  "Requires acceptance by target tenant"
							}
							"permission": {
								rule: "one of [\"read\", \"read_write\"]"
								why:  "Limited permission options"
							}
							"expires_at": {
								rule: "valid ISO8601 datetime"
								why:  "Shares must have expiration"
							}
						}
					}
				},
				{
					name:   "access-shared-resource"
					intent: "Target tenant accesses shared resource"

					notes: """
						Cross-tenant access is:
						- Logged in both tenants' audit logs
						- Subject to target tenant's rate limits
						- Revocable by source tenant at any time
						"""

					request: {
						method: "GET"
						path:   "/shared/share_xyz789/resource"
						headers: {
							"Authorization": "Bearer ${partner_token}"
						}
					}

					response: {
						status: 200

						example: {
							resource: {
								id:   "res_abc123"
								name: "Shared Document"
								data: {}
							}
							share_meta: {
								source_tenant: "tenant_acme"
								permission:    "read"
								expires_at:    "2024-01-22T10:30:00Z"
								accessed_via:  "share_xyz789"
							}
						}

						checks: {
							"share_meta.source_tenant": {
								rule: "non-empty string"
								why:  "Shows where data came from"
							}
							"share_meta.permission": {
								rule: "equals read"
								why:  "Shows what access level granted"
							}
						}
					}
				},
			]
		},
	]

	rules: [
		{
			name:        "tenant-context-required"
			description: "All data responses must include tenant context"

			check: {
				fields_must_exist: ["tenant_id"]
			}
		},
		{
			name:        "no-cross-tenant-data"
			description: "Regular API calls cannot access other tenants"

			check: {
				body_must_not_contain: ["all_tenants", "cross_tenant", "bypass_isolation"]
			}
		},
	]

	anti_patterns: [
		{
			name:        "tenant-in-url-only"
			description: "Don't rely on URL for tenant identification"

			bad_example: {
				url: "/tenants/acme/users"
			}

			good_example: {
				url:     "/users"
				headers: {"X-Tenant-ID": "from_jwt"}
			}

			why: """
				Tenant ID in URL can be manipulated. Extract tenant from
				authenticated JWT token instead.
				"""
		},
		{
			name:        "global-admin-bypass"
			description: "Don't create god-mode admin that bypasses isolation"

			bad_example: {
				role: "super_admin"
				can_access: "all_tenants"
			}

			good_example: {
				role: "platform_admin"
				must_impersonate: true
				audit_logged: true
			}

			why: """
				Even platform admins should impersonate specific tenants
				with full audit logging, not have global access.
				"""
		},
	]

	ai_hints: {
		implementation: {
			suggested_stack: ["Rust", "PostgreSQL with RLS", "Redis"]
		}

		entities: {
			tenant: {
				fields: {
					id:         "string, 'tenant_' + slug"
					name:       "string, display name"
					slug:       "string, URL-safe identifier"
					tier:       "enum: free, pro, enterprise"
					status:     "enum: provisioning, active, suspended, pending_deletion"
					encryption_key: "per-tenant encryption key, never exposed"
				}
			}
		}

		security: {
			password_hashing: "bcrypt with cost 12"
			jwt_algorithm:    "RS256"
			jwt_expiry:       "1 hour"
			rate_limiting:    "Tiered per plan: 100/1000/10000 req/min"
		}

		pitfalls: [
			"Never trust tenant ID from request headers alone",
			"Always validate JWT tenant claim matches request",
			"Log all cross-tenant access attempts",
			"Don't cache data without tenant context",
			"Rate limits must be per-tenant, not global",
			"Encryption keys must be per-tenant for true isolation",
		]
	}
}
