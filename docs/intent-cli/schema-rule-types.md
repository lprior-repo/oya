# Intent Schema: #Rule, #When, and #RuleCheck Types

## Overview

These three types work together to define global validation rules that apply to all responses in a specification. Rules help ensure consistent behavior across an entire API.

## #Rule Type

### Definition

```cue
#Rule: {
	name:        string
	description: string

	// When to apply this rule
	when: #When | *null

	// The check to perform
	check: #RuleCheck

	// Example of correct response
	example: {...} | *null
}
```

### Required Fields

#### `name: string`
Identifier for the rule.
- Should be descriptive
- Examples: `"Error Response Format"`, `"Rate Limit Headers"`, `"CORS Headers"`

#### `description: string`
Human-readable explanation of the rule.
- Explains what the rule validates
- Examples:
  - `"All error responses must have consistent error format"`
  - `"Rate limiting headers must be present on all responses"`
  - `"CORS headers must allow requests from authorized origins"`

#### `check: #RuleCheck`
The validation check to perform.
- See `#RuleCheck` type below
- Defines what to validate (body, fields, headers)

### Optional Fields

#### `when: #When` (default: `null`)
Conditions for when this rule applies.
- If null, rule applies to all responses
- If specified, rule only applies when conditions match
- See `#When` type below
- Examples: Only check for error responses, only on POST requests

#### `example: {...}` (default: `null`)
Example of a response that passes the rule.
- For documentation and AI learning
- Shows what a correct response looks like

## #When Type

### Definition

```cue
#When: {
	status?: string // e.g., ">= 400"
	method?: #Method
	path?:   string // regex pattern
}
```

### Optional Fields

All fields are optional - rule applies when ANY match (OR logic):

#### `status?: string`
Status code condition (e.g., `">= 400"`, `"== 401"`, `"4xx"`)
- Expressions: `==`, `!=`, `<`, `>`, `<=`, `>=`
- Ranges: `"4xx"` (400-499), `"5xx"` (500-599)
- Multiple: `"401 | 403"` (401 or 403)
- Examples:
  - `">= 400"` - All error responses
  - `"== 401"` - Unauthorized only
  - `"4xx"` - All client errors
  - `"== 204"` - No content responses

#### `method?: #Method`
HTTP method to match.
- Values: `"GET"`, `"POST"`, `"PUT"`, `"PATCH"`, `"DELETE"`, `"HEAD"`, `"OPTIONS"`
- Examples:
  - `"GET"` - Only apply to GET requests
  - `"POST"` - Only POST requests
  - (Can combine with other conditions)

#### `path?: string`
URL path regex pattern to match.
- Regex pattern for path matching
- Examples:
  - `"/users/.*"` - All user endpoints
  - `"/api/v[0-9]+/.*"` - All versioned API endpoints
  - `".*/admin/.*"` - All admin endpoints

## #RuleCheck Type

### Definition

```cue
#RuleCheck: {
	body_must_not_contain?: [...string]
	body_must_contain?: [...string]
	fields_must_exist?: [...string]
	fields_must_not_exist?: [...string]
	header_must_exist?:     string
	header_must_not_exist?: string
}
```

### Fields

All fields are optional - use only what you need to check:

#### `body_must_contain?: [...string]`
Response body must contain all these strings.
- Case-sensitive substring matching
- Examples:
  ```cue
  body_must_contain: [
    "error_code"
    "message"
  ]
  ```
- Use for: Ensuring error responses have required fields in JSON

#### `body_must_not_contain?: [...string]`
Response body must NOT contain any of these strings.
- Useful for security (hiding sensitive data)
- Examples:
  ```cue
  body_must_not_contain: [
    "stack trace"
    "exception"
    "password"
    "secret"
  ]
  ```
- Use for: Preventing sensitive data leaks

#### `fields_must_exist?: [...string]`
Response JSON must have these fields.
- Dot notation for nested fields
- Examples:
  ```cue
  fields_must_exist: [
    "id"
    "message"
    "error.code"
  ]
  ```
- Use for: Validating required response structure

#### `fields_must_not_exist?: [...string]`
Response JSON must NOT have these fields.
- Useful for security and privacy
- Examples:
  ```cue
  fields_must_not_exist: [
    "password"
    "api_secret"
    "internal_id"
  ]
  ```
- Use for: Preventing unintended data exposure

#### `header_must_exist?: string`
Response must have this HTTP header.
- Checks for header presence (not value)
- Examples:
  ```cue
  header_must_exist: "Content-Type"
  ```
- Use for: Verifying essential headers

#### `header_must_not_exist?: string`
Response must NOT have this HTTP header.
- Examples:
  ```cue
  header_must_not_exist: "X-Debug-Info"
  ```
- Use for: Security (removing debug headers in production)

## Complete Examples

### Basic Rule (Always Applied)

```cue
{
	name:        "Error Response Format"
	description: "All error responses must have error_code and message fields"
	when:        null  // Applies to all responses
	check: {
		fields_must_exist: ["error_code", "message"]
	}
	example: {
		error_code: "VALIDATION_ERROR"
		message:    "Invalid email format"
	}
}
```

### Rule with Status Condition

```cue
{
	name:        "Error Response Format"
	description: "All error responses must have consistent format"
	when: {
		status: ">= 400"  // Only apply to errors
	}
	check: {
		fields_must_exist: ["error_code", "message"]
		body_must_not_contain: ["exception", "stack trace"]
	}
	example: {
		error_code: "NOT_FOUND"
		message:    "Resource not found"
	}
}
```

### Rule with Method Condition

```cue
{
	name:        "POST/PUT Response Format"
	description: "Creation/update responses must include timestamp"
	when: {
		method: "POST"
	}
	check: {
		fields_must_exist: ["id", "created_at"]
	}
	example: {
		id:         "resource_123"
		created_at: "2024-01-15T10:30:00Z"
	}
}
```

### Rule with Path Condition

```cue
{
	name:        "User Endpoints Format"
	description: "User endpoints must never expose password"
	when: {
		path: "/users/.*"
	}
	check: {
		fields_must_not_exist: ["password", "password_hash"]
	}
	example: {
		id:    "user_123"
		email: "user@example.com"
		name:  "John Doe"
	}
}
```

### Rule with Multiple Conditions

```cue
{
	name:        "Successful User Creation"
	description: "POST to /users must return user with ID and timestamp"
	when: {
		status: "== 201"
		method: "POST"
		path:   "/users"
	}
	check: {
		fields_must_exist: ["id", "email", "created_at"]
		fields_must_not_exist: ["password"]
	}
	example: {
		id:         "user_123"
		email:      "new@example.com"
		created_at: "2024-01-15T10:30:00Z"
	}
}
```

### Security Rule - Hide Sensitive Data

```cue
{
	name:        "Security: Hide Sensitive Data"
	description: "No sensitive data in any response"
	when:        null  // All responses
	check: {
		body_must_not_contain: [
			"password"
			"api_key"
			"secret_token"
			"credit_card"
			"ssn"
			"sql"
			"exception"
			"trace"
		]
		fields_must_not_exist: ["password", "secret", "api_key"]
	}
}
```

### Rate Limiting Rule

```cue
{
	name:        "Rate Limit Headers"
	description: "All responses must include rate limit info"
	when:        null
	check: {
		header_must_exist: "X-Rate-Limit-Limit"
		header_must_exist: "X-Rate-Limit-Remaining"
		header_must_exist: "X-Rate-Limit-Reset"
	}
	example: {
		id:   "resource_123"
		data: "content"
	}
}
```

### CORS Rule

```cue
{
	name:        "CORS Headers"
	description: "API responses must include CORS headers"
	when:        null
	check: {
		header_must_exist: "Access-Control-Allow-Origin"
		header_must_exist: "Access-Control-Allow-Methods"
	}
	example: null
}
```

### Consistency Rule

```cue
{
	name:        "Response Structure Consistency"
	description: "All successful responses have consistent structure"
	when: {
		status: "== 200"
	}
	check: {
		fields_must_exist: ["id", "created_at", "updated_at"]
		body_must_contain: ["data"]  // JSON string "data" present
	}
	example: {
		id:         "item_123"
		data:       "content"
		created_at: "2024-01-15T10:30:00Z"
		updated_at: "2024-01-15T11:00:00Z"
	}
}
```

## Rules in Specifications

Rules are defined globally in a spec:

```cue
spec: intent.#Spec & {
	name:        "User API"
	description: "User management API"

	// ...other fields...

	rules: [
		{
			name:        "Error Response Format"
			description: "All errors must have error_code and message"
			when: { status: ">= 400" }
			check: {
				fields_must_exist: ["error_code", "message"]
				body_must_not_contain: ["stack trace", "exception"]
			}
		},
		{
			name:        "Security: Hide Passwords"
			description: "Never expose password fields"
			check: {
				fields_must_not_exist: ["password", "password_hash"]
			}
		},
		{
			name:        "Rate Limit Headers"
			description: "All responses include rate limit info"
			check: {
				header_must_exist: "X-Rate-Limit-Limit"
			}
		},
	]
}
```

## Execution Order

Rules are applied:

1. **After behavior execution** - All behaviors run first
2. **To all responses** - Each rule checks every response
3. **With condition matching** - Rules only apply if conditions match
4. **For validation** - Responses must pass all applicable rules

Example execution flow:

```
Response from GET /users/123 (status: 200)
↓
Apply Rule 1: "Error Response Format"
  → when.status ">= 400"? → No, skip
↓
Apply Rule 2: "Security: Hide Passwords"
  → when: null? → Yes, apply
  → fields_must_not_exist: ["password"]? → Check: ✓ Pass
↓
Apply Rule 3: "Rate Limit Headers"
  → when: null? → Yes, apply
  → header_must_exist: "X-Rate-Limit-Limit"? → Check: ✓ Pass
↓
Result: All rules passed ✓
```

## Best Practices

### ✅ Do

- **Be specific** - Use specific conditions (status, method, path)
- **Document purpose** - Explain why the rule matters
- **Check structure** - Validate required fields exist
- **Hide sensitive** - Use must_not_contain/exist for security
- **Include examples** - Show what correct responses look like
- **Keep simple** - One rule per concern

### ❌ Don't

- **Overuse wildcards** - Specify conditions when possible
- **Create redundant rules** - Don't repeat checks
- **Check trivial things** - Focus on important constraints
- **Mix concerns** - Keep rules focused
- **Skip security** - Always validate no secrets are exposed

## Common Rule Patterns

### Error Handling
```cue
rules: [{
	when: { status: ">= 400" }
	check: {
		fields_must_exist: ["error_code", "message"]
		body_must_not_contain: ["stack", "exception"]
	}
}]
```

### Security
```cue
rules: [{
	check: {
		fields_must_not_exist: ["password", "secret", "api_key"]
		body_must_not_contain: ["password", "secret"]
	}
}]
```

### Compliance
```cue
rules: [{
	check: {
		header_must_exist: "Content-Type"
		header_must_exist: "X-Request-ID"
		header_must_exist: "Cache-Control"
	}
}]
```

## Integration with Spec

Rules are part of the spec for global consistency:

```cue
spec: intent.#Spec & {
	config: { base_url: "https://api.example.com", ... }
	features: [...]
	rules: [...]  // Global validation rules
	anti_patterns: [...]
}
```

## See Also

- [`#Spec`](./schema-spec-type.md) - Top-level spec containing rules
- [`#Behavior`](./schema-behavior-type.md) - Individual tests (rules apply to their responses)
- [`#Response`](./schema-response-type.md) - Response validation (local checks)
