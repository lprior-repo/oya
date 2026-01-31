# Intent CLI User Guide

This comprehensive guide covers everything you need to use Intent effectively for AI-guided planning and rigorous requirement decomposition.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Writing Specifications](#writing-specifications)
3. [Running Tests](#running-tests)
4. [Understanding Results](#understanding-results)
5. [Advanced Features](#advanced-features)
6. [Best Practices](#best-practices)
7. [Common Patterns](#common-patterns)
8. [Troubleshooting](#troubleshooting)

## Getting Started

### Basic Workflow

The typical Intent workflow is:

1. **Create a specification** - Write a CUE file describing your API
2. **Start your API server** - Ensure your API is running
3. **Run Intent** - Execute the specification against your API
4. **Review results** - Analyze test results and fix any failures
5. **Iterate** - Improve your spec and API until all tests pass

### Your First Specification

Create a file named `api.cue`:

```cue
package api

spec: {
    name: "My API"
    description: "API for managing items"
    audience: "API consumers"
    version: "1.0.0"

    config: {
        base_url: "http://localhost:8080"
        timeout_ms: 5000
        headers: {
            "Content-Type": "application/json"
        }
    }

    features: [{
        name: "Item Management"
        description: "Create and retrieve items"
        behaviors: [{
            name: "list-items"
            intent: "Retrieve all items"
            request: {
                method: "GET"
                path: "/items"
                headers: {}
                query: {}
                body: null
            }
            response: {
                status: 200
                example: [{
                    id: "item-1"
                    name: "Test Item"
                    created_at: "2024-01-04T12:00:00Z"
                }]
                checks: {}
                headers: {}
            }
            captures: {}
        }]
    }]

    rules: []
    anti_patterns: []
    success_criteria: ["All behaviors pass"]
    ai_hints: {
        implementation: { suggested_stack: [] }
        entities: {}
        security: {
            password_hashing: ""
            jwt_algorithm: ""
            jwt_expiry: ""
            rate_limiting: ""
        }
        pitfalls: []
    }
}
```

### Running Your First Test

```bash
# Start your API server in one terminal
npm start  # or however you start your API

# In another terminal, run Intent
gleam run -- check api.cue --target http://localhost:8080
```

You should see output like:

```
Running 1 behaviors...

PASS
Passed: 1 / Failed: 0 / Blocked: 0 / Total: 1
All behaviors passed
```

## Writing Specifications

### Specification Structure

Every Intent specification has this structure:

```cue
spec: {
    name: String                    // Name of the API
    description: String             // What this API does
    audience: String                // Who uses it
    version: String                 // Version (semantic versioning)
    success_criteria: [String]      // What success looks like
    config: Config                  // Global configuration
    features: [Feature]             // Groups of related behaviors
    rules: [Rule]                   // Global validation rules
    anti_patterns: [AntiPattern]   // Common mistakes to detect
    ai_hints: AIHints              // Hints for AI implementation
}
```

### Configuration

The `config` section sets defaults for all requests:

```cue
config: {
    base_url: "http://localhost:8080"           // Base URL for all requests
    timeout_ms: 5000                             // Request timeout in milliseconds
    headers: {                                   // Default headers
        "Content-Type": "application/json"
        "Authorization": "Bearer token"
    }
}
```

### Features

Features group related behaviors:

```cue
features: [{
    name: "User Management"
    description: "User CRUD operations"
    behaviors: [
        // Behaviors for user management
    ]
}]
```

### Behaviors

A behavior is a single test case:

```cue
{
    name: "create-user"                              // Unique name
    intent: "Create a new user with valid email"     // What we're testing
    notes: "User email must be unique"               // Additional notes
    requires: ["setup-database"]                     // Dependencies
    tags: ["happy-path", "create"]                   // Tags for filtering

    request: {
        method: "POST"
        path: "/users"
        headers: {
            "X-API-Key": "secret-key"
        }
        query: {
            "notify": "true"
        }
        body: {
            name: "John Doe"
            email: "john@example.com"
        }
    }

    response: {
        status: 201
        example: {
            id: "user-123"
            name: "John Doe"
            email: "john@example.com"
            created_at: "2024-01-04T12:00:00Z"
        }
        checks: {
            "id": {
                rule: "is uuid"
                why: "User IDs are UUIDs"
            }
            "created_at": {
                rule: "is iso8601"
                why: "Timestamps are ISO8601 format"
            }
        }
        headers: {
            "Content-Type": "application/json"
        }
    }

    captures: {
        user_id: "id"
        created_at: "created_at"
    }
}
```

### Request Fields

- **method** - HTTP method (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS)
- **path** - Request path (can include variables: `/users/${user_id}`)
- **headers** - Request headers (merged with config headers)
- **query** - Query parameters
- **body** - Request body (JSON)

### Response Fields

- **status** - Expected HTTP status code
- **example** - Example response body
- **checks** - Field-level validation rules
- **headers** - Expected response headers

### Response Checks

Validate specific fields in the response:

```cue
checks: {
    "user.id": {
        rule: "is uuid"
        why: "User IDs must be valid UUIDs"
    }
    "user.email": {
        rule: "matches ^[^@]+@[^@]+\\.[^@]+$"
        why: "Email must be valid format"
    }
    "created_at": {
        rule: "is iso8601"
        why: "Must be ISO8601 timestamp"
    }
}
```

Available rules:
- `is uuid` - Valid UUID
- `is iso8601` - ISO8601 timestamp
- `is email` - Valid email
- `is url` - Valid URL
- `matches <regex>` - Regex match
- `equals <value>` - Exact match
- `length <n>` - Exact length
- And many more...

## Running Tests

### Basic Execution

```bash
gleam run -- check spec.cue --target http://localhost:8080
```

### Command Options

```bash
# Verbose output with detailed information
gleam run -- check spec.cue --target http://localhost:8080 --verbose

# Filter to specific feature
gleam run -- check spec.cue --target http://localhost:8080 --feature "User Management"

# Filter to specific behavior
gleam run -- check spec.cue --target http://localhost:8080 --behavior "create-user"

# Output as JSON
gleam run -- check spec.cue --target http://localhost:8080 --output json

# Short flags
gleam run -- check spec.cue -t http://localhost:8080 -v -f "Users" -o json
```

### Running Multiple Specs

```bash
# Run multiple specification files
gleam run -- check spec1.cue --target http://localhost:8080
gleam run -- check spec2.cue --target http://localhost:8080

# Or use a script to run all specs in a directory
for spec in specs/*.cue; do
    gleam run -- check "$spec" --target http://localhost:8080
done
```

## Understanding Results

### Success Output

When all tests pass:

```
PASS
Passed: 5 / Failed: 0 / Blocked: 0 / Total: 5
All 5 behaviors passed
```

### Failure Output

When tests fail:

```
FAIL
Passed: 3 / Failed: 2 / Blocked: 0 / Total: 5
2 failures, 0 blocked out of 5 behaviors

FAILURES:

[User Management] create-user
Intent: Create a new user with valid data
Problems:
  - status: HTTP status code mismatch
    Expected: 201
    Actual: 200
  - id: Value mismatch
    Expected: uuid
    Actual: not-a-uuid
Request: POST /users
Response: 200
Hint: Check that the API is returning the correct status code
```

### Result Interpretation

- **PASS** - All behaviors passed and no rule violations
- **FAIL** - One or more behaviors failed or rules were violated
- **Problems** - Specific issues with the behavior
- **Hint** - Suggestion for fixing the problem
- **See Also** - Related behaviors in the spec

## Advanced Features

### Variable Capture and Interpolation

Capture values from responses and use them in subsequent requests:

```cue
// First behavior: Create a user and capture the ID
{
    name: "create-user"
    // ... request and response ...
    captures: {
        user_id: "id"           // Capture response.id as ${user_id}
        created_at: "created_at" // Capture response.created_at as ${created_at}
    }
}

// Second behavior: Get the user using captured ID
{
    name: "get-user"
    requires: ["create-user"]  // Must run after create-user
    request: {
        method: "GET"
        path: "/users/${user_id}"  // Use the captured user_id
        headers: {}
        query: {}
        body: null
    }
    // ... rest of behavior ...
    captures: {}
}

// Third behavior: Use multiple captured values
{
    name: "verify-timestamps"
    requires: ["create-user"]
    request: {
        method: "GET"
        path: "/users/${user_id}/events?since=${created_at}"
    }
    // ... rest of behavior ...
    captures: {}
}
```

### Behavior Dependencies

Control execution order with dependencies:

```cue
{
    name: "update-user"
    requires: ["create-user"]      // Single dependency
    // ...
}

{
    name: "share-item"
    requires: ["create-item", "create-user"]  // Multiple dependencies
    // ...
}
```

Intent automatically determines the correct order and blocks behaviors if dependencies fail.

### Global Rules

Apply rules across multiple endpoints:

```cue
rules: [
    {
        name: "no-exposed-secrets"
        description: "Responses should never expose secrets"
        when: {
            status: ">= 200"                    // Status condition
            method: "GET"                        // Method condition
            path: "/users.*"                     // Path regex pattern
        }
        check: {
            body_must_not_contain: [             // Forbidden strings
                "password",
                "secret",
                "token",
                "api_key"
            ]
            body_must_contain: []                // Required strings
            fields_must_exist: ["id"]            // Required fields
            fields_must_not_exist: []            // Forbidden fields
            header_must_exist: ""                // Required headers
            header_must_not_exist: ""            // Forbidden headers
        }
        example: {
            error: "Secrets exposed in response"
        }
    }
]
```

### Anti-Patterns

Detect common mistakes in API design:

```cue
anti_patterns: [
    {
        name: "missing-timestamps"
        description: "Responses should include created_at and updated_at"
        bad_example: {
            id: "123"
            name: "Product"
            // Missing timestamps!
        }
        good_example: {
            id: "123"
            name: "Product"
            created_at: "2024-01-04T12:00:00Z"
            updated_at: "2024-01-04T12:00:00Z"
        }
        why: "Timestamps are essential for auditing and debugging"
    }
]
```

## Best Practices

### 1. Organize by Feature

Group related behaviors in features:

```cue
features: [
    {
        name: "User Management"
        behaviors: [
            // Create, read, update, delete
        ]
    }
    {
        name: "Item Management"
        behaviors: [
            // Create, read, update, delete
        ]
    }
]
```

### 2. Clear Intent Statements

Write intent statements that describe the business value:

```cue
// Good
intent: "Create a new user with valid email and validate response format"

// Bad
intent: "POST to /users"
```

### 3. Test Both Happy Path and Error Cases

```cue
behaviors: [
    {
        name: "create-user-success"
        intent: "Successfully create a new user"
        request: { /* valid data */ }
        response: { status: 201 }
    }
    {
        name: "create-user-invalid-email"
        intent: "Reject user creation with invalid email"
        request: { /* invalid email */ }
        response: { status: 400 }
    }
    {
        name: "create-user-duplicate-email"
        intent: "Reject user creation with duplicate email"
        requires: ["create-user-success"]
        request: { /* duplicate email */ }
        response: { status: 409 }
    }
]
```

### 4. Validate at the Field Level

Check specific fields to catch subtle issues:

```cue
checks: {
    "id": { rule: "is uuid" }
    "email": { rule: "matches ^[^@]+@[^@]+\\.[^@]+$" }
    "age": { rule: "length 2" }
    "created_at": { rule: "is iso8601" }
}
```

### 5. Use Rules for Cross-Behavior Validation

```cue
rules: [
    {
        name: "consistent-status-codes"
        description: "Error responses should be 4xx"
        when: {
            status: ">= 400"
            method: "GET"
            path: ".*"
        }
        check: {
            fields_must_exist: ["error", "message"]
        }
    }
]
```

### 6. Document Edge Cases and Pitfalls

```cue
ai_hints: {
    pitfalls: [
        "User IDs must be stable across multiple requests",
        "Email addresses are case-insensitive but stored lowercase",
        "Passwords must never be returned in responses",
        "Rate limiting should return Retry-After header"
    ]
}
```

## Common Patterns

### Pattern: Create-Read-Update-Delete (CRUD)

```cue
features: [{
    name: "User CRUD"
    behaviors: [
        {
            name: "create-user"
            intent: "Create a new user"
            request: {
                method: "POST"
                path: "/users"
                body: { name: "John", email: "john@example.com" }
            }
            response: { status: 201 }
            captures: { user_id: "id" }
        }
        {
            name: "read-user"
            intent: "Read user details"
            requires: ["create-user"]
            request: {
                method: "GET"
                path: "/users/${user_id}"
            }
            response: { status: 200 }
        }
        {
            name: "update-user"
            intent: "Update user details"
            requires: ["create-user"]
            request: {
                method: "PUT"
                path: "/users/${user_id}"
                body: { name: "Jane" }
            }
            response: { status: 200 }
        }
        {
            name: "delete-user"
            intent: "Delete a user"
            requires: ["create-user"]
            request: {
                method: "DELETE"
                path: "/users/${user_id}"
            }
            response: { status: 204 }
        }
    ]
}]
```

### Pattern: Pagination

```cue
{
    name: "list-users-page-1"
    intent: "List first page of users"
    request: {
        method: "GET"
        path: "/users"
        query: {
            "page": "1"
            "size": "10"
        }
    }
    response: {
        status: 200
        checks: {
            "total": { rule: "length 3" }
            "items": { rule: "length 3" }
        }
    }
    captures: {
        next_page_url: "links.next"
    }
}
```

### Pattern: Authentication

```cue
{
    name: "authenticate"
    intent: "Get authentication token"
    request: {
        method: "POST"
        path: "/auth/login"
        body: {
            username: "admin"
            password: "secret"
        }
    }
    response: {
        status: 200
        checks: {
            "token": { rule: "length 32" }
        }
    }
    captures: {
        auth_token: "token"
    }
}

{
    name: "use-token"
    intent: "Use authentication token"
    requires: ["authenticate"]
    request: {
        method: "GET"
        path: "/protected/data"
        headers: {
            "Authorization": "Bearer ${auth_token}"
        }
    }
    response: { status: 200 }
}
```

## Troubleshooting

### Issue: Behaviors Execute in Wrong Order

Intent automatically resolves dependencies. If a behavior is blocked:

```
[Example Feature] update-item
Intent: Update an item
Blocked: Requires 'create-item' which failed
```

**Solution:**
1. Check that the dependency is spelled correctly
2. Check that the dependency doesn't have its own failures
3. Check that dependencies form a DAG (no cycles)

### Issue: Capture Not Working

If a variable isn't being interpolated:

```bash
# Use verbose output to see what's captured
gleam run -- check spec.cue --target http://localhost:8080 --verbose
```

**Solution:**
1. Check that the behavior it captures from runs first
2. Check that the path to the field is correct (e.g., "user.id" not "id")
3. Check that the field exists in the response

### Issue: Rule Violations Not Shown

Rules require both a `when` match and a violation:

**Solution:**
1. Check that the rule's `when` conditions match your behavior
2. Check that the `check` conditions fail for your response
3. Check status conditions: `">= 200"`, `"< 300"`, etc.
4. Check path patterns: `/users.*` matches `/users` and `/users/123`

### Issue: JSON Parse Errors

If responses aren't being parsed correctly:

```
Error: Failed to parse response body as JSON
```

**Solution:**
1. Verify your API returns valid JSON
2. Check Content-Type header is `application/json`
3. Check the response isn't empty

See [SPEC_FORMAT.md](SPEC_FORMAT.md) for more details on specification syntax.
