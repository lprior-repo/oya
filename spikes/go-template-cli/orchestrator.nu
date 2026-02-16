#!/usr/bin/env nu

def oc-get [base: string user: string password: string path: string] {
  http get --user $user --password $password $"($base)($path)"
}

def oc-post [base: string user: string password: string path: string body: any] {
  let payload = ($body | to json -r)
  http post --user $user --password $password --content-type application/json $"($base)($path)" $payload
}

def create-session [base: string user: string password: string title: string] {
  oc-post $base $user $password "/session" { title: $title } | get id
}

def session-state [base: string user: string password: string session_id: string] {
  let status_map = (oc-get $base $user $password "/session/status")
  if ($status_map | columns | any {|c| $c == $session_id }) {
    $status_map | get $session_id | get type
  } else {
    "idle"
  }
}

def latest-message [base: string user: string password: string session_id: string] {
  let messages = (oc-get $base $user $password $"/session/($session_id)/message?limit=1")
  if ($messages | length) == 0 { null } else { $messages | get 0 }
}

def extract-text-part [message: record] {
  let texts = ($message.parts | where type == "text" | get text)
  if ($texts | length) == 0 {
    return null
  }
  $texts | last
}

def compact-artifact-for-context [artifact: record] {
  let result = (try { $artifact.result } catch { {} })
  let evidence_cmds = (
    try {
      $result.evidence.commands
      | first 3
      | each {|c| {
          cmd: (try { $c.cmd } catch { null }),
          exit_code: (try { $c.exit_code } catch { null })
        }
      }
    } catch {
      []
    }
  )

  {
    stage: (try { $artifact.stage } catch { null }),
    attempt: (try { $artifact.attempt } catch { null }),
    pass: (try { $result.pass } catch { null }),
    next: (try { $result.next } catch { null }),
    failure_category: (try { $result.failure_category } catch { null }),
    summary: (try { $result.summary } catch { null }),
    risks: (try { $result.risks | first 2 } catch { [] }),
    evidence_commands: $evidence_cmds
  }
}

def poll-until-complete [base: string user: string password: string session_id: string max_polls: int] {
  mut tick = 0

  loop {
    let state = (session-state $base $user $password $session_id)
    let pending_permissions = (oc-get $base $user $password "/permission" | length)
    let pending_questions = (oc-get $base $user $password "/question" | length)

    if $pending_permissions > 0 {
      return {
        ok: false,
        reason: "pending_permission",
        state: $state,
        pending_permissions: $pending_permissions,
        pending_questions: $pending_questions
      }
    }

    if $pending_questions > 0 {
      return {
        ok: false,
        reason: "pending_question",
        state: $state,
        pending_permissions: $pending_permissions,
        pending_questions: $pending_questions
      }
    }

    let latest = (latest-message $base $user $password $session_id)
    if $latest != null {
      let role = ($latest | get info.role)
      let completed = (try { $latest | get info.time.completed } catch { null })
      if $role == "assistant" and $completed != null {
        return {
          ok: true,
          reason: "completed",
          state: $state,
          completed: $completed
        }
      }
    }

    if $tick >= $max_polls {
      return {
        ok: false,
        reason: "timeout",
        state: $state,
        pending_permissions: $pending_permissions,
        pending_questions: $pending_questions
      }
    }

    $tick = $tick + 1
    sleep 1sec
  }
}

def stage-prompt [bead: string stage: record attempt: int artifacts: list<any>] {
  let artifacts_jsonl = (
    if ($artifacts | length) == 0 {
      ""
    } else {
      $artifacts | each {|a| $a | to json -r } | str join "\n"
    }
  )
  let contract_ctx = (
    if (($artifacts | where stage == "contract" | length) == 0) {
      "null"
    } else {
      $artifacts | where stage == "contract" | get 0 | get result | to json -r
    }
  )

  let evidence_section = (
    if (stage-requires-evidence $stage.name) {
      "- evidence: { commands: [ { cmd: text, exit_code: int, stdout: text, stderr: text } ] } (required)\n- Include at least one real command and output\n- Prefer concrete commands such as: go test ./..., moon run :quick, moon run :test\n- Never use placeholders like 'not run', 'todo', or 'n/a'\n- Keep stdout/stderr concise snippets (not full logs)"
    } else {
      "- evidence: optional"
    }
  )

  $"You are stage ($stage.name) in a governed AI coding pipeline.\nBead: ($bead)\nAttempt: ($attempt)\nStage instruction: ($stage.instruction)\n\nYou MUST reference the contract context when deciding what to build/fix.\n\nReturn STRICT JSON ONLY with keys:\n- stage: text\n- pass: bool\n- summary: text\n- risks: array of text\n- next: one of [($stage.next_on_pass), ($stage.next_on_fail)]\n- failure_category: text and required when pass is false\n($evidence_section)\n\nHard rules:\n- No markdown\n- No prose outside JSON\n- Keep summary concise\n- Output must be valid JSON object, no trailing text\n\nContract context JSON:\n($contract_ctx)\n\nAll prior stage artifacts as JSONL, one full artifact per line:\n($artifacts_jsonl)"
}

def stage-request-body [model_provider: string model_id: string agent: string prompt: string] {
  {
    model: {
      providerID: $model_provider,
      modelID: $model_id
    },
    agent: $agent,
    parts: [
      {
        type: "text",
        text: $prompt
      }
    ]
  }
}

def stage-sync-exec [base: string user: string password: string session_id: string request: record] {
  try {
    oc-post $base $user $password $"/session/($session_id)/message" $request
  } catch {
    null
  }
}

def policy-next-stage [stage_name: string pass: bool] {
  if $pass {
    match $stage_name {
      "contract" => "implementation",
      "implementation" => "qa",
      "qa" => "red_queen",
      "red_queen" => "gpt_review",
      "gpt_review" => "ship_gate",
      _ => "ship_gate"
    }
  } else {
    match $stage_name {
      "contract" => "contract",
      "implementation" => "implementation",
      "qa" => "implementation",
      "red_queen" => "implementation",
      "gpt_review" => "implementation",
      _ => "implementation"
    }
  }
}

def stage-requires-evidence [stage_name: string] {
  ["qa", "red_queen", "gpt_review"] | any {|s| $s == $stage_name }
}

def evidence-valid [parsed: record] {
  let cmds = (try { $parsed.evidence.commands } catch { [] })
  if ($cmds | length) == 0 {
    return false
  }

  let shape_ok = ($cmds | all {|c| (try { $c.cmd } catch { null }) != null and (try { $c.exit_code } catch { null }) != null })
  if not $shape_ok {
    return false
  }

  let outputs_ok = (
    $cmds | all {|c|
      let out = (try { $c.stdout | into string | str trim } catch { "" })
      let err = (try { $c.stderr | into string | str trim } catch { "" })
      (($out | str length) > 0) or (($err | str length) > 0)
    }
  )
  if not $outputs_ok {
    return false
  }

  let bad_markers = ["not run", "placeholder", "todo", "n/a"]
  let marker_clean = (
    $cmds | all {|c|
      let out = (try { $c.stdout | into string } catch { "" })
      let err = (try { $c.stderr | into string } catch { "" })
      let combined = $"($out) ($err)" | str downcase
      not ($bad_markers | any {|m| $combined | str contains $m })
    }
  )
  if not $marker_clean {
    return false
  }

  let cmd_values = ($cmds | each {|c| (try { $c.cmd | into string | str trim } catch { "" }) })
  let non_empty_cmds = ($cmd_values | where {|x| ($x | str length) > 0 })
  let unique_cmds = ($non_empty_cmds | uniq)
  ($unique_cmds | length) > 0
}

def main [
  bead: string,
  --base: string = "http://127.0.0.1:4097",
  --user: string = "opencode",
  --password: string = "orchestrator-test",
  --model-provider: string = "openai",
  --model-id: string = "gpt-5.3-codex",
  --max-polls: int = 180,
  --max-transitions: int = 30
] {
  let effective_password = (
    if ($env | columns | any {|c| $c == "OPENCODE_SERVER_PASSWORD" }) {
      $env.OPENCODE_SERVER_PASSWORD
    } else {
      $password
    }
  )

  let stages = [
    {
      name: "contract",
      agent: "build",
      max_attempts: 1,
      next_on_pass: "implementation",
      next_on_fail: "contract",
      instruction: "Define scope, acceptance criteria, invariants, and explicit non-goals."
    },
    {
      name: "implementation",
      agent: "build",
      max_attempts: 6,
      next_on_pass: "qa",
      next_on_fail: "implementation",
      instruction: "Implement the smallest change set that satisfies the contract and latest failures."
    },
    {
      name: "qa",
      agent: "build",
      max_attempts: 4,
      next_on_pass: "red_queen",
      next_on_fail: "implementation",
      instruction: "Find correctness, reliability, and quality failures before release. Include concrete command evidence with exit codes."
    },
    {
      name: "red_queen",
      agent: "build",
      max_attempts: 3,
      next_on_pass: "gpt_review",
      next_on_fail: "implementation",
      instruction: "Apply adversarial, regression, and chaos pressure to break weak assumptions. Include command-level evidence and failing scenarios tested."
    },
    {
      name: "gpt_review",
      agent: "build",
      max_attempts: 3,
      next_on_pass: "ship_gate",
      next_on_fail: "implementation",
      instruction: "Give final review and explicit ship or no-ship recommendation with evidence references."
    }
  ]

  let run_id = (date now | format date "%Y%m%d-%H%M%S")
  mut current_stage = "contract"
  mut attempts = {}
  mut artifacts: list<any> = []
  mut transitions = 0
  mut failure_counts = {}

  print $"run_id=($run_id) bead=($bead)"

  loop {
    if $transitions >= $max_transitions {
      print ({
        run_id: $run_id,
        bead: $bead,
        ok: false,
        reason: "max_transitions_exceeded",
        max_transitions: $max_transitions,
        attempts: $attempts,
        artifacts: $artifacts
      } | to json -r)
      return
    }

    if $current_stage == "ship_gate" {
      break
    }

    let stage = ($stages | where name == $current_stage | get 0)
    let prior_attempt = (try { $attempts | get $current_stage } catch { 0 })
    let attempt = ($prior_attempt + 1)
    $attempts = ($attempts | upsert $current_stage $attempt)

    if $attempt > $stage.max_attempts {
      print ({
        run_id: $run_id,
        bead: $bead,
        stage: $current_stage,
        attempt: $attempt,
        ok: false,
        reason: "max_attempts_exceeded",
        attempts: $attempts,
        artifacts: $artifacts
      } | to json -r)
      return
    }

    let title = $"nu-orchestrator-($run_id)-($current_stage)-a($attempt)"
    let session_id = (create-session $base $user $effective_password $title)
    let prompt = (stage-prompt $bead $stage $attempt $artifacts)
    let request = (stage-request-body $model_provider $model_id $stage.agent $prompt)

    print $"stage=($current_stage) attempt=($attempt) session=($session_id)"

    let latest = (stage-sync-exec $base $user $effective_password $session_id $request)
    if $latest == null {
      print ({
        run_id: $run_id,
        bead: $bead,
        stage: $current_stage,
        attempt: $attempt,
        session_id: $session_id,
        ok: false,
        reason: "stage_request_failed",
        attempts: $attempts,
        artifacts: $artifacts
      } | to json -r)
      return
    }

    let assistant_text = (extract-text-part $latest)
    if $assistant_text == null {
      print ({
        run_id: $run_id,
        bead: $bead,
        stage: $current_stage,
        attempt: $attempt,
        session_id: $session_id,
        ok: false,
        reason: "assistant_output_missing_text_part",
        attempts: $attempts,
        artifacts: $artifacts
      } | to json -r)
      return
    }
    let parsed = (try { $assistant_text | from json } catch { null })

    if $parsed == null {
      print ({
        run_id: $run_id,
        bead: $bead,
        stage: $current_stage,
        attempt: $attempt,
        session_id: $session_id,
        ok: false,
        reason: "assistant_output_not_json",
        raw_text: $assistant_text
      } | to json -r)
      return
    }

    let pass = (try { $parsed.pass } catch { false })
    let needs_evidence = (stage-requires-evidence $current_stage)
    let evidence_ok = (if $needs_evidence { evidence-valid $parsed } else { true })
    let effective_pass = (if $pass and $evidence_ok { true } else { false })
    let failure_category = (
      if $effective_pass {
        null
      } else if not $evidence_ok {
        "missing_evidence_contract"
      } else {
        (try { $parsed.failure_category } catch { "unspecified_failure" })
      }
    )

    let final_result = (
      if $effective_pass {
        $parsed
      } else {
        ($parsed | upsert pass false | upsert failure_category $failure_category)
      }
    )
    let artifact = {
      run_id: $run_id,
      bead: $bead,
      stage: $current_stage,
      attempt: $attempt,
      session_id: $session_id,
      message_id: ($latest | get info.id),
      provider: ($latest | get info.providerID),
      model: ($latest | get info.modelID),
      tokens: (try { $latest | get info.tokens } catch { null }),
      result: $final_result
    }

    $artifacts = ($artifacts | append $artifact)

    if $effective_pass == false {
      let fkey = $"($current_stage)|($failure_category)"
      let fcount = ((try { $failure_counts | get $fkey } catch { 0 }) + 1)
      $failure_counts = ($failure_counts | upsert $fkey $fcount)

      if $current_stage == "implementation" and $fcount >= 3 {
        print ({
          run_id: $run_id,
          bead: $bead,
          ok: false,
          reason: "stuck_repeated_failure",
          stuck_key: $fkey,
          attempts: $attempts,
          artifacts: $artifacts
        } | to json -r)
        return
      }
    }

    $current_stage = (policy-next-stage $current_stage $effective_pass)
    $transitions = $transitions + 1
  }

  print ({
    run_id: $run_id,
    bead: $bead,
    ok: true,
    final_stage: "ship_gate",
    attempts: $attempts,
    artifacts: $artifacts
  } | to json -r)
}
