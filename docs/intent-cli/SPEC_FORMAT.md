# CUE Specification Format Reference

Complete reference for the Intent CUE specification format.

## Top-Level Structure

```cue
package api

spec: {
    // Basic metadata
    name: String
    description: String
    audience: String
    version: String
    success_criteria: [String]

    // Configuration
    config: Config

    // Test definitions
    features: [Feature]

    // Global validation
    rules: [Rule]
    anti_patterns: [AntiPattern]

    // Implementation hints
    ai_hints: AIHints
}
```

## Config

Global configuration for all requests.

```cue
config: {
    base_url: "http://localhost:8080"
    timeout_ms: 5000
    headers: {
        "Content-Type": "application/json"
        "Authorization": "Bearer token"
    }
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `base_url` | String | Base URL for all requests (required) |
| `timeout_ms` | Int | Request timeout in milliseconds (required) |
| `headers` | Dict | Default headers merged with request headers (required) |

## Feature

A logical grouping of related behaviors.

```cue
{
    name: "User Management"
    description: "User CRUD operations"
    behaviors: [Behavior]
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | String | Feature name (required) |
| `description` | String | Feature description (required) |
| `behaviors` | [Behavior] | List of behaviors in this feature (required) |

## Behavior

A single test case - one request-response pair.

```cue
{
    name: "create-user"
    intent: "Create a new user with valid data"
    notes: "User email must be unique"
    requires: ["setup-users"]
    tags: ["happy-path", "create"]
    request: Request
    response: Response
    captures: { user_id: "id" }
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | String | Unique behavior name (required) |
| `intent` | String | What this behavior tests (required) |
| `notes` | String | Additional notes (required, can be empty) |
| `requires` | [String] | Behavior names this depends on (required, can be empty) |
| `tags` | [String] | Tags for filtering (required, can be empty) |
| `request` | Request | Request definition (required) |
| `response` | Response | Expected response (required) |
| `captures` | Dict | Values to capture from response (required, can be empty) |

## Request

HTTP request definition.

```cue
request: {
    method: "POST"
    path: "/users/${user_id}"
    headers: {
        "X-API-Key": "secret"
    }
    query: {
        "page": "1"
        "size": "10"
    }
    body: {
        name: "John"
        email: "john@example.com"
    }
}
```

### Methods

- `"GET"` - Retrieve resource
- `"POST"` - Create resource
- `"PUT"` - Replace resource
- `"PATCH"` - Partial update
- `"DELETE"` - Delete resource
- `"HEAD"` - Like GET but no body
- `"OPTIONS"` - Query available methods

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `method` | String | HTTP method (required) |
| `path` | String | Request path, may include variables (required) |
| `headers` | Dict | Request headers (required) |
| `query` | Dict | Query parameters (required) |
| `body` | Json | Request body (required) |

### Variable Interpolation

Use captured values from previous behaviors:

```cue
path: "/users/${user_id}/items/${item_id}"
```

Variables are available from:
- Captures from dependent behaviors
- Response bodies and fields

## Response

Expected response definition.

```cue
response: {
    status: 200
    example: {
        id: "123"
        name: "John"
        created_at: "2024-01-04T12:00:00Z"
    }
    checks: {
        "id": { rule: "is uuid", why: "..." }
        "email": { rule: "is email", why: "..." }
    }
    headers: {
        "Content-Type": "application/json"
    }
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `status` | Int | Expected HTTP status code (required) |
| `example` | Json | Example response body (required) |
| `checks` | Dict | Field-level validation rules (required) |
| `headers` | Dict | Expected response headers (required) |

### Check Rules

Field validation rules. Use field paths with dot notation:

```cue
checks: {
    "id": { rule: "is uuid" }
    "user.id": { rule: "is uuid" }
    "items[0].id": { rule: "is uuid" }
}
```

#### Available Rules

| Rule | Description | Example |
|------|-------------|---------|
| `is uuid` | Valid UUID v4 | `"550e8400-e29b-41d4-a716-446655440000"` |
| `is email` | Valid email | `"user@example.com"` |
| `is url` | Valid URL | `"https://example.com"` |
| `is iso8601` | ISO8601 timestamp | `"2024-01-04T12:00:00Z"` |
| `is json` | Valid JSON | Any valid JSON value |
| `matches <regex>` | Regex pattern | `matches "^[0-9]+$"` |
| `equals <value>` | Exact value | `equals "active"` |
| `length <n>` | String/array length | `length 36` |
| `min_length <n>` | Minimum length | `min_length 8` |
| `max_length <n>` | Maximum length | `max_length 255` |
| `is integer` | Integer value | `123` |
| `is number` | Number (int or float) | `123.45` |
| `is string` | String value | `"text"` |
| `is boolean` | Boolean value | `true` |
| `is array` | Array value | `[]` |
| `is object` | Object value | `{}` |
| `is null` | Null value | `null` |

## Rule

Global rule applying to multiple behaviors.

```cue
{
    name: "no-exposed-secrets"
    description: "Responses should not expose secrets"
    when: {
        status: ">= 200"
        method: "GET"
        path: "/users.*"
    }
    check: {
        body_must_not_contain: ["password", "secret"]
        body_must_contain: []
        fields_must_exist: ["id"]
        fields_must_not_exist: []
        header_must_exist: ""
        header_must_not_exist: ""
    }
    example: {
        error: "Secrets exposed"
    }
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | String | Rule name (required) |
| `description` | String | What the rule validates (required) |
| `when` | When | Conditions for rule application (required) |
| `check` | RuleCheck | What to check (required) |
| `example` | Json | Example of violation (required) |

### When Conditions

```cue
when: {
    status: ">= 200"     // Status condition
    method: "GET"        // HTTP method
    path: "/users.*"     // Path regex
}
```

All conditions must match for the rule to apply.

#### Status Conditions

- `"200"` - Exact status
- `">= 200"` - Greater than or equal
- `"> 200"` - Greater than
- `"<= 299"` - Less than or equal
- `"< 300"` - Less than

#### Path Patterns

Paths are treated as regex patterns:
- `/users.*` - Matches `/users`, `/users/123`, `/users/123/items`
- `/items/[0-9]+` - Matches `/items/123` but not `/items/abc`
- `/api/v[12]/users` - Matches `/api/v1/users` and `/api/v2/users`

### RuleCheck

```cue
check: {
    body_must_not_contain: ["password", "secret"]
    body_must_contain: []
    fields_must_exist: ["id", "created_at"]
    fields_must_not_exist: ["internal_notes"]
    header_must_exist: "Content-Type"
    header_must_not_exist: ""
}
```

All fields are required but can be empty lists/strings.

## AntiPattern

Detect common mistakes in API responses.

```cue
{
    name: "missing-timestamps"
    description: "Responses should include created_at and updated_at"
    bad_example: {
        id: "123"
        name: "Product"
    }
    good_example: {
        id: "123"
        name: "Product"
        created_at: "2024-01-04T12:00:00Z"
        updated_at: "2024-01-04T12:00:00Z"
    }
    why: "Timestamps are essential for auditing"
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | String | Pattern name (required) |
| `description` | String | Pattern description (required) |
| `bad_example` | Json | Example showing the anti-pattern (required) |
| `good_example` | Json | Example showing correct implementation (required) |
| `why` | String | Explanation of why it matters (required) |

## AIHints

Guidance for AI-powered implementation.

```cue
ai_hints: {
    implementation: {
        suggested_stack: ["PostgreSQL", "Express.js", "Node.js"]
    }
    entities: {
        User: {
            fields: {
                id: "UUID primary key"
                name: "User full name (required)"
                email: "User email (required, unique)"
                created_at: "ISO8601 timestamp"
                updated_at: "ISO8601 timestamp"
            }
        }
        Item: {
            fields: {
                id: "UUID primary key"
                user_id: "Foreign key to User"
                title: "Item title (required)"
                created_at: "ISO8601 timestamp"
            }
        }
    }
    security: {
        password_hashing: "Use bcrypt with >= 10 rounds"
        jwt_algorithm: "HS256 or RS256"
        jwt_expiry: "15-30 minutes for access tokens"
        rate_limiting: "100 requests per minute per IP"
    }
    pitfalls: [
        "Never return passwords in responses",
        "Always validate input on server",
        "Use HTTPS in production",
        "Implement proper error handling",
        "Don't expose internal errors to clients"
    ]
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `implementation` | ImplementationHints | Stack recommendations (required) |
| `entities` | Dict | Entity/model definitions (required) |
| `security` | SecurityHints | Security best practices (required) |
| `pitfalls` | [String] | Common mistakes to avoid (required) |

## Complete Example

```cue
package api

spec: {
    name: "Item API"
    description: "API for managing items"
    audience: "Mobile and web clients"
    version: "1.0.0"

    config: {
        base_url: "https://api.example.com"
        timeout_ms: 5000
        headers: {
            "Content-Type": "application/json"
            "Accept": "application/json"
        }
    }

    features: [
        {
            name: "Item Management"
            description: "Create, read, update, and delete items"
            behaviors: [
                {
                    name: "create-item"
                    intent: "Create a new item with valid data"
                    notes: "Item names must be unique per user"
                    requires: []
                    tags: ["create", "happy-path"]
                    request: {
                        method: "POST"
                        path: "/items"
                        headers: {}
                        query: {}
                        body: {
                            title: "Test Item"
                            description: "A test item"
                            category: "test"
                        }
                    }
                    response: {
                        status: 201
                        example: {
                            id: "item-123"
                            user_id: "user-456"
                            title: "Test Item"
                            description: "A test item"
                            category: "test"
                            created_at: "2024-01-04T12:00:00Z"
                            updated_at: "2024-01-04T12:00:00Z"
                        }
                        checks: {
                            "id": { rule: "is uuid", why: "IDs are UUIDs" }
                            "created_at": { rule: "is iso8601", why: "Timestamps are ISO8601" }
                        }
                        headers: {
                            "Content-Type": "application/json"
                        }
                    }
                    captures: {
                        item_id: "id"
                    }
                }
                {
                    name: "get-item"
                    intent: "Retrieve a specific item"
                    notes: ""
                    requires: ["create-item"]
                    tags: ["read", "happy-path"]
                    request: {
                        method: "GET"
                        path: "/items/${item_id}"
                        headers: {}
                        query: {}
                        body: null
                    }
                    response: {
                        status: 200
                        example: {
                            id: "item-123"
                            title: "Test Item"
                            created_at: "2024-01-04T12:00:00Z"
                        }
                        checks: {
                            "id": { rule: "equals item-123", why: "Should return the requested item" }
                        }
                        headers: {}
                    }
                    captures: {}
                }
            ]
        }
    ]

    rules: [
        {
            name: "no-internal-fields"
            description: "Responses should not expose internal fields"
            when: {
                status: ">= 200"
                method: "GET"
                path: "/items.*"
            }
            check: {
                body_must_not_contain: []
                body_must_contain: []
                fields_must_exist: ["id", "created_at"]
                fields_must_not_exist: ["internal_id", "_version"]
                header_must_exist: "Content-Type"
                header_must_not_exist: ""
            }
            example: {
                error: "Internal fields exposed"
            }
        }
    ]

    anti_patterns: [
        {
            name: "missing-timestamps"
            description: "All items should have timestamps"
            bad_example: { id: "123", title: "Item" }
            good_example: {
                id: "123"
                title: "Item"
                created_at: "2024-01-04T12:00:00Z"
                updated_at: "2024-01-04T12:00:00Z"
            }
            why: "Timestamps enable auditing and caching"
        }
    ]

    ai_hints: {
        implementation: {
            suggested_stack: ["PostgreSQL", "Python/FastAPI", "Docker"]
        }
        entities: {
            Item: {
                fields: {
                    id: "UUID primary key"
                    user_id: "UUID foreign key to User"
                    title: "String (100 chars max, required)"
                    description: "String (500 chars max)"
                    category: "String (required)"
                    created_at: "ISO8601 timestamp"
                    updated_at: "ISO8601 timestamp"
                }
            }
        }
        security: {
            password_hashing: "Use argon2"
            jwt_algorithm: "HS256"
            jwt_expiry: "1 hour"
            rate_limiting: "100 requests/min per user"
        }
        pitfalls: [
            "Ensure items belong to the requesting user",
            "Validate category values",
            "Sanitize title and description",
            "Use transactions for multi-step operations"
        ]
    }

    success_criteria: [
        "All behaviors pass"
        "No rule violations"
        "Response times < 200ms"
        "No anti-patterns detected"
    ]
}
```

## Tips for Writing Specifications

1. **Use descriptive names** - Names should clearly indicate what's being tested
2. **Keep checks focused** - Check specific important fields, not everything
3. **Document the why** - Explain why each rule exists
4. **Test error cases** - Don't only test happy paths
5. **Use realistic examples** - Examples should match real API responses
6. **Keep it maintainable** - Avoid overly complex checks or patterns
