#!/usr/bin/env nu
# Liza: Behavioral Contract + Peer-Supervised Multi-Agent Code Evolution
# Blackboard-based state machine with adversarial validation loops.
# Targets: Rust/Gleam with end-to-end E2E testing as the moving objective.

# =========================
# Configuration & Paths
# =========================
def liza-dir [] { ".liza" }
def bb-path [] { $"(liza-dir)/blackboard.yml" }
def bb-lock-path [] { $"(liza-dir)/.blackboard.lock" }

# Shell runner: adapt this for your OS/env
# Default: bash (macOS/Linux). For Windows, use: pwsh -NoProfile -Command
def run-shell-cmd [cmd: string, --shell: string = "bash", --flag: string = "-lc"] {
  try {
    let result = (do { ^$shell $flag $cmd } | complete)
    {
      exit_code: $result.exit_code,
      stdout: ($result.stdout | str trim),
      stderr: ($result.stderr | str trim),
      ok: ($result.exit_code == 0)
    }
  } catch { |e|
    {
      exit_code: 126,
      stdout: "",
      stderr: $"Failed to run command: ($e.msg)",
      ok: false
    }
  }
}

# Atomic save with lock-and-temp pattern (idempotent directory creation)
def bb-save [bb: record] {
  try {
    # Ensure directory exists (idempotent)
    if not ((liza-dir) | path exists) {
      mkdir (liza-dir)
    }

    # Write to temp file (in system temp, then move atomically)
    let tmp = (mktemp -t "bb.XXXXXX.yml")
    $bb | to yaml | save --force $tmp

    # Atomic move (overwrites if exists)
    mv --force $tmp (bb-path)
  } catch { |e|
    error make { msg: $"Failed to save blackboard: ($e.msg)" }
  }
}

# Load blackboard with fallback for missing file
def bb-load [] {
  try {
    if not ((bb-path) | path exists) {
      error make { msg: $"Missing (bb-path). Run: nu liza.nu init" }
    }
    open (bb-path) --raw | from yaml
  } catch { |e|
    error make { msg: $"Failed to load blackboard: ($e.msg)" }
  }
}

# Timestamps (ISO8601)
def now-iso [] {
  (date now) | format date "%+"
}

def lease-expires-iso [minutes: int] {
  ((date now) + ($"($minutes)min" | into duration)) | format date "%+"
}

# =========================
# Blackboard Helpers
# =========================

# Task lookup (nil-safe)
def task-get [bb: record, task_id: string] {
  try {
    $bb.tasks | where id == $task_id | first
  } catch {
    null
  }
}

# Task upsert (ensure task exists in list)
def task-upsert [bb: record, task: record] {
  let others = ($bb.tasks | default [] | where id != $task.id)
  $bb | upsert tasks ($others | append $task)
}

# Task update by closure (functional transformation)
def task-update [bb: record, task_id: string, f: closure] {
  let updated = (
    $bb.tasks
    | default []
    | each {|t|
        if $t.id == $task_id { do $f $t } else { $t }
      }
  )
  $bb | upsert tasks $updated
}

# Task update: replace entire task record
def task-replace [bb: record, task: record] {
  let others = ($bb.tasks | default [] | where id != $task.id)
  $bb | upsert tasks ($others | append $task)
}

# Activity log (append-only journal)
def bb-log [bb: record, who: string, msg: string] {
  let entry = { ts: (now-iso), who: $who, msg: $msg }
  $bb | upsert activity (($bb.activity? | default []) | append $entry)
}

# State dump for debugging
def bb-show [--task: string = ""] {
  let bb = (bb-load)
  if $task == "" {
    $bb
  } else {
    task-get $bb $task
  }
}

# =========================
# Contract Gates (Assertions)
# =========================

# Gate: Prevent submission of unvalidated work
def assert-can-submit [t: record] {
  if ($t.status? | default "") != "IN_PROGRESS" {
    let st = ($t.status? | default "")
    error make { msg: $"Task ($t.id) must be IN_PROGRESS to submit, current status is ($st)." }
  }
  if (($t.claim.agent_id? | default "") | str length) == 0 {
    error make { msg: $"Task ($t.id) missing claim.agent_id." }
  }
}

# Gate: Prevent review without validation evidence
def assert-can-review [t: record] {
  if ($t.status? | default "") != "READY_FOR_REVIEW" {
    let st = ($t.status? | default "")
    error make { msg: $"Task ($t.id) must be READY_FOR_REVIEW to review, current status is ($st)." }
  }
  let commit = ($t.submission.commit? | default "" | str trim)
  if ($commit | str length) == 0 {
    error make { msg: $"Task ($t.id) missing submission.commit SHA." }
  }
  let validations = ($t.submission.validation? | default [])
  if ($validations | length) == 0 {
    error make { msg: $"Task ($t.id) missing submission.validation results. Run: nu liza.nu validate $task_id" }
  }
}

# Gate: Prevent merging unreviewed code
def assert-can-merge [t: record] {
  if ($t.status? | default "") != "APPROVED" {
    let st = ($t.status? | default "")
    error make { msg: $"Task ($t.id) must be APPROVED to merge, current status is ($st)." }
  }
  if (($t.review.decision? | default "") | str trim) != "APPROVED" {
    error make { msg: $"Task ($t.id) review decision is not APPROVED." }
  }
}

# Guard: Prevent test weakening (tests are part of contract)
def assert-no-test-weakening [t: record] {
  if ($t.allow_test_changes? | default false) { return }
  let changed = ($t.submission.changed_files? | default [])
  let test_changes = ($changed | where {|p| ($p | str starts-with "tests/") or ($p | str starts-with "test_") })
  if ($test_changes | length) > 0 {
    error make { msg: $"Task ($t.id) modifies tests, but allow_test_changes is false. Tests are load-bearing." }
  }
}

# =========================
# Commands: Initialization
# =========================

def cmd-init [--force] {
  if ((bb-path) | path exists) and not $force {
    error make { msg: $"(bb-path) already exists. Use --force to overwrite." }
  }

  let bb = {
    version: 1,
    created_at: (now-iso),
    tasks: [],
    activity: []
  }

  bb-save $bb
  print $"✓ Initialized (bb-path)"
}

# =========================
# Commands: Task Management
# =========================

def cmd-task-add [
  task_id: string,
  --spec_ref: string = "specs/vision.md",
  --lease_min: int = 30
] {
  let bb = (bb-load)
  let existing = (task-get $bb $task_id)

  if $existing != null {
    error make { msg: $"Task already exists: ($task_id)" }
  }

  let t = {
    id: $task_id,
    status: "UNCLAIMED",
    spec_ref: $spec_ref,
    lease_minutes: $lease_min,
    done_when: [],
    allow_test_changes: false,
    generation: 0,
    landscape: {},

    claim: { agent_id: null, lease_expires: null, base_commit: null },
    submission: { commit: null, changed_files: [], validation: [] },
    review: { agent_id: null, decision: null, notes: null }
  }

  let bb2 = (bb-log (task-upsert $bb $t) "supervisor" $"Added task ($task_id)")
  bb-save $bb2
  print $"✓ Created task: ($task_id)"
}

# IMPORTANT: When a survivor is found (bug proven by exit code), the caller MUST:
#   1. Call task-add-check to lock the regression (this function)
#   2. Call `bd create --title "[Red Queen] <SEVERITY>: <finding>" --type=bug` IMMEDIATELY
#   Beads are filed at the moment of selection, NEVER deferred to session end.
def cmd-task-add-check [
  task_id: string,
  cmd: string,
  --expect_exit: int = 0
] {
  let bb = (bb-load)
  let t = (task-get $bb $task_id)

  if $t == null {
    error make { msg: $"Task not found: ($task_id)" }
  }

  # Create updated task with new check
  let exit_val = $expect_exit
  let updated_task = $t | upsert done_when (($t.done_when | default []) | append { cmd: $cmd, expect_exit: $exit_val })

  let bb2 = (bb-log (task-replace $bb $updated_task) "planner" $"Added check to ($task_id): ($cmd)")

  bb-save $bb2
  let msg = $"Added done_when check with exit code: ($exit_val)"
  print $msg
}

def cmd-task-list [--detail = false] {
  let bb = (bb-load)
  let tasks = ($bb.tasks | default [])

  if $detail {
    $tasks
  } else {
    $tasks | select id status spec_ref (claim.agent_id? | default "—")
  }
}

# =========================
# Commands: Workflow (Claim → Code → Submit → Validate → Review → Merge)
# =========================

def cmd-claim [task_id: string, agent_id: string] {
  let bb = (bb-load)
  let t = (task-get $bb $task_id)

  if $t == null {
    error make { msg: $"Task not found: ($task_id)" }
  }

  if ($t.status? | default "") != "UNCLAIMED" {
    error make { msg: $"Task ($task_id) not claimable (status: ($t.status?))." }
  }

  # Capture base commit (fail hard if git is unavailable)
  let head = (run-shell-cmd "git rev-parse HEAD")
  if not $head.ok {
    error make { msg: $"git rev-parse failed: ($head.stderr)" }
  }

  let updated_task = $t
    | upsert status "IN_PROGRESS"
    | upsert claim {
        agent_id: $agent_id,
        lease_expires: (lease-expires-iso ($t.lease_minutes | default 30)),
        base_commit: ($head.stdout | str trim)
      }

  let bb2 = (bb-log (task-replace $bb $updated_task) $agent_id $"Claimed ($task_id)")

  bb-save $bb2
  print $"✓ UNCLAIMED → IN_PROGRESS: ($task_id) by ($agent_id)"
}

def cmd-coder-submit [task_id: string, agent_id: string, --commit: string = ""] {
  let bb = (bb-load)
  let t = (task-get $bb $task_id)

  if $t == null {
    error make { msg: $"Task not found: ($task_id)" }
  }

  assert-can-submit $t

  # Resolve commit SHA (from argument or git HEAD)
  let c = (
    if (($commit | str length) > 0) {
      $commit
    } else {
      let head = (run-shell-cmd "git rev-parse HEAD")
      if not $head.ok { error make { msg: $"git rev-parse failed: ($head.stderr)" } }
      $head.stdout | str trim
    }
  )

  # Compute changed files
  let base = ($t.claim.base_commit? | default "")
  if (($base | str length) == 0) {
    error make { msg: $"Task ($task_id) missing claim.base_commit" }
  }

  let diff = (run-shell-cmd $"git diff --name-only ($base) ($c)")
  if not $diff.ok {
    error make { msg: $"git diff failed: ($diff.stderr)" }
  }

  let changed = ($diff.stdout | lines | where ($it | str length) > 0)

  let updated_task = $t
    | upsert status "READY_FOR_REVIEW"
    | upsert submission (
        $t.submission
        | upsert commit $c
        | upsert changed_files $changed
      )

  let bb2 = (bb-log (task-replace $bb $updated_task) $agent_id $"Submitted ($task_id) at ($c)")

  bb-save $bb2
  print $"✓ IN_PROGRESS → READY_FOR_REVIEW: ($task_id)"
  print $"  Files changed: (($changed | length))"
}

# =========================
# Commands: Validation (Supervisor)
# =========================

def cmd-supervisor-validate [
  task_id: string,
  --shell: string = "bash",
  --flag: string = "-lc"
] {
  let bb = (bb-load)
  let t = (task-get $bb $task_id)

  if $t == null {
    error make { msg: $"Task not found: ($task_id)" }
  }

  if ($t.status? | default "") != "READY_FOR_REVIEW" {
    let st = ($t.status? | default "")
    error make { msg: $"Task ($task_id) is not READY_FOR_REVIEW, status is ($st)" }
  }

  let checks = ($t.done_when? | default [])
  if ($checks | length) == 0 {
    error make { msg: $"Task ($task_id) has no done_when checks. Define them with: nu liza.nu task-add-check ($task_id) <cmd>" }
  }

  # Run all checks, capture results
  let results = (
    $checks
    | each {|dw|
        let r = (run-shell-cmd $dw.cmd --shell $shell --flag $flag)
        {
          ts: (now-iso),
          cmd: $dw.cmd,
          expect_exit: $dw.expect_exit,
          exit_code: $r.exit_code,
          ok: ($r.exit_code == $dw.expect_exit),
          stdout: $r.stdout,
          stderr: $r.stderr
        }
      }
  )

  # Check if all passed
  let all_ok = ($results | all {|r| $r.ok })

  # Save results regardless (for audit trail)
  let updated_task = $t | upsert submission ($t.submission | upsert validation $results)
  let bb2 = (bb-log (task-replace $bb $updated_task) "supervisor" (if $all_ok { $"Validation OK for ($task_id)" } else { $"Validation FAILED for ($task_id)" }))
  bb-save $bb2

  if not $all_ok {
    print $"✗ FAILED Validation for ($task_id)"
    $results | where not ok | each {|r|
      print $"  ✗ [exit: ($r.exit_code), expected: ($r.expect_exit)]: ($r.cmd)"
      if (($r.stderr | str length) > 0) { print $"    stderr: ($r.stderr)" }
    }
    error make { msg: $"Validation failed. See blackboard for details." }
  }

  print $"✓ Validation OK: ($task_id)"
  print $"  Checks passed: (($results | length))"
}

# =========================
# Commands: Review (Reviewer)
# =========================

def cmd-review-approve [
  task_id: string,
  reviewer_id: string,
  --notes: string = ""
] {
  let bb = (bb-load)
  let t = (task-get $bb $task_id)

  if $t == null {
    error make { msg: $"Task not found: ($task_id)" }
  }

  assert-can-review $t
  assert-no-test-weakening $t

  let updated_task = $t
    | upsert status "APPROVED"
    | upsert review { agent_id: $reviewer_id, decision: "APPROVED", notes: $notes }

  let bb2 = (bb-log (task-replace $bb $updated_task) $reviewer_id $"Approved ($task_id)")

  bb-save $bb2
  print $"✓ READY_FOR_REVIEW → APPROVED: ($task_id)"
}

def cmd-review-reject [
  task_id: string,
  reviewer_id: string,
  notes: string
] {
  let bb = (bb-load)
  let t = (task-get $bb $task_id)

  if $t == null {
    error make { msg: $"Task not found: ($task_id)" }
  }

  assert-can-review $t

  let updated_task = $t
    | upsert status "IN_PROGRESS"
    | upsert review { agent_id: $reviewer_id, decision: "REJECTED", notes: $notes }

  let bb2 = (bb-log (task-replace $bb $updated_task) $reviewer_id $"Rejected ($task_id): ($notes)")

  bb-save $bb2
  print $"✓ READY_FOR_REVIEW → IN_PROGRESS for rework: ($task_id)"
  print $"  Reviewer notes: ($notes)"
}

# =========================
# Commands: Merge (Supervisor)
# =========================

def cmd-supervisor-merge [
  task_id: string,
  --branch: string = "integration"
] {
  let bb = (bb-load)
  let t = (task-get $bb $task_id)

  if $t == null {
    error make { msg: $"Task not found: ($task_id)" }
  }

  assert-can-merge $t

  let sha = ($t.submission.commit? | default "" | str trim)
  if (($sha | str length) == 0) {
    error make { msg: $"Task ($task_id) missing submission.commit" }
  }

  # Switch to branch
  let co = (run-shell-cmd $"git checkout ($branch)")
  if not $co.ok {
    error make { msg: $"git checkout ($branch) failed: ($co.stderr)" }
  }

  # Fast-forward merge
  let ff = (run-shell-cmd $"git merge --ff-only ($sha)")
  if not $ff.ok {
    error make { msg: $"git merge --ff-only failed: ($ff.stderr)" }
  }

  let updated_task = $t | upsert status "MERGED"
  let bb2 = (bb-log (task-replace $bb $updated_task) "supervisor" $"Merged ($task_id) at ($sha) into ($branch)")

  bb-save $bb2
  print $"✓ APPROVED → MERGED: ($task_id)"
  print $"  Branch: ($branch)"
  print $"  Commit: ($sha)"
}

# =========================
# Commands: Regression (DRQ dynamic objective)
# =========================

# Add a new done_when check ONLY if:
#   (1) it fails on the current champion (blackboard commit)
#   (2) it passes on the candidate (provided commit)
# This enforces that the test bank only grows with real regressions, not false positives.
#
# IMPORTANT: After calling regress, the caller MUST also run:
#   bd create --title "[Red Queen] <SEVERITY>: <finding>" --type=bug
# Beads are filed at the moment of selection, NEVER deferred to session end.

def cmd-regress [
  task_id: string,
  cmd: string,
  --expect_exit: int = 0,
  --force
] {
  if not $force {
    # Validate the command works (dry-run on current champion)
    let champion_test = (run-shell-cmd $cmd)
    let candidate_test = (run-shell-cmd $cmd)

    if $champion_test.ok {
      print "⚠ Warning: Test already passes on current code. Skipping regression (use --force to override)."
      return
    }

    if not $candidate_test.ok {
      error make { msg: $"Candidate code does not pass the test yet. Fix the code first." }
    }
  }

  # Test passes on candidate, fails on champion → add to permanent test bank
  let bb = (bb-load)
  let t = (task-get $bb $task_id)

  if $t == null {
    error make { msg: $"Task not found: ($task_id)" }
  }

  let updated_task = $t | upsert done_when (($t.done_when | default []) | append { cmd: $cmd, expect_exit: $expect_exit })
  let bb2 = (bb-log (task-replace $bb $updated_task) "supervisor" $"Added regression check to ($task_id): ($cmd)")
  bb-save $bb2
  print $"✓ Regression locked in: ($cmd)"
}

# =========================
# Commands: DRQ Evolution (Generation + Landscape)
# =========================

# Start a new generation — increments the counter deterministically
def cmd-gen-start [task_id: string] {
  let bb = (bb-load)
  let t = (task-get $bb $task_id)

  if $t == null {
    error make { msg: $"Task not found: ($task_id)" }
  }

  let gen = (($t.generation? | default 0) + 1)
  let updated_task = $t | upsert generation $gen
  let bb2 = (bb-log (task-replace $bb $updated_task) "supervisor" $"Generation ($gen) started for ($task_id)")
  bb-save $bb2
  print $"✓ Generation ($gen) started: ($task_id)"
}

# Record a survivor — locks regression + updates landscape atomically
# The caller MUST also run: bd create --title "[Red Queen] ..." --type=bug
def cmd-gen-survivor [
  task_id: string,
  dimension: string,
  cmd: string,
  --expect_exit: int = 0,
  --severity: string = "MAJOR"
] {
  let bb = (bb-load)
  let t = (task-get $bb $task_id)

  if $t == null {
    error make { msg: $"Task not found: ($task_id)" }
  }

  let gen = ($t.generation? | default 0)

  # Lock regression into done_when
  let done_entry = { cmd: $cmd, expect_exit: $expect_exit, generation: $gen, dimension: $dimension, severity: $severity }
  let updated_done = (($t.done_when | default []) | append $done_entry)

  # Update landscape: increment survivors for this dimension
  let landscape = ($t.landscape? | default {})
  let dim_data = ($landscape | get -o $dimension | default { tests_run: 0, survivors: 0 })
  let updated_dim = $dim_data | upsert survivors (($dim_data.survivors? | default 0) + 1)
  let updated_landscape = ($landscape | upsert $dimension $updated_dim)

  let updated_task = $t
    | upsert done_when $updated_done
    | upsert landscape $updated_landscape

  let bb2 = (bb-log (task-replace $bb $updated_task) "red-queen" $"Gen ($gen) survivor [($severity)] in ($dimension): ($cmd)")
  bb-save $bb2
  print $"✓ Survivor locked [($severity)] gen=($gen) dim=($dimension): ($cmd)"
}

# Record a test that found nothing — updates landscape tests_run only
def cmd-gen-discard [
  task_id: string,
  dimension: string
] {
  let bb = (bb-load)
  let t = (task-get $bb $task_id)

  if $t == null {
    error make { msg: $"Task not found: ($task_id)" }
  }

  # Update landscape: increment tests_run for this dimension
  let landscape = ($t.landscape? | default {})
  let dim_data = ($landscape | get -o $dimension | default { tests_run: 0, survivors: 0 })
  let updated_dim = $dim_data | upsert tests_run (($dim_data.tests_run? | default 0) + 1)
  let updated_landscape = ($landscape | upsert $dimension $updated_dim)

  let updated_task = $t | upsert landscape $updated_landscape
  let bb2 = (task-replace $bb $updated_task)
  bb-save $bb2
}

# Show landscape fitness scores — deterministic computation
def cmd-landscape [task_id: string] {
  let bb = (bb-load)
  let t = (task-get $bb $task_id)

  if $t == null {
    error make { msg: $"Task not found: ($task_id)" }
  }

  let gen = ($t.generation? | default 0)
  let landscape = ($t.landscape? | default {})

  print $"Generation: ($gen)"
  print $"Lineage size: (($t.done_when | default [] | length))"
  print ""
  print "Dimension              Tests  Survivors  Fitness  Status"
  print "─────────────────────  ─────  ─────────  ───────  ──────────"

  $landscape | transpose dim data | each {|row|
    let tests = ($row.data.tests_run? | default 0)
    let survivors = ($row.data.survivors? | default 0)
    let fitness = if $tests > 0 { ($survivors / $tests) | math round --precision 3 } else { 0.0 }
    let status = if $tests == 0 { "UNEXPLORED" } else if $fitness == 0.0 { "EXHAUSTED" } else if $fitness < 0.2 { "COOLING" } else { "ACTIVE" }
    let dim_padded = ($row.dim | fill -a left -w 23 -c ' ')
    print $"($dim_padded)  ($tests | fill -a right -w 5)  ($survivors | fill -a right -w 9)  ($fitness | fill -a right -w 7)  ($status)"
  }

  # Crown status
  print ""
  let total_survivors = ($landscape | transpose dim data | each {|r| $r.data.survivors? | default 0 } | math sum)
  let has_critical = (($t.done_when | default []) | any {|dw| ($dw.severity? | default "") == "CRITICAL" })

  if $has_critical {
    print "Crown: FORFEIT (CRITICAL survivor exists)"
  } else if $total_survivors > 0 {
    let surv_count = $total_survivors
    print $"Crown: CONTESTED — ($surv_count) survivors in lineage"
  } else {
    print "Crown: DEFENDED (no survivors — all tests passed)"
  }
}

# =========================
# CLI: Nushell native subcommands
# =========================

# =========================
# CLI: Nushell native subcommands
# =========================

def main [] {
  print ([
    "Liza: Behavioral Contract + Peer-Supervised Code Evolution",
    "",
    "Usage: nu liza-advanced.nu <command> [args]",
    "",
    "Setup:",
    "  init                  Initialize blackboard",
    "",
    "Task Management:",
    "  task-add              Create a new task",
    "  task-add-check        Add a done_when check to a task",
    "  task-list             List all tasks",
    "",
    "Workflow:",
    "  claim                 Claim a task for work",
    "  coder-submit          Submit work for review",
    "  validate              Run all done_when checks (the ratchet)",
    "  approve               Approve a task",
    "  reject                Reject a task with notes",
    "  merge                 Merge an approved task",
    "",
    "DRQ Evolution:",
    "  gen-start             Start a new generation (increments counter)",
    "  gen-survivor          Lock a survivor: regression + landscape update + bd create reminder",
    "  gen-discard           Record a test that found nothing (landscape only)",
    "  landscape             Show fitness scores, crown status (all computed)",
    "  regress               Add a regression test to the permanent bank",
    "",
    "State:",
    "  show                  Show blackboard or task state",
  ] | str join "\n")
}

def "main init" [--force] {
  if $force {
    cmd-init --force
  } else {
    cmd-init
  }
}

def "main task-add" [
  task_id: string,
  --spec_ref: string = "specs/vision.md",
  --lease_min: int = 30
] {
  cmd-task-add $task_id --spec_ref $spec_ref --lease_min $lease_min
}

def "main task-add-check" [
  task_id: string,
  cmd: string,
  --expect_exit: int = 0
] {
  cmd-task-add-check $task_id $cmd --expect_exit $expect_exit
}

def "main task-list" [--detail] {
  if $detail {
    cmd-task-list --detail=true
  } else {
    cmd-task-list
  }
}

def "main claim" [task_id: string, agent_id: string] {
  cmd-claim $task_id $agent_id
}

def "main coder-submit" [
  task_id: string,
  agent_id: string,
  --commit: string = ""
] {
  if ($commit | str length) > 0 {
    cmd-coder-submit $task_id $agent_id --commit $commit
  } else {
    cmd-coder-submit $task_id $agent_id
  }
}

def "main validate" [
  task_id: string,
  --shell: string = "bash",
  --flag: string = "-lc"
] {
  cmd-supervisor-validate $task_id --shell $shell --flag $flag
}

def "main approve" [
  task_id: string,
  reviewer_id: string,
  --notes: string = ""
] {
  cmd-review-approve $task_id $reviewer_id --notes $notes
}

def "main reject" [
  task_id: string,
  reviewer_id: string,
  notes: string
] {
  cmd-review-reject $task_id $reviewer_id $notes
}

def "main merge" [
  task_id: string,
  --branch: string = "integration"
] {
  cmd-supervisor-merge $task_id --branch $branch
}

def "main regress" [
  task_id: string,
  cmd: string,
  --expect_exit: int = 0,
  --force
] {
  if $force {
    cmd-regress $task_id $cmd --expect_exit $expect_exit --force
  } else {
    cmd-regress $task_id $cmd --expect_exit $expect_exit
  }
}

def "main show" [--task: string = ""] {
  if $task == "" {
    bb-show
  } else {
    bb-show --task $task
  }
}

# DRQ Evolution commands
def "main gen-start" [task_id: string] {
  cmd-gen-start $task_id
}

def "main gen-survivor" [
  task_id: string,
  dimension: string,
  cmd: string,
  --expect_exit: int = 0,
  --severity: string = "MAJOR"
] {
  cmd-gen-survivor $task_id $dimension $cmd --expect_exit $expect_exit --severity $severity
}

def "main gen-discard" [
  task_id: string,
  dimension: string
] {
  cmd-gen-discard $task_id $dimension
}

def "main landscape" [task_id: string] {
  cmd-landscape $task_id
}
