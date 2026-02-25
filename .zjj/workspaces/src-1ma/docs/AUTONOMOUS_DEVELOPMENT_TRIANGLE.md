# Autonomous Development Triangle - Oya Integration
# Quality Gate for AI-driven development

## Triangle Components

### 1. HIGH-QUALITY SPECS (Agent CAN see)
- Location: `/home/lewis/src/new-app/specs/`
- Files:
  - `flow-wasm-v1.yaml` - The specification
  - `schema/spec.schema.yaml` - Schema definition
  - `linter/rules.yaml` - Linter rules

### 2. DIGITAL TWINS (Agent CAN see)
- Location: `/home/lewis/src/new-app/twins/`
- Files:
  - `local-storage-twin/definition.yaml` - LocalStorage twin
  - `flow-wasm-universe.yaml` - Universe manifest

### 3. BEHAVIORAL SCENARIOS (Agent CANNOT see)
- Location: `/home/lewis/src/scenarios-vault/`
- Files:
  - `flow-wasm/happy-path/` - Happy path scenarios
  - `flow-wasm/error-handling/` - Error handling scenarios
  - `flow-wasm/security/` - Security scenarios
  - `feedback-config.yaml` - Feedback level configuration

## Information Barrier

**AGENT SIDE (can access):**
- ✓ Spec (full detail)
- ✓ Twin endpoints
- ✓ Twin inspection APIs
- ✓ Acceptance criteria
- ✓ Own test results
- ✓ Sanitized feedback

**HOLDOUT SIDE (cannot access):**
- ✗ Scenario definitions
- ✗ Scenario assertions
- ✗ Scenario step sequences
- ✗ Expected HTTP responses
- ✗ Edge case test data

## Quality Gate Process

1. **SPEC_VALIDATION** - Run spec linter, require score ≥ 80
2. **TWINS_SETUP** - Spin up twin universe
3. **AGENT_DEV** - Agent receives: spec + twin URLs + (feedback if retry)
4. **VALIDATION** - Run holdout scenarios (agent-blind)
5. **FEEDBACK** - Sanitize results, send to agent if failed

## Configuration

- Max agent iterations: 5
- Spec quality threshold: 80
- Feedback level: 3 (guided)
- Total timeout: 4 hours
