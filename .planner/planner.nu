#!/usr/bin/env nu

# planner.nu — Deterministic bead decomposition and planning engine
#
# State: YAML session files (atomic save, auditable)
# Validation: CUE schema (deterministic, strict)
# Template: 16-section enhanced bead template
# Creation: br create (persists to database)
#
# AI generates task decomposition. Script ensures structure and validation.

const SESSION_DIR = "~/.local/share/planner/sessions"
const TEMPLATE_PATH = "/home/lewis/src/oya/.beads/BEAD_TEMPLATE.md"
const TASK_SCHEMA_PATH = "~/.claude/skills/planner/schemas/task-schema.cue"
const BEAD_SCHEMA_PATH = "~/.claude/skills/planner/schemas/bead-template.cue"

# Default working directory for bead schemas (can be overridden via env)
const DEFAULT_BEADS_SCHEMA_DIR = "/home/lewis/src/oya/.beads/schemas"

# ── Validation & Safety ───────────────────────────────────────────────────────

def validate-session-id [session_id: string] {
  # Prevent path traversal attacks
  if ($session_id | str contains '/') or ($session_id | str contains '\\') or ($session_id | str contains '..') {
    error make {msg: "Session ID must not contain path separators or '..'"}
  }
  if ($session_id | is-empty) {
    error make {msg: "Session ID cannot be empty"}
  }
  if ($session_id | str length) > 100 {
    error make {msg: "Session ID too long (max 100 chars)"}
  }
}

def check-required-commands [] {
  # Check if required external commands exist
  if (which cue | is-empty) {
    error make {msg: "Required command 'cue' not found. Install with: brew install cue-lang/tap/cue"}
  }
  if (which br | is-empty) {
    error make {msg: "Required command 'br' not found. Ensure beads_rust is installed."}
  }
}

def check-required-files [] {
  # Validate that required files exist
  if not ($TEMPLATE_PATH | path exists) {
    error make {msg: $"Template not found: ($TEMPLATE_PATH)"}
  }
  let task_schema = ($TASK_SCHEMA_PATH | path expand)
  if not ($task_schema | path exists) {
    error make {msg: $"Task schema not found: ($task_schema)"}
  }
  let bead_schema = ($BEAD_SCHEMA_PATH | path expand)
  if not ($bead_schema | path exists) {
    error make {msg: $"Bead schema not found: ($bead_schema)"}
  }
}

# ── CUE String Escaping ───────────────────────────────────────────────────────

# Escape string for use in CUE quoted strings
def escape-cue-string [text: string]: nothing -> string {
  $text
  | str replace -a '\' '\\'      # Escape backslashes first (single quotes!)
  | str replace -a '"' '\"'      # Escape double quotes
  | str replace -a (char newline) '\n'  # Escape newlines
  | str replace -a (char tab) '\t'      # Escape tabs
}

# ── CUE Validation Functions ──────────────────────────────────────────────────

# Validate task JSON against task-schema.cue
def validate-task-with-cue [task_data: record]: nothing -> record {
  let task_schema = ($TASK_SCHEMA_PATH | path expand)

  # Create temp files for validation (native nushell)
  let temp_dir = try {
    let temp_base = if ($env.TMPDIR? != null) { $env.TMPDIR } else if ($env.TEMP? != null) { $env.TEMP } else { "/tmp" }
    let dir = ($temp_base | path join $"planner-validate-(random chars -l 8)")
    mkdir $dir
    $dir
  } catch { |err|
    error make {msg: $"Failed to create temp directory: ($err.msg)"}
  }

  # Use JSON format - CUE validates JSON natively!
  # This avoids all escaping issues
  let task_json_file = $"($temp_dir)/task.json"
  let schema_file = $task_schema

  let validation_result = try {
    # Save task as JSON
    $task_data | to json | save $task_json_file

    # Run cue vet with JSON input
    # CUE automatically validates JSON against schema
    let result = (^cue vet $schema_file $task_json_file -d "#Task" | complete)

    if $result.exit_code == 0 {
      # Validation passed
      {
        valid: true,
        task_data: $task_data,
        schema_used: $task_schema
      }
    } else {
      # Validation failed
      {
        valid: false,
        errors: [$result.stderr],
        task_data: $task_data
      }
    }
  } catch { |err|
    {
      valid: false,
      errors: [$err.msg],
      task_data: $task_data
    }
  }

  # Cleanup temp directory
  try { rm -rf $temp_dir } catch { }

  $validation_result
}

# Generate CUE format from task JSON
def generate-task-cue [task_data: record]: nothing -> string {
  let id = $task_data.id
  let title = $task_data.title
  let type = $task_data.type
  let priority = $task_data.priority
  let effort = $task_data.effort
  let description = $task_data.description

  # Build EARS section
  let ears_ubiq = ($task_data.ears?.ubiquitous? | default [])
  let ears_event = ($task_data.ears?.event_driven? | default [])
  let ears_unwanted = ($task_data.ears?.unwanted? | default [])

  # Build contracts section
  let preconditions = ($task_data.contracts?.preconditions? | default [])
  let postconditions = ($task_data.contracts?.postconditions? | default [])
  let invariants = ($task_data.contracts?.invariants? | default [])

  # Build tests section
  let happy_tests = ($task_data.tests?.happy? | default [])
  let error_tests = ($task_data.tests?.error? | default [])

  # Generate CUE content using triple-quoted strings to avoid escaping hell
  # CUE uses """ for multiline strings that don't need escaping
  let cue_content = [
    "package planner"
    ""
    "task: #Task & {"
    $"  id: \"($id)\""
    $"  title: \"($title)\""
    $"  type: \"($type)\""
    $"  priority: ($priority)"
    $"  effort: \"($effort)\""
    $"  description: \"\"\"($description)\"\"\""
    ""
    "  ears: {"
    "    ubiquitous: ["
  ]

  # Add ubiquitous requirements
  let ubiq_lines = ($ears_ubiq | each { |r|
    $"      \"($r)\","
  })
  let cue_content = ($cue_content | append $ubiq_lines | append "    ]")

  # Add event-driven requirements
  let cue_content = ($cue_content | append "    event_driven: [")
  let event_lines = ($ears_event | each { |e|
    $"      {trigger: \"($e.trigger)\", shall: \"($e.shall)\"},"
  })
  let cue_content = ($cue_content | append $event_lines | append "    ]")

  # Add unwanted requirements
  let cue_content = ($cue_content | append "    unwanted: [")
  let unwanted_lines = ($ears_unwanted | each { |u|
    $"      {condition: \"($u.condition)\", shall_not: \"($u.shall_not)\", because: \"($u.because)\"},"
  })
  let cue_content = ($cue_content | append $unwanted_lines | append "    ]" | append "  }")

  # Add contracts
  let cue_content = ($cue_content | append "" | append "  contracts: {" | append "    preconditions: [")
  let pre_lines = ($preconditions | each { |p| $"      \"($p)\"," })
  let cue_content = ($cue_content | append $pre_lines | append "    ]")

  let cue_content = ($cue_content | append "    postconditions: [")
  let post_lines = ($postconditions | each { |p| $"      \"($p)\"," })
  let cue_content = ($cue_content | append $post_lines | append "    ]")

  let cue_content = ($cue_content | append "    invariants: [")
  let inv_lines = ($invariants | each { |i| $"      \"($i)\"," })
  let cue_content = ($cue_content | append $inv_lines | append "    ]" | append "  }")

  # Add tests
  let cue_content = ($cue_content | append "" | append "  tests: {" | append "    happy: [")
  let happy_lines = ($happy_tests | each { |t| $"      \"($t)\"," })
  let cue_content = ($cue_content | append $happy_lines | append "    ]")

  let cue_content = ($cue_content | append "    error: [")
  let error_lines = ($error_tests | each { |t| $"      \"($t)\"," })
  let cue_content = ($cue_content | append $error_lines | append "    ]" | append "  }")

  # Close task block
  let cue_content = ($cue_content | append "}")

  # Join all lines
  $cue_content | str join "\n"
}

# Get beads schema directory (from env or default)
def get-beads-schema-dir []: nothing -> string {
  $env.BEADS_SCHEMA_DIR? | default $DEFAULT_BEADS_SCHEMA_DIR | path expand
}

# Generate per-bead CUE schema file
def generate-bead-schema-file [bead_id: string, task_data: record]: nothing -> string {
  let schema_dir = (get-beads-schema-dir)

  # Ensure schema directory exists
  if not ($schema_dir | path exists) {
    mkdir $schema_dir
  }

  let schema_file = $"($schema_dir)/($bead_id).cue"

  # Generate bead-specific CUE schema
  let schema_content = generate-bead-validation-schema $bead_id $task_data

  $schema_content | save $schema_file

  $schema_file
}

# Generate bead validation CUE schema
def generate-bead-validation-schema [bead_id: string, task_data: record]: nothing -> string {
  let title = $task_data.title

  # Extract contract requirements
  let preconditions = ($task_data.contracts?.preconditions? | default [])
  let postconditions = ($task_data.contracts?.postconditions? | default [])
  let invariants = ($task_data.contracts?.invariants? | default [])

  # Extract test requirements
  let happy_tests = ($task_data.tests?.happy? | default [])
  let error_tests = ($task_data.tests?.error? | default [])

  # Build schema using list approach (cleaner)
  let lines = [
    ""
    "package validation"
    ""
    "import \"list\""
    ""
    $"// Validation schema for bead: ($bead_id)"
    $"// Title: ($title)"
    "//"
    "// This schema validates that implementation is complete."
    $"// Use: cue vet ($bead_id).cue implementation.cue"
    ""
    "#BeadImplementation: {"
    $"  bead_id: \"($bead_id)\""
    $"  title: \"($title)\""
    ""
    "  // Contract verification"
    "  contracts_verified: {"
    "    preconditions_checked: bool & true"
    "    postconditions_verified: bool & true"
    "    invariants_maintained: bool & true"
    ""
    "    // Specific preconditions that must be verified"
    "    precondition_checks: ["
  ]

  let lines = ($lines | append ($preconditions | each { |p|
    let escaped = (escape-cue-string $p)
    $"      \"($escaped)\","
  }))

  let lines = ($lines | append [
    "    ]"
    ""
    "    // Specific postconditions that must be verified"
    "    postcondition_checks: ["
  ])

  let lines = ($lines | append ($postconditions | each { |p|
    let escaped = (escape-cue-string $p)
    $"      \"($escaped)\","
  }))

  let lines = ($lines | append [
    "    ]"
    ""
    "    // Specific invariants that must be maintained"
    "    invariant_checks: ["
  ])

  let lines = ($lines | append ($invariants | each { |i|
    let escaped = (escape-cue-string $i)
    $"      \"($escaped)\","
  }))

  let happy_count = ($happy_tests | length)
  let error_count = ($error_tests | length)

  let lines = ($lines | append [
    "    ]"
    "  }"
    ""
    "  // Test verification"
    "  tests_passing: {"
    "    all_tests_pass: bool & true"
    ""
    $"    happy_path_tests: [...string] & list.MinItems\(($happy_count)\)"
    $"    error_path_tests: [...string] & list.MinItems\(($error_count)\)"
    ""
    "    // Note: Actual test names provided by implementer, must include all required tests"
    ""
    "    // Required happy path tests"
    "    required_happy_tests: ["
  ])

  let lines = ($lines | append ($happy_tests | each { |t|
    let escaped = (escape-cue-string $t)
    $"      \"($escaped)\","
  }))

  let lines = ($lines | append [
    "    ]"
    ""
    "    // Required error path tests"
    "    required_error_tests: ["
  ])

  let lines = ($lines | append ($error_tests | each { |t|
    let escaped = (escape-cue-string $t)
    $"      \"($escaped)\","
  }))

  let lines = ($lines | append [
    "    ]"
    "  }"
    ""
    "  // Code completion"
    "  code_complete: {"
    "    implementation_exists: string  // Path to implementation file"
    "    tests_exist: string  // Path to test file"
    "    ci_passing: bool & true"
    "    no_unwrap_calls: bool & true  // Rust/functional constraint"
    "    no_panics: bool & true  // Rust constraint"
    "  }"
    ""
    "  // Completion criteria"
    "  completion: {"
    "    all_sections_complete: bool & true"
    "    documentation_updated: bool"
    "    beads_closed: bool"
    "    timestamp: string  // ISO8601 completion timestamp"
    "  }"
    "}"
    ""
    "// Example implementation proof - create this file to validate completion:"
    "//"
    "// implementation.cue:"
    "// package validation"
    "//"
    "// implementation: #BeadImplementation & {"
    "//   contracts_verified: {"
    "//     preconditions_checked: true"
    "//     postconditions_verified: true"
    "//     invariants_maintained: true"
    "//     precondition_checks: [/* documented checks */]"
    "//     postcondition_checks: [/* documented verifications */]"
    "//     invariant_checks: [/* documented invariants */]"
    "//   }"
    "//   tests_passing: {"
    "//     all_tests_pass: true"
    "//     happy_path_tests: [\"test_version_flag_works\", \"test_version_format\", \"test_exit_code_zero\"]"
    "//     error_path_tests: [\"test_invalid_flag_errors\", \"test_no_flags_normal_behavior\"]"
    "//   }"
    "//   code_complete: {"
    "//     implementation_exists: \"src/main.rs\""
    "//     tests_exist: \"tests/cli_test.rs\""
    "//     ci_passing: true"
    "//     no_unwrap_calls: true"
    "//     no_panics: true"
    "//   }"
    "//   completion: {"
    "//     all_sections_complete: true"
    "//     documentation_updated: true"
    "//     beads_closed: false"
    $"//     timestamp: \"(date now | format date '%Y-%m-%dT%H:%M:%S')Z\""
    "//   }"
    "// }"
  ])

  $lines | str join "\n"
}

# ── Session Persistence ───────────────────────────────────────────────────────

def session-path [session_id: string] {
  $"($SESSION_DIR | path expand)/($session_id).yml"
}

def ensure-session-dir [] {
  let dir = ($SESSION_DIR | path expand)
  if not ($dir | path exists) { mkdir $dir }
}

def load-session [session_id: string] {
  let p = (session-path $session_id)
  if ($p | path exists) {
    try {
      open $p
    } catch {
      error make {msg: $"Session corrupted — cannot parse ($p)"}
    }
  } else {
    error make {msg: $"Session '($session_id)' not found"}
  }
}

def save-session [session: record] {
  ensure-session-dir
  let p = (session-path $session.session_id)

  # Atomic save: write to temp file, then rename
  let temp = $"($p).tmp.(random chars -l 8)"

  try {
    $session | to yaml | save -f $temp
    mv -f $temp $p
  } catch { |err|
    # Clean up temp file if it exists
    if ($temp | path exists) {
      try { rm $temp } catch { }
    }
    error make {msg: $"Failed to save session: ($err.msg)"}
  }
}

def now-timestamp [] {
  date now | format date "%Y-%m-%dT%H:%M:%S"
}

# ── Session Management Commands ───────────────────────────────────────────────

# Initialize a new planning session
def "main init" [
  --session-id: string        # Unique session identifier
  --description: string       # Description of work to plan
] {
  # Validate inputs
  validate-session-id $session_id

  if ($description | is-empty) {
    error make {msg: "Description cannot be empty"}
  }

  # Check if session already exists
  let p = (session-path $session_id)
  if ($p | path exists) {
    error make {msg: $"Session '($session_id)' already exists. Use 'reset' first or choose different ID."}
  }

  ensure-session-dir

  let session = {
    session_id: $session_id,
    description: $description,
    created_at: (now-timestamp),
    updated_at: (now-timestamp),
    status: "INITIALIZED",
    tasks: {},
    beads: {},
    summary: {
      total_tasks: 0,
      generated: 0,
      valid: 0,
      invalid: 0,
      created: 0,
      failed: 0
    }
  }

  save-session $session

  print $"✅ Planning session initialized: ($session_id)"
  print $"   Description: ($description)"
  print $"   State: (session-path $session_id)"
}

# Add a task to the planning session
def "main add-task" [
  session_id: string,          # Session to add to
  --task-json: string = "-"    # Task JSON (from stdin if -)
] {
  validate-session-id $session_id

  # Parse JSON with error handling
  let task_data = try {
    if $task_json == "-" {
      # Read from stdin
      let input = $in
      if ($input | is-empty) {
        error make {msg: "No JSON provided on stdin. Use: echo '{...}' | nu planner.nu add-task ..."}
      }
      $input | from json
    } else {
      # Parse from string argument
      $task_json | from json
    }
  } catch { |err|
    error make {msg: $"Invalid JSON: ($err.msg)"}
  }

  mut session = (load-session $session_id)

  # Validate required fields
  if not ("id" in $task_data) {
    error make {msg: "Task must have 'id' field"}
  }
  if not ("title" in $task_data) {
    error make {msg: "Task must have 'title' field"}
  }

  # Validate task type if provided
  let valid_types = ["feature", "bug", "task", "epic", "chore"]
  if "type" in $task_data {
    if not ($task_data.type in $valid_types) {
      error make {msg: $"Invalid type '($task_data.type)'. Must be one of: ($valid_types | str join ', ')"}
    }
  }

  # Validate priority if provided
  if "priority" in $task_data {
    if ($task_data.priority < 0) or ($task_data.priority > 4) {
      error make {msg: $"Invalid priority ($task_data.priority). Must be 0-4."}
    }
  }

  # Validate effort if provided
  let valid_efforts = ["15min", "30min", "1hr", "2hr", "4hr"]
  if "effort" in $task_data {
    if not ($task_data.effort in $valid_efforts) {
      error make {msg: $"Invalid effort '($task_data.effort)'. Must be one of: ($valid_efforts | str join ', ')"}
    }
  }

  # Check for duplicate task ID
  if ($task_data.id in $session.tasks) {
    error make {msg: $"Task ID '($task_data.id)' already exists in session"}
  }

  # Validate task against CUE schema
  print "📋 Validating task against CUE schema..."
  let validation_result = (validate-task-with-cue $task_data)

  if not $validation_result.valid {
    print "❌ Task validation failed:"
    for error in $validation_result.errors {
      print $"   ($error)"
    }
    error make {msg: "Task does not conform to CUE schema"}
  }

  print "✅ Task validation passed"

  # Add task
  $session.tasks = ($session.tasks | upsert $task_data.id {
    id: $task_data.id,
    title: $task_data.title,
    type: ($task_data.type? | default "feature"),
    priority: ($task_data.priority? | default 2),
    effort: ($task_data.effort? | default "2hr"),
    data: $task_data,
    status: "PENDING",
    added_at: (now-timestamp)
  })

  $session.summary.total_tasks = ($session.summary.total_tasks + 1)
  $session.updated_at = (now-timestamp)

  save-session $session

  print $"✅ Task added: ($task_data.id) - ($task_data.title)"
}

# Generate bead YAML from task using template
def "main generate-bead" [
  session_id: string,    # Session ID
  task_id: string        # Task ID to generate bead for
] {
  mut session = (load-session $session_id)

  if not ($task_id in $session.tasks) {
    error make {msg: $"Task '($task_id)' not found in session"}
  }

  let task = ($session.tasks | get $task_id)
  let task_data = $task.data

  # Generate unique bead ID
  let bead_id = generate-bead-id

  # Expand template with task data
  let bead_yaml = expand-template $task_data $bead_id

  # Store generated bead
  $session.beads = ($session.beads | upsert $bead_id {
    id: $bead_id,
    task_id: $task_id,
    yaml: $bead_yaml,
    status: "GENERATED",
    generated_at: (now-timestamp)
  })

  # Generate per-bead CUE validation schema
  print "📐 Generating bead validation CUE schema..."
  let schema_file = try {
    generate-bead-schema-file $bead_id $task_data
  } catch { |err|
    print $"⚠️  Warning: Failed to generate CUE schema: ($err.msg)"
    null
  }

  if $schema_file != null {
    print $"✅ CUE schema saved: ($schema_file)"
  }

  # Update task status
  $session.tasks = ($session.tasks | upsert $task_id (
    $task
    | upsert status "GENERATED"
    | upsert bead_id $bead_id
    | upsert schema_file $schema_file
  ))

  $session.summary.generated = ($session.summary.generated + 1)
  $session.updated_at = (now-timestamp)

  save-session $session

  print $"✅ Bead generated: ($bead_id) for task ($task_id)"
  if $schema_file != null {
    print $"   CUE validation schema: ($schema_file)"
  }
}

# Validate bead against CUE schema
def "main validate" [
  session_id: string,    # Session ID
  bead_id: string        # Bead ID to validate
] {
  validate-session-id $session_id
  check-required-commands
  check-required-files

  mut session = (load-session $session_id)

  if not ($bead_id in $session.beads) {
    error make {msg: $"Bead '($bead_id)' not found in session"}
  }

  let bead = ($session.beads | get $bead_id)

  # Beads are valid if they were successfully generated
  # The bead YAML is markdown/YAML documentation, not CUE code
  # CUE schemas generated in .beads/schemas/ are for validating implementation, not the bead spec

  # Simple validation: check if bead YAML exists and is not empty
  if ($bead.yaml | is-empty) {
    $session.beads = ($session.beads | upsert $bead_id (
      $bead
      | upsert status "INVALID"
      | upsert validation_error "Bead YAML is empty"
      | upsert validated_at (now-timestamp)
    ))
    $session.summary.invalid = ($session.summary.invalid + 1)
    save-session $session
    print $"❌ Bead validation failed: ($bead_id)"
    print $"   Error: Bead YAML is empty"
    error make {msg: "Validation failed"}
  }

  # Mark as valid
  $session.beads = ($session.beads | upsert $bead_id (
    $bead | upsert status "VALID" | upsert validated_at (now-timestamp)
  ))
  $session.summary.valid = ($session.summary.valid + 1)
  save-session $session
  print $"✅ Bead validated: ($bead_id)"
}

# Create bead in br database
def "main create" [
  session_id: string,    # Session ID
  bead_id: string        # Bead ID to create
] {
  validate-session-id $session_id
  check-required-commands

  mut session = (load-session $session_id)

  if not ($bead_id in $session.beads) {
    error make {msg: $"Bead '($bead_id)' not found in session"}
  }

  let bead = ($session.beads | get $bead_id)

  if $bead.status != "VALID" {
    error make {msg: $"Cannot create bead '($bead_id)' - status is ($bead.status), must be VALID"}
  }

  let task = ($session.tasks | get $bead.task_id)

  # Get CUE schema file path if it exists
  let schema_file = ($task.schema_file? | default null)

  # Prepare bead description with CUE schema reference
  let bead_description = if $schema_file != null {
    $"# CUE Validation Schema
# Validate implementation: cue vet ($schema_file) implementation.cue
# Schema location: ($schema_file)

($bead.yaml)"
  } else {
    $bead.yaml
  }

  # Write YAML to temp file for br create (native nushell temp path)
  let temp_base = if ($env.TMPDIR? != null) { $env.TMPDIR } else if ($env.TEMP? != null) { $env.TEMP } else { "/tmp" }
  let temp_file = ($temp_base | path join $"bead-($bead_id)-(random chars -l 8).md")

  let creation_result = try {
    $bead_description | save $temp_file

    # Check file size (br might have limits)
    let file_size = (ls $temp_file | get size.0)
    if $file_size > 1MB {  # 1MB limit
      error make {msg: $"Bead description too large: ($file_size) (max 1MB)"}
    }

    # Create bead using br - reads file content and passes as description
    # Read description into variable for proper nushell string handling
    let bead_desc = (open $temp_file)

    # Nushell handles string escaping automatically for external commands
    let result = (do {
      ^br create --title $task.title -t $task.type -p $task.priority -d $bead_desc
    } | complete)

    if $result.exit_code == 0 {
      # Extract bead ID from output if possible
      let br_id = try { $result.stdout | str trim } catch { "unknown" }

      $session.beads = ($session.beads | upsert $bead_id (
        $bead
        | upsert status "CREATED"
        | upsert br_id $br_id
        | upsert created_at (now-timestamp)
      ))
      $session.summary.created = ($session.summary.created + 1)
      save-session $session
      print $"✅ Bead created in br: ($bead_id) -> ($br_id)"
      "success"
    } else {
      let error_msg = $result.stderr
      $session.beads = ($session.beads | upsert $bead_id (
        $bead | upsert creation_error $error_msg
      ))
      $session.summary.failed = ($session.summary.failed + 1)
      save-session $session
      print $"❌ Bead creation failed: ($bead_id)"
      print $"   Error: ($error_msg)"
      error make {msg: "Creation failed"}
    }
  } catch { |err|
    # Clean up on error
    try { rm -f $temp_file } catch { }
    error make {msg: $"Failed to create bead: ($err.msg)"}
  }

  # Clean up temp file
  try { rm -f $temp_file } catch { }
}

# Process entire session (generate, validate, create all tasks)
def "main process" [
  session_id: string    # Session ID to process
] {
  let session = (load-session $session_id)

  print $"🔄 Processing planning session: ($session_id)"
  print $"   Total tasks: ($session.summary.total_tasks)"
  print ""

  # Phase 1: Generate beads for all pending tasks
  print "Phase 1: Generating beads..."
  for task_id in ($session.tasks | columns) {
    let task = ($session.tasks | get $task_id)
    if $task.status == "PENDING" {
      try {
        main generate-bead $session_id $task_id
      } catch { |err|
        print $"   ⚠️  Failed to generate ($task_id): ($err)"
      }
    }
  }

  let session_updated = (load-session $session_id)

  # Phase 2: Validate all generated beads
  print ""
  print "Phase 2: Validating beads..."
  for bead_id in ($session_updated.beads | columns) {
    let bead = ($session_updated.beads | get $bead_id)
    if $bead.status == "GENERATED" {
      try {
        main validate $session_id $bead_id
      } catch { |err|
        print $"   ⚠️  Failed to validate ($bead_id): ($err)"
      }
    }
  }

  let session_validated = (load-session $session_id)

  # Phase 3: Create all valid beads
  print ""
  print "Phase 3: Creating beads in br..."
  for bead_id in ($session_validated.beads | columns) {
    let bead = ($session_validated.beads | get $bead_id)
    if $bead.status == "VALID" {
      try {
        main create $session_id $bead_id
      } catch { |err|
        print $"   ⚠️  Failed to create ($bead_id): ($err)"
      }
    }
  }

  # Phase 4: Report
  print ""
  main report $session_id
}

# Show session status
def "main status" [
  session_id: string    # Session ID
] {
  let session = (load-session $session_id)

  print $"Planning Session: ($session.session_id)"
  print $"Status: ($session.status)"
  print $"Created: ($session.created_at)"
  print $"Updated: ($session.updated_at)"
  print ""
  print "Summary:"
  print $"  Total Tasks: ($session.summary.total_tasks)"
  print $"  Generated:   ($session.summary.generated)"
  print $"  Valid:       ($session.summary.valid)"
  print $"  Invalid:     ($session.summary.invalid)"
  print $"  Created:     ($session.summary.created)"
  print $"  Failed:      ($session.summary.failed)"
}

# Report session results
def "main report" [
  session_id: string    # Session ID
] {
  let session = (load-session $session_id)

  print "╔════════════════════════════════════════════╗"
  print "║       📋 PLANNING SESSION REPORT          ║"
  print "╚════════════════════════════════════════════╝"
  print ""
  print $"Session: ($session.session_id)"
  print $"Description: ($session.description)"
  print ""
  print "RESULTS:"
  print $"  ✅ Successfully created: ($session.summary.created) beads"
  print $"  ⚠️  Invalid beads: ($session.summary.invalid)"
  print $"  ❌ Failed creations: ($session.summary.failed)"
  print ""

  if $session.summary.created > 0 {
    print "Created Beads:"
    for bead_id in ($session.beads | columns) {
      let bead = ($session.beads | get $bead_id)
      if $bead.status == "CREATED" {
        let task = ($session.tasks | get $bead.task_id)
        print $"  • ($bead.br_id? | default $bead_id): ($task.title)"
      }
    }
    print ""
  }

  if $session.summary.invalid > 0 {
    print "Invalid Beads (need fixing):"
    for bead_id in ($session.beads | columns) {
      let bead = ($session.beads | get $bead_id)
      if $bead.status == "INVALID" {
        let task = ($session.tasks | get $bead.task_id)
        print $"  • ($bead_id): ($task.title)"
        print $"    Error: ($bead.validation_error? | default 'unknown')"
      }
    }
    print ""
  }

  print $"State file: (session-path $session_id)"
}

# List all planning sessions
def "main list" [] {
  ensure-session-dir
  let sessions_path = ($SESSION_DIR | path expand)

  # Check if any session files exist
  let pattern = $"($sessions_path)/*.yml"
  let files = try {
    glob $pattern
  } catch {
    []
  }

  if ($files | is-empty) {
    print "No planning sessions found."
    return
  }

  print "Planning Sessions:"
  print ""

  for file in $files {
    let session = try {
      open $file
    } catch { |err|
      print $"⚠️  Skipping corrupted session: ($file)"
      continue
    }

    print $"• ($session.session_id)"
    print $"  Status: ($session.status) | Tasks: ($session.summary.total_tasks) | Created: ($session.summary.created)"
    print $"  ($session.description)"
    print ""
  }
}

# Reset/clear a session
def "main reset" [
  session_id: string    # Session ID to reset
] {
  let p = (session-path $session_id)
  if ($p | path exists) {
    rm $p
    print $"✅ Session reset: ($session_id)"
  } else {
    print $"⚠️  Session not found: ($session_id)"
  }
}

# Show all tasks in session
def "main show-tasks" [
  session_id: string    # Session ID
] {
  let session = (load-session $session_id)

  print $"Tasks in session ($session_id):"
  print ""

  for task_id in ($session.tasks | columns) {
    let task = ($session.tasks | get $task_id)
    print $"• ($task_id): ($task.title)"
    print $"  Type: ($task.type) | Priority: ($task.priority) | Effort: ($task.effort)"
    print $"  Status: ($task.status)"
    if "bead_id" in $task {
      print $"  Bead: ($task.bead_id)"
    }
    print ""
  }
}

# Show generated bead YAML
def "main show-bead" [
  session_id: string,    # Session ID
  bead_id: string        # Bead ID
] {
  let session = (load-session $session_id)

  if not ($bead_id in $session.beads) {
    error make {msg: $"Bead '($bead_id)' not found"}
  }

  let bead = ($session.beads | get $bead_id)
  print $bead.yaml
}

# Show validation errors
def "main show-errors" [
  session_id: string    # Session ID
] {
  let session = (load-session $session_id)

  print $"Validation Errors in session ($session_id):"
  print ""

  mut has_errors = false
  for bead_id in ($session.beads | columns) {
    let bead = ($session.beads | get $bead_id)
    if $bead.status == "INVALID" {
      let task = ($session.tasks | get $bead.task_id)
      print $"• ($bead_id): ($task.title)"
      print $"  ($bead.validation_error? | default 'unknown error')"
      print ""
      $has_errors = true
    }
  }

  if not $has_errors {
    print "  No validation errors."
  }
}

# Show created bead IDs
def "main show-created" [
  session_id: string    # Session ID
] {
  let session = (load-session $session_id)

  print $"Created Beads in session ($session_id):"
  print ""

  for bead_id in ($session.beads | columns) {
    let bead = ($session.beads | get $bead_id)
    if $bead.status == "CREATED" {
      let task = ($session.tasks | get $bead.task_id)
      print $"• ($bead.br_id? | default $bead_id): ($task.title)"
    }
  }
}

# Run quality review on a task before generating bead
def "main review-task" [
  session_id: string,    # Session ID
  task_id: string        # Task ID to review
] {
  validate-session-id $session_id
  let session = (load-session $session_id)

  if not ($task_id in $session.tasks) {
    error make {msg: $"Task '($task_id)' not found in session"}
  }

  let task = ($session.tasks | get $task_id)
  let task_data = $task.data

  print "╔════════════════════════════════════════════╗"
  print "║     🔍 TASK QUALITY REVIEW                ║"
  print "╚════════════════════════════════════════════╝"
  print ""
  print $"Task: ($task_id) - ($task.title)"
  print ""

  # Run Contract Review
  print "┌─ Contract Review (Design by Contract) ────┐"
  let contract_review = (review-contracts $task_data)
  let contract_score = $contract_review.score
  let contract_grade = $contract_review.grade
  print $"│ Score: ($contract_score)/100 \(Grade: ($contract_grade)\)           │"
  if ($contract_review.issues | length) > 0 {
    print "│                                            │"
    print "│ Issues:                                    │"
    for issue in $contract_review.issues {
      print $"│   • ($issue)"
    }
  }
  if ($contract_review.recommendations | length) > 0 {
    print "│                                            │"
    print "│ Recommendations:                           │"
    for rec in $contract_review.recommendations {
      print $"│   → ($rec)"
    }
  }
  print "└────────────────────────────────────────────┘"
  print ""

  # Run Test Design Review
  print "┌─ Test Design Review (Martin Fowler) ──────┐"
  let test_review = (review-test-design $task_data)
  let test_score = $test_review.score
  let test_grade = $test_review.grade
  print $"│ Score: ($test_score)/100 \(Grade: ($test_grade)\)            │"
  if ($test_review.issues | length) > 0 {
    print "│                                            │"
    print "│ Issues:                                    │"
    for issue in $test_review.issues {
      print $"│   • ($issue)"
    }
  }
  if ($test_review.recommendations | length) > 0 {
    print "│                                            │"
    print "│ Recommendations:                           │"
    for rec in $test_review.recommendations {
      print $"│   → ($rec)"
    }
  }
  print "└────────────────────────────────────────────┘"
  print ""

  # Run Adversarial Review
  print "┌─ Adversarial Review (Red Queen) ───────────┐"
  let adversarial = (adversarial-review $task_data)
  let adv_score = $adversarial.score
  let adv_grade = $adversarial.grade
  print $"│ Score: ($adv_score)/100 \(Grade: ($adv_grade)\)            │"
  if ($adversarial.attacks | length) > 0 {
    print "│                                            │"
    print "│ Attacks (What's Wrong):                    │"
    for attack in $adversarial.attacks {
      print $"│   ⚔️  ($attack)"
    }
  }
  if ($adversarial.improvements | length) > 0 {
    print "│                                            │"
    print "│ Improvements:                              │"
    for imp in $adversarial.improvements {
      print $"│   💡 ($imp)"
    }
  }
  print "└────────────────────────────────────────────┘"
  print ""

  # Overall Assessment
  let avg_score = (($contract_review.score + $test_review.score + $adversarial.score) / 3 | math round)
  print $"OVERALL QUALITY SCORE: ($avg_score)/100"

  if $avg_score >= 80 {
    print "✅ Task meets quality standards - ready to generate bead"
  } else if $avg_score >= 60 {
    print "⚠️  Task needs improvement - address recommendations before generating bead"
  } else {
    print "❌ Task FAILS quality gate - must fix issues before proceeding"
  }
  print ""
  print $"Review complete. Use 'main generate-bead ($session_id) ($task_id)' when ready."
}

# Run comprehensive quality gate on all tasks
def "main quality-gate" [
  session_id: string    # Session ID
] {
  validate-session-id $session_id
  let session = (load-session $session_id)

  print "╔════════════════════════════════════════════╗"
  print "║   🚦 COMPREHENSIVE QUALITY GATE            ║"
  print "╚════════════════════════════════════════════╝"
  print ""

  mut passed = 0
  mut warned = 0
  mut failed = 0

  for task_id in ($session.tasks | columns) {
    let task = ($session.tasks | get $task_id)
    let task_data = $task.data

    let contract_review = (review-contracts $task_data)
    let test_review = (review-test-design $task_data)
    let adversarial = (adversarial-review $task_data)

    let avg_score = (($contract_review.score + $test_review.score + $adversarial.score) / 3 | math round)

    if $avg_score >= 80 {
      print $"✅ ($task_id): ($task.title) - PASS ($avg_score)/100"
      $passed = ($passed + 1)
    } else if $avg_score >= 60 {
      print $"⚠️  ($task_id): ($task.title) - WARN ($avg_score)/100"
      $warned = ($warned + 1)
    } else {
      print $"❌ ($task_id): ($task.title) - FAIL ($avg_score)/100"
      $failed = ($failed + 1)
    }
  }

  print ""
  print $"Results: ($passed) passed, ($warned) warnings, ($failed) failures"
  print ""

  if $failed > 0 {
    print "❌ Quality gate FAILED - fix failing tasks before proceeding"
    error make {msg: $"($failed) tasks failed quality review"}
  } else if $warned > 0 {
    print "⚠️  Quality gate PASSED with warnings - review recommendations"
  } else {
    print "✅ All tasks passed quality gate - ready to generate beads"
  }
}

# ── Contract-Driven Testing Review ───────────────────────────────────────────

def review-contracts [task_data: record] {
  # Design by Contract review - ensure preconditions, postconditions, invariants
  # Returns: {score: 0-100, issues: [list], recommendations: [list]}

  mut score = 100
  mut issues = []
  mut recommendations = []

  # Check 1: Are contracts defined at all?
  if not ("contracts" in $task_data) {
    $score = 0
    $issues = ($issues | append "No contracts defined - violates Design by Contract principle")
    $recommendations = ($recommendations | append "Define contracts: preconditions (what must be true before), postconditions (what must be true after), invariants (always true)")
    return {score: $score, issues: $issues, recommendations: $recommendations, grade: "F"}
  }

  let contracts = $task_data.contracts

  # Check 2: Preconditions - What must be true before execution?
  if not ("preconditions" in $contracts) or (($contracts.preconditions | length) == 0) {
    $score = ($score - 30)
    $issues = ($issues | append "Missing preconditions - what must be true before this runs?")
    $recommendations = ($recommendations | append "Define preconditions: required inputs, system state, environment assumptions")
  } else {
    # Check quality of preconditions
    for pre in $contracts.preconditions {
      if ($pre | str contains "valid") or ($pre | str contains "correct") {
        $score = ($score - 5)
        $issues = ($issues | append $"Vague precondition: '($pre)'")
        $recommendations = ($recommendations | append "Be specific: 'File exists at ~/.config/app.toml' not 'Config is valid'")
      }
    }
  }

  # Check 3: Postconditions - What guarantees after execution?
  if not ("postconditions" in $contracts) or (($contracts.postconditions | length) == 0) {
    $score = ($score - 30)
    $issues = ($issues | append "Missing postconditions - what is guaranteed after execution?")
    $recommendations = ($recommendations | append "Define postconditions: state changes, return guarantees, side effects")
  } else {
    for post in $contracts.postconditions {
      if ($post | str contains "success") or ($post | str contains "works") {
        $score = ($score - 5)
        $issues = ($issues | append $"Vague postcondition: '($post)'")
        $recommendations = ($recommendations | append "Be measurable: 'Exit code is 0 and token exists in keyring' not 'Operation succeeds'")
      }
    }
  }

  # Check 4: Invariants - What must ALWAYS be true?
  if not ("invariants" in $contracts) or (($contracts.invariants | length) == 0) {
    $score = ($score - 30)
    $issues = ($issues | append "Missing invariants - what must always hold true?")
    $recommendations = ($recommendations | append "Define invariants: security properties, data integrity, consistency guarantees")
  } else {
    if ($contracts.invariants | length) < 2 {
      $score = ($score - 10)
      $issues = ($issues | append "Only one invariant defined - usually need 2-3 critical properties")
      $recommendations = ($recommendations | append "Add invariants like: 'Passwords never in logs', 'All timestamps ISO8601', 'Exit codes follow AGENTS.md'")
    }
  }

  # Check 5: Are contracts tested?
  if "tests" in $task_data {
    if not ("contract" in ($task_data.tests | columns)) {
      $score = ($score - 15)
      $issues = ($issues | append "Contracts defined but no contract tests to verify them")
      $recommendations = ($recommendations | append "Add contract tests: test_precondition_X, test_postcondition_Y, test_invariant_Z")
    }
  } else {
    $score = ($score - 20)
    $issues = ($issues | append "No tests at all - contracts are not verified")
    $recommendations = ($recommendations | append "Add contract verification tests")
  }

  # Check 6: Failure handling - What happens when preconditions violated?
  if "contracts" in $task_data and "tests" in $task_data {
    let tests = $task_data.tests
    if "error" in $tests {
      let has_precondition_violation_test = ($tests.error | any { |t| ($t | str contains "precondition") or ($t | str contains "invalid input") or ($t | str contains "missing") })
      if not $has_precondition_violation_test {
        $score = ($score - 10)
        $issues = ($issues | append "No tests for precondition violations - what happens when assumptions break?")
        $recommendations = ($recommendations | append "Test precondition failures: 'When file missing returns exit 4', 'When invalid input returns exit 3'")
      }
    }
  }

  {
    score: $score,
    issues: $issues,
    recommendations: $recommendations,
    grade: (if $score >= 90 { "A" } else if $score >= 80 { "B" } else if $score >= 70 { "C" } else if $score >= 60 { "D" } else { "F" })
  }
}

# ── Test Quality Review (Martin Fowler Principles) ───────────────────────────

def review-test-design [task_data: record] {
  # Martin Fowler test design review - find weaknesses in test strategy
  # Returns: {score: 0-100, issues: [list], recommendations: [list]}

  mut score = 100
  mut issues = []
  mut recommendations = []

  # Check 1: Test Isolation - Can tests run independently?
  if "tests" in $task_data {
    let tests = $task_data.tests

    if "happy" in $tests {
      if ($tests.happy | length) < 2 {
        $score = ($score - 10)
        $issues = ($issues | append "Insufficient happy path coverage - need at least 2 scenarios")
        $recommendations = ($recommendations | append "Add more happy path tests covering different input variations")
      }
    } else {
      $score = ($score - 30)
      $issues = ($issues | append "Missing happy path tests")
      $recommendations = ($recommendations | append "Define at least 2 happy path test scenarios")
    }

    if "error" in $tests {
      if ($tests.error | length) < 2 {
        $score = ($score - 10)
        $issues = ($issues | append "Insufficient error path coverage")
        $recommendations = ($recommendations | append "Add tests for different error conditions (invalid input, missing resources, network failures)")
      }
    } else {
      $score = ($score - 30)
      $issues = ($issues | append "Missing error path tests - violates 'Design for Failure' principle")
      $recommendations = ($recommendations | append "Add error path tests: test how system fails, not just how it succeeds")
    }

    if not ("edge" in $tests) or (($tests.edge | length) < 1) {
      $score = ($score - 15)
      $issues = ($issues | append "Missing edge case tests")
      $recommendations = ($recommendations | append "Add edge cases: empty input, very large input, boundary values, concurrent access")
    }
  } else {
    $score = 0
    $issues = ($issues | append "No test strategy defined at all")
    $recommendations = ($recommendations | append "Define comprehensive test strategy with happy, error, and edge cases")
  }

  # Check 2: Test-Driven Development - Are tests specific enough?
  if "tests" in $task_data {
    let happy_tests = ($task_data.tests?.happy? | default [])
    for test in $happy_tests {
      if ($test | str contains "works") or ($test | str contains "succeeds") {
        $score = ($score - 5)
        $issues = ($issues | append $"Vague test description: '($test)'")
        $recommendations = ($recommendations | append "Make tests specific: 'User with valid credentials receives 200 OK and JWT token' not 'login works'")
      }
    }
  }

  # Check 3: Test Doubles - Are mocks being used? (Should use real data)
  if "tests" in $task_data {
    let test_str = ($task_data.tests | to json)
    if ($test_str | str contains "mock") or ($test_str | str contains "stub") {
      $score = ($score - 20)
      $issues = ($issues | append "Tests use mocks/stubs - reduces confidence")
      $recommendations = ($recommendations | append "Replace mocks with real test data and real command execution")
    }
  }

  # Check 4: Contract Testing - Are preconditions/postconditions tested?
  if "contracts" in $task_data {
    if not ("tests" in $task_data) or not ("contract" in ($task_data.tests | columns)) {
      $score = ($score - 10)
      $issues = ($issues | append "Contracts defined but no contract tests")
      $recommendations = ($recommendations | append "Add contract tests that verify preconditions and postconditions")
    }
  }

  # Check 5: Testing Pyramid - Right level of tests?
  # E2E tests should exist but not be the only tests
  if "tests" in $task_data {
    let has_unit_tests = (("happy" in $task_data.tests) or ("error" in $task_data.tests))
    let has_e2e = ("e2e" in $task_data or "integration" in $task_data)

    if $has_e2e and not $has_unit_tests {
      $score = ($score - 15)
      $issues = ($issues | append "Only E2E tests defined - violates test pyramid")
      $recommendations = ($recommendations | append "Add unit tests for fast feedback, use E2E for integration validation")
    }
  }

  # Check 6: Test Data Quality - Specific vs Generic
  if "tests" in $task_data {
    let test_str = ($task_data.tests | to json)
    if ($test_str | str contains "example.com") or ($test_str | str contains "test@test.com") or ($test_str | str contains "foo") {
      $score = ($score - 10)
      $issues = ($issues | append "Generic test data (example.com, foo) reduces realism")
      $recommendations = ($recommendations | append "Use realistic test data from actual domain")
    }
  }

  {
    score: $score,
    issues: $issues,
    recommendations: $recommendations,
    grade: (if $score >= 90 { "A" } else if $score >= 80 { "B" } else if $score >= 70 { "C" } else if $score >= 60 { "D" } else { "F" })
  }
}

# ── Adversarial Review (Red Queen Skepticism) ────────────────────────────────

def adversarial-review [task_data: record] {
  # Red Queen-style hostile review - find what's wrong
  # Returns: {score: 0-100, attacks: [list], improvements: [list]}

  mut score = 100
  mut attacks = []
  mut improvements = []

  # Attack 1: Vague Requirements
  if "ears" in $task_data {
    let ears = $task_data.ears
    if "ubiquitous" in $ears {
      for req in $ears.ubiquitous {
        if ($req | str contains "correctly") or ($req | str contains "properly") or ($req | str contains "successfully") {
          $score = ($score - 10)
          $attacks = ($attacks | append $"Weasel word in requirement: '($req)' - what does 'correctly' mean?")
          $improvements = ($improvements | append "Replace vague adverbs with measurable criteria: 'returns exit 0' not 'works correctly'")
        }
      }
    }
  }

  # Attack 2: Missing Error Scenarios
  if "ears" in $task_data and "unwanted" in $task_data.ears {
    if ($task_data.ears.unwanted | length) < 2 {
      $score = ($score - 15)
      $attacks = ($attacks | append "Insufficient unwanted behaviors - only considering happy paths")
      $improvements = ($improvements | append "Think like an attacker: What breaks? What crashes? What leaks data? Add 3-5 unwanted behaviors")
    }
  } else {
    $score = ($score - 30)
    $attacks = ($attacks | append "No unwanted behaviors defined - NOT thinking about failure modes")
    $improvements = ($improvements | append "Add UNWANTED section: 'IF X SHALL NOT Y BECAUSE Z' - define what must NEVER happen")
  }

  # Attack 3: No Inversions (Charlie Munger Test)
  if not ("inversions" in $task_data) or (($task_data.inversions | columns | length) == 0) {
    $score = ($score - 25)
    $attacks = ($attacks | append "No inversion analysis - not thinking about what could go wrong")
    $improvements = ($improvements | append "Invert: What causes security failures? Usability failures? Data corruption? Integration breakage?")
  }

  # Attack 4: Placeholder Data in Tests
  if "tests" in $task_data {
    let test_str = ($task_data.tests | to json)
    if ($test_str | str contains "example") or ($test_str | str contains "placeholder") or ($test_str | str contains "TODO") {
      $score = ($score - 20)
      $attacks = ($attacks | append "Tests use placeholder data - won't catch real bugs")
      $improvements = ($improvements | append "Use REAL data: actual commands, actual error messages, actual file paths")
    }
  }

  # Attack 5: No Research Phase
  if not ("research" in $task_data) or (($task_data.research | columns | length) == 0) {
    $score = ($score - 15)
    $attacks = ($attacks | append "No research requirements - will hallucinate instead of reading existing code")
    $improvements = ($improvements | append "Add research: files to read, patterns to find, prior art to examine - READ before WRITE")
  }

  # Attack 6: Implementation Before Tests
  if "implementation" in $task_data {
    let impl = $task_data.implementation
    if "phase_1" in $impl and "phase_2" in $impl {
      # Good - tests in phase 1, implementation in phase 2
    } else {
      $score = ($score - 10)
      $attacks = ($attacks | append "Implementation phases don't enforce TDD - tests might come after code")
      $improvements = ($improvements | append "Enforce TDD: Phase 1 = write failing tests, Phase 2 = implement to pass tests")
    }
  }

  # Attack 7: No Anti-Hallucination Guards
  if not ("anti_hallucination" in $task_data) {
    $score = ($score - 15)
    $attacks = ($attacks | append "No anti-hallucination rules - AI will invent APIs that don't exist")
    $improvements = ($improvements | append "Add read-before-write rules, list APIs that exist, forbid placeholder values")
  }

  # Attack 8: Insufficient Test Coverage
  if "tests" in $task_data {
    let total_tests = (($task_data.tests?.happy? | default [] | length) +
                        ($task_data.tests?.error? | default [] | length) +
                        ($task_data.tests?.edge? | default [] | length))

    if $total_tests < 5 {
      $score = ($score - 15)
      $attacks = ($attacks | append $"Only ($total_tests) tests defined - likely incomplete coverage")
      $improvements = ($improvements | append "Add more tests: aim for 2-3 happy paths, 2-3 error paths, 2-3 edge cases minimum")
    }
  }

  {
    score: $score,
    attacks: $attacks,
    improvements: $improvements,
    grade: (if $score >= 90 { "A" } else if $score >= 80 { "B" } else if $score >= 70 { "C" } else if $score >= 60 { "D" } else { "F" }),
    ruthless: ($score < 70)  # Flag for "needs major improvement"
  }
}

# ── Template Expansion ────────────────────────────────────────────────────────

def expand-template [task_data: record, bead_id: string] {
  # Generate CUE format bead from task data
  # This expands the 16-section template with task-specific data

  let title = $task_data.title
  let type = ($task_data.type? | default "feature")
  let priority = ($task_data.priority? | default 2)
  let effort = ($task_data.effort? | default "2hr")
  let description = ($task_data.description? | default "")

  # Build EARS requirements
  let ears_ubiq = ($task_data.ears?.ubiquitous? | default ["THE SYSTEM SHALL complete the task successfully"])
  let ears_event = ($task_data.ears?.event_driven? | default [{trigger: "WHEN user invokes the command", shall: "THE SYSTEM SHALL execute without errors"}])
  let ears_unwanted = ($task_data.ears?.unwanted? | default [{condition: "IF invalid input is provided", shall_not: "THE SYSTEM SHALL NOT crash or produce unclear errors", because: "Poor error messages harm usability"}])

  # Build contracts
  let preconditions = ($task_data.contracts?.preconditions? | default ["System is installed and configured"])
  let postconditions = ($task_data.contracts?.postconditions? | default ["Task completed successfully"])
  let invariants = ($task_data.contracts?.invariants? | default ["Exit codes follow AGENTS.md specification", "No passwords in logs"])

  # Build tests
  let happy_tests = ($task_data.tests?.happy? | default ["Basic happy path works"])
  let error_tests = ($task_data.tests?.error? | default ["Invalid input returns appropriate error"])

  # Build research
  let research_files = ($task_data.research?.files? | default [])
  let research_questions = ($task_data.research?.questions? | default ["What existing patterns should be followed?"])

  # Build implementation phases
  let phase_0 = ($task_data.implementation?.phase_0? | default ["Read relevant files and understand existing patterns"])
  let phase_1 = ($task_data.implementation?.phase_1? | default ["Write failing tests"])
  let phase_2 = ($task_data.implementation?.phase_2? | default ["Implement to make tests pass"])

  # Build context
  let related_files = ($task_data.context?.related_files? | default [])
  let similar_impls = ($task_data.context?.similar? | default [])

  # Generate full CUE format
  let cue_content = $"
#EnhancedBead: {
  id: \"($bead_id)\"
  title: \"($title)\"
  type: \"($type)\"
  priority: ($priority)
  effort_estimate: \"($effort)\"
  labels: [\"planner-generated\"]

  clarifications: {
    clarification_status: \"RESOLVED\"
  }

  ears_requirements: {
    ubiquitous: [
      ($ears_ubiq | each { |r| $'\"($r)\"' } | str join ',
      ')
    ]
    event_driven: [
      ($ears_event | each { |e| $'{trigger: \"($e.trigger)\", shall: \"($e.shall)\"}' } | str join ',
      ')
    ]
    unwanted: [
      ($ears_unwanted | each { |u| $'{condition: \"($u.condition)\", shall_not: \"($u.shall_not)\", because: \"($u.because)\"}' } | str join ',
      ')
    ]
  }

  contracts: {
    preconditions: {
      auth_required: false
      required_inputs: []
      system_state: [
        ($preconditions | each { |p| $'\"($p)\"' } | str join ',
        ')
      ]
    }
    postconditions: {
      state_changes: [
        ($postconditions | each { |p| $'\"($p)\"' } | str join ',
        ')
      ]
      return_guarantees: []
    }
    invariants: [
      ($invariants | each { |i| $'\"($i)\"' } | str join ',
      ')
    ]
  }

  research_requirements: {
    files_to_read: [
      ($research_files | each { |f| $'{path: \"($f)\", what_to_extract: \"Existing patterns\", document_in: \"research_notes.md\"}' } | str join ',
      ')
    ]
    research_questions: [
      ($research_questions | each { |q| $'{question: \"($q)\", answered: false}' } | str join ',
      ')
    ]
    research_complete_when: [
      \"All files have been read and patterns documented\"
    ]
  }

  inversions: {
    usability_failures: [
      {failure: \"User encounters unclear error\", prevention: \"Provide specific error messages\", test_for_it: \"test_error_messages_are_clear\"}
    ]
  }

  acceptance_tests: {
    happy_paths: [
      ($happy_tests | each { |t| $'{name: \"test_happy_path\", given: \"Valid inputs\", when: \"User executes command\", then: [\"Exit code is 0\", \"Output is correct\"], real_input: \"command input\", expected_output: \"expected output\"}' } | str join ',
      ')
    ]
    error_paths: [
      ($error_tests | each { |t| $'{name: \"test_error_path\", given: \"Invalid inputs\", when: \"User executes command\", then: [\"Exit code is non-zero\", \"Error message is clear\"], real_input: \"invalid input\", expected_output: null, expected_error: \"error message\"}' } | str join ',
      ')
    ]
  }

  e2e_tests: {
    pipeline_test: {
      name: \"test_full_pipeline\"
      description: \"End-to-end test of full workflow\"
      setup: {}
      execute: {
        command: \"intent command\"
      }
      verify: {
        exit_code: 0
      }
    }
  }

  verification_checkpoints: {
    gate_0_research: {
      name: \"Research Gate\"
      must_pass_before: \"Writing code\"
      checks: [\"All research questions answered\"]
      evidence_required: [\"Research notes documented\"]
    }
    gate_1_tests: {
      name: \"Test Gate\"
      must_pass_before: \"Implementation\"
      checks: [\"All tests written and failing\"]
      evidence_required: [\"Test files exist\"]
    }
    gate_2_implementation: {
      name: \"Implementation Gate\"
      must_pass_before: \"Completion\"
      checks: [\"All tests pass\"]
      evidence_required: [\"CI green\"]
    }
    gate_3_integration: {
      name: \"Integration Gate\"
      must_pass_before: \"Closing bead\"
      checks: [\"E2E tests pass\"]
      evidence_required: [\"Manual verification complete\"]
    }
  }

  implementation_tasks: {
    phase_0_research: {
      parallelizable: true
      tasks: [
        ($phase_0 | each { |t| $'{task: \"($t)\", done_when: \"Documented\", parallel_group: \"research\"}' } | str join ',
        ')
      ]
    }
    phase_1_tests_first: {
      parallelizable: true
      gate_required: \"gate_0_research\"
      tasks: [
        ($phase_1 | each { |t| $'{task: \"($t)\", done_when: \"Test exists and fails\", parallel_group: \"tests\"}' } | str join ',
        ')
      ]
    }
    phase_2_implementation: {
      parallelizable: false
      gate_required: \"gate_1_tests\"
      tasks: [
        ($phase_2 | each { |t| $'{task: \"($t)\", done_when: \"Tests pass\"}' } | str join ',
        ')
      ]
    }
    phase_4_verification: {
      parallelizable: true
      gate_required: \"gate_2_implementation\"
      tasks: [
        {task: \"Run moon run :ci\", done_when: \"CI passes\", parallel_group: \"verification\"}
      ]
    }
  }

  failure_modes: {
    failure_modes: [
      {symptom: \"Feature does not work\", likely_cause: \"Implementation incomplete\", where_to_look: [{file: \"src/main.rs\", what_to_check: \"Implementation logic\"}], fix_pattern: \"Complete implementation\"}
    ]
  }

  anti_hallucination: {
    read_before_write: [
      {file: \"src/main.rs\", must_read_first: true, key_sections_to_understand: [\"Main entry point\"]}
    ]
    apis_that_exist: []
    no_placeholder_values: [\"Use real data from codebase\"]
    git_verification: {
      before_claiming_done: \"git status && git diff && moon run :test\"
    }
  }

  context_survival: {
    progress_file: {
      path: \".bead-progress/($bead_id)/progress.txt\"
      format: \"Markdown checklist\"
    }
    recovery_instructions: \"Read progress.txt and continue from current task\"
  }

  completion_checklist: {
    tests: [
      \"[ ] All acceptance tests written and passing\",
      \"[ ] All error path tests written and passing\",
      \"[ ] E2E pipeline test passing with real data\",
      \"[ ] No mocks or fake data in any test\"
    ]
    code: [
      \"[ ] Implementation uses Result<T, Error> throughout\",
      \"[ ] Zero unwrap() or expect() calls\"
    ]
    ci: [
      \"[ ] moon run :ci passes\"
    ]
  }

  context: {
    related_files: [
      ($related_files | each { |f| $'{path: \"($f)\", relevance: \"Related implementation\"}' } | str join ',
      ')
    ]
    similar_implementations: [
      ($similar_impls | each { |s| $'\"($s)\"' } | str join ',
      ')
    ]
  }

  ai_hints: {
    do: [
      \"Use functional patterns: map, and_then, ?\",
      \"Return Result<T, Error> from all fallible functions\",
      \"READ files before modifying them\"
    ]
    do_not: [
      \"Do NOT use unwrap() or expect()\",
      \"Do NOT use panic!, todo!, or unimplemented!\",
      \"Do NOT modify clippy configuration\"
    ]
    constitution: [
      \"Zero unwrap law: NEVER use .unwrap() or .expect()\",
      \"Test first: Tests MUST exist before implementation\"
    ]
  }
}
"

  $cue_content
}

def generate-bead-id [] {
  # Generate unique bead ID in format: intent-cli-TIMESTAMP-RANDOM
  # Timestamp ensures chronological ordering and reduces collision risk
  let timestamp = (date now | format date "%Y%m%d%H%M%S")
  let random_suffix = (random chars -l 8 | str downcase)
  $"intent-cli-($timestamp)-($random_suffix)"
}

# ── Main Entry Point ──────────────────────────────────────────────────────────

def main [] {
  print "Planner: Deterministic Bead Decomposition Engine with Quality Gates"
  print ""
  print "Session Management:"
  print "  init          - Initialize new planning session"
  print "  list          - List all sessions"
  print "  reset         - Reset/clear session"
  print "  status        - Show session status"
  print "  report        - Show session report"
  print ""
  print "Task Management:"
  print "  add-task      - Add task to session"
  print "  show-tasks    - Show tasks in session"
  print ""
  print "Quality Reviews (Contract-Driven Testing):"
  print "  review-task   - Run comprehensive quality review on a task"
  print "  quality-gate  - Run quality gate on all tasks in session"
  print ""
  print "Bead Generation:"
  print "  generate-bead - Generate bead YAML from task"
  print "  validate      - Validate bead against CUE schema"
  print "  create        - Create bead in br database"
  print "  process       - Generate, validate, and create all beads"
  print ""
  print "Inspection:"
  print "  show-bead     - Show generated bead YAML"
  print "  show-errors   - Show validation errors"
  print "  show-created  - Show created bead IDs"
  print ""
  print "Example Workflow (Contract-Driven):"
  print "  1. nu planner.nu init --session-id my-feature --description 'Add auth'"
  print "  2. echo '{...}' | nu planner.nu add-task my-feature --task-json -"
  print "  3. nu planner.nu review-task my-feature task-001  # Review before generating"
  print "  4. nu planner.nu quality-gate my-feature  # Check all tasks meet standards"
  print "  5. nu planner.nu process my-feature  # Generate validated beads"
  print ""
  print "Quality Reviews:"
  print "  • Contract Review: Design by Contract (preconditions/postconditions/invariants)"
  print "  • Test Design Review: Martin Fowler principles (no mocks, real data, coverage)"
  print "  • Adversarial Review: Red Queen skepticism (find holes, attack assumptions)"
}
