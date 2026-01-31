package regex_rules

import "github.com/intent-cli/intent/schema:intent"

// Example: Regex Pattern Validation
// Demonstrates string matching, starting with, ending with, and containing rules

spec: intent.#Spec & {
	name: "Document Management API"

	description: """
		A document management API demonstrating various regex validation patterns.
		Shows how to validate IDs, file names, slugs, codes, and formatted strings.
		"""

	audience: "Content management systems"

	success_criteria: [
		"All identifiers follow predictable patterns",
		"File names and slugs are sanitized",
		"Codes and references match expected formats",
	]

	config: {
		base_url:   "http://localhost:8080"
		timeout_ms: 5000
	}

	features: [
		{
			name: "Document Operations"

			description: """
				Create and manage documents with validated naming patterns.
				"""

			behaviors: [
				{
					name:   "create-document"
					intent: "Create document with validated ID and slug"

					request: {
						method: "POST"
						path:   "/documents"
						headers: {
							"Authorization": "Bearer ${auth_token}"
						}
						body: {
							title:   "Getting Started Guide"
							content: "Welcome to our platform..."
						}
					}

					response: {
						status: 201

						example: {
							id:         "doc_a1b2c3d4e5"
							slug:       "getting-started-guide"
							title:      "Getting Started Guide"
							version:    "1.0.0"
							created_at: "2024-01-15T10:30:00Z"
						}

						checks: {
							"id": {
								rule: "string matching doc_[a-z0-9]{10}"
								why:  "Document IDs are prefixed with 'doc_' followed by 10 alphanumeric chars"
							}
							"slug": {
								rule: "string matching [a-z0-9]+(-[a-z0-9]+)*"
								why:  "Slugs must be lowercase kebab-case for URLs"
							}
							"version": {
								rule: "string matching [0-9]+\\.[0-9]+\\.[0-9]+"
								why:  "Version follows semantic versioning format"
							}
							"created_at": {
								rule: "valid ISO8601 datetime"
								why:  "Timestamps in ISO8601 format"
							}
						}
					}

					captures: {
						doc_id:   "response.body.id"
						doc_slug: "response.body.slug"
					}
				},
				{
					name:   "get-document-by-slug"
					intent: "Retrieve document using URL-safe slug"

					requires: ["create-document"]

					request: {
						method: "GET"
						path:   "/documents/by-slug/${doc_slug}"
					}

					response: {
						status: 200

						checks: {
							"id": {
								rule: "equals ${doc_id}"
								why:  "Retrieved by slug returns correct document"
							}
							"slug": {
								rule: "string matching [a-z0-9-]+"
								why:  "Slug only contains lowercase letters, numbers, hyphens"
							}
						}
					}
				},
			]
		},
		{
			name: "File Uploads"

			description: """
				File upload endpoints with extension and naming validation.
				"""

			behaviors: [
				{
					name:   "upload-image"
					intent: "Upload image with validated filename"

					request: {
						method: "POST"
						path:   "/files/images"
						headers: {
							"Content-Type":  "multipart/form-data"
							"Authorization": "Bearer ${auth_token}"
						}
					}

					response: {
						status: 201

						example: {
							file_id:       "file_img_abc123"
							original_name: "my-photo.jpg"
							stored_name:   "file_img_abc123.jpg"
							mime_type:     "image/jpeg"
							size_bytes:    245678
							url:           "https://cdn.example.com/images/file_img_abc123.jpg"
						}

						checks: {
							"file_id": {
								rule: "string matching file_img_[a-z0-9]+"
								why:  "Image file IDs have 'file_img_' prefix"
							}
							"original_name": {
								rule: "string ending with .jpg"
								why:  "Original filename preserved with extension"
							}
							"stored_name": {
								rule: "string matching file_img_[a-z0-9]+\\.(jpg|jpeg|png|gif|webp)"
								why:  "Stored name uses file ID with valid image extension"
							}
							"mime_type": {
								rule: "string starting with image/"
								why:  "MIME type must be an image type"
							}
							"url": {
								rule: "string starting with https://"
								why:  "CDN URLs must use HTTPS"
							}
						}
					}
				},
				{
					name:   "upload-document-file"
					intent: "Upload document with specific extension validation"

					request: {
						method: "POST"
						path:   "/files/documents"
						headers: {
							"Authorization": "Bearer ${auth_token}"
						}
					}

					response: {
						status: 201

						example: {
							file_id:       "file_doc_xyz789"
							original_name: "annual-report-2024.pdf"
							stored_name:   "file_doc_xyz789.pdf"
							mime_type:     "application/pdf"
							size_bytes:    1234567
						}

						checks: {
							"file_id": {
								rule: "string matching file_doc_[a-z0-9]+"
								why:  "Document file IDs have 'file_doc_' prefix"
							}
							"stored_name": {
								rule: "string matching file_doc_[a-z0-9]+\\.(pdf|doc|docx|txt|md)"
								why:  "Only allowed document formats"
							}
							"mime_type": {
								rule: "string matching (application/pdf|application/msword|text/plain|text/markdown)"
								why:  "MIME type matches allowed document types"
							}
						}
					}
				},
			]
		},
		{
			name: "Reference Codes"

			description: """
				Various business reference codes with strict formatting.
				"""

			behaviors: [
				{
					name:   "create-invoice"
					intent: "Create invoice with formatted reference number"

					request: {
						method: "POST"
						path:   "/invoices"
						headers: {
							"Authorization": "Bearer ${auth_token}"
						}
						body: {
							customer_id: "cust_123"
							amount:      150.00
						}
					}

					response: {
						status: 201

						example: {
							id:         "inv_20240115_0001"
							reference:  "INV-2024-00001"
							customer:   "cust_123"
							amount:     150.00
							status:     "pending"
							issued_at:  "2024-01-15T10:30:00Z"
							due_date:   "2024-02-15"
							payment_id: null
						}

						checks: {
							"id": {
								rule: "string matching inv_[0-9]{8}_[0-9]{4}"
								why:  "Invoice ID contains date and sequence number"
							}
							"reference": {
								rule: "string matching INV-[0-9]{4}-[0-9]{5}"
								why:  "Human-readable reference: INV-YYYY-NNNNN"
							}
							"customer": {
								rule: "string starting with cust_"
								why:  "Customer ID prefix"
							}
							"due_date": {
								rule: "string matching [0-9]{4}-[0-9]{2}-[0-9]{2}"
								why:  "Due date in YYYY-MM-DD format"
							}
							"payment_id": {
								rule: "absent"
								why:  "No payment yet on new invoice"
							}
						}
					}

					captures: {
						invoice_id: "response.body.id"
					}
				},
				{
					name:   "record-payment"
					intent: "Record payment with transaction reference"

					requires: ["create-invoice"]

					request: {
						method: "POST"
						path:   "/invoices/${invoice_id}/payments"
						headers: {
							"Authorization": "Bearer ${auth_token}"
						}
						body: {
							amount: 150.00
							method: "credit_card"
						}
					}

					response: {
						status: 201

						example: {
							payment_id:     "pay_cc_abc123xyz"
							transaction_id: "TXN20240115103045ABC"
							amount:         150.00
							method:         "credit_card"
							status:         "completed"
							receipt_url:    "https://payments.example.com/receipts/pay_cc_abc123xyz"
						}

						checks: {
							"payment_id": {
								rule: "string matching pay_(cc|bank|wire|check)_[a-z0-9]+"
								why:  "Payment ID includes method prefix"
							}
							"transaction_id": {
								rule: "string matching TXN[0-9]{14}[A-Z]{3}"
								why:  "Transaction ID: TXN + timestamp + 3 letter code"
							}
							"method": {
								rule: "one of [\"credit_card\", \"bank_transfer\", \"wire\", \"check\"]"
								why:  "Only supported payment methods"
							}
							"receipt_url": {
								rule: "string containing /receipts/"
								why:  "Receipt URL contains receipts path"
							}
						}
					}
				},
				{
					name:   "create-shipping-label"
					intent: "Create shipping label with carrier-specific tracking"

					request: {
						method: "POST"
						path:   "/shipments/labels"
						headers: {
							"Authorization": "Bearer ${auth_token}"
						}
						body: {
							carrier:  "ups"
							order_id: "ord_abc123"
						}
					}

					response: {
						status: 201

						example: {
							label_id:        "lbl_ups_20240115_001"
							carrier:         "ups"
							tracking_number: "1Z999AA10123456784"
							service_code:    "UPS-GROUND"
							label_url:       "https://labels.example.com/lbl_ups_20240115_001.pdf"
						}

						checks: {
							"label_id": {
								rule: "string matching lbl_(ups|fedex|usps|dhl)_[0-9]{8}_[0-9]{3}"
								why:  "Label ID includes carrier and date"
							}
							"tracking_number": {
								rule: "string matching 1Z[A-Z0-9]{16}"
								why:  "UPS tracking numbers start with 1Z and are 18 chars"
							}
							"service_code": {
								rule: "string matching (UPS|FEDEX|USPS|DHL)-[A-Z]+"
								why:  "Service code is CARRIER-SERVICE format"
							}
							"label_url": {
								rule: "string ending with .pdf"
								why:  "Labels are PDF files"
							}
						}
					}
				},
			]
		},
		{
			name: "User Identifiers"

			description: """
				Various user identifier formats with specific patterns.
				"""

			behaviors: [
				{
					name:   "create-api-key"
					intent: "Generate API key with specific format"

					request: {
						method: "POST"
						path:   "/users/me/api-keys"
						headers: {
							"Authorization": "Bearer ${auth_token}"
						}
						body: {
							name: "Production Key"
						}
					}

					response: {
						status: 201

						example: {
							key_id:     "key_live_abc123"
							api_key:    "example_key_xxxxxxxxxxxxxxxxxxxxxxxxxxxx"
							name:       "Production Key"
							prefix:     "example_"
							created_at: "2024-01-15T10:30:00Z"
						}

						checks: {
							"key_id": {
								rule: "string matching key_(live|test)_[a-z0-9]+"
								why:  "Key ID indicates environment"
							}
							"api_key": {
								rule: "string matching example_key_[x]{32}"
								why:  "API key format: example_key_32chars"
							}
							"prefix": {
								rule: "string matching example_"
								why:  "Prefix for identification"
							}
						}
					}

					notes: """
						The full api_key is only shown once at creation time.
						Store it securely as it cannot be retrieved later.
						"""
				},
				{
					name:   "validate-phone-number"
					intent: "Validate and format phone number"

					request: {
						method: "POST"
						path:   "/validation/phone"
						body: {
							phone: "+15551234567"
						}
					}

					response: {
						status: 200

						example: {
							valid:         true
							original:      "+15551234567"
							formatted:     "+1 (555) 123-4567"
							country_code:  "1"
							national:      "(555) 123-4567"
							e164:          "+15551234567"
							type:          "mobile"
							carrier:       "Example Wireless"
						}

						checks: {
							"e164": {
								rule: "string matching \\+[1-9][0-9]{6,14}"
								why:  "E.164 format: + followed by up to 15 digits"
							}
							"formatted": {
								rule: "string matching \\+[0-9]+ \\([0-9]{3}\\) [0-9]{3}-[0-9]{4}"
								why:  "US formatted: +1 (XXX) XXX-XXXX"
							}
							"country_code": {
								rule: "string matching [1-9][0-9]{0,2}"
								why:  "Country code is 1-3 digits, no leading zero"
							}
							"type": {
								rule: "one of [\"mobile\", \"landline\", \"voip\", \"unknown\"]"
								why:  "Phone type classification"
							}
						}
					}
				},
				{
					name:   "validate-credit-card"
					intent: "Validate credit card with masked display"

					request: {
						method: "POST"
						path:   "/validation/credit-card"
						body: {
							number: "4111111111111111"
						}
					}

					response: {
						status: 200

						example: {
							valid:   true
							masked:  "**** **** **** 1111"
							last4:   "1111"
							brand:   "visa"
							type:    "credit"
							bin:     "411111"
							country: "US"
						}

						checks: {
							"masked": {
								rule: "string matching \\*{4} \\*{4} \\*{4} [0-9]{4}"
								why:  "Masked format shows only last 4 digits"
							}
							"last4": {
								rule: "string matching [0-9]{4}"
								why:  "Last 4 digits for display"
							}
							"brand": {
								rule: "one of [\"visa\", \"mastercard\", \"amex\", \"discover\"]"
								why:  "Supported card brands"
							}
							"bin": {
								rule: "string matching [0-9]{6}"
								why:  "Bank Identification Number is first 6 digits"
							}
						}
					}
				},
			]
		},
	]

	rules: [
		{
			name:        "id-format-consistency"
			description: "All IDs should follow prefix_identifier pattern"

			check: {
				body_must_not_contain: ["\"id\": 1", "\"id\": \"1\""]
			}
		},
	]

	anti_patterns: [
		{
			name:        "numeric-ids"
			description: "Don't use numeric IDs, use prefixed strings"

			bad_example: {
				id: 12345
			}

			good_example: {
				id: "doc_abc123xyz"
			}

			why: """
				Prefixed string IDs are self-documenting (you know it's a
				document) and don't leak information about record counts.
				"""
		},
		{
			name:        "inconsistent-id-patterns"
			description: "Don't mix ID formats across the API"

			bad_example: {
				user_id:    "USR-123"
				order_id:   "ord_456"
				product_id: "PROD:789"
			}

			good_example: {
				user_id:    "usr_abc123"
				order_id:   "ord_def456"
				product_id: "prod_ghi789"
			}

			why: """
				Consistent ID patterns make parsing predictable and
				help with debugging and log analysis.
				"""
		},
		{
			name:        "unvalidated-slugs"
			description: "Don't accept any string as a slug"

			bad_example: {
				slug: "My Document Title!!!"
			}

			good_example: {
				slug: "my-document-title"
			}

			why: """
				Slugs should be URL-safe: lowercase, alphanumeric, and
				hyphens only. Special characters cause encoding issues.
				"""
		},
	]

	ai_hints: {
		implementation: {
			suggested_stack: ["TypeScript", "Express", "PostgreSQL"]
		}

		entities: {
			document: {
				fields: {
					id:      "string, 'doc_' + 10 random alphanumeric"
					slug:    "string, generated from title, lowercase kebab-case"
					version: "string, semantic version X.Y.Z"
				}
			}
			invoice: {
				fields: {
					id:        "string, 'inv_YYYYMMDD_NNNN' format"
					reference: "string, 'INV-YYYY-NNNNN' for humans"
				}
			}
		}

		security: {
			password_hashing: "bcrypt with cost factor >= 10 for user passwords"
			jwt_algorithm:    "HS256 with secure secret key"
			jwt_expiry:       "1 hour for access tokens, 7 days for refresh tokens"
			rate_limiting:    "100 requests per minute per user"
		}

		pitfalls: [
			"Validate regex patterns on input, not just output",
			"Be careful with regex escaping in different languages",
			"Consider Unicode in regex patterns",
			"Test edge cases like empty strings",
			"Compile regex once, not on every request",
		]
	}
}
