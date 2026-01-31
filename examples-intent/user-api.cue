package user_api

import "github.com/intent-cli/intent/schema:intent"

spec: intent.#Spec & {
	name: "User Management API"

	description: """
		This API manages user accounts. Users can register with an email
		and password, log in to receive a JWT token, and manage their
		profile. Passwords must never appear in any API response.
		"""

	audience: "Mobile and web clients"

	success_criteria: [
		"Users can register, login, and manage their profile",
		"Authentication uses JWT tokens",
		"Passwords are never exposed in responses",
		"All errors return structured error objects",
	]

	config: {
		base_url:   "http://localhost:8080"
		timeout_ms: 5000
	}

	features: [
		{
			name: "User Registration"

			description: """
				New users register with email and password. The system
				validates the email format and password strength, creates
				the account, and returns the new user (without password).
				"""

			behaviors: [
				{
					name:   "successful-registration"
					intent: "A new user can create an account"

					request: {
						method: "POST"
						path:   "/users"
						body: {
							email:    "newuser@example.com"
							password: "SecurePass123!"
							name:     "New User"
						}
					}

					response: {
						status: 201

						example: {
							id:         "usr_abc123xyz"
							email:      "newuser@example.com"
							name:       "New User"
							created_at: "2024-01-15T10:30:00Z"
						}

						checks: {
							"id": {
								rule: "string matching usr_[a-z0-9]+"
								why:  "IDs are prefixed for debuggability"
							}
							"email": {
								rule: "equals request.body.email"
								why:  "Confirms the email was saved correctly"
							}
							"password": {
								rule: "absent"
								why:  "SECURITY: Never expose passwords"
							}
							"created_at": {
								rule: "valid ISO8601 datetime"
								why:  "Timestamps for audit trail"
							}
						}
					}

					captures: {
						new_user_id: "response.body.id"
					}
				},
				{
					name:   "duplicate-email-rejected"
					intent: "Cannot register with an email that's already taken"

					requires: ["successful-registration"]

					request: {
						method: "POST"
						path:   "/users"
						body: {
							email:    "newuser@example.com"
							password: "DifferentPass456!"
						}
					}

					response: {
						status: 409

						example: {
							error: {
								code:    "EMAIL_EXISTS"
								message: "An account with this email already exists"
							}
						}

						checks: {
							"error.code": {
								rule: "equals EMAIL_EXISTS"
								why:  "Specific error code for client handling"
							}
							"error.message": {
								rule: "non-empty string"
								why:  "Human-readable message for display"
							}
						}
					}
				},
				{
					name:   "invalid-email-rejected"
					intent: "Email format is validated"

					request: {
						method: "POST"
						path:   "/users"
						body: {
							email:    "not-an-email"
							password: "SecurePass123!"
						}
					}

					response: {
						status: 400

						checks: {
							"error.code": {rule: "equals INVALID_EMAIL"}
						}
					}
				},
				{
					name:   "weak-password-rejected"
					intent: "Password must meet strength requirements"

					request: {
						method: "POST"
						path:   "/users"
						body: {
							email:    "another@example.com"
							password: "weak"
						}
					}

					response: {
						status: 400

						checks: {
							"error.code": {rule: "equals WEAK_PASSWORD"}
						}
					}

					notes: """
						Password requirements:
						- At least 8 characters
						- At least one uppercase letter
						- At least one number
						- At least one special character
						"""
				},
			]
		},
		{
			name: "Authentication"

			description: """
				Users authenticate with email/password and receive a JWT
				token. The token is used in the Authorization header for
				subsequent requests.
				"""

			behaviors: [
				{
					name:   "successful-login"
					intent: "Valid credentials return a JWT token"

					requires: ["successful-registration"]

					request: {
						method: "POST"
						path:   "/auth/login"
						body: {
							email:    "newuser@example.com"
							password: "SecurePass123!"
						}
					}

					response: {
						status: 200

						example: {
							token:         "eyJhbGciOiJIUzI1NiIs..."
							token_type:    "Bearer"
							expires_in:    3600
							refresh_token: "dGhpcyBpcyBhIHJlZnJl..."
						}

						checks: {
							"token": {
								rule: "valid JWT"
								why:  "Main authentication credential"
							}
							"token_type": {
								rule: "equals Bearer"
								why:  "Standard OAuth2 token type"
							}
							"expires_in": {
								rule: "integer >= 3600"
								why:  "Token valid for at least 1 hour"
							}
						}
					}

					captures: {
						auth_token: "response.body.token"
					}
				},
				{
					name:   "wrong-password-rejected"
					intent: "Invalid password returns 401"

					requires: ["successful-registration"]

					request: {
						method: "POST"
						path:   "/auth/login"
						body: {
							email:    "newuser@example.com"
							password: "WrongPassword!"
						}
					}

					response: {
						status: 401

						checks: {
							"error.code": {rule: "equals INVALID_CREDENTIALS"}
						}
					}

					notes: """
						Error message must NOT reveal whether email exists.
						Always return generic INVALID_CREDENTIALS, never
						EMAIL_NOT_FOUND or WRONG_PASSWORD separately.
						"""
				},
				{
					name:   "unknown-email-rejected"
					intent: "Unknown email returns same error as wrong password"

					request: {
						method: "POST"
						path:   "/auth/login"
						body: {
							email:    "nonexistent@example.com"
							password: "AnyPassword123!"
						}
					}

					response: {
						status: 401

						checks: {
							"error.code": {rule: "equals INVALID_CREDENTIALS"}
						}
					}
				},
			]
		},
		{
			name: "Profile Management"

			description: """
				Authenticated users can read and update their profile.
				"""

			behaviors: [
				{
					name:   "get-own-profile"
					intent: "User can retrieve their own profile"

					requires: ["successful-login"]

					request: {
						method: "GET"
						path:   "/users/${new_user_id}"
						headers: {
							"Authorization": "Bearer ${auth_token}"
						}
					}

					response: {
						status: 200

						checks: {
							"id":    {rule: "equals ${new_user_id}"}
							"email": {rule: "equals newuser@example.com"}
							"name":  {rule: "equals New User"}
						}
					}
				},
				{
					name:   "update-profile"
					intent: "User can update their name"

					requires: ["get-own-profile"]

					request: {
						method: "PATCH"
						path:   "/users/${new_user_id}"
						headers: {
							"Authorization": "Bearer ${auth_token}"
						}
						body: {
							name: "Updated Name"
						}
					}

					response: {
						status: 200

						checks: {
							"name":       {rule: "equals Updated Name"}
							"updated_at": {rule: "valid ISO8601 datetime"}
						}
					}
				},
				{
					name:   "unauthenticated-access-denied"
					intent: "Cannot access profile without token"

					request: {
						method: "GET"
						path:   "/users/${new_user_id}"
					}

					response: {
						status: 401

						checks: {
							"error.code": {rule: "equals UNAUTHORIZED"}
						}
					}
				},
			]
		},
	]

	rules: [
		{
			name:        "no-sensitive-data"
			description: "Passwords and secrets must never appear in responses"

			check: {
				body_must_not_contain: ["password", "secret", "api_key", "private_key"]
			}
		},
		{
			name:        "structured-errors"
			description: "All error responses have consistent structure"

			when: {status: ">= 400"}

			check: {
				fields_must_exist: ["error.code", "error.message"]
			}

			example: {
				error: {
					code:    "ERROR_CODE"
					message: "Human readable description"
				}
			}
		},
		{
			name:        "content-type-header"
			description: "All responses declare content type"

			check: {
				header_must_exist: "Content-Type"
			}
		},
	]

	anti_patterns: [
		{
			name:        "password-in-response"
			description: "NEVER return password in any response"

			bad_example: {
				id:       "usr_123"
				email:    "user@example.com"
				password: "secret123"
			}

			good_example: {
				id:    "usr_123"
				email: "user@example.com"
			}
		},
		{
			name:        "user-enumeration"
			description: "Login errors must not reveal if email exists"

			bad_example: {
				error: {
					code:    "EMAIL_NOT_FOUND"
					message: "No account with this email"
				}
			}

			good_example: {
				error: {
					code:    "INVALID_CREDENTIALS"
					message: "Invalid email or password"
				}
			}
		},
		{
			name:        "plain-text-ids"
			description: "IDs should not be sequential integers"

			bad_example: {
				id: 1
			}

			good_example: {
				id: "usr_x7k9m2p4q"
			}

			why: """
				Sequential IDs reveal business metrics and enable enumeration
				attacks. Use prefixed random strings instead.
				"""
		},
	]

	ai_hints: {
		implementation: {
			suggested_stack: ["Node.js", "Express", "PostgreSQL"]
		}

		entities: {
			user: {
				fields: {
					id:         "string, prefixed 'usr_', randomly generated"
					email:      "string, unique, validated format"
					password:   "string, hashed with bcrypt, NEVER returned"
					name:       "string, 1-100 chars"
					created_at: "datetime, set on creation"
					updated_at: "datetime, set on every update"
				}
			}
		}

		security: {
			password_hashing: "bcrypt with cost factor >= 10"
			jwt_algorithm:    "HS256 or RS256"
			jwt_expiry:       "1 hour minimum"
			rate_limiting:    "100 requests per minute per IP"
		}

		pitfalls: [
			"Don't return password field even if it's hashed",
			"Don't use sequential integer IDs",
			"Don't reveal whether email exists in login errors",
			"Don't forget to validate email format",
			"Don't allow empty passwords",
		]
	}
}
