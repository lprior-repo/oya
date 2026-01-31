# Intent Schema: #Response Type

## Overview

The `#Response` type defines the expected response from an HTTP request. It specifies the expected status code, provides an example response, and defines validation checks that should pass.

## Type Definition

```cue
#Response: {
	status: int & >=100 & <=599

	// Example of a valid response (for AI learning)
	example: {...} | *null

	// Structured checks
	checks: #Checks | *{}

	// Optional headers to check
	headers?: #Headers
}
```

## Required Fields

### `status: int`
Expected HTTP status code.
- Must be between 100 and 599 (valid HTTP status codes)
- Compared against actual response status
- Behavior passes if actual status matches
- Common status codes:
  - `200` - OK (successful GET, POST, PUT, PATCH)
  - `201` - Created (successful POST creating new resource)
  - `204` - No Content (successful DELETE or empty response)
  - `400` - Bad Request (validation error, missing fields)
  - `401` - Unauthorized (authentication required/invalid)
  - `403` - Forbidden (authenticated but not authorized)
  - `404` - Not Found (resource doesn't exist)
  - `409` - Conflict (duplicate resource, state conflict)
  - `429` - Too Many Requests (rate limited)
  - `500` - Internal Server Error (server error)
  - `503` - Service Unavailable (service down)

## Optional Fields

### `example: {...}` (default: `null`)
Example of a valid response body.
- Used by Intent for documentation
- Can be used by AI systems for learning
- Helps developers understand expected format
- Type is flexible - can be object, array, string, etc.
- Should match actual successful response structure
- Examples:
  ```cue
  // Object response
  example: {
    id: "user_123"
    name: "John Doe"
    email: "john@example.com"
  }

  // Array response
  example: [
    { id: "1", name: "Item 1" }
    { id: "2", name: "Item 2" }
  ]

  // Null response (for 204 No Content)
  example: null
  ```

### `checks: #Checks` (default: `{}`)
Response validation rules.
- Type: `[string]: #Check` (field path to check definition)
- Validates that response matches expected structure
- Each check has a rule and explanation
- See `#Check` and `#Checks` type documentation
- Examples:
  ```cue
  checks: {
    "id": {
      rule: "must be non-empty string"
      why:  "ID is required for resource tracking"
    }
    "name": {
      rule: "must be string with 1-100 characters"
      why:  "Name must be properly formatted"
    }
  }
  ```

### `headers: #Headers` (optional)
Expected response headers to validate.
- Type: `[string]: string` map
- Validates that response includes these headers
- Examples:
  ```cue
  headers: {
    "Content-Type": "application/json"
    "X-Rate-Limit-Remaining": "99"
  }
  ```

## Complete Examples

### Simple Success Response (200 OK)

```cue
response: {
	status: 200
	example: {
		id:    "user_123"
		name:  "John Doe"
		email: "john@example.com"
	}
	checks: {
		"id": {
			rule: "must be non-empty string"
			why:  "User ID is required"
		}
		"name": {
			rule: "must be non-empty string"
			why:  "User name is required"
		}
		"email": {
			rule: "must match email format"
			why:  "Email must be valid"
		}
	}
}
```

### Created Response (201 Created)

```cue
response: {
	status: 201
	example: {
		id:    "user_123"
		name:  "New User"
		email: "new@example.com"
		created_at: "2024-01-15T10:30:00Z"
	}
	checks: {
		"id": {
			rule: "must be non-empty string"
			why:  "Created resource must have ID"
		}
		"created_at": {
			rule: "must be ISO8601 timestamp"
			why:  "Record creation time for audit trail"
		}
	}
}
```

### No Content Response (204)

```cue
response: {
	status: 204
	example: null
	checks: {}
}
```

### Error Response (400 Bad Request)

```cue
response: {
	status: 400
	example: {
		error_code: "VALIDATION_ERROR"
		message:    "Email is required"
		fields: {
			email: "Email is required"
		}
	}
	checks: {
		"error_code": {
			rule: "must equal VALIDATION_ERROR"
			why:  "Specific error code for validation errors"
		}
		"message": {
			rule: "must be non-empty string"
			why:  "Error message explains what went wrong"
		}
		"fields.email": {
			rule: "must contain field error"
			why:  "Indicates which field failed validation"
		}
	}
}
```

### Unauthorized Response (401)

```cue
response: {
	status: 401
	example: {
		error_code: "UNAUTHORIZED"
		message:    "Authentication required"
	}
	checks: {
		"error_code": {
			rule: "must equal UNAUTHORIZED"
			why:  "Standard error code for missing/invalid auth"
		}
		"message": {
			rule: "must be non-empty"
			why:  "Explains why authentication failed"
		}
	}
	headers: {
		"WWW-Authenticate": "Bearer"
	}
}
```

### Not Found Response (404)

```cue
response: {
	status: 404
	example: {
		error_code: "NOT_FOUND"
		message:    "User not found"
	}
	checks: {
		"error_code": {
			rule: "must equal NOT_FOUND"
			why:  "Standard error code for missing resources"
		}
	}
}
```

### Conflict Response (409)

```cue
response: {
	status: 409
	example: {
		error_code: "DUPLICATE_EMAIL"
		message:    "Email already registered"
	}
	checks: {
		"error_code": {
			rule: "must equal DUPLICATE_EMAIL"
			why:  "Specific error for duplicate constraint"
		}
	}
}
```

### Array Response (List)

```cue
response: {
	status: 200
	example: [
		{
			id:    "user_123"
			name:  "Alice"
			email: "alice@example.com"
		},
		{
			id:    "user_456"
			name:  "Bob"
			email: "bob@example.com"
		}
	]
	checks: {
		"[0].id": {
			rule: "must be non-empty string"
			why:  "Each user must have an ID"
		}
		"[0].name": {
			rule: "must be non-empty string"
			why:  "Each user must have a name"
		}
	}
}
```

### Paginated Response

```cue
response: {
	status: 200
	example: {
		data: [
			{ id: "1", name: "Item 1" }
			{ id: "2", name: "Item 2" }
		]
		pagination: {
			limit:  10
			offset: 0
			total:  100
		}
	}
	checks: {
		"data": {
			rule: "must be array"
			why:  "Response data should be array"
		}
		"pagination.total": {
			rule: "must be integer >= 0"
			why:  "Total count needed for pagination UI"
		}
	}
}
```

### Nested Object Response

```cue
response: {
	status: 200
	example: {
		order: {
			id: "order_123"
			status: "confirmed"
			items: [
				{ product_id: "prod_1", quantity: 2 }
			]
		}
		shipping: {
			address: "123 Main St"
			city: "San Francisco"
			state: "CA"
		}
	}
	checks: {
		"order.id": {
			rule: "must be non-empty string"
			why:  "Order ID required for tracking"
		}
		"shipping.address": {
			rule: "must be non-empty string"
			why:  "Shipping address required"
		}
	}
}
```

### Response with Headers

```cue
response: {
	status: 200
	example: {
		id:    "resource_123"
		data:  "content"
	}
	checks: {
		"id": {
			rule: "must be non-empty"
			why:  "Resource identifier required"
		}
	}
	headers: {
		"Content-Type": "application/json"
		"X-Rate-Limit-Limit": "1000"
		"X-Rate-Limit-Remaining": "999"
		"Cache-Control": "public, max-age=300"
	}
}
```

## Check Field Paths

The `checks` object maps field paths to validation rules:

### Simple Fields
```cue
checks: {
	"id": { rule: "...", why: "..." }
	"name": { rule: "...", why: "..." }
}
```

### Nested Fields (Dot Notation)
```cue
checks: {
	"user.id": { rule: "...", why: "..." }
	"user.profile.name": { rule: "...", why: "..." }
}
```

### Array Elements
```cue
checks: {
	"[0].id": { rule: "...", why: "..." }     // First element
	"[].name": { rule: "...", why: "..." }    // All elements
}
```

### Array with Nested Path
```cue
checks: {
	"items[0].product.id": { rule: "...", why: "..." }
}
```

## Status Code Categories

### 1xx - Informational (100-199)
- Rarely used in REST APIs
- `100` - Continue
- `101` - Switching Protocols

### 2xx - Success (200-299)
- Operation succeeded
- `200` - OK (most common)
- `201` - Created
- `202` - Accepted (async)
- `204` - No Content
- `206` - Partial Content

### 3xx - Redirection (300-399)
- Client must take action
- `301` - Moved Permanently
- `302` - Found (temporary redirect)
- `304` - Not Modified (cached)

### 4xx - Client Error (400-499)
- Client's fault
- `400` - Bad Request
- `401` - Unauthorized
- `403` - Forbidden
- `404` - Not Found
- `409` - Conflict
- `422` - Unprocessable Entity
- `429` - Too Many Requests

### 5xx - Server Error (500-599)
- Server's fault
- `500` - Internal Server Error
- `502` - Bad Gateway
- `503` - Service Unavailable
- `504` - Gateway Timeout

## Validation Rules

Common check rules:

```cue
checks: {
	// Type checks
	"id": {
		rule: "must be non-empty string"
		why: "ID is required"
	}

	// Format checks
	"email": {
		rule: "must match email format"
		why: "Valid email required"
	}

	// Length checks
	"name": {
		rule: "must be string with 1-100 characters"
		why: "Name has length constraints"
	}

	// Numeric checks
	"count": {
		rule: "must be integer >= 0"
		why: "Count cannot be negative"
	}

	// Enum checks
	"status": {
		rule: "must be one of: pending, active, completed"
		why: "Status must be valid state"
	}

	// Date checks
	"created_at": {
		rule: "must be ISO8601 timestamp"
		why: "Timestamp required for audit"
	}
}
```

## Best Practices

### ✅ Do

- **Match real responses** - Example should match actual API responses
- **Be specific in checks** - Explain validation requirements clearly
- **Check required fields** - Validate all important fields
- **Include error examples** - Define error response examples
- **Validate headers** - Check important response headers
- **Use meaningful why** - Explain why each check matters
- **Test edge cases** - Check boundary conditions

### ❌ Don't

- **Use placeholder examples** - Make examples realistic
- **Assume format** - Explicitly validate response format
- **Skip error cases** - Test error responses too
- **Over-validate** - Don't check every tiny detail
- **Ignore response structure** - Validate nested paths
- **Use vague rules** - Be specific about expectations
- **Skip important headers** - Validate critical headers

## Response Interpretation

Intent interprets responses as follows:

1. **Status code check**: Does `response.status` match actual?
2. **Field validation**: Do all `checks` pass?
3. **Header validation**: Do optional `headers` match?
4. **Overall result**: All checks pass = behavior passed

## Integration with Behavior

Responses are defined within behaviors:

```cue
behaviors: [{
	name:   "get_user"
	intent: "Retrieve user by ID"
	request: {...}
	response: {
		status: 200
		example: {
			id:    "user_123"
			name:  "John Doe"
			email: "john@example.com"
		}
		checks: {
			"id": {
				rule: "must be non-empty string"
				why:  "User ID required"
			}
		}
	}
	// ...
}]
```

## See Also

- [`#Check`](./schema-check-type.md) - Individual validation rule
- [`#Checks`](./schema-checks-type.md) - Response checks map
- [`#Headers`](./schema-headers-type.md) - HTTP headers
- [`#Behavior`](./schema-behavior-type.md) - Behavior containing response
- [`#Request`](./schema-request-type.md) - HTTP request
