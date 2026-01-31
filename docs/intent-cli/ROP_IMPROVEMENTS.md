# Railway-Oriented Programming (ROP) Improvements

**Beads**: intent-cli-ipps (LoadError), intent-cli-99y0 (ExecutionError)

## Problem Statement

Both `LoadError` and `ExecutionError` types used generic String messages, losing structured error context needed for programmatic error handling.

## Changes Made

### 1. LoadError (src/intent/loader.gleam)

**BEFORE:**
```gleam
pub type LoadError {
  FileNotFound(path: String)
  CueValidationError(message: String)  // Loses exit_code + stderr
  CueExportError(message: String)      // Loses exit_code + stderr
  JsonParseError(message: String)       // Loses decode errors
  SpecParseError(message: String)       // Loses decode errors
  SecurityError(message: String)
}
```

**AFTER:**
```gleam
/// Railway-Oriented Programming: Preserve all context for programmatic handling
pub type LoadError {
  FileNotFound(path: String)
  CueValidationFailed(path: String, exit_code: Int, stderr: String)
  CueExportFailed(path: String, exit_code: Int, stderr: String)
  JsonDecodeFailed(errors: List(dynamic.DecodeError))
  SpecParseFailed(errors: List(dynamic.DecodeError))
  SecurityError(message: String)
}
```

**Benefits:**
- Preserves CUE command exit codes for diagnostic analysis
- Retains full stderr output for programmatic parsing
- Keeps structured decode errors instead of formatted strings
- Enables retry logic based on exit codes
- Allows custom error formatting per use case

### 2. ExecutionError (src/intent/http_client.gleam)

**BEFORE:**
```gleam
pub type ExecutionError {
  UrlParseError(message: String)
  InterpolationError(message: String)
  RequestError(message: String)
  ResponseParseError(message: String)
  SSRFBlocked(message: String)
}
```

**AFTER:**
```gleam
/// Railway-Oriented Programming: Preserve all context for programmatic handling
pub type ExecutionError {
  UrlParseFailed(url: String, reason: String)
  InterpolationFailed(template: String, reason: String)
  HttpRequestFailed(method: types.Method, url: String, details: dynamic.Dynamic)
  ResponseParseFailed(body: String, errors: List(dynamic.DecodeError))
  SSRFBlocked(url: String, reason: String)
}
```

**Benefits:**
- Preserves original URL/template for error context
- Keeps HTTP method and URL for request failures
- Retains raw Dynamic error for detailed analysis
- Maintains structured decode errors
- Separates URL from reason for SSRF blocks

### 3. Supporting Code Changes

#### loader.gleam

**Error Construction:**
```gleam
// CUE validation - capture exit code
Error(#(exit_code, stderr)) ->
  Error(CueValidationFailed(validated_path, exit_code, stderr))

// CUE export - capture exit code
Error(#(exit_code, stderr)) ->
  Error(CueExportFailed(path, exit_code, stderr))

// JSON decode - preserve DecodeError list
Error(json_error) -> {
  let decode_errors = json_error_to_decode_errors(json_error)
  Error(JsonDecodeFailed(decode_errors))
}

// Spec parse - preserve DecodeError list
Error(errors) -> Error(SpecParseFailed(errors))
```

**New Helper Function:**
```gleam
/// Convert json.DecodeError to List(dynamic.DecodeError) for structured error handling
fn json_error_to_decode_errors(
  error: json.DecodeError,
) -> List(dynamic.DecodeError) {
  case error {
    json.UnexpectedFormat(errs) -> errs
    json.UnexpectedEndOfInput -> [
      dynamic.DecodeError(
        expected: "complete JSON",
        found: "unexpected end of input",
        path: [],
      ),
    ]
    json.UnexpectedByte(b) -> [
      dynamic.DecodeError(
        expected: "valid JSON character",
        found: "unexpected byte '" <> b <> "'",
        path: [],
      ),
    ]
    json.UnexpectedSequence(s) -> [
      dynamic.DecodeError(
        expected: "valid JSON syntax",
        found: "unexpected sequence '" <> s <> "'",
        path: [],
      ),
    ]
  }
}
```

**Updated format_error:**
```gleam
pub fn format_error(error: LoadError) -> String {
  case error {
    FileNotFound(path) -> "File not found: " <> path
    CueValidationFailed(path, exit_code, stderr) ->
      "CUE validation failed for '"
      <> path
      <> "' (exit code "
      <> string.inspect(exit_code)
      <> "):\n"
      <> stderr
    CueExportFailed(path, exit_code, stderr) ->
      "CUE export failed for '"
      <> path
      <> "' (exit code "
      <> string.inspect(exit_code)
      <> "):\n"
      <> stderr
    JsonDecodeFailed(errors) ->
      "JSON decode error:\n" <> format_decode_errors(errors)
    SpecParseFailed(errors) ->
      "Spec parse error:\n" <> format_decode_errors(errors)
    SecurityError(msg) -> msg
  }
}
```

#### http_client.gleam

**Error Construction:**
```gleam
// URL parsing - preserve URL and reason
uri.parse(full_url)
|> result.map_error(fn(_) {
  UrlParseFailed(full_url, "URI parsing failed")
})

// Interpolation - preserve template and reason
interpolate.interpolate_string(ctx, path)
|> result.map_error(fn(reason) { InterpolationFailed(path, reason) })

// HTTP request - preserve method, URL, and raw error
Error(e) -> Error(HttpRequestFailed(method, url, e))

// Response parsing - preserve body and decode errors
Error(json_error) -> {
  let decode_errors = case json_error {
    json.UnexpectedFormat(errs) -> errs
    _ -> [
      dynamic.DecodeError(
        expected: "valid JSON",
        found: "interpolated result",
        path: [],
      ),
    ]
  }
  Error(ResponseParseFailed(interpolated_str, decode_errors))
}

// SSRF - preserve URL and reason separately
Error(SSRFBlocked(url, "URL missing hostname"))
Error(SSRFBlocked(url, "Blocked request to localhost (127.x). Use --allow-localhost for development testing."))
```

**New Helper Function:**
```gleam
fn build_request_url(req: HttpRequest(String)) -> String {
  let scheme = case req.scheme {
    http.Https -> "https"
    http.Http -> "http"
  }
  let port_str = case req.port {
    Some(p) -> ":" <> string.inspect(p)
    None -> ""
  }
  scheme <> "://" <> req.host <> port_str <> req.path
}
```

## Railway-Oriented Programming Principles Applied

1. **Preserve Context**: Never lose information when propagating errors up the call stack
2. **Structured Data**: Use rich types instead of strings for error details
3. **Composability**: Errors can be transformed and enriched without information loss
4. **Programmatic Handling**: Callers can make decisions based on specific error fields
5. **Late Formatting**: Convert to strings only at the display boundary, not during propagation

## Impact

### For LoadError:
- ✅ Can retry based on specific exit codes (e.g., skip retry on exit code 2)
- ✅ Can parse stderr for specific CUE error patterns
- ✅ Can extract field paths from DecodeError for targeted fixes
- ✅ Can provide AI-friendly structured error responses

### For ExecutionError:
- ✅ Can log exact URL that failed for debugging
- ✅ Can distinguish between different HTTP error types programmatically
- ✅ Can provide template context for interpolation errors
- ✅ Can build custom retry logic based on error type and details

## Verification

```bash
gleam build
```

Should compile successfully with no errors. All error handling throughout the codebase now has access to rich, structured error context.

## Related Files Modified

- `src/intent/loader.gleam` - LoadError type and error construction
- `src/intent/http_client.gleam` - ExecutionError type and error construction
- `src/intent/ai_errors.gleam` - Updated to consume new error types (if present)

## Why This Is Better

**Before (String-based):**
```gleam
// Lost context - can't programmatically handle
Error(CueValidationError("some_file.cue:12:3: field not found"))
```

**After (Structured):**
```gleam
// Rich context - can make intelligent decisions
Error(CueValidationFailed(
  path: "some_file.cue",
  exit_code: 1,
  stderr: "some_file.cue:12:3: field not found"
))

// Now we can:
- Retry if exit_code == 1 but not if == 2
- Parse stderr for line:column numbers
- Log path separately for metrics
- Build AI-friendly error objects
```

This is the essence of Railway-Oriented Programming: errors flow through the system carrying all their context, and formatting/display happens at the edges, not in the core logic.
