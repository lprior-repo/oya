package nested_paths

import "github.com/intent-cli/intent/schema:intent"

// Example: Nested JSON Path Validation
// Demonstrates checking deeply nested fields like "user.profile.address.city"

spec: intent.#Spec & {
	name: "Customer Profile API"

	description: """
		A customer profile API with deeply nested data structures.
		Demonstrates validating nested paths like user.profile.address.city
		and array indexing like orders[0].items[0].product.
		"""

	audience: "CRM and support applications"

	success_criteria: [
		"User profiles contain complete nested data",
		"Addresses are validated at all nesting levels",
		"Order history with nested items is accessible",
	]

	config: {
		base_url:   "http://localhost:8080"
		timeout_ms: 5000
	}

	features: [
		{
			name: "Profile Management"

			description: """
				User profiles with nested personal info, addresses, and preferences.
				"""

			behaviors: [
				{
					name:   "get-full-profile"
					intent: "Retrieve complete user profile with all nested data"

					request: {
						method: "GET"
						path:   "/users/usr_12345/profile"
						headers: {
							"Authorization": "Bearer ${auth_token}"
						}
					}

					response: {
						status: 200

						example: {
							user: {
								id:    "usr_12345"
								email: "customer@example.com"
								profile: {
									first_name: "Jane"
									last_name:  "Smith"
									phone:      "+1-555-123-4567"
									address: {
										street:  "123 Main St"
										city:    "Seattle"
										state:   "WA"
										zip:     "98101"
										country: "US"
									}
									preferences: {
										newsletter: true
										language:   "en"
										timezone:   "America/Los_Angeles"
									}
								}
							}
						}

						checks: {
							"user.id": {
								rule: "string matching usr_[a-z0-9]+"
								why:  "User IDs follow standard format"
							}
							"user.email": {
								rule: "email"
								why:  "Email must be valid format"
							}
							"user.profile.first_name": {
								rule: "non-empty string"
								why:  "First name is required"
							}
							"user.profile.last_name": {
								rule: "non-empty string"
								why:  "Last name is required"
							}
							"user.profile.address.city": {
								rule: "non-empty string"
								why:  "City is required for shipping"
							}
							"user.profile.address.state": {
								rule: "string matching [A-Z]{2}"
								why:  "State code must be 2 uppercase letters"
							}
							"user.profile.address.zip": {
								rule: "string matching [0-9]{5}(-[0-9]{4})?"
								why:  "ZIP code must be valid US format"
							}
							"user.profile.address.country": {
								rule: "one of [\"US\", \"CA\", \"MX\"]"
								why:  "Currently only support North America"
							}
							"user.profile.preferences.language": {
								rule: "string matching [a-z]{2}"
								why:  "Language code is ISO 639-1"
							}
							"user.profile.preferences.newsletter": {
								rule: "boolean"
								why:  "Newsletter preference must be boolean"
							}
						}
					}

					captures: {
						user_id: "response.body.user.id"
					}
				},
				{
					name:   "update-nested-address"
					intent: "Update specific nested address fields"

					requires: ["get-full-profile"]

					request: {
						method: "PATCH"
						path:   "/users/${user_id}/profile"
						headers: {
							"Authorization": "Bearer ${auth_token}"
						}
						body: {
							profile: {
								address: {
									street: "456 Oak Ave"
									city:   "Portland"
									state:  "OR"
									zip:    "97201"
								}
							}
						}
					}

					response: {
						status: 200

						example: {
							user: {
								id: "usr_12345"
								profile: {
									address: {
										street:  "456 Oak Ave"
										city:    "Portland"
										state:   "OR"
										zip:     "97201"
										country: "US"
									}
								}
							}
						}

						checks: {
							"user.profile.address.street": {
								rule: "equals 456 Oak Ave"
								why:  "Street was updated"
							}
							"user.profile.address.city": {
								rule: "equals Portland"
								why:  "City was updated"
							}
							"user.profile.address.state": {
								rule: "equals OR"
								why:  "State was updated"
							}
							"user.profile.address.country": {
								rule: "equals US"
								why:  "Country persists from previous value"
							}
						}
					}
				},
			]
		},
		{
			name: "Order History"

			description: """
				Order history with nested items, products, and shipping info.
				"""

			behaviors: [
				{
					name:   "get-order-with-nested-items"
					intent: "Retrieve order with deeply nested product information"

					request: {
						method: "GET"
						path:   "/orders/ord_xyz789"
						headers: {
							"Authorization": "Bearer ${auth_token}"
						}
					}

					response: {
						status: 200

						example: {
							order: {
								id:     "ord_xyz789"
								status: "shipped"
								items: [
									{
										quantity: 2
										product: {
											id:   "prod_abc"
											name: "Widget Pro"
											category: {
												id:   "cat_electronics"
												name: "Electronics"
												parent: {
													id:   "cat_all"
													name: "All Products"
												}
											}
											pricing: {
												base:     99.99
												discount: 10.00
												final:    89.99
											}
										}
									},
								]
								shipping: {
									carrier: "UPS"
									tracking: {
										number: "1Z999AA10123456784"
										url:    "https://ups.com/track/1Z999AA10123456784"
										events: [
											{
												timestamp: "2024-01-15T10:30:00Z"
												location:  "Seattle, WA"
												status:    "In Transit"
											},
										]
									}
									address: {
										recipient: "Jane Smith"
										street:    "123 Main St"
										city:      "Seattle"
										state:     "WA"
										zip:       "98101"
									}
								}
								totals: {
									subtotal: 179.98
									tax:      16.20
									shipping: 9.99
									total:    206.17
								}
							}
						}

						checks: {
							"order.id": {
								rule: "string matching ord_[a-z0-9]+"
								why:  "Order IDs follow standard format"
							}
							"order.status": {
								rule: "one of [\"pending\", \"confirmed\", \"shipped\", \"delivered\"]"
								why:  "Status must be a known value"
							}
							"order.items": {
								rule: "non-empty array"
								why:  "Order must have at least one item"
							}
							"order.items[0].quantity": {
								rule: "integer >= 1"
								why:  "Quantity must be positive"
							}
							"order.items[0].product.id": {
								rule: "string matching prod_[a-z0-9]+"
								why:  "Product ID format"
							}
							"order.items[0].product.name": {
								rule: "non-empty string"
								why:  "Product must have a name"
							}
							"order.items[0].product.category.id": {
								rule: "string starting with cat_"
								why:  "Category IDs are prefixed"
							}
							"order.items[0].product.category.parent.id": {
								rule: "string starting with cat_"
								why:  "Parent category also prefixed"
							}
							"order.items[0].product.pricing.final": {
								rule: "number between 0.0 and 10000.0"
								why:  "Final price must be reasonable"
							}
							"order.shipping.tracking.number": {
								rule: "non-empty string"
								why:  "Tracking number required when shipped"
							}
							"order.shipping.tracking.events": {
								rule: "non-empty array"
								why:  "Should have at least one tracking event"
							}
							"order.shipping.tracking.events[0].timestamp": {
								rule: "valid ISO8601 datetime"
								why:  "Event timestamp must be valid"
							}
							"order.shipping.address.city": {
								rule: "non-empty string"
								why:  "Shipping city required"
							}
							"order.totals.total": {
								rule: "number between 0.0 and 100000.0"
								why:  "Total must be reasonable"
							}
						}
					}
				},
				{
					name:   "list-orders-summary"
					intent: "List orders with minimal nested data"

					request: {
						method: "GET"
						path:   "/users/usr_12345/orders"
						headers: {
							"Authorization": "Bearer ${auth_token}"
						}
					}

					response: {
						status: 200

						example: {
							orders: [
								{
									id:         "ord_abc123"
									status:     "delivered"
									item_count: 3
									total:      150.00
									created_at: "2024-01-10T08:00:00Z"
								},
								{
									id:         "ord_def456"
									status:     "shipped"
									item_count: 1
									total:      89.99
									created_at: "2024-01-14T12:00:00Z"
								},
							]
							pagination: {
								page:        1
								per_page:    20
								total_pages: 5
								total_items: 87
							}
						}

						checks: {
							"orders": {
								rule: "array"
								why:  "Orders is an array (may be empty)"
							}
							"orders[0].id": {
								rule: "string matching ord_[a-z0-9]+"
								why:  "Order ID format"
							}
							"orders[0].total": {
								rule: "number between 0.0 and 100000.0"
								why:  "Order total must be reasonable"
							}
							"pagination.page": {
								rule: "integer >= 1"
								why:  "Page number starts at 1"
							}
							"pagination.total_pages": {
								rule: "integer >= 0"
								why:  "Total pages can be zero if empty"
							}
						}
					}
				},
			]
		},
		{
			name: "Organization Hierarchy"

			description: """
				Organizational data with deep nesting for company structures.
				"""

			behaviors: [
				{
					name:   "get-org-structure"
					intent: "Retrieve deeply nested organizational hierarchy"

					request: {
						method: "GET"
						path:   "/organizations/org_acme/structure"
						headers: {
							"Authorization": "Bearer ${auth_token}"
						}
					}

					response: {
						status: 200

						example: {
							organization: {
								id:   "org_acme"
								name: "Acme Corp"
								headquarters: {
									address: {
										city:    "San Francisco"
										country: "US"
									}
								}
								departments: [
									{
										id:   "dept_eng"
										name: "Engineering"
										head: {
											id:    "emp_001"
											name:  "Alice Johnson"
											title: "VP Engineering"
										}
										teams: [
											{
												id:   "team_backend"
												name: "Backend"
												lead: {
													id:   "emp_010"
													name: "Bob Wilson"
												}
											},
										]
									},
								]
							}
						}

						checks: {
							"organization.id": {
								rule: "string starting with org_"
								why:  "Organization ID prefix"
							}
							"organization.headquarters.address.country": {
								rule: "string matching [A-Z]{2}"
								why:  "Country is ISO code"
							}
							"organization.departments": {
								rule: "non-empty array"
								why:  "Must have at least one department"
							}
							"organization.departments[0].id": {
								rule: "string starting with dept_"
								why:  "Department ID prefix"
							}
							"organization.departments[0].head.id": {
								rule: "string starting with emp_"
								why:  "Employee ID prefix"
							}
							"organization.departments[0].teams[0].id": {
								rule: "string starting with team_"
								why:  "Team ID prefix"
							}
							"organization.departments[0].teams[0].lead.name": {
								rule: "non-empty string"
								why:  "Team lead must have name"
							}
						}
					}
				},
			]
		},
	]

	rules: [
		{
			name:        "nested-id-consistency"
			description: "All IDs at any nesting level must be prefixed strings"

			check: {
				fields_must_not_exist: ["id"]
			}
		},
	]

	anti_patterns: [
		{
			name:        "flat-data-model"
			description: "Don't flatten nested relationships into the parent"

			bad_example: {
				user_id:                 "usr_123"
				user_profile_first_name: "Jane"
				user_profile_addr_city:  "Seattle"
			}

			good_example: {
				user: {
					id: "usr_123"
					profile: {
						first_name: "Jane"
						address: {
							city: "Seattle"
						}
					}
				}
			}

			why: """
				Nested structure is more intuitive and allows for proper
				typing. Flat keys with underscores are hard to parse.
				"""
		},
		{
			name:        "deep-nesting-without-purpose"
			description: "Don't nest more than 4-5 levels without good reason"

			bad_example: {
				a: {b: {c: {d: {e: {f: {value: 1}}}}}}
			}

			good_example: {
				entity: {
					metadata: {
						nested_value: 1
					}
				}
			}

			why: """
				Excessive nesting makes APIs hard to consume and
				validation rules difficult to write. Keep it practical.
				"""
		},
	]

	ai_hints: {
		implementation: {
			suggested_stack: ["Python", "FastAPI", "MongoDB"]
		}

		entities: {
			user: {
				fields: {
					id:      "string, prefixed 'usr_'"
					email:   "string, valid email"
					profile: "nested object with name, address, preferences"
				}
			}
			order: {
				fields: {
					id:       "string, prefixed 'ord_'"
					items:    "array of line items with nested products"
					shipping: "nested object with tracking and address"
					totals:   "nested object with subtotal, tax, shipping, total"
				}
			}
		}

		security: {
			password_hashing: "bcrypt with cost factor >= 10"
			jwt_algorithm:    "HS256 or RS256"
			jwt_expiry:       "1 hour minimum"
			rate_limiting:    "1000 requests per hour per user"
		}

		pitfalls: [
			"Validate at every nesting level, not just top-level",
			"Handle missing intermediate paths gracefully",
			"Don't assume array indexes exist",
			"Consider partial updates for deeply nested data",
		]
	}
}
