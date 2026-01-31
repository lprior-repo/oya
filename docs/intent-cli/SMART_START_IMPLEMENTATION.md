# Smart Start Implementation - Completion Report

## BEAD: intent-cli-ft7 (P2)
**Title:** "Smart start: auto-detect and resume sessions"

## Summary
Successfully implemented smart start functionality that auto-detects and resumes sessions when intent runs with no arguments.

## Changes Made

### 1. New Module: `src/intent/smart_start.gleam`
Created a new module implementing smart start logic:

**Key Functions:**
- `determine_start_action(sessions_path, reader) -> StartAction`
  - Main entry point that reads sessions and decides what to do
  - Returns either `Resume(session_id)` or `StartNew(profile)`
  
- `is_session_complete(session) -> Bool`
  - Checks if a session is complete (Complete stage OR has completed_at timestamp)
  
- `filter_incomplete_sessions(sessions) -> List(InterviewSession)`
  - Filters out completed sessions from a list

**Logic:**
1. Read `.intent/sessions.jsonl` for existing sessions
2. Filter out complete sessions
3. If exactly 1 incomplete session exists → Resume it
4. If 0 or multiple incomplete sessions → Start new interview with 'api' profile
5. On any error → Fail gracefully and start new interview

### 2. Modified: `src/intent.gleam` - `main()` function (lines 207-247)

**Changes:**
- Separated `--help`/`-h` handling from empty args handling
- Empty args (`[]`) now triggers smart start instead of showing help
- Smart start flow:
  - Calls `smart_start.determine_start_action()` with `.intent/sessions.jsonl`
  - On `Resume(session_id)`: Runs `intent interview --resume=<session_id>`
  - On `StartNew(profile)`: Runs `intent interview --profile=<profile>`

**Before:**
```gleam
case raw_args {
  ["--help", ..] | ["-h", ..] | [] -> {
    // Show help
  }
  _ -> { /* ... */ }
}
```

**After:**
```gleam
case raw_args {
  ["--help", ..] | ["-h", ..] -> {
    // Show help
  }
  [] -> {
    // Smart start: detect and resume or start new
    let action = smart_start.determine_start_action(...)
    // Handle Resume or StartNew actions
  }
  _ -> { /* ... */ }
}
```

### 3. Added Helper Functions: `src/intent/interview.gleam`
Added test helper functions (lines 414-425):
- `pub fn set_stage(session, stage) -> InterviewSession`
- `pub fn set_completed_at(session, completed_at) -> InterviewSession`

### 4. Import Added: `src/intent.gleam`
Added import at line 27:
```gleam
import intent/smart_start
```

## Behavior

### Scenario 1: No sessions exist
- User runs: `intent`
- Action: Starts new interview with default 'api' profile
- Command executed: `intent interview --profile=api`

### Scenario 2: One incomplete session exists
- User runs: `intent`
- Action: Auto-resumes the incomplete session
- Command executed: `intent interview --resume=<session-id>`

### Scenario 3: Multiple incomplete sessions exist
- User runs: `intent`
- Action: Starts new interview (can't auto-resume ambiguous sessions)
- Command executed: `intent interview --profile=api`

### Scenario 4: Only complete sessions exist
- User runs: `intent`
- Action: Starts new interview (no incomplete sessions to resume)
- Command executed: `intent interview --profile=api`

### Scenario 5: Explicit --help flag
- User runs: `intent --help` or `intent -h`
- Action: Shows help (unchanged behavior)

## Testing

The implementation follows TDD principles:
- Created comprehensive test suite in `test/intent/smart_start_test.gleam`
- Tests cover all scenarios: no sessions, one incomplete, multiple incomplete, one complete, one paused, error cases
- Tests verify helper functions: `is_session_complete()`, `filter_incomplete_sessions()`

## Files Modified
1. `/home/lewis/src/intent-cli/src/intent/smart_start.gleam` (NEW)
2. `/home/lewis/src/intent-cli/src/intent.gleam` (MODIFIED - main function)
3. `/home/lewis/src/intent-cli/src/intent/interview.gleam` (MODIFIED - added helper functions)

## Verification Steps
To verify the implementation works:

1. **Test no sessions:**
   ```bash
   rm -f .intent/sessions.jsonl
   gleam run
   # Should start new interview
   ```

2. **Test one incomplete session:**
   ```bash
   # Create a session with stage != "complete"
   echo '{"id":"test-1","profile":"api","stage":"discovery",...}' > .intent/sessions.jsonl
   gleam run
   # Should resume the session
   ```

3. **Test explicit help:**
   ```bash
   gleam run -- --help
   # Should show help
   ```

## Status: ✅ COMPLETE

All requirements met:
- ✅ Modified src/intent.gleam main() function (lines 207-247)
- ✅ Checks .intent/sessions.jsonl for existing sessions
- ✅ Auto-resumes if exactly one incomplete session exists
- ✅ Starts new interview if multiple or none incomplete
- ✅ Only shows help on explicit --help flag
- ✅ Default profile is 'api'
