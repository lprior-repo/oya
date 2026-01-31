# Intent CLI JSON Output Schema

Complete reference for Intent CLI's machine-readable JSON output format. All analysis commands return consistent, strongly-typed JSON responses designed for AI agents and CI/CD integration.

## Table of Contents

1. [Overview](#overview)
2. [Base Response Schema](#base-response-schema)
3. [Common Types](#common-types)
4. [Command-Specific Schemas](#command-specific-schemas)
5. [Error Handling](#error-handling)
6. [TypeScript Interfaces](#typescript-interfaces)
7. [Usage Examples](#usage-examples)

---

## Overview

### Design Principles

- **Consistent Structure**: All commands return the same base schema
- **Type Safety**: Strong typing enables validation and IDE support
- **AI-Friendly**: Includes `next_actions` for workflow guidance
- **Correlation**: UUID `correlation_id` for request tracing
- **Machine-Readable**: No ANSI codes or interactive UI in JSON mode

### Supported Commands

Commands with JSON output:

| Command | Action Type | Description |
|---------|------------|-------------|
| `validate` | `validate_result` | Validate CUE spec syntax |
| `check` | `check_result` | Execute HTTP tests |
| `quality` | `quality_report` | Multi-dimensional quality scoring |
| `coverage` | `coverage_report` | OWASP + edge case analysis |
| `gaps` | `gaps_report` | Mental model gap detection |
| `invert` | `inversion_report` | Failure mode analysis |
| `effects` | `effects_report` | Second-order effects analysis |
| `doctor` | `doctor_report` | Prioritized improvements |
| `lint` | `lint_result` | Anti-pattern detection |
| `improve` | `improve_result` | Improvement suggestions |
| `prompt` | `prompt_result` | AI implementation prompts |
| `feedback` | `feedback_result` | Fix beads from failures |
| `beads` | `beads_result` | Work item generation |
| `ready start` | `ready_start_result` | Start ready phase session |
| `ready check` | `ready_check_result` | Check ready phase status |
| `ready critique` | `ready_critique_result` | Generate critique questions |
| `ready respond` | `ready_respond_result` | Process critique responses |
| `ready agree` | `ready_agree_result` | Finalize ready phase |

---

## Base Response Schema

All JSON responses follow this structure:

```json
{
  "success": true,
  "action": "quality_report",
  "command": "quality",
  "data": { ... },
  "errors": [],
  "next_actions": [
    {
      "command": "intent gaps spec.cue",
      "reason": "Find coverage gaps"
    }
  ],
  "metadata": {
    "timestamp": "2026-01-25T14:30:00Z",
    "version": "0.1.0",
    "exit_code": 0,
    "correlation_id": "550e8400-e29b-41d4-a716-446655440000",
    "duration_ms": 0
  },
  "spec_path": "examples/user-api.cue"
}
```

### Field Definitions

| Field | Type | Description |
|-------|------|-------------|
| `success` | `boolean` | `true` if command achieved its goal, `false` on error |
| `action` | `string` | Type of result (e.g., `"check_result"`, `"error"`) |
| `command` | `string` | Command that produced this output (e.g., `"check"`, `"quality"`) |
| `data` | `object` | Command-specific output (schema varies by command) |
| `errors` | `array` | List of structured errors (empty if `success: true`) |
| `next_actions` | `array` | Suggested follow-up commands for workflow guidance |
| `metadata` | `object` | Timestamp, version, exit code, correlation ID, duration |
| `spec_path` | `string\|null` | Path to spec file if applicable |

---

## Common Types

### JsonError

Structured error information:

```json
{
  "code": "parse_error",
  "message": "Invalid CUE syntax at line 42",
  "location": "examples/user-api.cue:42",
  "fix_hint": "Missing closing brace",
  "fix_command": "intent doctor spec.cue"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `code` | `string` | Yes | Machine-readable error code |
| `message` | `string` | Yes | Human-readable error message |
| `location` | `string\|null` | No | File path and line number |
| `fix_hint` | `string\|null` | No | Suggestion for fixing the error |
| `fix_command` | `string\|null` | No | Command to run for automated fix |

### NextAction

Workflow guidance for AI agents:

```json
{
  "command": "intent gaps spec.cue",
  "reason": "Find coverage gaps in mental models"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `command` | `string` | Full command to execute |
| `reason` | `string` | Why this action is recommended |

### JsonMetadata

Tracking and debugging information:

```json
{
  "timestamp": "2026-01-25T14:30:00Z",
  "version": "0.1.0",
  "exit_code": 0,
  "correlation_id": "550e8400-e29b-41d4-a716-446655440000",
  "duration_ms": 0
}
```

| Field | Type | Description |
|-------|------|-------------|
| `timestamp` | `string` | ISO 8601 timestamp |
| `version` | `string` | Intent CLI version |
| `exit_code` | `integer` | Unix exit code (0=success, 1=fail, 3=invalid, 4=error) |
| `correlation_id` | `string` | UUID v4 for request tracing |
| `duration_ms` | `integer` | Command execution time in milliseconds |

---

## Command-Specific Schemas

### validate

**Action**: `validate_result`

Validates CUE spec syntax and structure.

```json
{
  "success": true,
  "action": "validate_result",
  "command": "validate",
  "data": {
    "valid": true,
    "message": "Spec is valid",
    "spec": {
      "name": "User API",
      "description": "REST API for user management",
      "version": "1.0.0"
    }
  },
  "errors": [],
  "next_actions": [
    {
      "command": "intent quality spec.cue",
      "reason": "Analyze spec quality"
    }
  ],
  "metadata": { ... },
  "spec_path": "examples/user-api.cue"
}
```

**Data Schema**:
- `valid`: boolean - Whether spec is syntactically valid
- `message`: string - Validation result message
- `spec` (optional): object - Basic spec metadata if valid
  - `name`: string
  - `description`: string
  - `version`: string

---

### check

**Action**: `check_result`

Execute HTTP tests against API behaviors.

```json
{
  "success": true,
  "action": "check_result",
  "command": "check",
  "data": {
    "total": 5,
    "passed": 4,
    "failed": 1,
    "skipped": 0,
    "success": false,
    "duration_ms": 1234,
    "behaviors": [
      {
        "name": "Create user with valid data",
        "feature": "User Management",
        "status": "passed",
        "duration_ms": 245,
        "request": {
          "method": "POST",
          "path": "/users",
          "url": "http://localhost:8080/users"
        },
        "response": {
          "status": 201,
          "body": {
            "id": "user-123",
            "email": "test@example.com"
          }
        },
        "checks": [
          {
            "field": "status",
            "rule": "== 201",
            "expected": 201,
            "actual": 201,
            "passed": true,
            "why": "User creation returns 201 Created"
          }
        ]
      }
    ]
  },
  "errors": [],
  "next_actions": [
    {
      "command": "intent feedback --results check-output.json",
      "reason": "Generate fix beads for failures"
    }
  ],
  "metadata": { ... },
  "spec_path": "examples/user-api.cue"
}
```

**Data Schema**:
- `total`: integer - Total behaviors tested
- `passed`: integer - Passed behaviors
- `failed`: integer - Failed behaviors
- `skipped`: integer - Skipped behaviors
- `success`: boolean - Overall test suite result
- `duration_ms`: integer - Total execution time
- `behaviors`: array of BehaviorResult
  - `name`: string - Behavior name
  - `feature`: string - Feature name
  - `status`: "passed" | "failed" | "skipped"
  - `duration_ms`: integer
  - `request`: object
    - `method`: string - HTTP method
    - `path`: string - Request path
    - `url`: string - Full URL
  - `response`: object
    - `status`: integer - HTTP status code
    - `body` (optional): any - Response body
  - `checks`: array of CheckResult
    - `field`: string - Field being checked
    - `rule`: string - Validation rule
    - `expected`: any - Expected value
    - `actual`: any - Actual value
    - `passed`: boolean - Check result
    - `why`: string - Check explanation
  - `error` (optional): string - Error message if failed

---

### quality

**Action**: `quality_report`

Multi-dimensional quality scoring (coverage, clarity, testability, AI readiness).

```json
{
  "success": true,
  "action": "quality_report",
  "command": "quality",
  "data": {
    "overall_score": 85,
    "coverage_score": 90,
    "clarity_score": 80,
    "testability_score": 85,
    "ai_readiness_score": 85,
    "issues": [
      "3 behaviors missing response examples",
      "2 vague validation rules"
    ],
    "suggestions": [
      "Add response.example to all behaviors",
      "Refine validation rules to be more specific"
    ]
  },
  "errors": [],
  "next_actions": [
    {
      "command": "intent gaps spec.cue",
      "reason": "Find coverage gaps"
    },
    {
      "command": "intent invert spec.cue",
      "reason": "Analyze failure modes"
    }
  ],
  "metadata": { ... },
  "spec_path": "examples/user-api.cue"
}
```

**Data Schema**:
- `overall_score`: integer (0-100) - Aggregate quality score
- `coverage_score`: integer (0-100) - Test coverage completeness
- `clarity_score`: integer (0-100) - Specification clarity
- `testability_score`: integer (0-100) - How testable the spec is
- `ai_readiness_score`: integer (0-100) - Readiness for AI implementation
- `issues`: array of string - Identified problems
- `suggestions`: array of string - Improvement recommendations

---

### coverage

**Action**: `coverage_report`

OWASP Top 10 and edge case coverage analysis.

```json
{
  "success": true,
  "action": "coverage_report",
  "command": "coverage",
  "data": {
    "overall_score": 75.5,
    "methods": {
      "GET": 3,
      "POST": 2,
      "PUT": 1,
      "DELETE": 1
    },
    "status_codes": {
      "200": 3,
      "201": 2,
      "404": 1,
      "400": 1
    },
    "paths": {
      "/users": ["GET", "POST"],
      "/users/{id}": ["GET", "PUT", "DELETE"]
    },
    "edge_cases": {
      "tested": [
        "Invalid email format",
        "Duplicate user creation"
      ],
      "suggested": [
        "Extremely long email (>255 chars)",
        "SQL injection in name field"
      ]
    },
    "owasp": {
      "score": 70.0,
      "categories": {
        "A01:2021-Broken Access Control": true,
        "A02:2021-Cryptographic Failures": false,
        "A03:2021-Injection": true
      },
      "missing": [
        "A02:2021-Cryptographic Failures",
        "A07:2021-Identification and Authentication Failures"
      ]
    }
  },
  "errors": [],
  "next_actions": [
    {
      "command": "intent effects spec.cue",
      "reason": "Analyze second-order effects"
    },
    {
      "command": "intent doctor spec.cue",
      "reason": "Get prioritized improvements"
    }
  ],
  "metadata": { ... },
  "spec_path": "examples/user-api.cue"
}
```

**Data Schema**:
- `overall_score`: number (0-100) - Overall coverage percentage
- `methods`: object - HTTP method counts (key: method, value: count)
- `status_codes`: object - Status code counts (key: code, value: count)
- `paths`: object - Paths with their methods (key: path, value: array of methods)
- `edge_cases`: object
  - `tested`: array of string - Edge cases covered
  - `suggested`: array of string - Missing edge cases
- `owasp`: object - OWASP Top 10 coverage
  - `score`: number (0-100) - OWASP coverage percentage
  - `categories`: object - Coverage by OWASP category (key: category, value: boolean)
  - `missing`: array of string - Uncovered OWASP categories

---

### gaps

**Action**: `gaps_report`

Mental model gap detection across 5 dimensions.

```json
{
  "success": true,
  "action": "gaps_report",
  "command": "gaps",
  "data": {
    "total_gaps": 7,
    "inversion_gaps": [
      {
        "gap_type": "inversion",
        "description": "No test for expired JWT token",
        "severity": "high",
        "suggestion": "Add behavior testing 401 response with expired token",
        "mental_model": "Inversion thinking (what could go wrong)"
      }
    ],
    "second_order_gaps": [
      {
        "gap_type": "second_order",
        "description": "User deletion doesn't verify orphaned resources",
        "severity": "medium",
        "suggestion": "Add verification that user's posts are also deleted",
        "mental_model": "Second-order effects (cascading changes)"
      }
    ],
    "checklist_gaps": [],
    "coverage_gaps": [
      {
        "gap_type": "coverage",
        "description": "Missing PATCH method tests",
        "severity": "low",
        "suggestion": "Add partial update behaviors using PATCH",
        "mental_model": "Checklist completeness (systematic coverage)"
      }
    ],
    "security_gaps": [
      {
        "gap_type": "security",
        "description": "No SQL injection tests",
        "severity": "critical",
        "suggestion": "Add behavior testing SQL injection in search queries",
        "mental_model": "Security thinking (OWASP Top 10)"
      }
    ],
    "severity_breakdown": {
      "critical": 1,
      "high": 2,
      "medium": 3,
      "low": 1
    }
  },
  "errors": [],
  "next_actions": [
    {
      "command": "intent quality spec.cue",
      "reason": "Check overall quality score"
    },
    {
      "command": "intent doctor spec.cue",
      "reason": "Get prioritized fix plan"
    }
  ],
  "metadata": { ... },
  "spec_path": "examples/user-api.cue"
}
```

**Data Schema**:
- `total_gaps`: integer - Total gaps found
- `inversion_gaps`: array of Gap - Failure mode gaps
- `second_order_gaps`: array of Gap - Cascading effect gaps
- `checklist_gaps`: array of Gap - Systematic coverage gaps
- `coverage_gaps`: array of Gap - Test coverage gaps
- `security_gaps`: array of Gap - Security-related gaps
- `severity_breakdown`: object
  - `critical`: integer
  - `high`: integer
  - `medium`: integer
  - `low`: integer

**Gap Schema**:
- `gap_type`: "inversion" | "second_order" | "checklist" | "coverage" | "security"
- `description`: string - What's missing
- `severity`: "low" | "medium" | "high" | "critical"
- `suggestion`: string - How to fix
- `mental_model`: string - Which thinking model detected this

---

### invert

**Action**: `inversion_report`

Failure mode analysis (24 failure patterns across security, usability, integration).

```json
{
  "success": true,
  "action": "inversion_report",
  "command": "invert",
  "data": {
    "score": 75.5,
    "security_gaps": [
      {
        "category": "Authentication",
        "description": "No test for expired tokens",
        "severity": "high",
        "what_could_fail": "Expired JWT tokens accepted, allowing unauthorized access"
      }
    ],
    "usability_gaps": [
      {
        "category": "Error Messages",
        "description": "No test for user-friendly error responses",
        "severity": "medium",
        "what_could_fail": "Users see technical stack traces instead of helpful errors"
      }
    ],
    "integration_gaps": [
      {
        "category": "External Dependencies",
        "description": "No test for email service failure",
        "severity": "high",
        "what_could_fail": "User creation succeeds but welcome email never sent"
      }
    ],
    "suggested_behaviors": [
      {
        "name": "Reject expired JWT token",
        "intent": "Verify that expired tokens are rejected with 401",
        "method": "GET",
        "path": "/users/me",
        "expected_status": 401,
        "category": "Authentication"
      }
    ]
  },
  "errors": [],
  "next_actions": [
    {
      "command": "intent gaps spec.cue",
      "reason": "Find other coverage gaps"
    },
    {
      "command": "intent effects spec.cue",
      "reason": "Analyze second-order effects"
    }
  ],
  "metadata": { ... },
  "spec_path": "examples/user-api.cue"
}
```

**Data Schema**:
- `score`: number (0-100) - Inversion coverage score
- `security_gaps`: array of InversionGap
- `usability_gaps`: array of InversionGap
- `integration_gaps`: array of InversionGap
- `suggested_behaviors`: array of SuggestedBehavior

**InversionGap Schema**:
- `category`: string - Gap category
- `description`: string - What's missing
- `severity`: "low" | "medium" | "high" | "critical"
- `what_could_fail`: string - Failure scenario

**SuggestedBehavior Schema**:
- `name`: string - Behavior name
- `intent`: string - What to test
- `method`: string - HTTP method
- `path`: string - Request path
- `expected_status`: integer - Expected status code
- `category`: string - Category

---

### effects

**Action**: `effects_report`

Second-order effects and cascade analysis.

```json
{
  "success": true,
  "action": "effects_report",
  "command": "effects",
  "data": {
    "total_second_order_effects": 12,
    "coverage_score": 68.5,
    "behavior_effects": [
      {
        "behavior_name": "Delete user",
        "first_order": "User record removed from database",
        "second_order": [
          {
            "description": "User's posts become orphaned",
            "severity": "warning",
            "category": "resource_lifecycle",
            "has_verification": false
          },
          {
            "description": "User's active sessions invalidated",
            "severity": "info",
            "category": "system_state",
            "has_verification": true
          }
        ],
        "missing_verifications": [
          "Verify user's posts are deleted or reassigned"
        ]
      }
    ],
    "orphaned_resources": [
      {
        "resource_type": "Post",
        "caused_by": "Delete user",
        "description": "Posts remain after user deletion",
        "mitigation": "Add CASCADE delete or reassign to system user"
      }
    ],
    "cascade_warnings": [
      {
        "operation": "Delete user",
        "cascades_to": ["posts", "sessions", "notifications"],
        "requires_transaction": true,
        "description": "Deletion affects multiple tables, needs atomic transaction"
      }
    ],
    "state_dependencies": [
      {
        "behavior": "Update user email",
        "depends_on": ["User exists", "Email not taken"],
        "state_mutations": ["user.email", "email_index"],
        "isolation_level": "READ_COMMITTED"
      }
    ]
  },
  "errors": [],
  "next_actions": [
    {
      "command": "intent gaps spec.cue",
      "reason": "Find other gaps"
    },
    {
      "command": "intent coverage spec.cue",
      "reason": "Check overall coverage"
    }
  ],
  "metadata": { ... },
  "spec_path": "examples/user-api.cue"
}
```

**Data Schema**:
- `total_second_order_effects`: integer - Total effects identified
- `coverage_score`: number (0-100) - Verification coverage
- `behavior_effects`: array of BehaviorEffects
- `orphaned_resources`: array of OrphanedResource
- `cascade_warnings`: array of CascadeWarning
- `state_dependencies`: array of StateDependency

**BehaviorEffects Schema**:
- `behavior_name`: string
- `first_order`: string - Direct effect
- `second_order`: array of SecondOrderEffect
- `missing_verifications`: array of string

**SecondOrderEffect Schema**:
- `description`: string
- `severity`: "info" | "warning" | "danger" | "critical"
- `category`: "resource_lifecycle" | "data_integrity" | "system_state" | "security_implication" | "performance_impact" | "external_dependency"
- `has_verification`: boolean

---

### doctor

**Action**: `doctor_report`

Prioritized improvement recommendations.

```json
{
  "success": true,
  "action": "doctor_report",
  "command": "doctor",
  "data": {
    "quality": {
      "overall_score": 85,
      "coverage_score": 90,
      "clarity_score": 80,
      "testability_score": 85,
      "ai_readiness_score": 85,
      "issues": [
        "3 behaviors missing response examples",
        "2 vague validation rules"
      ]
    },
    "lint": {
      "status": "warnings",
      "warnings": [
        {
          "severity": "warning",
          "category": "missing_example",
          "message": "Behavior 'Create user' missing response.example",
          "location": {
            "behavior": "Create user"
          }
        }
      ]
    },
    "suggestions": [
      {
        "title": "Add response examples",
        "description": "3 behaviors lack concrete response examples",
        "reasoning": "Examples improve AI implementation and human understanding",
        "impact_score": 90
      },
      {
        "title": "Refine validation rules",
        "description": "2 rules use vague expressions like 'valid email'",
        "reasoning": "Specific rules enable better testing and verification",
        "impact_score": 75
      }
    ]
  },
  "errors": [],
  "next_actions": [
    {
      "command": "intent improve spec.cue",
      "reason": "Get detailed improvement suggestions"
    }
  ],
  "metadata": { ... },
  "spec_path": "examples/user-api.cue"
}
```

**Data Schema**:
- `quality`: object - Quality scores and issues
- `lint`: object - Linting results
- `suggestions`: array of DoctorSuggestion (sorted by impact_score)

**DoctorSuggestion Schema**:
- `title`: string
- `description`: string
- `reasoning`: string - Why this matters
- `impact_score`: integer (0-100) - Priority ranking

---

### lint

**Action**: `lint_result`

Anti-pattern detection.

```json
{
  "success": true,
  "action": "lint_result",
  "command": "lint",
  "data": {
    "status": "warnings",
    "warnings": [
      {
        "severity": "warning",
        "category": "vague_rule",
        "message": "Rule 'response.status == 200' is too generic",
        "location": {
          "behavior": "Get user",
          "field": "response.checks[0].rule"
        }
      },
      {
        "severity": "error",
        "category": "duplicate_behavior",
        "message": "Duplicate behavior detected",
        "location": {
          "behavior1": "Create user",
          "behavior2": "Create new user"
        }
      }
    ]
  },
  "errors": [],
  "next_actions": [
    {
      "command": "intent improve spec.cue",
      "reason": "Get fix suggestions"
    }
  ],
  "metadata": { ... },
  "spec_path": "examples/user-api.cue"
}
```

**Data Schema**:
- `status`: "valid" | "warnings"
- `warnings`: array of LintWarning

**LintWarning Schema**:
- `severity`: "error" | "warning" | "info"
- `category`: "anti_pattern" | "vague_rule" | "missing_example" | "unused_anti_pattern" | "naming_convention" | "duplicate_behavior"
- `message`: string
- `location`: object (flexible fields based on category)

---

### improve

**Action**: `improve_result`

Concrete improvement suggestions with proposed changes.

```json
{
  "success": true,
  "action": "improve_result",
  "command": "improve",
  "data": {
    "suggestions": [
      {
        "title": "Add missing response example",
        "description": "Behavior 'Create user' lacks response.example",
        "reasoning": "Examples improve testability and AI implementation",
        "impact_score": 90,
        "proposed_change": {
          "type": "add_response_example",
          "behavior_name": "Create user",
          "example": {
            "id": "user-123",
            "email": "alice@example.com",
            "created_at": "2026-01-25T14:30:00Z"
          }
        }
      }
    ],
    "total_count": 5
  },
  "errors": [],
  "next_actions": [
    {
      "command": "intent doctor spec.cue",
      "reason": "Get prioritized action plan"
    }
  ],
  "metadata": { ... },
  "spec_path": "examples/user-api.cue"
}
```

**Data Schema**:
- `suggestions`: array of ImprovementSuggestion (sorted by impact_score)
- `total_count`: integer

**ImprovementSuggestion Schema**:
- `title`: string
- `description`: string
- `reasoning`: string
- `impact_score`: integer (0-100)
- `proposed_change`: object
  - `type`: "add_missing_test" | "refine_vague_rule" | "add_response_example" | "rename_for_clarity" | "simplify_rule" | "add_explanation"
  - Additional fields vary by type

---

### prompt

**Action**: `prompt_result`

AI implementation prompts generated from beads.

```json
{
  "success": true,
  "action": "prompt_result",
  "command": "prompt",
  "data": {
    "session_id": "interview-abc123",
    "prompts": [
      {
        "bead_id": "bead-001",
        "task_summary": "Implement user creation endpoint",
        "context_section": "API endpoint for POST /users",
        "requirements": [
          "Accept email and name in request body",
          "Validate email format",
          "Return 201 on success"
        ],
        "acceptance_criteria": [
          "POST /users returns 201 with user ID",
          "Invalid email returns 400"
        ],
        "relevant_code": [
          {
            "path": "src/handlers/users.gleam",
            "language": "gleam",
            "purpose": "User handler functions"
          }
        ],
        "suggested_approach": "Use validator.validate_email() for email validation",
        "pitfalls_to_avoid": [
          "Don't store plaintext passwords",
          "Check for duplicate emails"
        ],
        "guardrail_block": "MUST use bcrypt for password hashing",
        "verification_steps": [
          "Run intent check spec.cue",
          "Verify 201 status code"
        ]
      }
    ],
    "total": 5
  },
  "errors": [],
  "next_actions": [],
  "metadata": { ... },
  "spec_path": "examples/user-api.cue"
}
```

**Data Schema**:
- `session_id`: string
- `prompts`: array of ImplementationPrompt
- `total`: integer

**ImplementationPrompt Schema**:
- `bead_id`: string
- `task_summary`: string
- `context_section`: string
- `requirements`: array of string
- `acceptance_criteria`: array of string
- `relevant_code`: array of FileContext
- `suggested_approach`: string
- `pitfalls_to_avoid`: array of string
- `guardrail_block`: string
- `verification_steps`: array of string

---

### feedback

**Action**: `feedback_result`

Generate fix beads from check command failures.

```json
{
  "success": true,
  "action": "feedback_result",
  "command": "feedback",
  "data": {
    "source_file": "check-results.json",
    "fix_beads": [
      {
        "behavior_name": "Create user",
        "feature": "User Management",
        "failure_type": "check_failed",
        "description": "Status code check failed: expected 201, got 200",
        "priority": 90,
        "fix_suggestion": "Update response to return 201 Created instead of 200 OK",
        "related_checks": [
          "response.status == 201"
        ]
      }
    ],
    "total_fixes": 3,
    "behaviors_analyzed": 5
  },
  "errors": [],
  "next_actions": [
    {
      "command": "intent check spec.cue",
      "reason": "Re-run tests after fixes"
    }
  ],
  "metadata": { ... },
  "spec_path": "examples/user-api.cue"
}
```

**Data Schema**:
- `source_file`: string - Input check results file
- `fix_beads`: array of FixBead
- `total_fixes`: integer
- `behaviors_analyzed`: integer

**FixBead Schema**:
- `behavior_name`: string
- `feature`: string
- `failure_type`: "check_failed" | "status_mismatch" | "connection_error" | "timeout"
- `description`: string
- `priority`: integer (0-100)
- `fix_suggestion`: string
- `related_checks`: array of string

---

### beads

**Action**: `beads_result`

Work item generation from interview session.

```json
{
  "success": true,
  "action": "beads_result",
  "command": "beads",
  "data": {
    "session_id": "interview-abc123",
    "beads": [
      {
        "title": "Implement user authentication",
        "description": "Add JWT-based authentication for user endpoints",
        "profile_type": "api",
        "priority": 95,
        "issue_type": "feature",
        "labels": ["authentication", "security"],
        "ai_hints": "Use industry-standard JWT library, set 24h expiration",
        "acceptance_criteria": [
          "POST /auth/login returns JWT token",
          "Protected endpoints require valid JWT"
        ],
        "dependencies": []
      }
    ],
    "total": 12
  },
  "errors": [],
  "next_actions": [
    {
      "command": "intent prompt interview-abc123",
      "reason": "Generate AI implementation prompts"
    }
  ],
  "metadata": { ... },
  "spec_path": null
}
```

**Data Schema**:
- `session_id`: string
- `beads`: array of BeadRecord
- `total`: integer

**BeadRecord Schema**:
- `title`: string
- `description`: string
- `profile_type`: string
- `priority`: integer (0-100)
- `issue_type`: string
- `labels`: array of string
- `ai_hints`: string
- `acceptance_criteria`: array of string
- `dependencies`: array of string

---

### Ready Phase Commands

The ready phase commands all follow the same base schema with command-specific data.

#### ready start

**Action**: `ready_start_result`

```json
{
  "success": true,
  "action": "ready_start_result",
  "command": "ready start",
  "data": {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "spec_path": "examples/user-api.cue",
    "phase": "ready",
    "status": "in_progress",
    "created_at": "2026-01-25T14:30:00Z"
  },
  "errors": [],
  "next_actions": [
    {
      "command": "intent ready check --session=550e8400-e29b-41d4-a716-446655440000",
      "reason": "Check session status"
    },
    {
      "command": "intent ready critique --session=550e8400-e29b-41d4-a716-446655440000",
      "reason": "Run Pre-Launch Auditor critique"
    }
  ],
  "metadata": { ... },
  "spec_path": "examples/user-api.cue"
}
```

---

## Error Handling

### Error Response Example

```json
{
  "success": false,
  "action": "error",
  "command": "validate",
  "data": {},
  "errors": [
    {
      "code": "parse_error",
      "message": "Invalid CUE syntax at line 42: missing closing brace",
      "location": "examples/user-api.cue:42",
      "fix_hint": "Add closing brace for feature block",
      "fix_command": "intent doctor examples/user-api.cue"
    }
  ],
  "next_actions": [
    {
      "command": "intent doctor examples/user-api.cue",
      "reason": "Get prioritized fix suggestions"
    }
  ],
  "metadata": {
    "timestamp": "2026-01-25T14:30:00Z",
    "version": "0.1.0",
    "exit_code": 3,
    "correlation_id": "550e8400-e29b-41d4-a716-446655440000",
    "duration_ms": 0
  },
  "spec_path": "examples/user-api.cue"
}
```

### Common Error Codes

| Code | Description | Exit Code |
|------|-------------|-----------|
| `parse_error` | CUE syntax error | 3 |
| `validation_error` | Spec structure invalid | 3 |
| `file_not_found` | Spec file doesn't exist | 4 |
| `permission_denied` | Cannot read spec file | 4 |
| `network_error` | HTTP request failed | 4 |
| `timeout` | Request exceeded timeout | 4 |
| `check_failed` | Validation check failed | 1 |
| `missing_required_field` | Required field missing | 3 |

---

## TypeScript Interfaces

Complete TypeScript definitions for type-safe integration:

```typescript
// Base Response Types

export interface JsonResponse<T = unknown> {
  success: boolean;
  action: string;
  command: string;
  data: T;
  errors: JsonError[];
  next_actions: NextAction[];
  metadata: JsonMetadata;
  spec_path: string | null;
}

export interface JsonError {
  code: string;
  message: string;
  location?: string | null;
  fix_hint?: string | null;
  fix_command?: string | null;
}

export interface NextAction {
  command: string;
  reason: string;
}

export interface JsonMetadata {
  timestamp: string; // ISO 8601
  version: string;
  exit_code: number;
  correlation_id: string; // UUID v4
  duration_ms: number;
}

// Common Enums

export type Severity = "low" | "medium" | "high" | "critical";
export type LintSeverity = "error" | "warning" | "info";
export type HealthStatus = "ok" | "warning" | "error";
export type GapType = "inversion" | "second_order" | "checklist" | "coverage" | "security";
export type EffectSeverity = "info" | "warning" | "danger" | "critical";
export type EffectCategory =
  | "resource_lifecycle"
  | "data_integrity"
  | "system_state"
  | "security_implication"
  | "performance_impact"
  | "external_dependency";

// Command-Specific Data Types

export interface ValidateData {
  valid: boolean;
  message: string;
  spec?: {
    name: string;
    description: string;
    version: string;
  };
}

export interface CheckData {
  total: number;
  passed: number;
  failed: number;
  skipped: number;
  success: boolean;
  duration_ms: number;
  behaviors: BehaviorResult[];
}

export interface BehaviorResult {
  name: string;
  feature: string;
  status: "passed" | "failed" | "skipped";
  duration_ms: number;
  request: {
    method: string;
    path: string;
    url: string;
  };
  response: {
    status: number;
    body?: unknown;
  };
  checks: CheckResult[];
  error?: string;
}

export interface CheckResult {
  field: string;
  rule: string;
  expected: unknown;
  actual: unknown;
  passed: boolean;
  why: string;
}

export interface QualityData {
  overall_score: number;
  coverage_score: number;
  clarity_score: number;
  testability_score: number;
  ai_readiness_score: number;
  issues: string[];
  suggestions: string[];
}

export interface CoverageData {
  overall_score: number;
  methods: Record<string, number>;
  status_codes: Record<string, number>;
  paths: Record<string, string[]>;
  edge_cases: {
    tested: string[];
    suggested: string[];
  };
  owasp: {
    score: number;
    categories: Record<string, boolean>;
    missing: string[];
  };
}

export interface GapsData {
  total_gaps: number;
  inversion_gaps: Gap[];
  second_order_gaps: Gap[];
  checklist_gaps: Gap[];
  coverage_gaps: Gap[];
  security_gaps: Gap[];
  severity_breakdown: {
    critical: number;
    high: number;
    medium: number;
    low: number;
  };
}

export interface Gap {
  gap_type: GapType;
  description: string;
  severity: Severity;
  suggestion: string;
  mental_model: string;
}

export interface InvertData {
  score: number;
  security_gaps: InversionGap[];
  usability_gaps: InversionGap[];
  integration_gaps: InversionGap[];
  suggested_behaviors: SuggestedBehavior[];
}

export interface InversionGap {
  category: string;
  description: string;
  severity: Severity;
  what_could_fail: string;
}

export interface SuggestedBehavior {
  name: string;
  intent: string;
  method: string;
  path: string;
  expected_status: number;
  category: string;
}

export interface EffectsData {
  total_second_order_effects: number;
  coverage_score: number;
  behavior_effects: BehaviorEffects[];
  orphaned_resources: OrphanedResource[];
  cascade_warnings: CascadeWarning[];
  state_dependencies: StateDependency[];
}

export interface BehaviorEffects {
  behavior_name: string;
  first_order: string;
  second_order: SecondOrderEffect[];
  missing_verifications: string[];
}

export interface SecondOrderEffect {
  description: string;
  severity: EffectSeverity;
  category: EffectCategory;
  has_verification: boolean;
}

export interface OrphanedResource {
  resource_type: string;
  caused_by: string;
  description: string;
  mitigation: string;
}

export interface CascadeWarning {
  operation: string;
  cascades_to: string[];
  requires_transaction: boolean;
  description: string;
}

export interface StateDependency {
  behavior: string;
  depends_on: string[];
  state_mutations: string[];
  isolation_level: string;
}

export interface DoctorData {
  quality: {
    overall_score: number;
    coverage_score: number;
    clarity_score: number;
    testability_score: number;
    ai_readiness_score: number;
    issues: string[];
  };
  lint: {
    status: "valid" | "warnings";
    warnings: LintWarning[];
  };
  suggestions: DoctorSuggestion[];
}

export interface DoctorSuggestion {
  title: string;
  description: string;
  reasoning: string;
  impact_score: number;
}

export interface LintData {
  status: "valid" | "warnings";
  warnings: LintWarning[];
}

export interface LintWarning {
  severity: LintSeverity;
  category:
    | "anti_pattern"
    | "vague_rule"
    | "missing_example"
    | "unused_anti_pattern"
    | "naming_convention"
    | "duplicate_behavior";
  message: string;
  location: Record<string, string>;
}

export interface ImproveData {
  suggestions: ImprovementSuggestion[];
  total_count: number;
}

export interface ImprovementSuggestion {
  title: string;
  description: string;
  reasoning: string;
  impact_score: number;
  proposed_change: {
    type:
      | "add_missing_test"
      | "refine_vague_rule"
      | "add_response_example"
      | "rename_for_clarity"
      | "simplify_rule"
      | "add_explanation";
    [key: string]: unknown; // Additional fields vary by type
  };
}

export interface PromptData {
  session_id: string;
  prompts: ImplementationPrompt[];
  total: number;
}

export interface ImplementationPrompt {
  bead_id: string;
  task_summary: string;
  context_section: string;
  requirements: string[];
  acceptance_criteria: string[];
  relevant_code: FileContext[];
  suggested_approach: string;
  pitfalls_to_avoid: string[];
  guardrail_block: string;
  verification_steps: string[];
}

export interface FileContext {
  path: string;
  language: string;
  purpose: string;
  content_snippet?: string;
  relevant_lines?: LineReference[];
}

export interface LineReference {
  line_number: number;
  content: string;
  reason: string;
}

export interface FeedbackData {
  source_file: string;
  fix_beads: FixBead[];
  total_fixes: number;
  behaviors_analyzed: number;
}

export interface FixBead {
  behavior_name: string;
  feature: string;
  failure_type: "check_failed" | "status_mismatch" | "connection_error" | "timeout";
  description: string;
  priority: number;
  fix_suggestion: string;
  related_checks: string[];
}

export interface BeadsData {
  session_id: string;
  beads: BeadRecord[];
  total: number;
}

export interface BeadRecord {
  title: string;
  description: string;
  profile_type: string;
  priority: number;
  issue_type: string;
  labels: string[];
  ai_hints: string;
  acceptance_criteria: string[];
  dependencies: string[];
}

// Type-safe Response Types

export type ValidateResponse = JsonResponse<ValidateData>;
export type CheckResponse = JsonResponse<CheckData>;
export type QualityResponse = JsonResponse<QualityData>;
export type CoverageResponse = JsonResponse<CoverageData>;
export type GapsResponse = JsonResponse<GapsData>;
export type InvertResponse = JsonResponse<InvertData>;
export type EffectsResponse = JsonResponse<EffectsData>;
export type DoctorResponse = JsonResponse<DoctorData>;
export type LintResponse = JsonResponse<LintData>;
export type ImproveResponse = JsonResponse<ImproveData>;
export type PromptResponse = JsonResponse<PromptData>;
export type FeedbackResponse = JsonResponse<FeedbackData>;
export type BeadsResponse = JsonResponse<BeadsData>;

// Helper Functions

/**
 * Parse Intent CLI JSON output with type safety
 */
export function parseIntentOutput<T>(json: string): JsonResponse<T> {
  return JSON.parse(json) as JsonResponse<T>;
}

/**
 * Check if response indicates success
 */
export function isSuccess<T>(response: JsonResponse<T>): boolean {
  return response.success && response.metadata.exit_code === 0;
}

/**
 * Extract error messages from response
 */
export function getErrorMessages<T>(response: JsonResponse<T>): string[] {
  return response.errors.map(err => err.message);
}

/**
 * Get next action commands
 */
export function getNextActions<T>(response: JsonResponse<T>): string[] {
  return response.next_actions.map(action => action.command);
}
```

---

## Usage Examples

### Python Example

```python
import subprocess
import json
from typing import Dict, List, Any

def run_intent_command(command: List[str]) -> Dict[str, Any]:
    """Execute Intent CLI command and parse JSON output"""
    result = subprocess.run(
        command,
        capture_output=True,
        text=True,
        check=False  # Don't raise on non-zero exit
    )

    return json.loads(result.stdout)

# Quality analysis
response = run_intent_command(["intent", "quality", "spec.cue"])

if response["success"]:
    data = response["data"]
    print(f"Overall score: {data['overall_score']}/100")
    print(f"Issues: {len(data['issues'])}")

    # Follow suggested actions
    for action in response["next_actions"]:
        print(f"Next: {action['command']}")
        print(f"  Why: {action['reason']}")
else:
    for error in response["errors"]:
        print(f"Error: {error['message']}")
        if error.get("fix_command"):
            print(f"  Fix: {error['fix_command']}")
```

### Node.js Example

```javascript
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

async function runIntentCommand(args) {
  const command = `intent ${args.join(' ')}`;
  const { stdout } = await execAsync(command);
  return JSON.parse(stdout);
}

// Coverage analysis
const response = await runIntentCommand(['coverage', 'spec.cue']);

if (response.success) {
  const { owasp } = response.data;
  console.log(`OWASP coverage: ${owasp.score}%`);
  console.log(`Missing: ${owasp.missing.join(', ')}`);

  // Execute next actions
  for (const action of response.next_actions) {
    console.log(`Suggested: ${action.command}`);
  }
}
```

### Bash Example

```bash
#!/bin/bash

# Run quality check and extract score
RESPONSE=$(intent quality spec.cue)
SCORE=$(echo "$RESPONSE" | jq -r '.data.overall_score')

if [ "$SCORE" -lt 80 ]; then
  echo "Quality score too low: $SCORE"

  # Get improvement suggestions
  intent doctor spec.cue | jq -r '.data.suggestions[].title'
  exit 1
fi

echo "Quality check passed: $SCORE/100"
```

### CI/CD Integration (GitHub Actions)

```yaml
name: API Spec Quality Gate

on: [push, pull_request]

jobs:
  quality-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Intent CLI
        run: |
          # Installation steps here

      - name: Run Quality Analysis
        id: quality
        run: |
          RESPONSE=$(intent quality api-spec.cue)
          echo "response=$RESPONSE" >> $GITHUB_OUTPUT

          SCORE=$(echo "$RESPONSE" | jq -r '.data.overall_score')
          echo "score=$SCORE" >> $GITHUB_OUTPUT

      - name: Quality Gate
        run: |
          SCORE=${{ steps.quality.outputs.score }}
          if [ "$SCORE" -lt 80 ]; then
            echo "❌ Quality score below threshold: $SCORE/100"
            exit 1
          fi
          echo "✅ Quality check passed: $SCORE/100"

      - name: Run KIRK Analysis
        run: |
          intent gaps api-spec.cue > gaps.json
          intent invert api-spec.cue > invert.json
          intent effects api-spec.cue > effects.json

      - name: Upload Analysis Results
        uses: actions/upload-artifact@v3
        with:
          name: kirk-analysis
          path: |
            gaps.json
            invert.json
            effects.json
```

### Workflow Automation Example

```python
"""
Automated workflow: Analyze spec, identify gaps, generate fixes
"""
import subprocess
import json
from typing import List, Dict

class IntentWorkflow:
    def __init__(self, spec_path: str):
        self.spec_path = spec_path

    def run_command(self, command: str) -> Dict:
        """Execute Intent command with JSON output"""
        result = subprocess.run(
            f"intent {command} {self.spec_path}",
            shell=True,
            capture_output=True,
            text=True
        )
        return json.loads(result.stdout)

    def analyze(self) -> Dict:
        """Run complete KIRK analysis"""
        return {
            "quality": self.run_command("quality"),
            "coverage": self.run_command("coverage"),
            "gaps": self.run_command("gaps"),
            "invert": self.run_command("invert"),
            "effects": self.run_command("effects"),
        }

    def get_critical_issues(self, analysis: Dict) -> List[str]:
        """Extract critical issues from analysis"""
        issues = []

        # Check gaps
        gaps = analysis["gaps"]["data"]
        critical_gaps = [
            g for g in gaps.get("security_gaps", [])
            if g["severity"] == "critical"
        ]
        issues.extend([g["description"] for g in critical_gaps])

        return issues

    def auto_improve(self) -> None:
        """Automated improvement workflow"""
        print("Running analysis...")
        analysis = self.analyze()

        quality_score = analysis["quality"]["data"]["overall_score"]
        print(f"Quality score: {quality_score}/100")

        if quality_score < 80:
            print("\nGenerating improvements...")
            doctor = self.run_command("doctor")

            for suggestion in doctor["data"]["suggestions"][:3]:
                print(f"\n• {suggestion['title']}")
                print(f"  Impact: {suggestion['impact_score']}/100")
                print(f"  {suggestion['description']}")

        critical = self.get_critical_issues(analysis)
        if critical:
            print("\n⚠️  Critical issues found:")
            for issue in critical:
                print(f"  • {issue}")

# Usage
workflow = IntentWorkflow("api-spec.cue")
workflow.auto_improve()
```

---

## JSON Schema Files

For formal validation, JSON Schema files are available in `schema/ai/output/*.cue` (CUE format). These can be converted to JSON Schema using:

```bash
cue export schema/ai/output/quality.cue --out jsonschema
```

All schemas extend the base schema defined in `schema/ai/output/_common.cue`.

---

## Best Practices

### 1. Always Check `success` Field

```typescript
const response = parseIntentOutput<QualityData>(stdout);

if (response.success) {
  // Process data
  const score = response.data.overall_score;
} else {
  // Handle errors
  console.error(response.errors.map(e => e.message).join('\n'));
}
```

### 2. Use `correlation_id` for Tracing

```python
response = run_intent_command(["quality", "spec.cue"])
correlation_id = response["metadata"]["correlation_id"]

# Log for debugging
logger.info(f"Quality check {correlation_id}: score={response['data']['overall_score']}")
```

### 3. Follow `next_actions` for Workflows

```javascript
async function runWorkflow(specPath) {
  let response = await runIntentCommand(['quality', specPath]);

  while (response.next_actions.length > 0) {
    const action = response.next_actions[0];
    console.log(`Executing: ${action.command}`);
    console.log(`Reason: ${action.reason}`);

    // Execute next action
    response = await runIntentCommand(action.command.split(' ').slice(1));
  }
}
```

### 4. Handle Exit Codes Properly

```bash
intent check spec.cue > results.json
EXIT_CODE=$?

case $EXIT_CODE in
  0) echo "All checks passed" ;;
  1) echo "Some checks failed" ;;
  3) echo "Invalid spec" ;;
  4) echo "Execution error" ;;
esac
```

### 5. Validate Against Schema

```python
import jsonschema

# Load schema
with open("schema/quality.json") as f:
    schema = json.load(f)

# Validate response
try:
    jsonschema.validate(response, schema)
    print("Response is valid")
except jsonschema.ValidationError as e:
    print(f"Invalid response: {e.message}")
```
