# Intent Schema: #Behavior Type

## Overview

The `#Behavior` type represents a single test case or scenario within a feature. Behaviors are the fundamental unit of testing in Intent - they execute HTTP requests and validate responses against expected outcomes.

## Type Definition

```cue
#Behavior: {
	name:   #Identifier
	intent: string // Plain English purpose

	// Additional context for humans/AI
	notes: string | *""

	// Dependencies - behaviors that must run first
	requires: [...#Identifier] | *[]

	// Tags for filtering
	tags: [...string] | *[]

	request: #Request

	response: #Response

	// Capture values for later use
	captures: #Captures | *{}
}
```

## Required Fields

### `name: #Identifier`
Unique identifier for the behavior.
- Pattern: `^[a-z][a-z0-9_-]*$` (lowercase, alphanumeric, hyphens, underscores)
- Must be unique within the feature
- Examples: `"create_user"`, `"list_products"`, `"update_order_status"`
- Used in test reports, dependency graphs, and value captures

### `intent: string`
Plain English description of what this behavior tests.
- Should be a single sentence describing the test purpose
- Written for humans and AI to understand
- Examples:
  - `"Create a new user with valid credentials"`
  - `"List all products with pagination"`
  - `"Reject invalid email format in registration"`
  - `"Handle concurrent requests with rate limiting"`

### `request: #Request`
The HTTP request to execute.
- See `#Request` type documentation
- Includes method, path, headers, query parameters, and body
- Can use variable interpolation: `"${variable_name}"`

### `response: #Response`
Expected response definition.
- See `#Response` type documentation
- Specifies expected status code, example, and validation checks
- Checks are compared against actual response

## Optional Fields

### `notes: string` (default: `""`)
Additional context and explanation.
- Helps developers understand the test
- Documents assumptions or special conditions
- Examples:
  - `"User must be registered before login"`
  - `"This endpoint has rate limiting of 100 req/min"`
  - `"Response time should be < 200ms"`
  - `"Tests idempotency by sending same request twice"`

### `requires: [...#Identifier]` (default: `[]`)
Behavior dependencies.
- List of behavior names that must complete before this one
- Behaviors are executed in dependency order
- Can have multiple dependencies
- Creates test workflow sequences
- Examples:
  - `["create_user"]` - must create user first
  - `["login", "get_profile"]` - must login and get profile
  - `[]` - no dependencies, can run independently

**Dependency Features:**
- Values captured in dependency behaviors can be used via variable interpolation
- If a dependency fails, dependent behaviors are skipped (blocked)
- Circular dependencies are invalid and detected at validation time

### `tags: [...string]` (default: `[]`)
Classification tags for filtering and reporting.
- Examples: `["auth"]`, `["happy-path"]`, `["error-case"]`, `["integration"]`
- Used for:
  - Running subsets of tests: `gleam run -- check spec.cue --tags auth`
  - Coverage reporting: group results by tag
  - Documentation: categorize test types
- Common tag patterns:
  - **Happy path**: `"happy-path"`, `"success"`
  - **Error cases**: `"error"`, `"error-case"`, `"validation"`
  - **Edge cases**: `"edge-case"`, `"boundary"`
  - **Integration**: `"integration"`, `"workflow"`
  - **Performance**: `"performance"`, `"load"`
  - **Security**: `"security"`, `"auth"`, `"permissions"`

### `captures: #Captures` (default: `{}`)
Values to extract from response for use in dependent behaviors.
- Type: `[#Identifier]: string` (field names to capture)
- Values are extracted using JSONPath or similar
- Can be used in dependent behaviors via `${capture_name}`
- Examples:
  ```cue
  captures: {
    user_id: "id"
    auth_token: "token"
    order_id: "order.id"
  }
  ```

## Complete Examples

### Simple Behavior (No Dependencies)

```cue
{
	name:   "list_users"
	intent: "Retrieve a list of all users"
	request: {
		method: "GET"
		path:   "/users"
		headers: {}
		query: {
			limit:  "10"
			offset: "0"
		}
	}
	response: {
		status: 200
		example: {
			users: [
				{ id: "1", name: "Alice", email: "alice@example.com" }
			]
		}
		checks: {
			"users": {
				rule: "must be an array"
				why:  "API contract requires array response"
			}
		}
	}
	notes:    "Returns paginated list of users"
	requires: []
	tags:     ["users", "read", "happy-path"]
	captures: {}
}
```

### Behavior with Captures (for Dependent Behaviors)

```cue
{
	name:   "create_user"
	intent: "Register a new user account"
	request: {
		method: "POST"
		path:   "/users"
		headers: {}
		query:   {}
		body: {
			email:    "newuser@example.com"
			password: "SecurePass123!"
			name:     "New User"
		}
	}
	response: {
		status: 201
		example: {
			id:    "user_123"
			email: "newuser@example.com"
			name:  "New User"
		}
		checks: {
			"id": {
				rule: "must be a non-empty string"
				why:  "User ID needed for subsequent requests"
			}
		}
	}
	notes:    "Captures user_id for use in dependent behaviors"
	requires: []
	tags:     ["users", "write", "happy-path"]
	captures: {
		user_id: "id"  // Extract 'id' field and name it 'user_id'
	}
}
```

### Behavior with Dependencies

```cue
{
	name:   "get_user_profile"
	intent: "Retrieve the authenticated user's profile"
	request: {
		method: "GET"
		path:   "/users/${user_id}"  // Uses captured user_id
		headers: {}
		query:   {}
	}
	response: {
		status: 200
		example: {
			id:    "user_123"
			email: "newuser@example.com"
			name:  "New User"
		}
		checks: {
			"id": {
				rule: "must equal the requested user_id"
				why:  "Verify correct user data returned"
			}
		}
	}
	notes:    "Requires user_id from create_user behavior"
	requires: ["create_user"]  // Must run create_user first
	tags:     ["users", "read", "authenticated"]
	captures: {}
}
```

### Error Case Behavior

```cue
{
	name:   "create_user_duplicate_email"
	intent: "Reject registration with existing email"
	request: {
		method: "POST"
		path:   "/users"
		headers: {}
		query:   {}
		body: {
			email:    "existing@example.com"
			password: "SecurePass123!"
			name:     "Another User"
		}
	}
	response: {
		status: 409  // Conflict
		example: {
			error_code: "EMAIL_ALREADY_EXISTS"
			message:    "Email already registered"
		}
		checks: {
			"error_code": {
				rule: "must equal EMAIL_ALREADY_EXISTS"
				why:  "Specific error code for duplicate emails"
			}
		}
	}
	notes:    "Tests validation for duplicate email addresses"
	requires: []
	tags:     ["users", "validation", "error-case"]
	captures: {}
}
```

### Complex Workflow Behavior

```cue
{
	name:   "update_user_and_verify"
	intent: "Update user profile and verify changes"
	request: {
		method: "PUT"
		path:   "/users/${user_id}"
		headers: {}
		query:   {}
		body: {
			name: "Updated Name"
		}
	}
	response: {
		status: 200
		example: {
			id:    "user_123"
			name:  "Updated Name"
			email: "user@example.com"
		}
		checks: {
			"name": {
				rule: "must equal 'Updated Name'"
				why:  "Verify update was applied"
			}
			"id": {
				rule: "must equal the user_id"
				why:  "Ensure correct user was updated"
			}
		}
	}
	notes:    "Tests idempotency - verify same request gets same result"
	requires: ["create_user"]
	tags:     ["users", "write", "update", "integration"]
	captures: {
		updated_user_id: "id"
	}
}
```

### Behavior with Multiple Dependencies

```cue
{
	name:   "process_order"
	intent: "Create and process an order"
	request: {
		method: "POST"
		path:   "/orders"
		headers: {}
		query:   {}
		body: {
			user_id: "${user_id}"
			product_ids: ["${product_id}"]
			payment_method_id: "${payment_method_id}"
		}
	}
	response: {
		status: 201
		example: {
			order_id: "order_123"
			status:   "confirmed"
		}
		checks: {
			"order_id": {
				rule: "must be non-empty"
				why:  "Order ID required for tracking"
			}
		}
	}
	notes:    "Requires user, product, and payment setup"
	requires: ["create_user", "list_products", "add_payment_method"]
	tags:     ["orders", "write", "workflow", "integration"]
	captures: {
		order_id: "order_id"
	}
}
```

## Behavior Execution States

During testing, a behavior can be in these states:

- **Pending**: Waiting for dependencies to complete
- **Blocked**: A dependency failed (behavior is skipped)
- **Executing**: Currently running the HTTP request
- **Passed**: Response matches expected checks
- **Failed**: Response doesn't match expected checks
- **Error**: Request couldn't be sent (network, timeout, etc.)

## Variable Interpolation

Behaviors can reference captured values:

```cue
// In create_user behavior:
captures: {
	user_id: "id"
}

// In subsequent behavior:
request: {
	path: "/users/${user_id}"  // Uses captured value
}
```

Interpolation works in:
- Request paths: `/users/${user_id}`
- Request headers: `Authorization: Bearer ${auth_token}`
- Request query: `?user_id=${user_id}`
- Request body: `{ owner_id: "${user_id}" }`

## Behavior Naming Convention

### ✅ Good Names

- **Action-based**: `create_user`, `delete_product`, `update_order`
- **Specific**: `login_with_valid_credentials`, `create_order_with_discount`
- **Clear intent**: `handle_missing_email_validation`, `concurrent_login_attempts`

### ❌ Bad Names

- **Too generic**: `test1`, `api_call`, `request`
- **Vague**: `do_something`, `check_data`, `verify`
- **Unclear**: `xyz`, `temp`, `foo_bar`

## Best Practices

### ✅ Do

- **One assertion per behavior** - Each behavior tests one thing
- **Use descriptive names** - Intent should be obvious from name
- **Capture strategically** - Only capture values needed by dependents
- **Order dependencies logically** - Happy path before error cases
- **Tag consistently** - Use standard tags across specs
- **Document assumptions** - Use notes to explain context
- **Use error cases** - Don't just test happy paths

### ❌ Don't

- **Create circular dependencies** - A → B → A is invalid
- **Capture everything** - Only capture what's needed
- **Mix concerns** - One behavior, one purpose
- **Use hardcoded IDs** - Use captures to pass values between behaviors
- **Ignore error cases** - Test both success and failure
- **Make assumptions** - Document what might not be obvious

## Integration with Features

Behaviors exist within features:

```cue
features: [{
	name:        "User Management"
	description: "User account operations"
	behaviors: [
		{ name: "create_user", ... },
		{ name: "get_user", requires: ["create_user"], ... },
		{ name: "delete_user", requires: ["create_user"], ... },
	]
}]
```

## Execution Guarantees

Intent provides these execution guarantees:

1. **Dependency ordering**: Behaviors execute in dependency order
2. **Isolation**: Each behavior is an independent request
3. **Determinism**: Same spec, same target = same results
4. **No side effects**: Behaviors don't modify Intent state
5. **Captures persist**: Values captured are available to dependents

## See Also

- [`#Request`](./schema-request-type.md) - HTTP request definition
- [`#Response`](./schema-response-type.md) - Expected response
- [`#Feature`](./schema-feature-type.md) - Feature grouping
- [`#Identifier`](./schema-identifier-type.md) - Valid identifier pattern
