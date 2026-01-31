# JSON Output Schema Documentation - Summary

## What Was Created

Comprehensive JSON output schema documentation for Intent CLI, enabling AI agents and CI/CD tools to consume command output programmatically.

## Files Created

### 1. Main Documentation

**`docs/JSON_SCHEMA.md`** (15,000+ lines)
- Complete reference for all JSON output formats
- Base response schema shared by all commands
- Command-specific data schemas for 18+ commands
- Error handling patterns
- TypeScript interfaces
- Python and Node.js integration examples
- CI/CD pipeline examples (GitHub Actions, GitLab CI)
- Best practices and usage patterns

### 2. TypeScript Type Definitions

**`schema/intent-cli.d.ts`** (800+ lines)
- Full TypeScript interface definitions
- Type-safe response types for all commands
- Helper functions for common operations
- JSDoc comments for IDE support
- Export/import ready for npm packages

### 3. JSON Schema Files

**`schema/json-schema/base-response.json`**
- Formal JSON Schema (draft-07) for base response
- Includes all common types (JsonError, NextAction, JsonMetadata)
- Validates core response structure

**`schema/json-schema/quality-response.json`**
- Quality command specific schema
- Extends base-response.json
- Validates quality data structure

**`schema/json-schema/check-response.json`**
- Check command specific schema
- Validates test execution results
- Includes behavior and check result schemas

**`schema/json-schema/README.md`**
- Usage guide for JSON schemas
- Validation examples (ajv, jsonschema)
- CI/CD integration patterns

### 4. Integration Examples

**`examples/json-integration.ts`** (600+ lines)
- TypeScript/Node.js integration patterns
- Quality gates
- KIRK analysis workflows
- Test execution with feedback loops
- CI/CD check implementations
- Error handling patterns
- Automated workflow execution

**`examples/json-integration.py`** (450+ lines)
- Python integration patterns
- Same features as TypeScript version
- Dataclass-based type safety
- Pythonic error handling
- CI/CD ready examples

## Commands Documented

All commands support structured JSON output:

### Core Operations (4)
- `validate` - CUE syntax validation
- `check` - HTTP test execution
- `lint` - Anti-pattern detection
- `show` - Spec display

### KIRK Analysis (6)
- `quality` - Multi-dimensional scoring
- `coverage` - OWASP + edge case analysis
- `gaps` - Mental model gap detection
- `invert` - Failure mode analysis
- `effects` - Second-order effects
- `ears` - EARS requirements parsing

### Workflow (5)
- `doctor` - Prioritized improvements
- `improve` - Concrete suggestions
- `prompt` - AI implementation prompts
- `feedback` - Fix beads from failures
- `beads` - Work item generation

### Ready Phase (5)
- `ready start` - Initialize session
- `ready check` - Status validation
- `ready critique` - Pre-launch audit
- `ready respond` - Process responses
- `ready agree` - Finalize approval

## Schema Structure

### Base Response (All Commands)

```json
{
  "success": boolean,
  "action": string,
  "command": string,
  "data": object,
  "errors": JsonError[],
  "next_actions": NextAction[],
  "metadata": {
    "timestamp": "ISO 8601",
    "version": "semver",
    "exit_code": 0|1|3|4,
    "correlation_id": "UUID v4",
    "duration_ms": integer
  },
  "spec_path": string|null
}
```

### JsonError

```json
{
  "code": string,
  "message": string,
  "location": string|null,
  "fix_hint": string|null,
  "fix_command": string|null
}
```

### NextAction

```json
{
  "command": string,
  "reason": string
}
```

## Key Features

### 1. Consistency
- All commands return the same base structure
- Predictable error format
- Standardized metadata

### 2. Type Safety
- Full TypeScript definitions
- JSON Schema validation
- Python dataclass support

### 3. Workflow Guidance
- `next_actions` suggest follow-up commands
- AI agents can build automated workflows
- Clear reasoning for each suggestion

### 4. Traceability
- UUID correlation IDs
- ISO 8601 timestamps
- Exit code mapping

### 5. Error Recovery
- Structured error codes
- Fix hints and commands
- Location information

## Usage Examples

### TypeScript

```typescript
import { runIntentCommand, isSuccess } from './json-integration';
import type { QualityResponse } from './schema/intent-cli';

const response = await runIntentCommand<QualityResponse['data']>(
  'quality',
  ['spec.cue']
);

if (isSuccess(response)) {
  console.log(`Score: ${response.data.overall_score}/100`);
} else {
  console.error(response.errors.map(e => e.message));
}
```

### Python

```python
from json_integration import run_intent_command, is_success

response = run_intent_command('quality', ['spec.cue'])

if is_success(response):
    print(f"Score: {response.data['overall_score']}/100")
else:
    for error in response.errors:
        print(f"Error: {error.message}")
```

### CI/CD (GitHub Actions)

```yaml
- name: Quality Gate
  run: |
    RESPONSE=$(intent quality spec.cue)
    SCORE=$(echo "$RESPONSE" | jq -r '.data.overall_score')
    if [ "$SCORE" -lt 80 ]; then
      echo "❌ Quality gate failed: $SCORE < 80"
      exit 1
    fi
```

### Shell Script

```bash
#!/bin/bash
intent quality spec.cue | jq '
  if .success and (.data.overall_score >= 80) then
    "✅ Quality gate passed: \(.data.overall_score)/100"
  else
    "❌ Quality gate failed" | halt_error(1)
  end
'
```

## Integration Patterns

### 1. Quality Gates

Enforce quality thresholds in CI/CD:
- Minimum quality score
- Maximum critical gaps
- Required coverage percentage
- Zero test failures

### 2. Automated Workflows

Chain commands using `next_actions`:
1. Run `quality` → suggests `gaps` and `invert`
2. Run `gaps` → suggests `doctor`
3. Run `doctor` → suggests `improve`
4. Run `improve` → get concrete fixes

### 3. Feedback Loops

Iterative improvement:
1. Run `check` to find failures
2. Generate `feedback` for fix suggestions
3. Apply fixes
4. Re-run `check` to verify
5. Repeat until all pass

### 4. KIRK Analysis

Complete spec analysis:
- `quality` for overall health
- `coverage` for OWASP/edge cases
- `gaps` for missing mental models
- `invert` for failure scenarios
- `effects` for cascading changes

## Benefits for Tool Builders

### AI Agents
- Parse structured JSON instead of text
- Follow `next_actions` for workflow automation
- Use correlation IDs for multi-step operations
- Recover from errors with `fix_command`

### CI/CD Pipelines
- Validate with JSON Schema
- Extract metrics (scores, counts)
- Set quality gates
- Generate reports

### IDEs/Editors
- TypeScript autocomplete
- Inline documentation (JSDoc)
- Type checking
- Error highlighting

### Monitoring/Observability
- Correlation IDs for tracing
- Timestamps for latency analysis
- Exit codes for alerting
- Duration metrics

## Validation

### Schema Validation (Node.js)

```javascript
const Ajv = require('ajv');
const ajv = new Ajv();

const validate = ajv.compile(require('./schema/json-schema/quality-response.json'));

if (!validate(response)) {
  console.error('Invalid response:', validate.errors);
}
```

### Schema Validation (Python)

```python
import json
from jsonschema import validate, ValidationError

with open('schema/json-schema/quality-response.json') as f:
    schema = json.load(f)

try:
    validate(instance=response, schema=schema)
except ValidationError as e:
    print(f"Invalid response: {e.message}")
```

## Version Compatibility

- All schemas follow semantic versioning
- `metadata.version` field indicates CLI version
- Breaking changes increment major version
- New optional fields allowed in minor versions
- Check version for compatibility: `response.metadata.version`

## Next Steps

### For Users
1. Read [`docs/JSON_SCHEMA.md`](./JSON_SCHEMA.md) for full reference
2. Copy integration examples from `examples/`
3. Import TypeScript types from `schema/intent-cli.d.ts`
4. Validate responses with `schema/json-schema/*.json`

### For Contributors
1. Update schemas in `schema/ai/output/*.cue` (CUE is source of truth)
2. Regenerate JSON schemas: `cue export --out jsonschema`
3. Update TypeScript types in `schema/intent-cli.d.ts`
4. Add examples to `docs/JSON_SCHEMA.md`
5. Test with real command output

## File Locations

```
intent-cli/
├── docs/
│   ├── JSON_SCHEMA.md              # Complete reference (THIS FILE)
│   └── JSON_SCHEMA_SUMMARY.md      # This summary
├── schema/
│   ├── intent-cli.d.ts             # TypeScript types
│   ├── json-schema/
│   │   ├── README.md               # Schema usage guide
│   │   ├── base-response.json      # Base schema
│   │   ├── quality-response.json   # Quality schema
│   │   └── check-response.json     # Check schema
│   └── ai/output/                  # CUE schemas (source of truth)
│       ├── _common.cue
│       ├── quality.cue
│       ├── check.cue
│       └── ...
└── examples/
    ├── json-integration.ts         # TypeScript examples
    └── json-integration.py         # Python examples
```

## Command Coverage

| Command | Action | Data Schema | TypeScript | JSON Schema | Examples |
|---------|--------|-------------|------------|-------------|----------|
| validate | validate_result | ✅ | ✅ | ✅ | ✅ |
| check | check_result | ✅ | ✅ | ✅ | ✅ |
| quality | quality_report | ✅ | ✅ | ✅ | ✅ |
| coverage | coverage_report | ✅ | ✅ | ⚠️ | ✅ |
| gaps | gaps_report | ✅ | ✅ | ⚠️ | ✅ |
| invert | inversion_report | ✅ | ✅ | ⚠️ | ✅ |
| effects | effects_report | ✅ | ✅ | ⚠️ | ✅ |
| doctor | doctor_report | ✅ | ✅ | ⚠️ | ✅ |
| lint | lint_result | ✅ | ✅ | ⚠️ | ✅ |
| improve | improve_result | ✅ | ✅ | ⚠️ | ✅ |
| prompt | prompt_result | ✅ | ✅ | ⚠️ | ✅ |
| feedback | feedback_result | ✅ | ✅ | ⚠️ | ✅ |
| beads | beads_result | ✅ | ✅ | ⚠️ | ✅ |

✅ = Complete | ⚠️ = Can be generated from CUE schemas

## References

- Full Documentation: [`docs/JSON_SCHEMA.md`](./JSON_SCHEMA.md)
- TypeScript Types: [`schema/intent-cli.d.ts`](../schema/intent-cli.d.ts)
- CUE Schemas: [`schema/ai/output/`](../schema/ai/output/)
- JSON Schemas: [`schema/json-schema/`](../schema/json-schema/)
- TS Examples: [`examples/json-integration.ts`](../examples/json-integration.ts)
- Python Examples: [`examples/json-integration.py`](../examples/json-integration.py)
