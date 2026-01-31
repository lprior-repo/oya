# Intent Schema: #AntiPattern and #AIHints Types

## #AntiPattern Type

### Overview

The `#AntiPattern` type documents common mistakes and patterns to avoid when implementing an API. It helps developers learn what NOT to do.

### Definition

```cue
#AntiPattern: {
	name:        string
	description: string

	// What NOT to do
	bad_example: {...}

	// What TO do
	good_example: {...}

	// Explanation
	why: string | *""
}
```

### Required Fields

#### `name: string`
Name of the anti-pattern.
- Should be descriptive
- Examples: `"Status Code Misuse"`, `"Exposing Sensitive Data"`, `"Vague Error Messages"`

#### `description: string`
Description of what the anti-pattern is.
- Explains the mistake
- Examples:
  - `"Using HTTP 200 for error responses instead of appropriate error codes"`
  - `"Returning password hashes or API keys in response"`
  - `"Error messages that don't explain what went wrong"`

#### `bad_example: {...}`
Example showing the problematic pattern.
- Real response object showing what NOT to do
- Type is flexible (object, array, string, etc.)
- Should clearly demonstrate the problem

#### `good_example: {...}`
Example showing the correct pattern.
- Real response object showing the right way
- Same structure as bad_example but correct
- Should show recommended approach

### Optional Fields

#### `why: string` (default: `""`)
Explanation of why this is an anti-pattern.
- Explains the consequences
- Examples:
  - `"Clients can't distinguish success from failure if error returns 200"`
  - `"Exposed secrets allow unauthorized access to resources"`
  - `"Clients need specific error codes for proper error handling"`

## #AIHints Type

### Overview

The `#AIHints` type provides implementation guidance for AI systems generating code. It includes stack suggestions, security guidelines, and common pitfalls.

### Definition

```cue
#AIHints: {
	implementation?: {
		suggested_stack?: [...string]
	}

	entities?: [string]: #EntityHint

	security?: {
		password_hashing?: string
		jwt_algorithm?:    string
		jwt_expiry?:       string
		rate_limiting?:    string
	}

	pitfalls?: [...string]
}
```

### Optional Fields

#### `implementation?: { suggested_stack?: [...string] }`
Recommended technology stack for implementation.
- Array of technology names
- Helps AI generate code using appropriate tools
- Examples:
  ```cue
  implementation: {
    suggested_stack: [
      "Node.js"
      "Express"
      "PostgreSQL"
      "jsonwebtoken"
      "bcryptjs"
    ]
  }
  ```

#### `entities?: [string]: #EntityHint`
Entity/model definitions for implementation.
- Maps entity names to field definitions
- Helps AI generate database schemas and models
- Type: `#EntityHint` (defined below)
- Example:
  ```cue
  entities: {
    User: {
      fields: {
        id: "UUID, primary key"
        email: "string, unique, required"
        password_hash: "string, required"
        name: "string, required"
        created_at: "timestamp, default=now()"
      }
    }
  }
  ```

#### `security?: {...}`
Security implementation guidelines.

##### `password_hashing?: string`
How to hash passwords.
- Examples:
  - `"bcrypt with salt rounds >= 10"`
  - `"Argon2id with memory=19456, time=2, parallelism=1"`
  - `"PBKDF2-SHA256 with 600,000 iterations"`

##### `jwt_algorithm?: string`
JWT signing algorithm.
- Examples:
  - `"HS256 with strong secret (32+ bytes)"`
  - `"RS256 with RSA-2048 keypair"`
  - `"ES256 with ECDSA"`

##### `jwt_expiry?: string`
JWT token expiration times.
- Examples:
  - `"15 minutes for access token, 7 days for refresh token"`
  - `"1 hour for access token, 30 days for refresh token"`
  - `"No expiry for API keys, implement rotation"`

##### `rate_limiting?: string`
Rate limiting strategy.
- Examples:
  - `"100 requests per minute per IP"`
  - `"1000 requests per hour per user"`
  - `"Token bucket with capacity=100, refill rate=10/sec"`

#### `pitfalls?: [...string]`
Common mistakes to avoid.
- Array of specific warnings
- Helps AI avoid known issues
- Examples:
  - `"Don't store passwords in plain text"`
  - `"Don't hardcode secrets in code"`
  - `"Don't return user object with password field"`
  - `"Don't allow parallel login attempts from same account"`

## #EntityHint Type

### Definition

```cue
#EntityHint: {
	fields?: [string]: string
}
```

### Optional Fields

#### `fields?: [string]: string`
Entity field definitions.
- Maps field names to descriptions
- Descriptions can include type and constraints
- Helps AI generate database schema

## Complete Examples

### Anti-Pattern: Status Code Misuse

```cue
{
	name:        "Status Code Misuse"
	description: "Using HTTP 200 for error responses instead of appropriate error codes"
	bad_example: {
		status: 200
		body: {
			success: false
			error:   "Invalid credentials"
		}
	}
	good_example: {
		status: 401
		body: {
			error_code: "INVALID_CREDENTIALS"
			message:    "Invalid email or password"
		}
	}
	why: "HTTP 200 indicates success. Clients need correct status codes to handle errors properly. Error responses should use 4xx/5xx status codes."
}
```

### Anti-Pattern: Exposing Sensitive Data

```cue
{
	name:        "Exposing Sensitive Data"
	description: "Returning sensitive information like passwords or API keys in responses"
	bad_example: {
		id:        "user_123"
		email:     "user@example.com"
		password:  "MySecurePass123!"  // ❌ NEVER return plaintext password
		api_key:   "sk_live_abc123"    // ❌ NEVER return API key
	}
	good_example: {
		id:    "user_123"
		email: "user@example.com"
		name:  "John Doe"
		// Password, API keys, and secrets are NOT included
	}
	why: "Returning sensitive data allows attackers to compromise accounts or access restricted resources. Only return data users need to see."
}
```

### Anti-Pattern: Vague Error Messages

```cue
{
	name:        "Vague Error Messages"
	description: "Error messages that don't explain what went wrong"
	bad_example: {
		status: 400
		body: {
			error: "Invalid request"
		}
	}
	good_example: {
		status: 400
		body: {
			error_code: "VALIDATION_ERROR"
			message:    "Email format is invalid"
			fields: {
				email: "Must be a valid email address (e.g., user@example.com)"
			}
		}
	}
	why: "Vague errors don't help clients understand what went wrong. Include specific error codes and field-level validation messages."
}
```

### Anti-Pattern: No Pagination

```cue
{
	name:        "Missing Pagination"
	description: "Returning all results without pagination, causing performance issues"
	bad_example: {
		status: 200
		body: [
			// 100,000 items - client must load all at once!
			{ id: "1", name: "Item 1" }
			// ... thousands more ...
		]
	}
	good_example: {
		status: 200
		body: {
			data: [
				{ id: "1", name: "Item 1" }
				{ id: "2", name: "Item 2" }
				// ... only 20 items ...
			]
			pagination: {
				limit:  20
				offset: 0
				total:  100000
			}
		}
	}
	why: "Returning all results causes memory issues and slow responses. Use pagination with limit/offset or cursor-based pagination."
}
```

### AI Hints: User Management API

```cue
{
	implementation: {
		suggested_stack: [
			"Node.js/Express"
			"PostgreSQL"
			"jsonwebtoken"
			"bcryptjs"
			"joi (validation)"
		]
	}

	entities: {
		User: {
			fields: {
				id:            "UUID, primary key, auto-generated"
				email:         "string, unique, required, max 255 chars"
				name:          "string, required, max 255 chars"
				password_hash: "string, required (never return)"
				created_at:    "timestamp, default=now()"
				updated_at:    "timestamp, default=now()"
				deleted_at:    "timestamp nullable for soft deletes"
			}
		}

		RefreshToken: {
			fields: {
				id:        "UUID, primary key"
				user_id:   "UUID, foreign key to User"
				token:     "string, hashed, unique"
				expires_at: "timestamp"
				created_at: "timestamp"
			}
		}
	}

	security: {
		password_hashing: "bcryptjs with salt rounds = 10"
		jwt_algorithm:    "HS256 with secret >= 32 bytes OR RS256 with RSA-2048"
		jwt_expiry:       "Access token: 15 minutes, Refresh token: 7 days"
		rate_limiting:    "100 login attempts per 15 minutes per IP"
	}

	pitfalls: [
		"Don't store plaintext passwords"
		"Don't return password_hash in user responses"
		"Don't hardcode JWT secret in code"
		"Don't allow unlimited password reset attempts"
		"Don't return refresh tokens in access token response"
		"Don't accept tokens after user deletes account"
		"Don't allow concurrent refresh token usage"
	]
}
```

### AI Hints: Payment Processing API

```cue
{
	implementation: {
		suggested_stack: [
			"Node.js/Go"
			"PostgreSQL"
			"Stripe API (payment processor)"
			"Bull (job queue)"
			"Redis (session store)"
		]
	}

	entities: {
		Payment: {
			fields: {
				id:          "UUID, primary key"
				user_id:     "UUID, foreign key"
				amount:      "decimal(19,2), required"
				currency:    "string, default=USD"
				status:      "enum: pending, succeeded, failed, refunded"
				stripe_id:   "string, stripe payment ID"
				created_at:  "timestamp"
				updated_at:  "timestamp"
			}
		}

		Refund: {
			fields: {
				id:         "UUID, primary key"
				payment_id: "UUID, foreign key"
				amount:     "decimal(19,2), required"
				reason:     "string, max 255"
				status:     "enum: pending, succeeded, failed"
				stripe_id:  "string, stripe refund ID"
				created_at: "timestamp"
			}
		}
	}

	security: {
		password_hashing: "N/A - external payment processor"
		jwt_algorithm:    "RS256 with RSA-2048 for API keys"
		jwt_expiry:       "API keys: no expiry, implement rotation"
		rate_limiting:    "50 payments per minute per user"
	}

	pitfalls: [
		"Don't store credit card data directly - use Stripe tokenization"
		"Don't return full card number in responses"
		"Don't log full payment amounts (log masked amounts)"
		"Don't retry failed payments automatically without user consent"
		"Don't process same payment ID twice (implement idempotency)"
		"Don't expose internal payment IDs to clients"
		"Don't allow refunds beyond 90 days without approval"
	]
}
```

## Using Anti-Patterns in Specifications

Anti-patterns are included in specs for documentation:

```cue
spec: intent.#Spec & {
	name: "User API"
	// ... other fields ...

	anti_patterns: [
		{
			name: "Status Code Misuse"
			// ...
		},
		{
			name: "Exposing Sensitive Data"
			// ...
		},
	]
}
```

## Using AI Hints in Specifications

AI hints guide implementation:

```cue
spec: intent.#Spec & {
	name: "User Management API"
	// ... other fields ...

	ai_hints: {
		implementation: {
			suggested_stack: ["Node.js", "Express", "PostgreSQL"]
		}
		entities: {
			User: {
				fields: {
					id: "UUID"
					email: "string, unique"
					password_hash: "string"
				}
			}
		}
		security: {
			password_hashing: "bcryptjs with salt rounds >= 10"
			jwt_algorithm: "RS256"
			rate_limiting: "100 requests per minute"
		}
		pitfalls: [
			"Don't store plaintext passwords"
			"Don't return password hashes"
		]
	}
}
```

## Best Practices

### ✅ Do

- **Show both good and bad** - Anti-pattern examples must have both
- **Be specific** - Explain exactly what's wrong
- **Include consequences** - Explain why it matters
- **Provide solutions** - Good examples show the fix
- **Cover security** - Anti-patterns should include security issues
- **Document stack** - List actual technologies you use
- **List real pitfalls** - Warn about actual mistakes you've seen

### ❌ Don't

- **Be vague** - Be specific about what's wrong
- **Skip the why** - Explain consequences
- **Mix patterns** - One anti-pattern per entry
- **Assume AI knowledge** - Be explicit about pitfalls
- **Skip security** - Always include security anti-patterns
- **List theoretical stack** - Use technologies you actually use

## Integration with Spec

Both types are part of the overall spec:

```cue
spec: intent.#Spec & {
	name:        "Example API"
	description: "..."

	// ... features, rules ...

	anti_patterns: [
		{ name: "...", ... }
	]

	ai_hints: {
		implementation: { ... }
		entities: { ... }
		security: { ... }
		pitfalls: [ ... ]
	}
}
```

## See Also

- [`#Spec`](./schema-spec-type.md) - Top-level spec containing anti-patterns and hints
- [`#Rule`](./schema-rule-types.md) - Global validation rules
