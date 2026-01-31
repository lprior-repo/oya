# Intent Schema: #Request Type

## Overview

The `#Request` type defines an HTTP request to execute against the target API. It specifies the method, path, headers, query parameters, and body for the request.

## Type Definition

```cue
#Request: {
	method:  #Method
	path:    string
	headers: #Headers | *{}
	query: {...} | *{}
	body: _ | *null
}
```

## Required Fields

### `method: #Method`
The HTTP method for the request.
- Valid values: `"GET"`, `"POST"`, `"PUT"`, `"PATCH"`, `"DELETE"`, `"HEAD"`, `"OPTIONS"`
- Determines how the request is sent
- Examples:
  - `"GET"` - Retrieve data
  - `"POST"` - Create new resource
  - `"PUT"` - Replace entire resource
  - `"PATCH"` - Partial update
  - `"DELETE"` - Remove resource
  - `"HEAD"` - Like GET but no response body
  - `"OPTIONS"` - Check allowed methods

### `path: string`
The API endpoint path (relative to `config.base_url`).
- Appended to `config.base_url` to form the full URL
- Must start with `/`
- Supports variable interpolation: `"/users/${user_id}"`
- Examples:
  - `"/users"` - Base users endpoint
  - `"/users/123"` - Specific user
  - `"/users/${user_id}/posts"` - User's posts using captured ID
  - `"/api/v2/products"` - Versioned endpoint

## Optional Fields

### `headers: #Headers` (default: `{}`)
HTTP headers for the request.
- Type: `[string]: string` (header name to value)
- Merged with config headers (request headers override config)
- Supports variable interpolation: `"Bearer ${auth_token}"`
- Common headers:
  - `"Authorization"` - Authentication credentials
  - `"Content-Type"` - Request body format
  - `"Accept"` - Expected response format
  - `"User-Agent"` - Client identification
  - `"X-API-Key"` - API key authentication
- Examples:
  ```cue
  headers: {
    "Authorization": "Bearer ${auth_token}"
    "Content-Type": "application/json"
    "X-Request-ID": "req_123"
  }
  ```

### `query: {...}` (default: `{}`)
Query parameters appended to the URL.
- Type: Object with string values
- Converted to URL query string
- Examples:
  ```cue
  query: {
    limit:  "10"
    offset: "0"
    sort:   "created_at"
  }
  // Results in: ?limit=10&offset=0&sort=created_at
  ```

### `body: _` (default: `null`)
Request body content.
- Type is flexible (`_`) - can be any JSON type
- `null` for requests with no body (GET, DELETE, HEAD)
- String for raw content
- Object for JSON payloads
- Array for JSON arrays
- Supports variable interpolation in strings
- Examples:
  ```cue
  // Object body (common)
  body: {
    name: "John Doe"
    email: "john@example.com"
  }

  // Array body
  body: ["item1", "item2"]

  // String body (raw text)
  body: "raw content"

  // No body
  body: null
  ```

## Complete Examples

### Simple GET Request

```cue
request: {
	method:  "GET"
	path:    "/users"
	headers: {}
	query:   {}
	body:    null
}
```

### GET with Query Parameters

```cue
request: {
	method:  "GET"
	path:    "/users"
	headers: {}
	query: {
		limit:   "10"
		offset:  "0"
		sort:    "created_at"
		filter:  "active"
	}
	body: null
}

// Results in URL: /users?limit=10&offset=0&sort=created_at&filter=active
```

### GET with Variable Path

```cue
request: {
	method:  "GET"
	path:    "/users/${user_id}"
	headers: {}
	query:   {}
	body:    null
}

// If user_id = "123", URL becomes: /users/123
```

### GET with Authentication Header

```cue
request: {
	method: "GET"
	path:   "/users/me"
	headers: {
		"Authorization": "Bearer ${auth_token}"
	}
	query: {}
	body:  null
}

// If auth_token = "eyJhbGc...", header becomes: Authorization: Bearer eyJhbGc...
```

### POST with JSON Body

```cue
request: {
	method: "POST"
	path:   "/users"
	headers: {
		"Content-Type": "application/json"
	}
	query: {}
	body: {
		name:     "John Doe"
		email:    "john@example.com"
		password: "SecurePass123!"
	}
}
```

### POST with Variable in Body

```cue
request: {
	method: "POST"
	path:   "/users/${user_id}/posts"
	headers: {
		"Content-Type": "application/json"
		"Authorization": "Bearer ${auth_token}"
	}
	query: {}
	body: {
		title:    "My Post"
		content:  "Post content"
		author_id: "${user_id}"  // Uses captured user_id
	}
}
```

### PUT Request (Full Update)

```cue
request: {
	method: "PUT"
	path:   "/users/${user_id}"
	headers: {
		"Content-Type": "application/json"
	}
	query: {}
	body: {
		name:     "Updated Name"
		email:    "updated@example.com"
		age:      30
	}
}
```

### PATCH Request (Partial Update)

```cue
request: {
	method: "PATCH"
	path:   "/users/${user_id}"
	headers: {
		"Content-Type": "application/json"
	}
	query: {}
	body: {
		name: "New Name"
		// Only updating name, not other fields
	}
}
```

### DELETE Request

```cue
request: {
	method:  "DELETE"
	path:    "/users/${user_id}"
	headers: {}
	query:   {}
	body:    null
}
```

### Complex POST with Nested Body

```cue
request: {
	method: "POST"
	path:   "/orders"
	headers: {
		"Content-Type": "application/json"
	}
	query: {}
	body: {
		user_id: "${user_id}"
		items: [
			{
				product_id: "prod_123"
				quantity:   5
				price:      29.99
			},
			{
				product_id: "prod_456"
				quantity:   2
				price:      49.99
			}
		]
		shipping: {
			address:     "123 Main St"
			city:        "San Francisco"
			country:     "US"
			postal_code: "94102"
		}
		payment: {
			method_id: "${payment_method_id}"
			amount:    199.95
		}
	}
}
```

### Request with All Features

```cue
request: {
	method: "POST"
	path:   "/api/v2/users/${user_id}/notifications"
	headers: {
		"Authorization": "Bearer ${auth_token}"
		"Content-Type": "application/json"
		"X-Request-ID": "req_unique_id"
		"User-Agent": "Intent-CLI/2.0"
	}
	query: {
		delay_seconds: "5"
		async: "true"
	}
	body: {
		type:      "email"
		recipient: "${user_email}"
		subject:   "Account Update"
		content:   "Your account has been updated"
	}
}
```

## URL Construction

The final URL is constructed as:

```
{config.base_url}{path}?{query}
```

Examples:

```
Config base_url: "https://api.example.com"

Request path: "/users"
Final URL: https://api.example.com/users

Request path: "/users/123"
Final URL: https://api.example.com/users/123

Request path: "/api/v2/users"
Final URL: https://api.example.com/api/v2/users

With query parameters:
Request path: "/users"
Request query: { limit: "10", offset: "0" }
Final URL: https://api.example.com/users?limit=10&offset=0
```

## Variable Interpolation

Variables are embedded in template syntax: `${variable_name}`

Variables available during request execution:

1. **Captured values** from dependency behaviors
2. **Environment variables** (if supported by Intent)
3. **Config headers** (if used as body values)

Examples:

```cue
// Captured user_id = "123"
path: "/users/${user_id}"  // → /users/123

// Captured auth_token = "abc..."
headers: {
  "Authorization": "Bearer ${auth_token}"  // → Bearer abc...
}

// Captured user_id = "123"
body: {
  creator_id: "${user_id}"  // → "123"
}
```

## HTTP Method Reference

### GET
- Retrieves data
- No body (body: null)
- Use query parameters for filtering
- Idempotent and cacheable

```cue
request: {
	method: "GET"
	path:   "/products"
	query: { category: "books" }
}
```

### POST
- Creates new resource
- Includes body with data
- Query parameters optional
- Not idempotent by default

```cue
request: {
	method: "POST"
	path:   "/users"
	body: { name: "John", email: "john@example.com" }
}
```

### PUT
- Replaces entire resource
- Includes complete body
- Idempotent (same request = same result)
- All fields must be provided

```cue
request: {
	method: "PUT"
	path:   "/users/123"
	body: { name: "Jane", email: "jane@example.com", age: 25 }
}
```

### PATCH
- Partial update of resource
- Only include fields to change
- Idempotent (usually)
- Safer than PUT for partial updates

```cue
request: {
	method: "PATCH"
	path:   "/users/123"
	body: { name: "Jane" }  // Only update name
}
```

### DELETE
- Removes resource
- No body (body: null)
- Idempotent (deleting twice = same result)
- Use path to specify resource

```cue
request: {
	method: "DELETE"
	path:   "/users/123"
}
```

### HEAD
- Like GET but no response body
- Useful for checking if resource exists
- No body
- Idempotent

```cue
request: {
	method: "HEAD"
	path:   "/files/document.pdf"
}
```

### OPTIONS
- Checks allowed methods for resource
- No body
- Helps with CORS pre-flight checks

```cue
request: {
	method: "OPTIONS"
	path:   "/users"
}
```

## Best Practices

### ✅ Do

- **Use relative paths** - Start with `/`
- **Parametrize with captures** - Use `${variable}` for IDs
- **Specify Content-Type** - Include in headers
- **Use correct method** - GET for read, POST for create, etc.
- **Include required headers** - Authorization, API keys, etc.
- **Validate paths** - Start with `/` to avoid URL construction issues
- **Use HTTPS** - For production APIs

### ❌ Don't

- **Hardcode IDs** - Use captures instead
- **Include full URLs** - Use relative paths
- **Omit Content-Type** - Servers may reject requests
- **Use wrong methods** - Violates REST conventions
- **Mix HTTP and HTTPS** - Use HTTPS consistently
- **Forget variable escaping** - Ensure values are properly encoded
- **Use HTTP for sensitive data** - Always use HTTPS

## Security Considerations

### ⚠️ Sensitive Data in Headers

Avoid embedding secrets in request definitions:

❌ **Bad:**
```cue
headers: {
	"X-API-Key": "sk_live_1234567890abcdef_SuperSecretKey"
}
```

✅ **Better:**
```cue
headers: {
	"X-API-Key": "${API_KEY}"  // Injected from environment
}
```

### ⚠️ Sensitive Data in Body

Don't hardcode passwords or tokens:

❌ **Bad:**
```cue
body: {
	password: "MySecretPassword123!"
	api_token: "token_secret_value"
}
```

✅ **Better:**
```cue
body: {
	password: "${USER_PASSWORD}"
	api_token: "${API_TOKEN}"
}
```

## Integration with Behavior

Requests are defined within behaviors:

```cue
behaviors: [{
	name:   "create_user"
	intent: "Create a new user"
	request: {
		method: "POST"
		path:   "/users"
		headers: { "Content-Type": "application/json" }
		query:   {}
		body: { name: "John", email: "john@example.com" }
	}
	response: {...}
	// ...
}]
```

## See Also

- [`#Method`](./schema-method-type.md) - HTTP method enum
- [`#Headers`](./schema-headers-type.md) - HTTP headers map
- [`#Response`](./schema-response-type.md) - Expected response
- [`#Behavior`](./schema-behavior-type.md) - Behavior containing request
- [`#Config`](./schema-config-type.md) - Base URL configuration
