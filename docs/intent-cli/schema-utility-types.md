# Intent Schema: Utility Types

This document covers the supporting types used throughout the Intent schema.

## #Method Type

### Definition

```cue
#Method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS"
```

### Overview

`#Method` is an enumeration of valid HTTP request methods.

### Valid Values

| Method | Purpose | Has Body | Safe* | Idempotent* |
|--------|---------|----------|-------|------------|
| GET | Retrieve data | No | Yes | Yes |
| POST | Create resource | Yes | No | No |
| PUT | Replace entire resource | Yes | No | Yes |
| PATCH | Partial update | Yes | No | Usually |
| DELETE | Remove resource | No | No | Yes |
| HEAD | Like GET, no body | No | Yes | Yes |
| OPTIONS | Check allowed methods | No | Yes | Yes |

*Safe = doesn't modify server state
*Idempotent = same request multiple times = same result

### Usage in Requests

```cue
request: {
	method: "GET"      // or POST, PUT, PATCH, DELETE, HEAD, OPTIONS
	path:   "/users"
	// ...
}
```

### Common Patterns

**Reading data:**
```cue
method: "GET"
path: "/users/123"
```

**Creating resource:**
```cue
method: "POST"
path: "/users"
body: { name: "John", email: "john@example.com" }
```

**Full update:**
```cue
method: "PUT"
path: "/users/123"
body: { name: "Jane", email: "jane@example.com", age: 30 }
```

**Partial update:**
```cue
method: "PATCH"
path: "/users/123"
body: { name: "Jane" }  // Only update name
```

**Delete:**
```cue
method: "DELETE"
path: "/users/123"
```

---

## #Identifier Type

### Definition

```cue
#Identifier: =~"^[a-z][a-z0-9_-]*$"
```

### Overview

`#Identifier` is a regex pattern for valid identifier strings. Used for:
- Behavior names
- Capture names
- Field identifiers

### Pattern Breakdown

- `^` - Start of string
- `[a-z]` - First character must be lowercase letter
- `[a-z0-9_-]*` - Followed by 0 or more lowercase letters, digits, underscores, or hyphens
- `$` - End of string

### Valid Examples

✅ Valid identifiers:
- `"create_user"` - Lowercase with underscore
- `"list-products"` - Lowercase with hyphen
- `"get_user_by_id_123"` - Lowercase with numbers
- `"validate"` - Simple name
- `"a"` - Single character
- `"a1"` - Letter and digit
- `"api_v2_users"` - Complex name

### Invalid Examples

❌ Invalid identifiers:
- `"CreateUser"` - Capital letters not allowed
- `"create user"` - Spaces not allowed
- `"_create"` - Can't start with underscore
- `"123create"` - Can't start with digit
- `"create.user"` - Dots not allowed
- `"create user"` - Spaces not allowed
- `"create@user"` - Special characters not allowed

### Usage in Behaviors

```cue
behaviors: [{
	name: "create_user"      // Valid identifier
	request: { ... }
	response: { ... }
	captures: {
		user_id: "id"        // Capture name is identifier
		auth_token: "token"  // Valid identifier
	}
	requires: ["create_user"]  // Refers to behavior by identifier
}]
```

### Best Practices

- Use lowercase letters
- Use underscores for word separation (snake_case)
- Be descriptive but concise
- Match behavior action (create_*, get_*, list_*, delete_*, update_*)

---

## #Headers Type

### Definition

```cue
#Headers: [string]: string
```

### Overview

`#Headers` is a map of HTTP header names to values. Used in:
- Config global headers
- Request headers
- Response header validation

### Examples

### Config Global Headers

```cue
config: {
	headers: {
		"Content-Type": "application/json"
		"Accept": "application/json"
		"User-Agent": "Intent-CLI/2.0"
	}
}
```

### Request Headers

```cue
request: {
	headers: {
		"Authorization": "Bearer eyJhbGc..."
		"Content-Type": "application/json"
		"X-Request-ID": "req_123"
	}
}
```

### Response Headers

```cue
response: {
	headers: {
		"Content-Type": "application/json"
		"X-Rate-Limit-Limit": "1000"
		"Cache-Control": "public, max-age=300"
	}
}
```

### Common Headers

| Header | Purpose | Example |
|--------|---------|---------|
| `Content-Type` | Request/response body format | `"application/json"` |
| `Accept` | Expected response format | `"application/json"` |
| `Authorization` | Authentication credentials | `"Bearer token"` |
| `User-Agent` | Client identification | `"Intent-CLI/2.0"` |
| `X-API-Key` | API key authentication | `"sk_prod_..."` |
| `X-Request-ID` | Request tracking | `"req_123"` |
| `X-Rate-Limit-Limit` | Rate limit cap | `"1000"` |
| `X-Rate-Limit-Remaining` | Remaining requests | `"999"` |
| `Cache-Control` | Caching directive | `"public, max-age=300"` |
| `CORS` headers | Cross-origin access | Various |

### Header Merging

Headers are merged with precedence:
1. Config headers (applied to all)
2. Request headers (override config)
3. Variable values (override headers)

Example:
```cue
config: {
	headers: {
		"Authorization": "Bearer default"
		"Accept": "application/json"
	}
}

request: {
	headers: {
		"Authorization": "Bearer specific"  // Overrides config
	}
	// "Accept" inherited from config
}
```

Result:
```
Authorization: Bearer specific
Accept: application/json
```

### Variable Interpolation

Headers support variable interpolation:

```cue
request: {
	headers: {
		"Authorization": "Bearer ${auth_token}"
		"X-User-ID": "${user_id}"
	}
}
```

---

## #Captures Type

### Definition

```cue
#Captures: [#Identifier]: string
```

### Overview

`#Captures` is a map of capture names to field paths. Used to extract values from responses for use in dependent behaviors.

### Examples

### Simple Field Capture

```cue
captures: {
	user_id: "id"              // Extract 'id' field
	auth_token: "token"        // Extract 'token' field
}
```

### Nested Field Capture

```cue
captures: {
	order_id: "order.id"       // Extract nested 'order.id'
	product_name: "items[0].product.name"  // Extract from array
}
```

### Complete Example

```cue
{
	name: "create_user"
	intent: "Create a new user"
	request: {
		method: "POST"
		path: "/users"
		body: { name: "John", email: "john@example.com" }
	}
	response: {
		status: 201
		example: {
			id: "user_123"
			token: "eyJhbGc..."
			email: "john@example.com"
		}
		checks: { ... }
	}
	captures: {
		user_id: "id"      // Extract 'id' and name it 'user_id'
		auth_token: "token" // Extract 'token' and name it 'auth_token'
	}
}
```

### Using Captured Values

Captured values are available to dependent behaviors:

```cue
{
	name: "get_user_profile"
	requires: ["create_user"]  // Depends on create_user
	request: {
		path: "/users/${user_id}"  // Uses captured user_id
		headers: {
			"Authorization": "Bearer ${auth_token}"  // Uses captured token
		}
	}
	response: { ... }
}
```

### Best Practices

- Only capture values needed by dependents
- Use descriptive capture names
- Nest captures logically (user_id, order_id, etc.)
- Don't capture sensitive data unnecessarily

---

## #Check Type

### Definition

```cue
#Check: {
	rule: string           // Human-readable rule string
	why:  string | *""     // Explanation of why this matters
}
```

### Overview

`#Check` is a single validation rule for a response field.

### Fields

#### `rule: string`
The validation rule.
- Human-readable description of what to check
- Examples:
  - `"must be non-empty string"`
  - `"must match email format"`
  - `"must be positive integer"`
  - `"must equal 'active'"`
  - `"must be array with at least 1 element"`

#### `why: string`
Explanation of why this check matters.
- Provides context for the validation
- Examples:
  - `"ID is required for resource tracking"`
  - `"Email must be valid for communication"`
  - `"Amount must be positive for financial integrity"`
  - `"Status must be valid state"`

### Examples

```cue
checks: {
	"id": {
		rule: "must be non-empty string"
		why:  "User ID is required for tracking"
	}
	"email": {
		rule: "must match email format"
		why:  "Email must be valid for communication"
	}
	"age": {
		rule: "must be integer between 0 and 150"
		why:  "Age has physical constraints"
	}
}
```

---

## #Checks Type

### Definition

```cue
#Checks: [string]: #Check
```

### Overview

`#Checks` is a map of response field paths to check definitions.

### Field Paths

Field paths use dot notation:
- `"id"` - Top-level field
- `"user.id"` - Nested field
- `"items[0].name"` - Array element
- `"items[].id"` - All array elements
- `"nested.deep.value"` - Deeply nested

### Examples

### Simple Checks

```cue
checks: {
	"id": { rule: "must be string", why: "..." }
	"name": { rule: "must be string", why: "..." }
}
```

### Nested Checks

```cue
checks: {
	"user.id": { rule: "must be string", why: "..." }
	"user.email": { rule: "must be valid email", why: "..." }
	"user.profile.age": { rule: "must be positive integer", why: "..." }
}
```

### Array Checks

```cue
checks: {
	"items[0].id": { rule: "must be string", why: "..." }
	"items[].name": { rule: "must be non-empty", why: "..." }
}
```

---

## #EntityHint Type

### Definition

```cue
#EntityHint: {
	fields?: [string]: string
}
```

### Overview

`#EntityHint` provides database schema hints for an entity.

### Fields

#### `fields?: [string]: string`
Maps field names to descriptions.
- Descriptions can include type and constraints
- Used by AI to generate database schema
- Examples:
  ```cue
  fields: {
    id: "UUID, primary key, auto-generated"
    email: "string, unique, required, max 255 chars"
    name: "string, required, max 255 chars"
    created_at: "timestamp, default=now()"
  }
  ```

### Examples

### User Entity

```cue
User: {
	fields: {
		id: "UUID, primary key"
		email: "string, unique, required"
		password_hash: "string, required"
		name: "string, required"
		created_at: "timestamp, default=now()"
		updated_at: "timestamp, default=now()"
	}
}
```

### Order Entity

```cue
Order: {
	fields: {
		id: "UUID, primary key"
		user_id: "UUID, foreign key to User"
		status: "enum: pending, confirmed, shipped, delivered"
		total_amount: "decimal(19,2), required"
		created_at: "timestamp, default=now()"
	}
}
```

---

## Type Relationships

```
#Spec
├── #Config
│   └── #Headers
├── #Feature
│   └── #Behavior
│       ├── #Request
│       │   ├── #Method
│       │   └── #Headers
│       ├── #Response
│       │   ├── #Checks
│       │   │   └── #Check
│       │   └── #Headers
│       ├── #Captures
│       ├── #Identifier (name, requires)
│       └── tags
├── #Rule
│   ├── #When
│   └── #RuleCheck
├── #AntiPattern
└── #AIHints
    └── #EntityHint
```

---

## Summary Table

| Type | Purpose | Example |
|------|---------|---------|
| `#Method` | HTTP method | `"GET"`, `"POST"` |
| `#Identifier` | Valid identifier | `"create_user"` |
| `#Headers` | HTTP headers map | `{ "Content-Type": "application/json" }` |
| `#Captures` | Extract response values | `{ user_id: "id" }` |
| `#Check` | Single validation | `{ rule: "...", why: "..." }` |
| `#Checks` | Response validations | `{ "id": {...}, "email": {...} }` |
| `#EntityHint` | Database schema hint | `{ fields: { id: "UUID" } }` |

## See Also

- [`#Request`](./schema-request-type.md) - Uses #Method and #Headers
- [`#Response`](./schema-response-type.md) - Uses #Checks and #Headers
- [`#Behavior`](./schema-behavior-type.md) - Uses #Identifier and #Captures
- [`#AIHints`](./schema-antipattern-aihints.md) - Uses #EntityHint
