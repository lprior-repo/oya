# Unified Error Handling System

## Overview

The `unified_errors` module provides a comprehensive, standardized error handling infrastructure for the Intent CLI. It enables consistent error reporting across all commands with support for:

- **Structured error codes** for semantic error routing
- **Severity levels** (Warning, Error, Fatal)
- **Exit code mapping** following Unix conventions
- **Contextual information** for debugging and recovery
- **Machine-readable JSON output** for AI consumption
- **Human-friendly text formatting** for CLI users
- **Actionable recovery suggestions** with optional fix commands

## Quick Start

### Creating an Error

```gleam
import intent/unified_errors

// Simple error
let error = unified_errors.missing_input(
  what: "spec file",
  suggestion: "Provide a spec file path argument"
)

// Error with context
let error = unified_errors.file_not_found(path: "/tmp/spec.cue")
  |> unified_errors.with_context(key: "expected_dir", value: ".interview/")

// Error with fix command
let error = unified_errors.invalid_input(
  input: "{invalid json}",
  reason: "Unexpected brace",
  suggestion: "Check JSON syntax"
)
```

### Outputting an Error

```gleam
// Output and exit with proper code (no halt for testing)
unified_errors.output_error(error: error, is_json: False)

// Output and halt with exit code
unified_errors.output_and_halt(error: error, is_json: True)

// Format as text
let text = unified_errors.format_error_text(error)
io.println_error(text)

// Convert to JSON
let json = unified_errors.unified_error_to_json(error)
json |> json.to_string() |> io.println()
```

## Error Codes

All error codes map to standard Unix exit codes:

| ErrorCode | Exit Code | Category | Use When |
|-----------|-----------|----------|----------|
| `MissingInput` | 2 | User Input | Required argument/file missing |
| `FileNotFound` | 2 | User Input | Specified file doesn't exist |
| `SessionNotFound` | 2 | User Input | Session ID not found |
| `ConflictingFlags` | 2 | User Input | Mutually exclusive flags used |
| `ValidationFailed` | 3 | Validation | Business rule or format violation |
| `InvalidInput` | 3 | Validation | Input format/value is invalid |
| `SpecParseError` | 3 | Validation | Spec cannot be parsed |
| `FilePermissionDenied` | 4 | Runtime | File access denied |
| `LoadError` | 4 | Runtime | Resource loading failed |
| `InternalError` | 5 | Internal | Invariant or panic condition |

## Exit Codes

Standard exit codes following Unix conventions:

```gleam
pub const exit_success = 0              // Successful completion
pub const exit_user_input_error = 2     // User provided bad input
pub const exit_validation_error = 3     // Validation failed
pub const exit_runtime_error = 4        // Runtime/I/O error
pub const exit_internal_error = 5       // Internal error (should not happen)
```

## Type Definition

```gleam
pub type UnifiedError {
  UnifiedError(
    code: ErrorCode,              // Error categorization
    message: String,              // User-facing message
    severity: Severity,           // Warning, Error, or Fatal
    context: Dict(String, String), // Additional context
    suggestion: String,           // Recovery/remediation hint
    fix_command: Option(String),  // Exact command to fix (if known)
    exit_code: Int,              // Standard exit code
  )
}

pub type Severity {
  Warning                        // Non-blocking
  Error                         // Should stop execution
  Fatal                         // Must stop immediately
}

pub type ErrorCode {
  MissingInput
  ValidationFailed
  FileNotFound
  FilePermissionDenied
  InvalidInput
  SpecParseError
  LoadError
  SessionNotFound
  ConflictingFlags
  InternalError
}
```

## Factory Functions

### General Purpose

```gleam
// Create error with all fields
unified_error(
  code: ErrorCode,
  message: String,
  suggestion: String,
  fix_command: Option(String),
) -> UnifiedError

// Full control over all fields
unified_error_full(
  code: ErrorCode,
  message: String,
  severity: Severity,
  context: Dict(String, String),
  suggestion: String,
  fix_command: Option(String),
) -> UnifiedError
```

### Specialized Builders

```gleam
// Input errors
missing_input(what: String, suggestion: String) -> UnifiedError
invalid_input(input: String, reason: String, suggestion: String) -> UnifiedError

// File errors
file_not_found(path: String) -> UnifiedError
file_permission_denied(path: String, operation: String) -> UnifiedError

// Validation errors
validation_failed(what: String, reason: String, suggestion: String) -> UnifiedError
spec_parse_error(path: String, reason: String) -> UnifiedError

// Session errors
session_not_found(session_id: String) -> UnifiedError

// General errors
load_error(resource: String, reason: String) -> UnifiedError
conflicting_flags(flag1: String, flag2: String) -> UnifiedError
internal_error(operation: String, reason: String) -> UnifiedError
```

## Context Management

Add debugging information and recovery context:

```gleam
// Add single context entry
error
|> unified_errors.with_context(key: "path", value: "/tmp/spec.cue")

// Add multiple entries
error
|> unified_errors.with_context_list(entries: [
  #("path", "/tmp/spec.cue"),
  #("reason", "File not found"),
  #("checked_at", "14:30:45"),
])

// Change severity
error
|> unified_errors.with_severity(sev: unified_errors.Warning)
```

## Formatting

### Text Output

```gleam
// Full detailed format (for human reading)
let text = unified_errors.format_error_text(error)
// Output:
// Error (code=file_not_found, severity=error, exit=2)
//
// Message:
//   File not found: /tmp/spec.cue
//
// Context:
//   path: /tmp/spec.cue
//
// Suggestion:
//   Check that the file exists at the specified path
//
// Fix Command:
//   ls -l /tmp/spec.cue

// Brief one-line format
let brief = unified_errors.format_error_brief(error)
// Output: file_not_found: File not found: /tmp/spec.cue (exit 2)
```

### JSON Output

```gleam
let json = unified_errors.unified_error_to_json(error)
json |> json.to_string() |> io.println()

// Output:
// {
//   "action": "error",
//   "error": {
//     "code": "file_not_found",
//     "message": "File not found: /tmp/spec.cue",
//     "severity": "error",
//     "context": {
//       "path": "/tmp/spec.cue"
//     },
//     "suggestion": "Check that the file exists at the specified path",
//     "fix_command": "ls -l /tmp/spec.cue",
//     "exit_code": 2
//   }
// }
```

## Output Functions

```gleam
// Output to stderr and halt with exit code
output_and_halt(error: UnifiedError, is_json: Bool) -> Nil

// Output to stderr without halting (for logging/testing)
output_error(error: UnifiedError, is_json: Bool) -> Nil
```

## Usage Examples

### Example 1: Missing Required Argument

```gleam
pub fn validate_command(args: List(String)) -> Result(Nil, UnifiedError) {
  case args {
    [] ->
      Error(
        unified_errors.missing_input(
          what: "spec file path",
          suggestion: "Provide a CUE or JSON spec file: intent validate myspec.cue",
        )
      )
    [path, ..] -> Ok(Nil)
  }
}
```

### Example 2: File Processing with Context

```gleam
pub fn process_spec_file(path: String) -> Result(Spec, UnifiedError) {
  case simplifile.read(path) {
    Error(e) ->
      Error(
        unified_errors.file_not_found(path: path)
        |> unified_errors.with_context(key: "error", value: string.inspect(e))
        |> unified_errors.with_context(key: "checked_at", value: timestamp())
      )
    Ok(content) -> parse_spec(content)
  }
}
```

### Example 3: Validation with Recovery

```gleam
pub fn validate_request(req: Request) -> Result(Nil, UnifiedError) {
  case validate_structure(req) {
    Error(msg) ->
      Error(
        unified_errors.validation_failed(
          what: "request structure",
          reason: msg,
          suggestion: "Check that all required fields are present and valid",
        )
        |> unified_errors.with_context(key: "received", value: json.to_string(req))
      )
    Ok(Nil) -> Ok(Nil)
  }
}
```

### Example 4: Conflicting Options

```gleam
pub fn export_command(args: Args) -> Result(Nil, UnifiedError) {
  case args.json, args.cue {
    True, True ->
      Error(
        unified_errors.conflicting_flags(flag1: "json", flag2: "cue")
        |> unified_errors.with_context(
          key: "explanation",
          value: "Can only export to one format",
        )
      )
    _ -> Ok(Nil)
  }
}
```

## Integration with Commands

Typical command pattern:

```gleam
pub fn my_command(spec_path: String, is_json: Bool) -> Nil {
  case load_and_process(spec_path) {
    Ok(result) -> {
      output_result(result, is_json)
    }
    Error(error) -> {
      unified_errors.output_and_halt(error: error, is_json: is_json)
    }
  }
}
```

## Testing

All functionality is tested in `test/intent/unified_errors_test.gleam`:

```bash
gleam test
```

Test coverage includes:
- Error code conversions
- Exit code mapping
- Factory functions
- Context manipulation
- Severity modification
- Text formatting
- JSON serialization
- All specialized builders

## Design Principles

1. **Type Safety**: All errors are strongly typed with exhaustive pattern matching
2. **Composability**: Builder functions and context methods enable flexible error construction
3. **Standardization**: Exit codes follow Unix conventions for CLI integration
4. **Actionability**: Every error includes a suggestion and optional fix command
5. **AI-Friendly**: JSON output with structured context for machine consumption
6. **User-Friendly**: Clear, helpful text formatting for human reading
7. **Immutability**: Errors are immutable records; use `with_*` functions for modifications

## Migration from Other Error Systems

### From `ai_errors.StructuredError`

```gleam
// Old style
ai_errors.StructuredError(
  error_type: FileNotFound,
  message: "File not found",
  context: dict.from_list([#("path", "/tmp/spec.cue")]),
  suggestion: "Check path",
  recovery: ["Verify file exists"],
  retry_allowed: True,
  exit_code: 4,
)

// New style
unified_errors.file_not_found(path: "/tmp/spec.cue")
|> unified_errors.with_context(key: "detail", value: "Cannot access")
```

## See Also

- `src/intent/ai_errors.gleam` - Legacy structured error handling
- `src/intent/errors.gleam` - Response validation errors
- `test/intent/unified_errors_test.gleam` - Comprehensive test suite
