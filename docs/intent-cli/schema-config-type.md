# Intent Schema: #Config Type

## Overview

The `#Config` type defines runtime configuration for spec execution. It specifies how Intent should connect to and interact with the target API.

## Type Definition

```cue
#Config: {
	base_url:   string | *""
	timeout_ms: int | *5000
	headers:    #Headers | *{}
}

#DefaultConfig: #Config & {}
```

## Required Fields

All fields have defaults but can be explicitly overridden:

### `base_url: string` (default: `""`)
The base URL of the target API.
- Must be a valid HTTP or HTTPS URL
- Used as the prefix for all request paths in behaviors
- Examples:
  - `"https://api.example.com"`
  - `"http://localhost:8080"`
  - `"https://api.example.com/v1"` (includes version path)

**Path Resolution:**
If a behavior request path is `/users/123`, the final URL will be:
```
{config.base_url}{request.path}
→ https://api.example.com/users/123
```

### `timeout_ms: int` (default: `5000`)
Request timeout in milliseconds for all HTTP requests.
- Minimum: `100` ms (recommended)
- Maximum: `300000` ms (5 minutes, reasonable limit)
- Typical values: `5000` (5s), `10000` (10s), `30000` (30s)
- Applies to all requests executed during spec checking

### `headers: #Headers` (default: `{}`)
Default HTTP headers sent with every request.
- Type: `[string]: string` map
- These headers are merged with behavior-specific headers
- Behavior headers take precedence over config headers
- Common headers:
  - `"Content-Type": "application/json"` - Request body format
  - `"Accept": "application/json"` - Expected response format
  - `"User-Agent": "Intent-CLI/2.0"` - Identification
  - `"Authorization": "Bearer <token>"` - Authentication

## Complete Examples

### Minimal Configuration

```cue
config: {
	base_url:   "https://api.example.com"
	timeout_ms: 5000
	headers:    {}
}
```

### Development Environment

```cue
config: {
	base_url:   "http://localhost:3000"
	timeout_ms: 10000
	headers: {
		"Content-Type": "application/json"
		"Accept": "application/json"
		"User-Agent": "Intent-CLI/2.0"
	}
}
```

### Production with Authentication

```cue
config: {
	base_url:   "https://api.production.example.com"
	timeout_ms: 5000
	headers: {
		"Content-Type": "application/json"
		"Accept": "application/json"
		"Authorization": "Bearer eyJhbGciOiJIUzI1NiIs..."
		"User-Agent": "Intent-CLI/2.0"
		"X-API-Key": "sk_prod_1234567890abcdef"
	}
}
```

### API Version Handling

```cue
config: {
	// Version in base_url
	base_url:   "https://api.example.com/v2"
	timeout_ms: 5000
	headers: {
		"Content-Type": "application/json"
		"Accept": "application/vnd.api+json; version=2"
	}
}
```

### Multiple Environment Support

```cue
let env = "production"

config: {
	base_url: env == "development" ? "http://localhost:8080" :
	          env == "staging" ? "https://staging-api.example.com" :
	          "https://api.example.com"

	timeout_ms: env == "development" ? 10000 : 5000

	headers: {
		"Content-Type": "application/json"
		"Accept": "application/json"
		if env != "development" {
			"X-Environment": env
		}
	}
}
```

## Header Precedence

Headers are merged with this precedence (lowest to highest):

1. **Global config headers** - Applied to all requests
2. **Behavior request headers** - Override config headers
3. **Runtime variables** - Can inject/override headers at execution time

Example:

```cue
config: {
	headers: {
		"Authorization": "Bearer token1"
		"Accept": "application/json"
	}
}

behaviors: [{
	request: {
		headers: {
			"Authorization": "Bearer token2" // Overrides config
			// "Accept" inherited from config
		}
	}
}]
```

Result request headers:
```
Authorization: Bearer token2
Accept: application/json
```

## Timeout Behavior

When a request exceeds `timeout_ms`:
- The request is cancelled
- An error is recorded
- Behavior is marked as failed
- Execution continues to next behavior (respecting dependencies)

Example with 5-second timeout:
- Request takes 6 seconds → **Timeout error**
- Request takes 5 seconds exactly → **Success**
- Request takes 4.999 seconds → **Success**

## URL Construction Rules

Intent constructs URLs following standard HTTP conventions:

```
Final URL = base_url + request.path + query string

Examples:
- base_url: "https://api.example.com"
  path: "/users"
  → https://api.example.com/users

- base_url: "https://api.example.com/v1"
  path: "/users/123"
  → https://api.example.com/v1/users/123

- base_url: "http://localhost:8080"
  path: "/api/products"
  → http://localhost:8080/api/products
```

## Query String Handling

Query parameters are constructed from the request's `query` field:

```cue
request: {
	path: "/search"
	query: {
		q: "users"
		limit: "10"
		offset: "0"
	}
}
```

Final URL: `https://api.example.com/search?q=users&limit=10&offset=0`

## Security Considerations

### ⚠️ Sensitive Data in Config

Avoid storing sensitive credentials in specs:

❌ **Bad:**
```cue
config: {
	headers: {
		"Authorization": "Bearer sk_live_1234567890abcdef_SuperSecretToken"
	}
}
```

✅ **Better: Use environment variables or secret management**
```cue
config: {
	headers: {
		"Authorization": "Bearer ${API_TOKEN}"  // Injected at runtime
	}
}
```

### ⚠️ HTTPS in Production

Always use HTTPS for production APIs:

❌ **Bad:**
```cue
base_url: "http://api.production.example.com"  // Unencrypted!
```

✅ **Good:**
```cue
base_url: "https://api.production.example.com"  // Encrypted
```

### ⚠️ Timeout Too Short

Don't set timeouts too aggressively:

❌ **Bad:**
```cue
timeout_ms: 100  // Too short for most APIs
```

✅ **Good:**
```cue
timeout_ms: 5000  // Reasonable for most APIs
```

## Default Config

If no `config` is specified, Intent uses defaults:

```cue
#DefaultConfig: {
	base_url:   ""
	timeout_ms: 5000
	headers:    {}
}
```

This means:
- No base URL (requests won't resolve properly)
- 5-second timeout
- No default headers

## Integration with #Spec

The `config` field is required in every `#Spec`:

```cue
spec: intent.#Spec & {
	// ... other fields ...
	config: {
		base_url:   "https://api.example.com"
		timeout_ms: 5000
		headers:    {}
	}
	// ... behaviors, rules, etc. ...
}
```

## See Also

- [`#Spec`](./schema-spec-type.md) - Top-level specification type
- [`#Request`](./schema-request-type.md) - HTTP request definition
- [`#Headers`](./schema-headers-type.md) - HTTP headers map type
