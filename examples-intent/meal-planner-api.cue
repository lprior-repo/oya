package meal_planner_api

import "github.com/intent-cli/intent/schema:intent"

spec: intent.#Spec & {
	name: "Meal Planner API"

	description: """
		A meal planning API that allows users to scrape recipes from websites,
		store them in a structured format, create meal plans, and export
		everything to JSON. Recipes are scraped from external URLs and
		normalized into a consistent schema.
		"""

	audience: "Health-conscious individuals and families planning weekly meals"

	success_criteria: [
		"Users can scrape recipes from popular recipe websites",
		"Scraped recipes are normalized to a consistent JSON schema",
		"Users can create and manage meal plans",
		"All data can be exported to JSON files",
		"Recipes include nutritional information when available",
	]

	config: {
		base_url:   "http://localhost:8080"
		timeout_ms: 10000
	}

	features: [
		{
			name: "Recipe Management"

			description: """
				Core recipe operations: scraping from URLs, listing, retrieving,
				and deleting recipes. Scraped recipes are normalized to include
				title, ingredients, instructions, prep time, cook time, servings,
				and optional nutritional information.
				"""

			behaviors: [
				{
					name:   "create-recipe"
					intent: "Create a new recipe directly via API"

					notes: """
						Direct recipe creation for programmatic use. The scrape
						endpoint is for extracting recipes from URLs, but this
						endpoint allows creating recipes from structured data.
						"""

					request: {
						method: "POST"
						path:   "/recipes"
						body: {
							title:      "Famous Butter Chicken"
							source_url: "https://example.com/butter-chicken"
							ingredients: [
								"1 lb chicken breast",
								"2 tbsp butter",
								"1 cup tomato sauce",
								"1/2 cup heavy cream",
								"2 tsp garam masala",
							]
							instructions: [
								"Marinate the chicken in yogurt and spices for 2 hours",
								"Grill or bake the chicken until cooked through",
								"Prepare the butter sauce with tomatoes and cream",
								"Add the chicken to the sauce and simmer for 10 minutes",
							]
							prep_time_minutes:  30
							cook_time_minutes:  45
							servings:           4
							tags: ["indian", "chicken", "dinner"]
						}
					}

					response: {
						status: 201

						example: {
							id:          "rcp_abc123xyz"
							title:       "Famous Butter Chicken"
							source_url:  "https://example.com/butter-chicken"
							ingredients: [
								"1 lb chicken breast",
								"2 tbsp butter",
								"1 cup tomato sauce",
							]
							instructions: [
								"Marinate the chicken in yogurt and spices for 2 hours",
								"Grill or bake the chicken until cooked through",
							]
							prep_time_minutes:  30
							cook_time_minutes:  45
							servings:           4
							tags:       ["indian", "chicken", "dinner"]
							created_at: "2024-01-15T10:30:00Z"
						}

						checks: {
							"id": {
								rule: "string matching rcp_[a-z0-9]+"
								why:  "Recipe IDs are prefixed for debuggability"
							}
							"title": {
								rule: "equals Famous Butter Chicken"
								why:  "Confirms title was saved correctly"
							}
							"source_url": {
								rule: "uri"
								why:  "Source URL must be valid for attribution"
							}
							"ingredients": {
								rule: "non-empty array"
								why:  "Recipes must have at least one ingredient"
							}
							"instructions": {
								rule: "non-empty array"
								why:  "Recipes must have cooking instructions"
							}
							"servings": {
								rule: "equals 4"
								why:  "Confirms servings saved correctly"
							}
							"created_at": {
								rule: "valid ISO8601 datetime"
								why:  "Timestamp for when recipe was created"
							}
						}
					}

					captures: {
						recipe_id: "response.body.id"
					}
				},
				{
					name:   "scrape-recipe-invalid-url"
					intent: "Reject invalid or unreachable URLs"

					request: {
						method: "POST"
						path:   "/recipes/scrape"
						body: {
							url: "not-a-valid-url"
						}
					}

					response: {
						status: 400

						checks: {
							"error.code": {
								rule: "equals INVALID_URL"
								why:  "Clear error code for client handling"
							}
							"error.message": {
								rule: "non-empty string"
								why:  "Human-readable error message"
							}
						}
					}
				},
				{
					name:   "scrape-recipe-unsupported-site"
					intent: "Handle websites that cannot be scraped"

					request: {
						method: "POST"
						path:   "/recipes/scrape"
						body: {
							url: "https://example.com/some-random-page"
						}
					}

					response: {
						status: 422

						example: {
							error: {
								code:    "RECIPE_NOT_FOUND"
								message: "Could not extract recipe data from this URL"
								hint:    "Try a URL from AllRecipes, Food Network, or BBC Good Food"
							}
						}

						checks: {
							"error.code": {
								rule: "one of [\"RECIPE_NOT_FOUND\", \"UNSUPPORTED_SITE\"]"
								why:  "Distinguish between no recipe and unsupported site"
							}
							"error.hint": {
								rule: "non-empty string"
								why:  "Actionable guidance for the user"
							}
						}
					}
				},
				{
					name:   "list-all-recipes"
					intent: "Get all saved recipes"

					requires: ["create-recipe"]

					request: {
						method: "GET"
						path:   "/recipes"
					}

					response: {
						status: 200

						checks: {
							"recipes": {
								rule: "non-empty array"
								why:  "Should have at least the scraped recipe"
							}
							"total": {
								rule: "integer >= 1"
								why:  "Count matches array length"
							}
						}
					}
				},
				{
					name:   "get-recipe-by-id"
					intent: "Retrieve a specific recipe by ID"

					requires: ["create-recipe"]

					request: {
						method: "GET"
						path:   "/recipes/${recipe_id}"
					}

					response: {
						status: 200

						checks: {
							"id": {
								rule: "equals ${recipe_id}"
								why:  "Returns the requested recipe"
							}
							"title": {
								rule: "non-empty string"
								why:  "Recipe has a title"
							}
						}
					}
				},
				{
					name:   "get-recipe-not-found"
					intent: "Return 404 for non-existent recipe"

					request: {
						method: "GET"
						path:   "/recipes/rcp_nonexistent999"
					}

					response: {
						status: 404

						checks: {
							"error.code": {rule: "equals NOT_FOUND"}
						}
					}
				},
				{
					name:   "delete-recipe"
					intent: "Remove a recipe from the collection"

					// Run last since it removes the recipe used by other tests
					requires: ["export-meal-plan"]

					request: {
						method: "DELETE"
						path:   "/recipes/${recipe_id}"
					}

					response: {
						status: 204

						checks: {}
					}
				},
			]
		},
		{
			name: "Meal Planning"

			description: """
				Create and manage meal plans. A meal plan assigns recipes to
				specific days and meal types (breakfast, lunch, dinner, snack).
				Plans can span any date range and include shopping lists.
				"""

			behaviors: [
				{
					name:   "create-meal-plan"
					intent: "Create a new meal plan for a week"

					request: {
						method: "POST"
						path:   "/meal-plans"
						body: {
							name:       "Healthy Week"
							start_date: "2024-01-15"
							end_date:   "2024-01-21"
						}
					}

					response: {
						status: 201

						example: {
							id:         "plan_xyz789"
							name:       "Healthy Week"
							start_date: "2024-01-15"
							end_date:   "2024-01-21"
							meals:      []
							created_at: "2024-01-15T10:30:00Z"
						}

						checks: {
							"id": {
								rule: "string matching plan_[a-z0-9]+"
								why:  "Plan IDs are prefixed"
							}
							"name": {
								rule: "equals Healthy Week"
								why:  "Confirms name was saved"
							}
							"start_date": {
								rule: "equals 2024-01-15"
								why:  "Confirms start date"
							}
							"end_date": {
								rule: "equals 2024-01-21"
								why:  "Confirms end date"
							}
							"meals": {
								rule: "array"
								why:  "Starts with empty meals list"
							}
						}
					}

					captures: {
						plan_id: "response.body.id"
					}
				},
				{
					name:   "create-meal-plan-invalid-dates"
					intent: "Reject meal plan with end date before start date"

					request: {
						method: "POST"
						path:   "/meal-plans"
						body: {
							name:       "Bad Plan"
							start_date: "2024-01-21"
							end_date:   "2024-01-15"
						}
					}

					response: {
						status: 400

						checks: {
							"error.code": {rule: "equals INVALID_DATE_RANGE"}
						}
					}
				},
				{
					name:   "add-meal-to-plan"
					intent: "Schedule a recipe for a specific meal"

					requires: ["create-meal-plan", "create-recipe"]

					request: {
						method: "POST"
						path:   "/meal-plans/${plan_id}/meals"
						body: {
							recipe_id: "${recipe_id}"
							date:      "2024-01-15"
							meal_type: "dinner"
							servings:  4
						}
					}

					response: {
						status: 201

						example: {
							id:        "meal_abc123"
							recipe_id: "rcp_abc123xyz"
							date:      "2024-01-15"
							meal_type: "dinner"
							servings:  4
							recipe: {
								id:    "rcp_abc123xyz"
								title: "Famous Butter Chicken"
							}
						}

						checks: {
							"id": {
								rule: "string matching meal_[a-z0-9]+"
								why:  "Meal entries have unique IDs"
							}
							"meal_type": {
								rule: "one of [\"breakfast\", \"lunch\", \"dinner\", \"snack\"]"
								why:  "Valid meal type categories"
							}
							"servings": {
								rule: "integer >= 1"
								why:  "Must serve at least one person"
							}
							"recipe.title": {
								rule: "non-empty string"
								why:  "Includes recipe details for convenience"
							}
						}
					}

					captures: {
						meal_id: "response.body.id"
					}
				},
				{
					name:   "get-meal-plan"
					intent: "Retrieve a meal plan with all scheduled meals"

					requires: ["add-meal-to-plan"]

					request: {
						method: "GET"
						path:   "/meal-plans/${plan_id}"
					}

					response: {
						status: 200

						checks: {
							"id": {
								rule: "equals ${plan_id}"
								why:  "Returns requested plan"
							}
							"meals": {
								rule: "non-empty array"
								why:  "Has the meal we added"
							}
						}
					}
				},
				{
					name:   "generate-shopping-list"
					intent: "Generate aggregated shopping list from meal plan"

					requires: ["add-meal-to-plan"]

					request: {
						method: "GET"
						path:   "/meal-plans/${plan_id}/shopping-list"
					}

					response: {
						status: 200

						example: {
							plan_id: "plan_xyz789"
							items: [
								{
									ingredient: "chicken breast"
									quantity:   "1 lb"
									recipes:    ["Famous Butter Chicken"]
								},
								{
									ingredient: "butter"
									quantity:   "2 tbsp"
									recipes:    ["Famous Butter Chicken"]
								},
							]
							generated_at: "2024-01-15T10:35:00Z"
						}

						checks: {
							"plan_id": {
								rule: "equals ${plan_id}"
								why:  "Shopping list for the correct plan"
							}
							"items": {
								rule: "non-empty array"
								why:  "Has ingredients from scheduled meals"
							}
							"generated_at": {
								rule: "valid ISO8601 datetime"
								why:  "Timestamp for freshness"
							}
						}
					}
				},
			]
		},
		{
			name: "Data Export"

			description: """
				Export recipes and meal plans to JSON files for backup,
				sharing, or use in other applications.
				"""

			behaviors: [
				{
					name:   "export-all-recipes"
					intent: "Export all recipes to a JSON file"

					requires: ["create-recipe"]

					request: {
						method: "GET"
						path:   "/export/recipes"
						query: {
							format: "json"
						}
					}

					response: {
						status: 200

						headers: {
							"Content-Type": "application/json"
						}

						checks: {
							"recipes": {
								rule: "non-empty array"
								why:  "Contains all saved recipes"
							}
							"exported_at": {
								rule: "valid ISO8601 datetime"
								why:  "Export timestamp for versioning"
							}
							"version": {
								rule: "string matching ^1\\.[0-9]+$"
								why:  "Schema version for compatibility"
							}
						}
					}
				},
				{
					name:   "export-meal-plan"
					intent: "Export a specific meal plan to JSON"

					requires: ["add-meal-to-plan"]

					request: {
						method: "GET"
						path:   "/export/meal-plans/${plan_id}"
					}

					response: {
						status: 200

						checks: {
							"meal_plan.id": {
								rule: "equals ${plan_id}"
								why:  "Exports the correct plan"
							}
							"meal_plan.meals": {
								rule: "non-empty array"
								why:  "Includes scheduled meals"
							}
							"recipes": {
								rule: "non-empty array"
								why:  "Includes full recipe data for offline use"
							}
						}
					}
				},
			]
		},
	]

	rules: [
		{
			name:        "consistent-error-format"
			description: "All errors return structured error objects"

			when: {status: ">= 400"}

			check: {
				fields_must_exist: ["error.code", "error.message"]
			}

			example: {
				error: {
					code:    "ERROR_CODE"
					message: "Human readable description"
				}
			}
		},
		{
			name:        "content-type-json"
			description: "All responses must have Content-Type header"

			check: {
				header_must_exist: "Content-Type"
			}
		},
		{
			name:        "no-internal-errors-exposed"
			description: "Internal implementation details not leaked"

			when: {status: ">= 500"}

			check: {
				body_must_not_contain: ["stack trace", "panic", "runtime error", "sql:", "pq:"]
			}
		},
	]

	anti_patterns: [
		{
			name:        "sequential-ids"
			description: "IDs should not be sequential integers"

			bad_example: {
				id: 1
			}

			good_example: {
				id: "rcp_x7k9m2p4q"
			}

			why: """
				Sequential IDs reveal business metrics and enable enumeration
				attacks. Use prefixed random strings instead.
				"""
		},
		{
			name:        "null-instead-of-empty-array"
			description: "Empty collections should be empty arrays, not null"

			bad_example: {
				ingredients: null
			}

			good_example: {
				ingredients: []
			}

			why: "Null vs empty array causes client-side null checks and crashes"
		},
		{
			name:        "inconsistent-time-format"
			description: "All timestamps must use ISO8601 format"

			bad_example: {
				created_at: "Jan 15, 2024 10:30 AM"
			}

			good_example: {
				created_at: "2024-01-15T10:30:00Z"
			}

			why: "ISO8601 is machine-parseable and timezone-aware"
		},
	]

	ai_hints: {
		implementation: {
			suggested_stack: ["Go", "net/http", "SQLite", "colly (web scraping)"]
		}

		entities: {
			recipe: {
				fields: {
					id:                 "string, prefixed 'rcp_', randomly generated"
					title:              "string, extracted from webpage"
					source_url:         "string, original URL"
					ingredients:        "[]string, list of ingredient lines"
					instructions:       "[]string, list of steps"
					prep_time_minutes:  "int, optional"
					cook_time_minutes:  "int, optional"
					total_time_minutes: "int, calculated or extracted"
					servings:           "int, number of servings"
					nutrition:          "object, optional nutritional info"
					tags:               "[]string, categorization"
					created_at:         "datetime, when scraped"
				}
			}
			meal_plan: {
				fields: {
					id:         "string, prefixed 'plan_'"
					name:       "string, user-provided name"
					start_date: "date, YYYY-MM-DD format"
					end_date:   "date, YYYY-MM-DD format"
					meals:      "[]meal, scheduled meals"
					created_at: "datetime"
				}
			}
			meal: {
				fields: {
					id:        "string, prefixed 'meal_'"
					recipe_id: "string, reference to recipe"
					date:      "date, when to prepare"
					meal_type: "enum: breakfast, lunch, dinner, snack"
					servings:  "int, portions for this meal"
				}
			}
		}

		security: {
			password_hashing: "N/A - no user authentication required"
			jwt_algorithm:    "N/A - no user authentication required"
			jwt_expiry:       "N/A - no user authentication required"
			rate_limiting:    "100 requests per minute per IP for scraping"
		}

		pitfalls: [
			"Don't scrape sites that block bots - respect robots.txt",
			"Don't assume all recipe sites have the same structure",
			"Don't forget to handle network timeouts when scraping",
			"Don't store raw HTML - extract and normalize to JSON",
			"Don't forget ingredient quantity parsing is hard - keep original strings",
		]
	}
}
