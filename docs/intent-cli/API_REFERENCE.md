# Intent CLI - API Reference

**Version:** 1.0.0
**Last Updated:** 2026-01-11

This document provides comprehensive API documentation for all public functions in the Intent CLI codebase. Intent is an AI-guided planning framework that transforms vague requirements into crystal-clear, atomic work items through systematic interviewing and rigorous decomposition.

---

## Table of Contents

- [Core Modules](#core-modules)
  - [intent (Main Entry Point)](#intent-main-entry-point)
  - [runner](#runner)
  - [loader](#loader)
  - [checker](#checker)
  - [validator](#validator)
- [Interview Engine](#interview-engine)
  - [interview](#interview)
  - [interview_questions](#interview_questions)
  - [interview_storage](#interview_storage)
  - [spec_builder](#spec_builder)
- [Quality Analysis](#quality-analysis)
  - [quality_analyzer](#quality_analyzer)
  - [spec_linter](#spec_linter)
  - [improver](#improver)
- [HTTP & Networking](#http--networking)
  - [http_client](#http_client)
- [Parsing & Formatting](#parsing--formatting)
  - [parser](#parser)
  - [output](#output)
  - [interpolate](#interpolate)
- [Utilities](#utilities)
  - [types](#types)
  - [security](#security)
  - [cli_ui](#cli_ui)

---

## Core Modules

### intent (Main Entry Point)

**Module:** `src/intent.gleam`

The main CLI entry point with all command definitions.

#### `main() -> Nil`

Main entry point for the CLI application. Sets up all commands and handles argument parsing.

**Example:**
```gleam
pub fn main() {
  glint.new()
  |> glint.with_name("intent")
  |> glint.add(at: ["check"], do: check_command())
  |> glint.run(argv.load().arguments)
}
```

**Commands Available:**
- `check` - Run spec against target URL
- `validate` - Validate CUE spec without running
- `show` - Pretty print parsed spec
- `export` - Export spec to JSON
- `lint` - Check for anti-patterns
- `analyze` - Analyze spec quality
- `improve` - Suggest improvements
- `interview` - Guided specification discovery
- `beads` - Generate work items from interview
- `bead-status` - Mark bead execution status
- `plan` - Display execution plan
- `plan-approve` - Approve execution plan
- `beads-regenerate` - Regenerate failed beads

**Exit Codes:**
- `0` (exit_pass) - All checks passed
- `1` (exit_fail) - Check failed
- `2` (exit_blocked) - Blocked behaviors detected
- `3` (exit_invalid) - Invalid spec
- `4` (exit_error) - General error

---

### runner

**Module:** `src/intent/runner.gleam`

Main test runner that orchestrates behavior execution and validation.

#### `run_spec(spec: Spec, target_url: String, options: RunOptions) -> SpecResult`

Run a spec against a target URL and return results using the default HTTP executor.

**Parameters:**
- `spec` - The parsed specification to run
- `target_url` - Base URL of the API to test
- `options` - Run options (filters, verbosity)

**Returns:** `SpecResult` containing pass/fail status and detailed results

**Example:**
```gleam
let spec = loader.load_spec("api-spec.cue")
let options = runner.default_options()
let result = runner.run_spec(spec, "http://localhost:8080", options)
```

#### `run_spec_with_executor(spec: Spec, target_url: String, options: RunOptions, executor: BehaviorExecutor) -> SpecResult`

Run a spec with a custom executor. Enables dependency injection for testing.

**Parameters:**
- `spec` - The parsed specification
- `target_url` - Base URL
- `options` - Run options
- `executor` - Custom executor (allows mocking HTTP responses)

**Returns:** `SpecResult`

**Use Case:** Testing the runner without making real HTTP requests

**Example:**
```gleam
let mock_executor = BehaviorExecutor(
  execute: fn(_config, _req, _ctx) {
    Ok(ExecutionResult(...))
  }
)
let result = runner.run_spec_with_executor(spec, url, options, mock_executor)
```

#### `default_executor() -> BehaviorExecutor`

Returns the default HTTP executor that makes real network requests.

**Returns:** `BehaviorExecutor` configured with `http_client.execute_request`

#### `default_options() -> RunOptions`

Returns default run options with Normal output level.

**Returns:** `RunOptions` with no filters and Normal verbosity

**Example:**
```gleam
let options = runner.default_options()
```

#### `is_verbose(options: RunOptions) -> Bool`

Check if output level is verbose.

**Parameters:**
- `options` - Run options to check

**Returns:** `True` if verbose mode is enabled

#### `is_quiet(options: RunOptions) -> Bool`

Check if output level is quiet (errors only).

**Parameters:**
- `options` - Run options to check

**Returns:** `True` if quiet mode is enabled

**Types:**

```gleam
pub type OutputLevel {
  Quiet      // Minimal output, errors only
  Normal     // Standard output with pass/fail summary
  Verbose    // Detailed output including request/response details
}

pub type RunOptions {
  RunOptions(
    feature_filter: Option(String),     // Filter to specific feature
    behavior_filter: Option(String),    // Run only specific behavior
    output_level: OutputLevel,
  )
}

pub type BehaviorExecutor {
  BehaviorExecutor(
    execute: fn(Config, Request, Context) -> Result(ExecutionResult, ExecutionError)
  )
}
```

---

### loader

**Module:** `src/intent/loader.gleam`

CUE spec loader - loads and validates CUE files using the cue command.

#### `load_spec(path: String) -> Result(Spec, LoadError)`

Load a spec from a CUE file with spinner UI feedback.

**Parameters:**
- `path` - Path to CUE specification file

**Returns:** `Result(Spec, LoadError)` - Parsed spec or error

**Security:** Validates file path for security before loading

**Example:**
```gleam
case loader.load_spec("examples/user-api.cue") {
  Ok(spec) -> // Use spec
  Error(e) -> io.println_error(loader.format_error(e))
}
```

#### `load_spec_quiet(path: String) -> Result(Spec, LoadError)`

Load a spec without spinner UI. Use for testing and automation.

**Parameters:**
- `path` - Path to CUE specification file

**Returns:** `Result(Spec, LoadError)`

**Example:**
```gleam
// In tests - no UI output
let spec = loader.load_spec_quiet("test-spec.cue")
```

#### `validate_cue(path: String) -> Result(Nil, LoadError)`

Validate a CUE file syntax without parsing to Spec.

**Parameters:**
- `path` - Path to CUE file

**Returns:** `Result(Nil, LoadError)` - Ok if valid CUE syntax

**Example:**
```gleam
case loader.validate_cue("spec.cue") {
  Ok(_) -> io.println("Valid CUE syntax")
  Error(e) -> io.println_error("Invalid: " <> loader.format_error(e))
}
```

#### `export_spec_json(path: String) -> Result(String, LoadError)`

Export a spec to JSON format (for AI consumption).

**Parameters:**
- `path` - Path to CUE file

**Returns:** `Result(String, LoadError)` - JSON string representation

**Example:**
```gleam
case loader.export_spec_json("spec.cue") {
  Ok(json_str) -> io.println(json_str)
  Error(e) -> io.println_error(loader.format_error(e))
}
```

#### `format_error(error: LoadError) -> String`

Format a LoadError as human-readable string.

**Parameters:**
- `error` - The error to format

**Returns:** Formatted error message

**Types:**

```gleam
pub type LoadError {
  FileNotFound(path: String)
  CueValidationError(message: String)
  CueExportError(message: String)
  JsonParseError(message: String)
  SpecParseError(message: String)
  SecurityError(message: String)
}
```

---

### checker

**Module:** `src/intent/checker.gleam`

Response validation engine - checks HTTP responses against expected rules.

#### `check_response(expected: Response, actual: ExecutionResult, ctx: Context) -> ResponseCheckResult`

Check an execution result against expected response definition.

**Parameters:**
- `expected` - Expected response from spec
- `actual` - Actual HTTP execution result
- `ctx` - Interpolation context for variable substitution

**Returns:** `ResponseCheckResult` with passed/failed checks

**Example:**
```gleam
let expected = behavior.response
let actual = http_client.execute_request(config, request, ctx)
let result = checker.check_response(expected, actual, ctx)

case result.status_ok && list.is_empty(result.failed) {
  True -> io.println("All checks passed")
  False -> io.println("Some checks failed")
}
```

**What it checks:**
- HTTP status code matches expected
- All field rules pass (equality, existence, types, patterns)
- Expected headers are present and match
- Response body structure matches

**Types:**

```gleam
pub type CheckResult {
  CheckPassed(field: String, rule: String)
  CheckFailed(
    field: String,
    rule: String,
    expected: String,
    actual: String,
    explanation: String,
  )
}

pub type ResponseCheckResult {
  ResponseCheckResult(
    passed: List(CheckResult),
    failed: List(CheckResult),
    status_ok: Bool,
    status_expected: Int,
    status_actual: Int,
  )
}
```

---

### validator

**Module:** `src/intent/validator.gleam`

Pre-execution static validation of specs. Validates before making any HTTP requests.

#### `validate_spec(spec: Spec) -> ValidationResult`

Validate a complete spec before execution.

**Parameters:**
- `spec` - The specification to validate

**Returns:** `ValidationResult` - Valid or list of issues

**What it validates:**
- Rule syntax in all checks
- Variable references are available when used
- All dependencies exist
- No circular dependencies
- Capture paths are valid

**Example:**
```gleam
case validator.validate_spec(spec) {
  validator.ValidationValid -> io.println("Spec is valid")
  validator.ValidationInvalid(issues) -> {
    io.println_error(validator.format_issues(issues))
  }
}
```

#### `format_issues(issues: List(ValidationIssue)) -> String`

Format validation issues for display.

**Parameters:**
- `issues` - List of validation issues

**Returns:** Human-readable formatted string

**Types:**

```gleam
pub type ValidationResult {
  ValidationValid
  ValidationInvalid(issues: List(ValidationIssue))
}

pub type ValidationIssue {
  RuleSyntaxError(behavior: String, field: String, rule: String, error: String)
  UndefinedVariable(behavior: String, field: String, var_name: String, suggestion: String)
  InvalidPath(behavior: String, path: String, error: String)
  MissingDependency(behavior: String, depends_on: String)
  CircularDependency(behaviors: List(String))
  MissingCapture(behavior: String, field: String, var_name: String, captured_by: List(String))
}
```

---

## Interview Engine

### interview

**Module:** `src/intent/interview.gleam`

Structured interrogation system for discovering and refining specifications through 5 rounds of questioning across multiple perspectives.

#### `create_session(id: String, profile: Profile, timestamp: String) -> InterviewSession`

Create a new interview session.

**Parameters:**
- `id` - Unique session identifier
- `profile` - System profile (Api, Cli, Event, Data, Workflow, UI)
- `timestamp` - Creation timestamp

**Returns:** Fresh `InterviewSession` in Discovery stage

#### `add_answer(session: InterviewSession, answer: Answer) -> InterviewSession`

Add an answer to the session.

**Parameters:**
- `session` - Current session state
- `answer` - Answer to add

**Returns:** Updated session

#### `extract_from_answer(question_id: String, response: String, extract_fields: List(String)) -> Dict(String, String)`

Extract structured fields from free-form answer text.

**Parameters:**
- `question_id` - Question identifier
- `response` - User's text response
- `extract_fields` - Fields to extract (e.g., ["auth_method", "entities"])

**Returns:** Dictionary of extracted values

**Example:**
```gleam
let extracted = interview.extract_from_answer(
  "q1",
  "We use JWT authentication with OAuth2",
  ["auth_method"]
)
// Returns: {"auth_method": "jwt"}
```

#### `calculate_confidence(question_id: String, response: String, extracted: Dict(String, String)) -> Float`

Calculate confidence score for an answer (0.0 to 1.0).

**Parameters:**
- `question_id` - Question identifier
- `response` - Original response text
- `extracted` - Extracted structured data

**Returns:** Confidence score between 0.0 and 1.0

#### `check_for_gaps(session: InterviewSession, question: Question, answer: Answer) -> #(InterviewSession, List(Gap))`

Check if an answer reveals missing information.

**Parameters:**
- `session` - Current session
- `question` - Question that was asked
- `answer` - Answer provided

**Returns:** Tuple of (updated session, detected gaps)

#### `check_for_conflicts(session: InterviewSession, answer: Answer) -> #(InterviewSession, List(Conflict))`

Check if an answer conflicts with previous answers.

**Parameters:**
- `session` - Current session
- `answer` - New answer to check

**Returns:** Tuple of (updated session, detected conflicts)

#### `get_blocking_gaps(session: InterviewSession) -> List(Gap)`

Get all blocking gaps that prevent spec completion.

**Parameters:**
- `session` - Session to check

**Returns:** List of blocking gaps

#### `get_first_question_for_round(session: InterviewSession, round: Int) -> Result(Question, String)`

Get the first unanswered question for a round.

**Parameters:**
- `session` - Current session
- `round` - Round number (1-5)

**Returns:** Result with first question or error if no questions

**Types:**

```gleam
pub type Profile {
  Api        // REST/GraphQL APIs
  Cli        // Command-line tools
  Event      // Event-driven systems
  Data       // Data pipelines
  Workflow   // Business workflows
  UI         // User interfaces
}

pub type InterviewStage {
  Discovery    // Initial requirements gathering
  Refinement   // Clarifying details
  Validation   // Confirming understanding
  Complete     // Interview finished
  Paused       // Temporarily stopped
}

pub type Answer {
  Answer(
    question_id: String,
    question_text: String,
    perspective: Perspective,
    round: Int,
    response: String,
    extracted: Dict(String, String),
    confidence: Float,
    notes: String,
    timestamp: String,
  )
}

pub type Gap {
  Gap(
    id: String,
    field: String,
    description: String,
    blocking: Bool,
    suggested_default: String,
    why_needed: String,
    round: Int,
    resolved: Bool,
    resolution: String,
  )
}

pub type Conflict {
  Conflict(
    id: String,
    between: #(String, String),
    description: String,
    impact: String,
    options: List(ConflictResolution),
    chosen: Int,
  )
}

pub type InterviewSession {
  InterviewSession(
    id: String,
    profile: Profile,
    created_at: String,
    updated_at: String,
    completed_at: String,
    stage: InterviewStage,
    rounds_completed: Int,
    answers: List(Answer),
    gaps: List(Gap),
    conflicts: List(Conflict),
    raw_notes: String,
  )
}
```

---

### spec_builder

**Module:** `src/intent/spec_builder.gleam`

Converts interview session answers into valid CUE specifications.

#### `build_spec_from_session(session: InterviewSession) -> String`

Build a CUE spec from completed interview session.

**Parameters:**
- `session` - Completed interview session

**Returns:** CUE specification as string

**Example:**
```gleam
let spec_cue = spec_builder.build_spec_from_session(session)
simplifile.write("generated-spec.cue", spec_cue)
```

#### `extract_features_from_answers(answers: List(Answer)) -> List(String)`

Extract feature names from interview answers.

**Parameters:**
- `answers` - All answers from session

**Returns:** List of feature names

#### `extract_behaviors_from_answers(answers: List(Answer), profile: Profile) -> String`

Extract API behaviors from answers.

**Parameters:**
- `answers` - All answers
- `profile` - System profile

**Returns:** CUE behavior definitions as string

#### `extract_constraints_from_answers(answers: List(Answer)) -> List(String)`

Extract constraints and limits from answers.

**Parameters:**
- `answers` - All answers

**Returns:** List of constraint strings

---

## Quality Analysis

### quality_analyzer

**Module:** `src/intent/quality_analyzer.gleam`

Analyzes spec quality across multiple dimensions.

#### `analyze_spec(spec: Spec) -> QualityReport`

Analyze spec quality and generate report.

**Parameters:**
- `spec` - Specification to analyze

**Returns:** `QualityReport` with scores and suggestions

**Scoring Dimensions:**
- **Coverage** (0-100) - Error cases, edge cases, authentication
- **Clarity** (0-100) - Documentation, naming, examples
- **Testability** (0-100) - Rule specificity, examples, assertions
- **AI Readiness** (0-100) - Hints, entity definitions, implementation guidance

**Example:**
```gleam
let report = quality_analyzer.analyze_spec(spec)
io.println("Overall score: " <> int.to_string(report.overall_score))
io.println("Coverage: " <> int.to_string(report.coverage_score))
```

#### `format_report(report: QualityReport) -> String`

Format quality report as human-readable text.

**Parameters:**
- `report` - Quality report to format

**Returns:** Formatted report string

**Types:**

```gleam
pub type QualityReport {
  QualityReport(
    coverage_score: Int,
    clarity_score: Int,
    testability_score: Int,
    ai_readiness_score: Int,
    overall_score: Int,
    issues: List(QualityIssue),
    suggestions: List(String),
  )
}

pub type QualityIssue {
  MissingErrorTests
  MissingAuthenticationTest
  MissingEdgeCases
  VagueRules
  NoExamples
  MissingExplanations
  UntestedRules
  MissingAIHints
}
```

---

### spec_linter

**Module:** `src/intent/spec_linter.gleam`

Proactive detection of anti-patterns and quality issues.

#### `lint_spec(spec: Spec) -> LintResult`

Lint a complete spec for issues.

**Parameters:**
- `spec` - Specification to lint

**Returns:** `LintResult` - Valid or list of warnings

**What it checks:**
- Anti-patterns in response examples
- Vague or ambiguous rules
- Missing examples
- Naming convention violations
- Duplicate behaviors
- Unused anti-pattern definitions

**Example:**
```gleam
case spec_linter.lint_spec(spec) {
  spec_linter.LintValid -> io.println("No issues found")
  spec_linter.LintWarnings(warnings) -> {
    io.println(spec_linter.format_warnings(warnings))
  }
}
```

#### `format_warnings(warnings: List(LintWarning)) -> String`

Format lint warnings as human-readable text.

**Parameters:**
- `warnings` - List of warnings

**Returns:** Formatted warning text

**Types:**

```gleam
pub type LintResult {
  LintValid
  LintWarnings(warnings: List(LintWarning))
}

pub type LintWarning {
  AntiPatternDetected(behavior: String, pattern_name: String, details: String)
  VagueRule(behavior: String, field: String, rule: String)
  MissingExample(behavior: String)
  UnusedAntiPattern(pattern_name: String)
  NamingConvention(behavior: String, suggestion: String)
  DuplicateBehavior(behavior1: String, behavior2: String, similarity: String)
}
```

---

### improver

**Module:** `src/intent/improver.gleam`

Interactive specification refinement with improvement suggestions.

#### `suggest_improvements(context: ImprovementContext) -> List(ImprovementSuggestion)`

Generate improvement suggestions from analysis results.

**Parameters:**
- `context` - Context containing quality report, lint results, and spec

**Returns:** List of suggestions sorted by impact score (highest first)

**Example:**
```gleam
let context = improver.ImprovementContext(
  quality_report: quality_analyzer.analyze_spec(spec),
  lint_result: spec_linter.lint_spec(spec),
  spec: spec,
)
let suggestions = improver.suggest_improvements(context)
```

#### `format_improvements(suggestions: List(ImprovementSuggestion)) -> String`

Format suggestions as human-readable text.

**Parameters:**
- `suggestions` - List of suggestions

**Returns:** Formatted suggestion text

**Types:**

```gleam
pub type ImprovementSuggestion {
  ImprovementSuggestion(
    title: String,
    description: String,
    reasoning: String,
    impact_score: Int,
    proposed_change: ProposedChange,
  )
}

pub type ProposedChange {
  AddMissingTest(behavior_name: String, test_description: String)
  RefineVagueRule(behavior_name: String, field: String, better_rule: String)
  AddResponseExample(behavior_name: String)
  RenameForClarity(old_name: String, new_name: String)
  SimplifyRule(behavior_name: String, field: String, simpler_rule: String)
  AddExplanation(behavior_name: String, field: String, explanation: String)
}

pub type ImprovementContext {
  ImprovementContext(
    quality_report: QualityReport,
    lint_result: LintResult,
    spec: Spec,
  )
}
```

---

## HTTP & Networking

### http_client

**Module:** `src/intent/http_client.gleam`

HTTP client for executing behavior requests.

#### `execute_request(config: Config, req: Request, ctx: Context) -> Result(ExecutionResult, ExecutionError)`

Execute a behavior request against the target API.

**Parameters:**
- `config` - Configuration (base URL, timeout, headers)
- `req` - Request definition from behavior
- `ctx` - Interpolation context for variable substitution

**Returns:** `Result(ExecutionResult, ExecutionError)`

**Features:**
- Variable interpolation in paths, headers, and body
- SSRF protection (blocks private IPs, localhost, etc.)
- Request timing measurement
- Header merging (config + request headers)
- JSON body handling

**Example:**
```gleam
let config = types.Config(
  base_url: "http://localhost:8080",
  timeout_ms: 5000,
  headers: dict.from_list([#("Content-Type", "application/json")])
)
let ctx = interpolate.new_context()
case http_client.execute_request(config, request, ctx) {
  Ok(result) -> io.println("Status: " <> int.to_string(result.status))
  Error(e) -> io.println_error("Request failed")
}
```

**Types:**

```gleam
pub type ExecutionResult {
  ExecutionResult(
    status: Int,
    headers: Dict(String, String),
    body: Json,
    raw_body: String,
    elapsed_ms: Int,
    request_method: types.Method,
    request_path: String,
  )
}

pub type ExecutionError {
  UrlParseError(message: String)
  InterpolationError(message: String)
  RequestError(message: String)
  ResponseParseError(message: String)
  SSRFBlocked(message: String)
}
```

---

## Parsing & Formatting

### parser

**Module:** `src/intent/parser.gleam`

Parser for Intent specs from JSON (exported from CUE).

#### `parse_spec(data: Dynamic) -> Result(Spec, List(DecodeError))`

Parse a spec from JSON dynamic value.

**Parameters:**
- `data` - Dynamic JSON data from CUE export

**Returns:** `Result(Spec, List(DecodeError))`

**Note:** All fields are required - no backwards compatibility defaults

**Example:**
```gleam
case json.decode(json_str, dynamic.dynamic) {
  Ok(data) -> {
    case parser.parse_spec(data) {
      Ok(spec) -> // Use spec
      Error(errors) -> // Handle parse errors
    }
  }
  Error(e) -> // Handle JSON error
}
```

#### `dynamic_to_json(data: Dynamic) -> Json`

Convert a Dynamic value to Json.

**Parameters:**
- `data` - Dynamic value from CUE/JSON

**Returns:** Json value

**Utility:** Shared helper used across parsers for converting Dynamic to Json

**Example:**
```gleam
let json_value = parser.dynamic_to_json(dynamic_data)
```

---

### output

**Module:** `src/intent/output.gleam`

Output formatters for Intent results.

#### `spec_result_to_json(result: SpecResult) -> Json`

Convert spec result to JSON format.

**Parameters:**
- `result` - Spec execution result

**Returns:** Json object

**Example:**
```gleam
let result = runner.run_spec(spec, url, options)
let json_output = output.spec_result_to_json(result)
io.println(json.to_string(json_output))
```

#### `spec_result_to_text(result: SpecResult) -> String`

Convert spec result to human-readable text.

**Parameters:**
- `result` - Spec execution result

**Returns:** Formatted text output

**Example:**
```gleam
let result = runner.run_spec(spec, url, options)
io.println(output.spec_result_to_text(result))
```

#### `create_failure(feature: String, behavior: Behavior, check_result: ResponseCheckResult, execution: ExecutionResult, base_url: String) -> BehaviorFailure`

Create a failure record from check results.

**Parameters:**
- `feature` - Feature name
- `behavior` - Behavior that failed
- `check_result` - Check results
- `execution` - Execution result
- `base_url` - API base URL

**Returns:** `BehaviorFailure` record

#### `create_blocked(name: String, dependency: String) -> BlockedBehavior`

Create a blocked behavior record.

**Parameters:**
- `name` - Behavior name
- `dependency` - Failed dependency name

**Returns:** `BlockedBehavior` record

**Types:**

```gleam
pub type SpecResult {
  SpecResult(
    pass: Bool,
    passed: Int,
    failed: Int,
    blocked: Int,
    total: Int,
    summary: String,
    failures: List(BehaviorFailure),
    blocked_behaviors: List(BlockedBehavior),
    rule_violations: List(RuleViolationGroup),
    anti_patterns_detected: List(AntiPatternResult),
  )
}

pub type BehaviorFailure {
  BehaviorFailure(
    feature: String,
    behavior: String,
    intent: String,
    problems: List(Problem),
    request_sent: RequestSummary,
    response_received: ResponseSummary,
    hint: String,
    see_also: List(String),
  )
}

pub type BlockedBehavior {
  BlockedBehavior(
    behavior: String,
    reason: String,
    hint: String,
  )
}
```

---

### interpolate

**Module:** `src/intent/interpolate.gleam`

Variable interpolation for captured values. Handles `${variable}` syntax with array indexing support.

#### `new_context() -> Context`

Create a new empty context.

**Returns:** Empty interpolation context

**Example:**
```gleam
let ctx = interpolate.new_context()
```

#### `set_variable(ctx: Context, name: String, value: Json) -> Context`

Add a captured variable to context.

**Parameters:**
- `ctx` - Current context
- `name` - Variable name
- `value` - Variable value (Json)

**Returns:** Updated context

**Example:**
```gleam
let ctx = interpolate.new_context()
let ctx = interpolate.set_variable(ctx, "user_id", json.int(42))
```

#### `set_request_body(ctx: Context, body: Json) -> Context`

Set the request body in context for reference.

**Parameters:**
- `ctx` - Current context
- `body` - Request body Json

**Returns:** Updated context

#### `set_response_body(ctx: Context, body: Json) -> Context`

Set the response body in context for extraction.

**Parameters:**
- `ctx` - Current context
- `body` - Response body Json

**Returns:** Updated context

#### `get_variable(ctx: Context, name: String) -> Option(Json)`

Get a variable value from context.

**Parameters:**
- `ctx` - Context to query
- `name` - Variable name

**Returns:** `Option(Json)` - Some(value) if exists, None otherwise

#### `interpolate_string(ctx: Context, s: String) -> Result(String, String)`

Interpolate variables in a string. Replaces `${var_name}` with stringified values.

**Parameters:**
- `ctx` - Context with variables
- `s` - String with `${...}` placeholders

**Returns:** `Result(String, String)` - Interpolated string or error

**Features:**
- Supports nested paths: `${user.email}`
- Supports array indexing: `${items[0].id}`, `${array[-1]}`
- Detects circular references
- Depth limit (10) prevents infinite recursion

**Example:**
```gleam
let ctx = interpolate.set_variable(ctx, "user_id", json.int(42))
let result = interpolate.interpolate_string(ctx, "/users/${user_id}")
// Returns: Ok("/users/42")
```

#### `interpolate_headers(ctx: Context, headers: Dict(String, String)) -> Result(Dict(String, String), String)`

Interpolate all headers.

**Parameters:**
- `ctx` - Context with variables
- `headers` - Headers dictionary

**Returns:** `Result` with interpolated headers or error

#### `extract_capture(ctx: Context, path: String) -> Result(Json, String)`

Extract a value from response body using JSON path.

**Parameters:**
- `ctx` - Context with response body
- `path` - JSON path (e.g., "data.users[0].id")

**Returns:** `Result(Json, String)` - Extracted value or error

**Example:**
```gleam
// After setting response_body
let result = interpolate.extract_capture(ctx, "data.id")
```

**Types:**

```gleam
pub type Context {
  Context(
    variables: Dict(String, Json),
    request_body: Option(Json),
    response_body: Option(Json),
  )
}
```

---

## Utilities

### types

**Module:** `src/intent/types.gleam`

Core type definitions for Intent specifications.

**Key Types:**

```gleam
pub type Spec {
  Spec(
    name: String,
    description: String,
    audience: String,
    version: String,
    success_criteria: List(String),
    config: Config,
    features: List(Feature),
    rules: List(Rule),
    anti_patterns: List(AntiPattern),
    ai_hints: AIHints,
  )
}

pub type Config {
  Config(
    base_url: String,
    timeout_ms: Int,
    headers: Dict(String, String),
  )
}

pub type Feature {
  Feature(
    name: String,
    description: String,
    behaviors: List(Behavior),
  )
}

pub type Behavior {
  Behavior(
    name: String,
    intent: String,
    notes: String,
    requires: List(String),
    tags: List(String),
    request: Request,
    response: Response,
    captures: Dict(String, String),
  )
}

pub type Request {
  Request(
    method: Method,
    path: String,
    headers: Dict(String, String),
    query: Dict(String, Json),
    body: Json,
  )
}

pub type Response {
  Response(
    status: Int,
    example: Json,
    checks: Dict(String, Check),
    headers: Dict(String, String),
  )
}

pub type Check {
  Check(
    rule: String,
    why: String,
  )
}

pub type Method {
  Get
  Post
  Put
  Patch
  Delete
  Head
  Options
}

pub type Rule {
  Rule(
    name: String,
    description: String,
    when: When,
    check: RuleCheck,
    example: Json,
  )
}

pub type AntiPattern {
  AntiPattern(
    name: String,
    description: String,
    bad_example: Json,
    good_example: Json,
    why: String,
  )
}

pub type AIHints {
  AIHints(
    implementation: ImplementationHints,
    entities: Dict(String, EntityHint),
    security: SecurityHints,
    pitfalls: List(String),
  )
}
```

---

### security

**Module:** `src/intent/security.gleam`

Security utilities for file path validation and SSRF protection.

#### `validate_file_path(path: String) -> Result(String, SecurityError)`

Validate a file path for security (path traversal, etc.).

**Parameters:**
- `path` - File path to validate

**Returns:** `Result(String, SecurityError)` - Validated path or error

**Checks:**
- No path traversal (`..`)
- No absolute paths outside allowed directories
- No special characters
- No null bytes

**Example:**
```gleam
case security.validate_file_path(user_input_path) {
  Ok(safe_path) -> // Use safe_path
  Error(e) -> io.println_error(security.format_security_error(e))
}
```

#### `format_security_error(error: SecurityError) -> String`

Format security error as human-readable string.

**Parameters:**
- `error` - Security error

**Returns:** Formatted error message

**Types:**

```gleam
pub type SecurityError {
  PathTraversalAttempt(path: String)
  AbsolutePathNotAllowed(path: String)
  InvalidCharacters(path: String)
}
```

---

### cli_ui

**Module:** `src/intent/cli_ui.gleam`

CLI user interface utilities for consistent output formatting.

#### `print_header(text: String) -> Nil`

Print a formatted header.

**Parameters:**
- `text` - Header text

**Example:**
```gleam
cli_ui.print_header("Running Tests")
```

#### `print_success(text: String) -> Nil`

Print a success message (green).

**Parameters:**
- `text` - Success message

#### `print_error(text: String) -> Nil`

Print an error message (red).

**Parameters:**
- `text` - Error message

#### `print_warning(text: String) -> Nil`

Print a warning message (yellow).

**Parameters:**
- `text` - Warning message

#### `print_info(text: String) -> Nil`

Print an info message (blue).

**Parameters:**
- `text` - Info message

---

## Additional Modules

The following modules provide specialized functionality:

- **anti_patterns** - Anti-pattern detection in responses
- **array_indexing** - Array index parsing and extraction
- **bead_templates** - Work item generation from interviews
- **bead_feedback** - Bead execution feedback tracking
- **case_insensitive** - Case-insensitive string utilities
- **errors** - Centralized error type definitions
- **formats** - Format detection and handling
- **interview_questions** - Question database for interviews
- **interview_storage** - Session persistence (JSONL)
- **json_validator** - JSON schema validation
- **plan_mode** - Execution plan computation
- **question_loader** - Dynamic question loading
- **question_types** - Question type definitions
- **resolver** - Behavior dependency resolution
- **rule** - Rule parsing and evaluation
- **rules_engine** - Global rule checking
- **stdin** - Standard input utilities

---

## Cross-References

### Common Workflows

**1. Running a spec:**
```gleam
loader.load_spec("spec.cue")
|> result.map(fn(spec) {
  runner.run_spec(spec, "http://localhost:8080", runner.default_options())
})
|> result.map(output.spec_result_to_text)
```

**2. Quality analysis:**
```gleam
loader.load_spec("spec.cue")
|> result.map(fn(spec) {
  let report = quality_analyzer.analyze_spec(spec)
  let lint_result = spec_linter.lint_spec(spec)
  let context = improver.ImprovementContext(report, lint_result, spec)
  improver.suggest_improvements(context)
})
```

**3. Interview to spec:**
```gleam
// Create session
let session = interview.create_session(id, interview.Api, timestamp)

// Ask questions and collect answers
let session = interview.add_answer(session, answer)

// Generate spec
let spec_cue = spec_builder.build_spec_from_session(session)
```

---

## Module Count Summary

**Total Modules Documented:** 15 core modules

**Categories:**
- Core: 5 modules (intent, runner, loader, checker, validator)
- Interview Engine: 4 modules (interview, interview_questions, interview_storage, spec_builder)
- Quality Analysis: 3 modules (quality_analyzer, spec_linter, improver)
- HTTP & Networking: 1 module (http_client)
- Parsing & Formatting: 3 modules (parser, output, interpolate)
- Utilities: 3 modules (types, security, cli_ui)

**Additional specialized modules:** 15+ supporting modules for specific functionality

---

## Version History

- **1.0.0** (2026-01-11) - Initial API reference documentation

---

## Contributing

When adding new public functions:

1. Document function signature with parameter types
2. Provide clear parameter descriptions
3. Include at least one usage example
4. Document return types and error cases
5. Add cross-references to related functions
6. Update this API reference

---

## See Also

- [CLAUDE.md](../CLAUDE.md) - Project instructions and workflow
- [README.md](../README.md) - Project overview
- [examples/](../examples/) - Specification examples
