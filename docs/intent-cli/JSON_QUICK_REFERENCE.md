# Intent CLI JSON Output - Quick Reference

Fast reference for developers integrating Intent CLI JSON output.

## Basic Usage

```bash
# All analysis commands return structured JSON by default
intent <command> <spec>
```

## Response Structure

Every response has this shape:

```typescript
{
  success: boolean              // true if command succeeded
  action: string                // e.g., "quality_report"
  command: string               // e.g., "quality"
  data: { ... }                 // command-specific output
  errors: JsonError[]           // empty if success
  next_actions: NextAction[]    // suggested follow-up commands
  metadata: {
    timestamp: string           // ISO 8601
    version: string             // CLI version
    exit_code: number           // 0|1|3|4
    correlation_id: string      // UUID v4
    duration_ms: number
  }
  spec_path: string | null
}
```

## Quick Command Reference

| Command | Use For | Key Data Fields |
|---------|---------|----------------|
| `validate` | Check spec syntax | `valid`, `message` |
| `check` | Run HTTP tests | `passed`, `failed`, `behaviors[]` |
| `quality` | Get quality scores | `overall_score`, `issues[]` |
| `coverage` | OWASP + edge cases | `overall_score`, `owasp.score` |
| `gaps` | Find missing tests | `total_gaps`, `severity_breakdown` |
| `invert` | Failure scenarios | `score`, `security_gaps[]` |
| `effects` | Cascading changes | `total_second_order_effects` |
| `doctor` | Prioritized fixes | `suggestions[]` |
| `lint` | Anti-patterns | `warnings[]` |
| `improve` | Concrete fixes | `suggestions[]` |
| `feedback` | Fixes from failures | `fix_beads[]` |
| `prompt` | AI prompts | `prompts[]` |
| `beads` | Work items | `beads[]` |

## Common Patterns

### Check Success

```typescript
if (response.success && response.metadata.exit_code === 0) {
  // command succeeded
}
```

```python
if response.success and response.metadata.exit_code == 0:
    # command succeeded
```

### Extract Errors

```typescript
const errors = response.errors.map(e => e.message);
```

```python
errors = [e.message for e in response.errors]
```

### Follow Workflow

```typescript
const nextCommands = response.next_actions.map(a => a.command);
```

```python
next_commands = [a.command for a in response.next_actions]
```

### Get Correlation ID

```typescript
const correlationId = response.metadata.correlation_id;
```

```python
correlation_id = response.metadata.correlation_id
```

## Exit Codes

| Code | Meaning | Action |
|------|---------|--------|
| 0 | Success | Continue |
| 1 | Failure | Review/fix |
| 3 | Invalid | Fix spec |
| 4 | Error | Fix command |

## Error Structure

```typescript
{
  code: string              // e.g., "parse_error"
  message: string           // human-readable
  location?: string         // e.g., "spec.cue:42"
  fix_hint?: string         // suggestion
  fix_command?: string      // runnable command
}
```

## Next Action Structure

```typescript
{
  command: string           // e.g., "intent gaps spec.cue"
  reason: string            // e.g., "Find coverage gaps"
}
```

## Common Data Shapes

### Quality Scores (0-100)

```typescript
{
  overall_score: number
  coverage_score: number
  clarity_score: number
  testability_score: number
  ai_readiness_score: number
  issues: string[]
  suggestions: string[]
}
```

### Check Results

```typescript
{
  total: number
  passed: number
  failed: number
  skipped: number
  success: boolean
  duration_ms: number
  behaviors: BehaviorResult[]
}
```

### Gaps Summary

```typescript
{
  total_gaps: number
  severity_breakdown: {
    critical: number
    high: number
    medium: number
    low: number
  }
  inversion_gaps: Gap[]
  second_order_gaps: Gap[]
  security_gaps: Gap[]
  // ...
}
```

## One-Liners

### Bash

```bash
# Get quality score
intent quality spec.cue | jq '.data.overall_score'

# Check if tests passed
intent check spec.cue | jq '.data.success'

# Count critical gaps
intent gaps spec.cue | jq '.data.severity_breakdown.critical'

# List next actions
intent quality spec.cue | jq -r '.next_actions[].command'

# Extract all errors
intent validate spec.cue | jq -r '.errors[].message'
```

### Node.js

```javascript
// Get quality score
const score = JSON.parse(stdout).data.overall_score;

// Check success
const passed = JSON.parse(stdout).success;

// Get next actions
const actions = JSON.parse(stdout).next_actions.map(a => a.command);
```

### Python

```python
# Get quality score
score = json.loads(stdout)["data"]["overall_score"]

# Check success
passed = json.loads(stdout)["success"]

# Get next actions
actions = [a["command"] for a in json.loads(stdout)["next_actions"]]
```

## Quality Gate Example

```bash
#!/bin/bash
set -e

RESPONSE=$(intent quality spec.cue)
SCORE=$(echo "$RESPONSE" | jq -r '.data.overall_score')

if [ "$SCORE" -lt 80 ]; then
  echo "❌ Quality gate failed: $SCORE < 80"
  echo "$RESPONSE" | jq -r '.data.issues[]'
  exit 1
fi

echo "✅ Quality gate passed: $SCORE >= 80"
```

## TypeScript Type Import

```typescript
import type {
  JsonResponse,
  QualityResponse,
  CheckResponse,
  GapsResponse,
} from './schema/intent-cli';

const response: QualityResponse = JSON.parse(stdout);
console.log(response.data.overall_score);
```

## Validation

### With ajv (Node.js)

```javascript
const Ajv = require('ajv');
const ajv = new Ajv();
const validate = ajv.compile(require('./schema/json-schema/base-response.json'));

if (!validate(response)) {
  console.error(validate.errors);
}
```

### With jsonschema (Python)

```python
from jsonschema import validate
import json

with open('schema/json-schema/base-response.json') as f:
    schema = json.load(f)

validate(instance=response, schema=schema)
```

## CI/CD Snippet

### GitHub Actions

```yaml
- name: Quality Gate
  run: |
    RESPONSE=$(intent quality spec.cue)
    echo "$RESPONSE" | jq -e '.success and (.data.overall_score >= 80)'
```

### GitLab CI

```yaml
quality_gate:
  script:
    - SCORE=$(intent quality spec.cue | jq -r '.data.overall_score')
    - test $SCORE -ge 80
```

## Common Workflows

### 1. Complete KIRK Analysis

```bash
intent quality spec.cue   > quality.json
intent coverage spec.cue  > coverage.json
intent gaps spec.cue      > gaps.json
intent invert spec.cue    > invert.json
intent effects spec.cue   > effects.json
```

### 2. Test → Feedback → Fix

```bash
intent check spec.cue > results.json
intent feedback --results results.json > fixes.json
# Apply fixes from fixes.json
intent check spec.cue  # Verify
```

### 3. Automated Workflow

```bash
# Start with quality
RESPONSE=$(intent quality spec.cue)

# Extract next action
NEXT=$(echo "$RESPONSE" | jq -r '.next_actions[0].command')

# Execute next action
eval "$NEXT"
```

## Field Quick Lookup

### response.success
Boolean indicating command achieved its goal

### response.action
String identifying result type (e.g., "check_result")

### response.command
String identifying command that ran (e.g., "check")

### response.data
Object with command-specific output

### response.errors
Array of structured errors (empty if success)

### response.next_actions
Array of suggested follow-up commands

### response.metadata.timestamp
ISO 8601 timestamp of execution

### response.metadata.version
Intent CLI version (semver)

### response.metadata.exit_code
Unix exit code (0/1/3/4)

### response.metadata.correlation_id
UUID v4 for request tracing

### response.metadata.duration_ms
Execution time in milliseconds

### response.spec_path
Path to spec file (null if not applicable)

## Error Codes

| Code | Description |
|------|-------------|
| `parse_error` | CUE syntax error |
| `validation_error` | Spec structure invalid |
| `file_not_found` | Spec file missing |
| `network_error` | HTTP request failed |
| `timeout` | Request exceeded limit |
| `check_failed` | Validation failed |

## See Also

- Full Documentation: [`docs/JSON_SCHEMA.md`](./JSON_SCHEMA.md)
- Summary: [`docs/JSON_SCHEMA_SUMMARY.md`](./JSON_SCHEMA_SUMMARY.md)
- TypeScript Types: [`schema/intent-cli.d.ts`](../schema/intent-cli.d.ts)
- Examples: [`examples/json-integration.ts`](../examples/json-integration.ts)
