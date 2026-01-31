# Skipped Lines Implementation - intent-cli-cbt1

## Summary

Surfaced the `skipped_lines` count in the `sessions` command JSON response to inform users when JSONL parse failures occur.

## Changes Made

### 1. interview_storage.gleam

#### Added Functions (Lines 1019-1025)
```gleam
/// List all sessions from JSONL file with corruption detection (with DI)
/// Returns ParseResult containing sessions and count of skipped corrupted lines
pub fn list_sessions_from_jsonl_with_warnings_io(
  jsonl_path: String,
  reader: FileReader,
) -> Result(ParseResult, String) {
  use content <- result.try(reader(jsonl_path))
  parse_sessions_content_with_warnings(content)
}
```

#### Added Convenience Wrapper (Lines 1073-1078)
```gleam
/// List all sessions from JSONL file with corruption detection using simplifile
/// Returns ParseResult containing sessions and count of skipped corrupted lines
pub fn list_sessions_from_jsonl_with_warnings(
  jsonl_path: String,
) -> Result(ParseResult, String) {
  list_sessions_from_jsonl_with_warnings_io(jsonl_path, simplifile_reader())
}
```

**Contract:** These functions use the existing `parse_sessions_content_with_warnings` function (lines 883-934) which:
- Returns `Ok(ParseResult)` with accurate `skipped_lines` count
- Returns `Error` if file appears completely corrupted (has content but no valid sessions)
- Never silently loses information about parse failures

### 2. session_commands.gleam

#### Updated sessions_command Function

**Changed:** Line 401
```gleam
// Before:
case interview_storage.list_sessions_from_jsonl(jsonl_path) {

// After:
case interview_storage.list_sessions_from_jsonl_with_warnings(jsonl_path) {
```

**Added:** skipped_lines field to all JSON responses:

1. Empty file response (line 428):
```gleam
json.object([
  #("sessions", json.array([], fn(_) { json.null() })),
  #("total", json.int(0)),
  #("skipped_lines", json.int(0)),  // NEW
]),
```

2. Empty sessions response (line 448):
```gleam
json.object([
  #("sessions", json.array([], fn(_) { json.null() })),
  #("total", json.int(0)),
  #("skipped_lines", json.int(parse_result.skipped_lines)),  // NEW
]),
```

3. Main sessions response (line 505):
```gleam
json.object([
  #("sessions", json.array(limited, interview_storage.session_to_json)),
  #("total", json.int(total_count)),
  #("shown", json.int(shown_count)),
  #("truncated", json.bool(was_limited)),
  #("skipped_lines", json.int(parse_result.skipped_lines)),  // NEW
]),
```

**Pattern Matching Update:** Changed from `Ok(sessions)` to `Ok(parse_result)` with nested match:
```gleam
Ok(parse_result) -> {
  case parse_result.sessions {
    [] -> { /* empty response */ }
    sessions -> { /* main processing */ }
  }
}
```

## Contract Fulfillment

### Break Scenarios Handled

1. **Empty collections**: Returns `skipped_lines: 0` for empty files
2. **Invalid JSON**: Counted in `skipped_lines`, sessions list excludes corrupted entries
3. **Completely corrupted file**: Returns Error (not silent failure)
4. **Mixed valid/invalid lines**: Valid sessions returned, invalid lines counted

### Type Safety

1. **ParseResult type** (already existed, lines 118-124 in interview_storage.gleam):
```gleam
pub type ParseResult {
  ParseResult(
    sessions: List(interview.InterviewSession),
    /// Number of lines that failed to parse (corrupted data)
    skipped_lines: Int,
  )
}
```

2. **Exhaustive pattern matching**: All cases handled (Ok/Error, empty/non-empty)
3. **No silent failures**: Every parse failure is counted and surfaced
4. **Pure functions**: `parse_sessions_content_with_warnings` is pure, I/O separated via DI

### JSON Schema Documentation

The `skipped_lines` field is now included in all sessions command responses:

**Field:** `skipped_lines`
**Type:** Integer
**Meaning:** Count of JSONL lines that failed to parse due to corruption
**Values:**
- `0`: All lines parsed successfully
- `> 0`: Some lines were corrupted and skipped

## Testing

### Manual Testing Approach

Create a test JSONL file with mixed content:
```bash
echo '{"id":"valid-1","profile":"api","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","completed_at":"","stage":"discovery","rounds_completed":0,"answers":[],"gaps":[],"conflicts":[],"raw_notes":""}
{"id":"corrupted","invalid_json
{"id":"valid-2","profile":"cli","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","completed_at":"","stage":"discovery","rounds_completed":0,"answers":[],"gaps":[],"conflicts":[],"raw_notes":""}' > .intent/sessions.jsonl

gleam run -- sessions
```

Expected output:
```json
{
  "sessions": [...2 sessions...],
  "total": 2,
  "shown": 2,
  "truncated": false,
  "skipped_lines": 1
}
```

### Adversarial Test Cases

1. **All valid lines**: `skipped_lines: 0`
2. **All invalid lines**: Error (file appears corrupted)
3. **Mixed valid/invalid**: `skipped_lines: N` where N = count of invalid lines
4. **Empty file**: `skipped_lines: 0`
5. **Whitespace only**: `skipped_lines: 0`

## Build Status

✅ Code compiles successfully with `gleam build`
✅ Type system enforces contract requirements
✅ No silent failures - all parse errors counted
✅ Backward compatible - existing valid JSONL files show `skipped_lines: 0`

## Files Modified

1. `/home/lewis/src/intent-cli/src/intent/interview_storage.gleam`
   - Added `list_sessions_from_jsonl_with_warnings_io` function
   - Added `list_sessions_from_jsonl_with_warnings` convenience wrapper

2. `/home/lewis/src/intent-cli/src/intent/session_commands.gleam`
   - Updated `sessions_command` to use ParseResult
   - Added `skipped_lines` field to all JSON responses
   - Updated pattern matching to handle ParseResult structure

## Incidental Fixes

Fixed pre-existing compilation errors in `/home/lewis/src/intent-cli/src/intent/atomic_file.gleam`:
1. Line 96: Changed `simplifile.Unknown(msg)` to `simplifile.Unknown` (arity fix)
2. Line 279: Changed `simplifile.rename` to `simplifile.rename_file` (API fix)

## Verification

```bash
# Verify compilation
gleam build

# Expected output: "Compiled in 0.05s" (with warnings but no errors)
```

## Contract Compliance Checklist

- [x] Accurate counting: Uses existing ParseResult.skipped_lines from parse_sessions_content_with_warnings
- [x] Field documented: Inline comments and this document
- [x] Type-safe: ParseResult type enforces Int for skipped_lines
- [x] JSON encoding: Uses json.int() for type-safe encoding
- [x] No information hiding: Every parse failure is counted and surfaced
- [x] Adversarial testing spec: Documented in "Adversarial Test Cases" section
- [x] All break scenarios handled: Documented in "Break Scenarios Handled" section

## Hostile Implementation Note

This implementation is **hostile** in the sense that it:
1. **Refuses to hide information**: Every parse failure is counted and reported
2. **Fails loudly**: Completely corrupted files return Error, not empty result
3. **Provides proof**: skipped_lines count serves as audit trail
4. **Cannot be bypassed**: Type system enforces ParseResult usage
