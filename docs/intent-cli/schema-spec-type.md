# Intent Schema: #Spec Type

## Overview

The `#Spec` type is the top-level type in the Intent schema. It defines a complete, rigorously analyzed specification that includes EARS requirements, KIRK contracts, and testable behaviors for implementation verification.

## Type Definition

```cue
#Spec: {
	name:        string
	description: string
	audience:    string | *""
	version:     string | *"1.0.0"

	success_criteria: [...string]

	config: #Config | *#DefaultConfig

	features:      [...#Feature]
	rules:         [...#Rule]
	anti_patterns: [...#AntiPattern]

	// Optional AI implementation hints
	ai_hints?: #AIHints
}
```

## Required Fields

### `name: string`
A unique identifier for the specification.
- Must be descriptive and concise
- Used in documentation and CLI output
- Example: `"User Management API"`, `"Payment Service"`

### `description: string`
A human-readable description of what this specification covers.
- Should explain the purpose and scope of the API
- Used in documentation and AI context
- Example: `"REST API for user authentication and profile management"`

### `success_criteria: [...string]`
A list of acceptance criteria that define success for the API.
- Each criterion should be testable through behaviors
- Used to validate that implementation meets requirements
- Examples:
  - `"All endpoints return appropriate HTTP status codes"`
  - `"Authentication tokens expire after 1 hour"`
  - `"Error responses include meaningful error messages"`

### `config: #Config`
Configuration for specification validation and verification, including base URL, timeout, and headers.
- See `#Config` type documentation
- Example:
  ```cue
  config: {
    base_url:   "https://api.example.com"
    timeout_ms: 5000
    headers: {
      "Accept": "application/json"
      "User-Agent": "Intent-CLI/2.0"
    }
  }
  ```

### `features: [...#Feature]`
A list of feature groups that organize related behaviors.
- At least one feature should be present for meaningful specs
- See `#Feature` type documentation
- Features can represent API domains (Users, Payments, etc.)

### `rules: [...#Rule]`
Global validation rules that apply to all responses.
- Optional but recommended for consistency checks
- See `#Rule` type documentation
- Examples:
  - All error responses must have consistent error format
  - All responses must include request ID header
  - Rate limit headers must be present on rate-limited endpoints

### `anti_patterns: [...#AntiPattern]`
Common mistakes and anti-patterns to avoid in implementation.
- Helps implementers understand what NOT to do
- See `#AntiPattern` type documentation
- Examples:
  - Don't use HTTP 200 for error responses
  - Don't expose stack traces in error messages
  - Don't return sensitive data in list responses

## Optional Fields

### `audience: string` (default: `""`)
Target users or developers for this specification.
- Example: `"Backend developers"`, `"Mobile app developers"`, `"Third-party integrators"`
- Helps contextualize the API's purpose

### `version: string` (default: `"1.0.0"`)
Semantic version of the specification.
- Follows semantic versioning (MAJOR.MINOR.PATCH)
- Examples: `"1.0.0"`, `"2.1.3"`, `"0.1.0-beta"`

### `ai_hints: #AIHints` (optional)
Implementation guidance for AI systems generating code.
- See `#AIHints` type documentation
- Includes suggested stack, security hints, and common pitfalls

## Complete Example

```cue
package api

import "github.com/intent-cli/intent/schema:intent"

spec: intent.#Spec & {
	name:        "User Management API"
	description: "REST API for user authentication, registration, and profile management"
	audience:    "Mobile and web application developers"
	version:     "2.0.0"

	success_criteria: [
		"All endpoints return correct HTTP status codes",
		"Authentication requires valid JWT token",
		"User passwords are never returned in responses",
		"All error responses include error codes and messages",
		"Rate limiting is enforced at 100 requests/minute",
	]

	config: {
		base_url:   "https://api.example.com"
		timeout_ms: 5000
		headers: {
			"Content-Type": "application/json"
			"Accept": "application/json"
		}
	}

	features: [
		{
			name:        "User Authentication"
			description: "Login and token management"
			behaviors: [...]
		},
		{
			name:        "User Profiles"
			description: "User profile CRUD operations"
			behaviors: [...]
		},
	]

	rules: [
		{
			name:        "Error Response Format"
			description: "All error responses must have consistent format"
			check: {
				fields_must_exist: ["error_code", "message"]
			}
		},
	]

	anti_patterns: [
		{
			name:        "Status Code Misuse"
			description: "Don't use 200 for error responses"
			bad_example: {
				status: 200
				body: { error: "Invalid credentials" }
			}
			good_example: {
				status: 401
				body: { error_code: "INVALID_CREDENTIALS", message: "Invalid email or password" }
			}
		},
	]

	ai_hints: {
		implementation: {
			suggested_stack: ["Node.js", "Express", "PostgreSQL", "jsonwebtoken"]
		}
		security: {
			password_hashing: "bcrypt with salt rounds >= 10"
			jwt_algorithm: "HS256 or RS256"
			jwt_expiry: "1 hour for access token, 7 days for refresh token"
			rate_limiting: "100 requests per minute per IP"
		}
		pitfalls: [
			"Don't store passwords in plain text",
			"Don't hardcode JWT secret in code",
			"Don't return user object with password field",
			"Don't allow username enumeration in login endpoints",
		]
	}
}
```

## Validation Rules

1. **Required fields must be non-empty**: All required fields must have values
2. **Name must be unique**: No two specs should have the same name
3. **Version must be semantic**: Follow MAJOR.MINOR.PATCH format
4. **Features must have behaviors**: Each feature must contain at least one behavior
5. **Success criteria should be testable**: Each criterion should map to behaviors
6. **Base URL must be valid**: Config base_url should be a valid HTTP/HTTPS URL
7. **No circular dependencies**: Behavior requires should not create cycles

## Integration with Intent CLI

The `#Spec` type is used by all Intent CLI commands:

- **`gleam run -- validate <spec.cue>`**: Validates spec against schema and structure
- **`gleam run -- check <spec.cue> --target <url>`**: Executes all behaviors against target
- **`gleam run -- lint <spec.cue>`**: Checks for common issues and improvements
- **`gleam run -- analyze <spec.cue>`**: Generates insights and coverage analysis

## See Also

- [`#Config`](./schema-config-type.md) - Configuration structure
- [`#Feature`](./schema-feature-type.md) - Feature groups
- [`#Behavior`](./schema-behavior-type.md) - Individual test cases
- [`#Rule`](./schema-rule-type.md) - Global validation rules
- [`#AntiPattern`](./schema-antipattern-type.md) - Anti-pattern definitions
- [`#AIHints`](./schema-aihints-type.md) - AI implementation guidance
