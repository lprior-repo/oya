package array_validation

import "github.com/intent-cli/intent/schema:intent"

// Example: Array Validation Rules
// Demonstrates various array checks like length, min/max items, and element validation

spec: intent.#Spec & {
	name: "Product Catalog API"

	description: """
		A product catalog API demonstrating array validation patterns.
		Shows how to validate list endpoints, pagination, and array elements.
		"""

	audience: "E-commerce applications"

	success_criteria: [
		"Products are returned as arrays with predictable structure",
		"Pagination limits are respected",
		"Array elements follow consistent schema",
	]

	config: {
		base_url:   "http://localhost:8080"
		timeout_ms: 5000
	}

	features: [
		{
			name: "Product Listing"

			description: """
				List products with filtering and pagination. Demonstrates
				various array validation rules.
				"""

			behaviors: [
				{
					name:   "list-all-products"
					intent: "Retrieve all products as a non-empty array"

					request: {
						method: "GET"
						path:   "/products"
					}

					response: {
						status: 200

						example: {
							products: [
								{id: "prod_abc123", name: "Widget", price: 9.99},
								{id: "prod_def456", name: "Gadget", price: 19.99},
							]
							total: 2
						}

						checks: {
							"products": {
								rule: "non-empty array"
								why:  "Catalog should have at least one product"
							}
							"total": {
								rule: "integer >= 1"
								why:  "Total count must match array length"
							}
						}
					}
				},
				{
					name:   "list-with-pagination"
					intent: "Pagination respects limit parameter"

					request: {
						method: "GET"
						path:   "/products"
						query: {
							limit: 5
						}
					}

					response: {
						status: 200

						example: {
							products: [
								{id: "prod_001", name: "Item 1", price: 10.00},
								{id: "prod_002", name: "Item 2", price: 20.00},
								{id: "prod_003", name: "Item 3", price: 30.00},
								{id: "prod_004", name: "Item 4", price: 40.00},
								{id: "prod_005", name: "Item 5", price: 50.00},
							]
							limit:  5
							offset: 0
							total:  100
						}

						checks: {
							"products": {
								rule: "array with max 5 items"
								why:  "Response must respect the limit parameter"
							}
							"limit": {
								rule: "equals 5"
								why:  "Echoes back the requested limit"
							}
						}
					}
				},
				{
					name:   "list-exact-length"
					intent: "Featured products returns exactly 3 items"

					request: {
						method: "GET"
						path:   "/products/featured"
					}

					response: {
						status: 200

						example: {
							featured: [
								{id: "prod_feat1", name: "Top Seller", price: 99.99},
								{id: "prod_feat2", name: "New Arrival", price: 49.99},
								{id: "prod_feat3", name: "Staff Pick", price: 29.99},
							]
						}

						checks: {
							"featured": {
								rule: "array of length 3"
								why:  "Featured section always shows exactly 3 products"
							}
						}
					}
				},
				{
					name:   "list-minimum-items"
					intent: "Search results have at least 1 result when query matches"

					request: {
						method: "GET"
						path:   "/products/search"
						query: {
							q: "widget"
						}
					}

					response: {
						status: 200

						example: {
							results: [
								{id: "prod_widget1", name: "Blue Widget", price: 12.99},
								{id: "prod_widget2", name: "Red Widget", price: 14.99},
							]
							query: "widget"
						}

						checks: {
							"results": {
								rule: "array with min 1 items"
								why:  "Matching query should return at least one result"
							}
							"query": {
								rule: "equals widget"
								why:  "Echoes back the search query"
							}
						}
					}
				},
				{
					name:   "list-with-element-validation"
					intent: "Each tag in product follows naming convention"

					request: {
						method: "GET"
						path:   "/products/prod_abc123"
					}

					response: {
						status: 200

						example: {
							id:   "prod_abc123"
							name: "Widget Pro"
							tags: ["electronics", "new-arrival", "sale"]
						}

						checks: {
							"id": {
								rule: "string matching prod_[a-z0-9]+"
								why:  "Product IDs follow prefixed format"
							}
							"tags": {
								rule: "array where each matches [a-z][a-z0-9-]*"
								why:  "Tags must be lowercase kebab-case"
							}
						}
					}
				},
				{
					name:   "empty-search-results"
					intent: "Search with no matches returns empty array (not null)"

					request: {
						method: "GET"
						path:   "/products/search"
						query: {
							q: "nonexistent_xyz_123"
						}
					}

					response: {
						status: 200

						example: {
							results: []
							query:   "nonexistent_xyz_123"
							total:   0
						}

						checks: {
							"results": {
								rule: "array"
								why:  "Must return array type even when empty"
							}
							"total": {
								rule: "equals 0"
								why:  "Zero results for non-matching query"
							}
						}
					}
				},
			]
		},
		{
			name: "Categories"

			description: """
				Category endpoints demonstrating nested array validation.
				"""

			behaviors: [
				{
					name:   "list-categories-with-products"
					intent: "Categories include product counts and nested arrays"

					request: {
						method: "GET"
						path:   "/categories"
					}

					response: {
						status: 200

						example: {
							categories: [
								{
									id:            "cat_electronics"
									name:          "Electronics"
									product_count: 150
									subcategories: ["phones", "laptops", "accessories"]
								},
								{
									id:            "cat_clothing"
									name:          "Clothing"
									product_count: 300
									subcategories: ["shirts", "pants", "shoes"]
								},
							]
						}

						checks: {
							"categories": {
								rule: "non-empty array"
								why:  "Store must have at least one category"
							}
							"categories[0].subcategories": {
								rule: "non-empty array"
								why:  "Categories should have subcategories"
							}
							"categories[0].product_count": {
								rule: "integer >= 0"
								why:  "Product count cannot be negative"
							}
						}
					}
				},
			]
		},
	]

	rules: [
		{
			name:        "array-responses"
			description: "List endpoints should return arrays, not objects"

			when: {path: ".*\\?.*"}

			check: {
				fields_must_not_exist: ["error"]
			}
		},
	]

	anti_patterns: [
		{
			name:        "null-for-empty"
			description: "Never return null for empty collections"

			bad_example: {
				products: null
			}

			good_example: {
				products: []
			}

			why: """
				Null requires special handling in clients. Empty arrays
				are more predictable and avoid null pointer exceptions.
				"""
		},
		{
			name:        "array-without-wrapper"
			description: "Don't return bare arrays, wrap in object"

			bad_example: {
				_comment: "Response is just a bare array"
				response: "[ {id: 1, name: Product} ]"
			}

			good_example: {
				products: [{id: "1", name: "Product"}]
				total:    1
			}

			why: """
				Bare arrays can't be extended with metadata. Object
				wrappers allow adding pagination, totals, and links.
				"""
		},
	]

	ai_hints: {
		implementation: {
			suggested_stack: ["Go", "PostgreSQL"]
		}

		entities: {
			product: {
				fields: {
					id:    "string, prefixed 'prod_', randomly generated"
					name:  "string, 1-200 chars"
					price: "decimal, >= 0"
					tags:  "array of strings, lowercase kebab-case"
				}
			}
		}

		security: {
			password_hashing: "N/A - uses API keys, no passwords"
			jwt_algorithm:    "HS256 for optional JWT authentication"
			jwt_expiry:       "24 hours for session tokens"
			rate_limiting:    "1000 requests per hour per client"
		}

		pitfalls: [
			"Don't return null for empty arrays",
			"Don't exceed pagination limits",
			"Ensure total count matches when not paginated",
			"Validate array element structure consistently",
		]
	}
}
