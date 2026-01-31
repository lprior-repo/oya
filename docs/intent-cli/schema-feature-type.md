# Intent Schema: #Feature Type

## Overview

The `#Feature` type groups related behaviors into logical domains or capabilities. Features help organize specifications by API area and make specs easier to understand and maintain.

## Type Definition

```cue
#Feature: {
	name:        string
	description: string
	behaviors:   [...#Behavior]
}
```

## Required Fields

### `name: string`
The feature identifier.
- Should be concise and descriptive
- Examples: `"User Authentication"`, `"Product Management"`, `"Payment Processing"`
- Used in CLI output and test reports
- Should be unique within a spec

### `description: string`
A human-readable description of what this feature provides.
- Explains the scope and purpose of the feature
- Used in documentation and AI context
- Should describe what users/developers can do with this feature
- Examples:
  - `"User login, registration, and token management"`
  - `"CRUD operations for product catalog"`
  - `"Payment creation, status checking, and refunds"`

### `behaviors: [...#Behavior]`
A list of test cases/behaviors for this feature.
- Must contain at least one behavior (non-empty array)
- Behaviors test different aspects of the feature
- See `#Behavior` type documentation
- Can represent:
  - Happy paths (successful operations)
  - Error cases (invalid inputs, authentication failures)
  - Edge cases (boundary conditions, rate limiting)
  - Integration scenarios (multiple operations)

## Complete Example

### User Management Feature

```cue
{
	name:        "User Management"
	description: "User account creation, authentication, and profile management"
	behaviors: [
		{
			name:   "register_user"
			intent: "Create a new user account with email and password"
			request: {
				method: "POST"
				path:   "/users/register"
				headers: {}
				query:   {}
				body: {
					email:    "newuser@example.com"
					password: "SecurePass123!"
					name:     "John Doe"
				}
			}
			response: {
				status: 201
				example: {
					id:    "user_123"
					email: "newuser@example.com"
					name:  "John Doe"
				}
				checks: {
					"id": {
						rule: "must be non-empty string"
						why:  "User ID is needed for future requests"
					}
					"email": {
						rule: "must match requested email"
						why:  "Confirm registration was successful"
					}
				}
			}
			notes:    "Verifies successful user creation"
			requires: []
			tags:     ["auth", "happy-path"]
			captures: {
				user_id: "id"
			}
		},
		{
			name:   "login_user"
			intent: "Authenticate with email and password"
			request: {
				method: "POST"
				path:   "/users/login"
				headers: {}
				query:   {}
				body: {
					email:    "user@example.com"
					password: "SecurePass123!"
				}
			}
			response: {
				status: 200
				example: {
					token: "eyJhbGciOiJIUzI1NiIs..."
					user: {
						id:    "user_123"
						email: "user@example.com"
						name:  "John Doe"
					}
				}
				checks: {
					"token": {
						rule: "must be non-empty JWT string"
						why:  "Token required for authenticated requests"
					}
					"user.email": {
						rule: "must match login email"
						why:  "Confirm correct user authenticated"
					}
				}
			}
			notes:    "Returns JWT token for subsequent requests"
			requires: []
			tags:     ["auth", "happy-path"]
			captures: {
				auth_token: "token"
			}
		},
		{
			name:   "login_invalid_password"
			intent: "Reject login with incorrect password"
			request: {
				method: "POST"
				path:   "/users/login"
				headers: {}
				query:   {}
				body: {
					email:    "user@example.com"
					password: "WrongPassword123!"
				}
			}
			response: {
				status: 401
				example: {
					error_code: "INVALID_CREDENTIALS"
					message:    "Invalid email or password"
				}
				checks: {
					"error_code": {
						rule: "must equal INVALID_CREDENTIALS"
						why:  "Specific error code for auth failures"
					}
					"message": {
						rule: "must be non-empty string"
						why:  "User needs to understand why login failed"
					}
				}
			}
			notes:    "Tests error handling for invalid credentials"
			requires: []
			tags:     ["auth", "error-case"]
			captures: {}
		},
		{
			name:   "get_profile"
			intent: "Retrieve authenticated user's profile"
			request: {
				method: "GET"
				path:   "/users/me"
				headers: {
					"Authorization": "Bearer ${auth_token}"
				}
				query: {}
			}
			response: {
				status: 200
				example: {
					id:    "user_123"
					email: "user@example.com"
					name:  "John Doe"
					created_at: "2024-01-15T10:30:00Z"
				}
				checks: {
					"id": {
						rule: "must be non-empty string"
						why:  "User identifier required"
					}
					"email": {
						rule: "must be valid email format"
						why:  "Email must be properly formatted"
					}
				}
			}
			notes:    "Uses captured auth_token from login_user"
			requires: ["login_user"]
			tags:     ["profile", "authenticated"]
			captures: {}
		},
	]
}
```

## Feature Organization Patterns

### By API Domain

```cue
features: [
	{
		name:        "Users"
		description: "User account management"
		behaviors: [...]
	},
	{
		name:        "Products"
		description: "Product catalog"
		behaviors: [...]
	},
	{
		name:        "Orders"
		description: "Order management"
		behaviors: [...]
	},
]
```

### By User Role

```cue
features: [
	{
		name:        "Customer Features"
		description: "Features available to customers"
		behaviors: [...]
	},
	{
		name:        "Admin Features"
		description: "Administrative operations"
		behaviors: [...]
	},
]
```

### By Operation Type

```cue
features: [
	{
		name:        "Read Operations"
		description: "GET endpoints for data retrieval"
		behaviors: [...]
	},
	{
		name:        "Write Operations"
		description: "POST/PUT/DELETE for data modification"
		behaviors: [...]
	},
	{
		name:        "Async Operations"
		description: "Long-running operations with webhooks"
		behaviors: [...]
	},
]
```

### By Workflow

```cue
features: [
	{
		name:        "Checkout Flow"
		description: "Multi-step checkout process"
		behaviors: [
			{name: "add_to_cart", ...},
			{name: "apply_coupon", ...},
			{name: "set_shipping", ...},
			{name: "create_payment", ...},
		]
	},
]
```

## Behavior Dependency Management

Features can have complex behavior sequences:

```cue
{
	name:        "Payment Processing"
	description: "Create and manage payments"
	behaviors: [
		{
			name: "create_payment"
			// ...
			captures: {
				payment_id: "id"
			}
		},
		{
			name: "get_payment_status"
			requires: ["create_payment"]
			request: {
				path: "/payments/${payment_id}"
				// ...
			}
		},
		{
			name: "refund_payment"
			requires: ["create_payment"]  // Must create payment first
			request: {
				method: "POST"
				path:   "/payments/${payment_id}/refund"
				// ...
			}
		},
	]
}
```

## Execution Order

When Intent executes a feature, it:

1. **Validates** all behaviors for syntax errors
2. **Resolves** dependency graph to determine execution order
3. **Executes** in order, skipping blocked behaviors
4. **Reports** results per behavior

Behaviors with no dependencies execute first (in definition order).
Behaviors with dependencies wait for their requirements to complete.

## Coverage Metrics

Features help with coverage analysis:

```
Feature: "User Management"
├── Happy Path: 3 behaviors (register, login, profile)
├── Error Cases: 2 behaviors (invalid password, not found)
├── Edge Cases: 1 behavior (concurrent login)
└── Coverage: 6/6 scenarios = 100%
```

## Best Practices

### ✅ Do

- **Group related behaviors** - Keep related operations together
- **Name clearly** - Use descriptive, unambiguous names
- **Document purpose** - Explain what the feature does
- **Order logically** - Put happy paths before error cases
- **Use dependencies** - Show workflow order with `requires`
- **Capture values** - Use `captures` to connect behaviors

### ❌ Don't

- **Mix unrelated behaviors** - Keep concerns separate
- **Use vague names** - "Test 1", "API Call", etc.
- **Create circular dependencies** - A → B → A creates deadlock
- **Make features too granular** - 1 behavior per feature is excessive
- **Forget dependencies** - Missing `requires` creates false positives
- **Make features too large** - 50+ behaviors per feature is hard to debug

## Feature Execution Report

Intent generates execution reports per feature:

```
Feature: User Management
  ✓ register_user (201)
  ✓ login_user (200)
  ✓ login_invalid_password (401)
  ✓ get_profile (200)

Summary: 4/4 passed (100%)
```

## Integration with #Spec

Features are required in every spec:

```cue
spec: intent.#Spec & {
	// ... other fields ...
	features: [
		{
			name:        "Feature 1"
			description: "Description"
			behaviors: [...]
		},
		{
			name:        "Feature 2"
			description: "Description"
			behaviors: [...]
		},
	]
	// ...
}
```

## See Also

- [`#Spec`](./schema-spec-type.md) - Top-level specification
- [`#Behavior`](./schema-behavior-type.md) - Individual test cases
- [`#Request`](./schema-request-type.md) - HTTP request definition
- [`#Response`](./schema-response-type.md) - Expected response
