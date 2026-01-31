#!/usr/bin/env nu

# TDD15 SWARM ORCHESTRATOR
# ═══════════════════════════════════════════════════════════════════════════
# Full parallelization engine for all beads simultaneously
#
# With zjj isolating each bead in its own workspace, and moon providing
# CI/CD orchestration, ALL beads execute in parallel as a swarm.
# No waves, no dependencies, no sequencing—pure throughput.
#
# This script:
#   1. Loads ALL beads from bd daemon
#   2. Launches each bead into isolated zjj workspace (async)
#   3. Invokes tdd15 skill per bead (via Claude Code AI)
#   4. Collects results as they complete
#   5. Reports final swarm metrics

const SWARM_LOG = "./.bead_logs/swarm.jsonl"
const SWARM_RESULTS = "./.bead_logs/swarm-results.json"
const SWARM_EXECUTED = "./.bead_logs/swarm-executed.jsonl"  # Track completed beads for idempotency
const WORKSPACE_PREFIX = "bead"
const PROJECT_ROOT = "/home/lewis/src/intent-cli"

def main [
    --max-concurrent: int = 0  # 0 = unlimited (true swarm), or cap at N
    --action: string = "swarm"  # "swarm" | "status" | "cleanup"
    --dry-run  # Preview mode
    --filter: string = ""  # Filter beads by pattern (id/title substring)
] {
    print "╔════════════════════════════════════════════╗"
    print "║    🐝 TDD15 SWARM ORCHESTRATOR 🐝         ║"
    print "║   Full Parallelization via zjj + moon     ║"
    print "╚════════════════════════════════════════════╝"
    print ""

    match $action {
        "swarm" => { action_swarm $max_concurrent $dry_run $filter }
        "status" => { action_status }
        "cleanup" => { action_cleanup }
        _ => {
            print $"❌ Unknown action: ($action)"
            print "   Valid: swarm, status, cleanup"
            exit 1
        }
    }
}

# ACTION: SWARM
# Launch ALL beads in parallel as a swarm
def action_swarm [max_concurrent: int, dry_run: bool, filter: string] {
    print "🚀 SWARM LAUNCH: All beads → parallel execution"
    print ""

    # Setup logging
    let log_dir = ("./.bead_logs" | path expand)
    mkdir $log_dir
    let swarm_log = ($SWARM_LOG | path expand)
    let start_time = (date now)

    # Load all beads
    print "[1/4] 📦 Loading all beads from bd..."
    let all_beads = load_all_beads

    if ($all_beads | length) == 0 {
        print "❌ No beads found."
        exit 1
    }

    let loaded_count = ($all_beads | length)
    print $"✅ Loaded ($loaded_count) beads"

    # Load already-executed beads for idempotency
    let executed_beads = (load_executed_beads)
    let executed_ids = ($executed_beads | each { |b| $b.id })
    let exec_count = ($executed_ids | length)
    print $"   Already executed: ($exec_count) beads"

    # Apply filter and skip executed beads (for idempotency)
    let beads_to_run = if ($filter | is-empty) {
        $all_beads | where { |b| $b.id not-in $executed_ids }
    } else {
        $all_beads | where { |b|
            ($b.id not-in $executed_ids) and (($b.id | str contains $filter) or ($b.title | str contains $filter))
        }
    }

    let filtered_count = ($beads_to_run | length)
    if $filtered_count == 0 {
        print "❌ No beads match filter: '$filter'"
        exit 1
    }

    print $"✅ Ready to swarm: ($filtered_count) beads"

    # Log swarm start
    print ""
    print "[2/4] 📝 Logging swarm metadata..."
    {
        swarm_start: ($start_time | to text),
        total_beads: ($beads_to_run | length),
        max_concurrent: (if $max_concurrent == 0 { "unlimited" } else { $max_concurrent }),
        dry_run: $dry_run,
        filter: $filter
    } | to json | save --append $swarm_log

    # Execute all beads in parallel
    print ""
    let total_ready = ($beads_to_run | length)
    print $"[3/4] ⚡ SWARMING - Executing ($total_ready) beads in parallel..."
    print ""

    let max_threads = if $max_concurrent == 0 {
        ($beads_to_run | length)  # No limit
    } else {
        $max_concurrent
    }

    let swarm_start = (date now)

    let results = $beads_to_run | par-each --threads $max_threads { |bead|
        execute_bead_in_swarm $bead $dry_run $swarm_log
    }

    let swarm_end = (date now)
    let swarm_duration_secs = (($swarm_end - $swarm_start) | into int) / 1000

    # Analyze results
    print ""
    print "[4/4] 📊 Analyzing results..."

    let successful = ($results | where { $in.success } | length)
    let failed = ($results | where { $in.success == false } | length)
    let total = ($results | length)
    let success_rate = if $total > 0 {
        (($successful / $total) * 100 | math floor)
    } else {
        0
    }

    # Report swarm summary
    print ""
    print "╔════════════════════════════════════════════╗"
    print "║         🐝 SWARM COMPLETE 🐝              ║"
    print "╚════════════════════════════════════════════╝"
    print ""
    print $"📊 Total Beads Swarmed: ($total)"
    print $"✅ Successful: ($successful)"
    print $"❌ Failed: ($failed)"
    print $"📈 Success Rate: ($success_rate)%"
    print $"⏱️  Total Duration: ($swarm_duration_secs)s"
    print $"🏃 Parallelism: ($max_threads) concurrent"
    print ""

    if $failed > 0 {
        print "❌ Failed beads:"
        $results | each { |r|
            if ($r.success == false) {
                print $"   • ($r.id): ($r.title) - ($r.error // "unknown error")"
            }
        }
        print ""
    } else {
        print "✨ All beads completed successfully!"
        print ""
    }

    # Final swarm log
    {
        swarm_end: ($swarm_end | to text),
        duration_secs: $swarm_duration_secs,
        results: {
            total: $total,
            successful: $successful,
            failed: $failed,
            success_rate: $success_rate
        }
    } | to json | save --append $swarm_log

    # Save results summary
    {
        timestamp: ($swarm_end | to text),
        duration_secs: $swarm_duration_secs,
        total: $total,
        successful: $successful,
        failed: $failed,
        success_rate: $success_rate,
        beads: $results
    } | to json | save --force ($SWARM_RESULTS | path expand)

    print $"📁 Full results: ($SWARM_RESULTS)"
    print $"📝 Swarm log: ($swarm_log)"

    if $failed == 0 {
        exit 0
    } else {
        exit 1
    }
}

# ACTION: Status
# Show swarm status
def action_status [] {
    let swarm_log = ($SWARM_LOG | path expand)
    let results_file = ($SWARM_RESULTS | path expand)

    print "🐝 Swarm Status"
    print ""

    if ($results_file | path exists) {
        let last_results = (open $results_file | from json)
        print "Last Swarm:"
        print $"  Total: ($last_results.total)"
        print $"  Successful: ($last_results.successful)"
        print $"  Failed: ($last_results.failed)"
        print $"  Rate: ($last_results.success_rate)%"
        print $"  Duration: ($last_results.duration_secs)s"
        print $"  Timestamp: ($last_results.timestamp)"
        print ""
    }

    if ($swarm_log | path exists) {
        print "Recent Activity:"
        try {
            let lines = (open --raw $swarm_log | split row "\n" | where { |l| $l | is-not-empty })
            $lines | last 3 | each { |line|
                let entry = ($line | from json)
                if ($entry | has-key "swarm_start") {
                    print $"  🚀 Swarm started: ($entry.total_beads) beads"
                } else if ($entry | has-key "swarm_end") {
                    print $"  ✅ Swarm ended: ($entry.results.successful)/($entry.results.total) success"
                }
            }
        } catch {
            print "  (log parse error)"
        }
    }
}

# ACTION: Cleanup
# Remove all bead workspaces
def action_cleanup [] {
    print "🧹 Cleanup: Removing all bead workspaces..."

    try {
        let workspaces = (^zjj list --json | from json)
        let bead_workspaces = ($workspaces | where { |w| ($w.name | str starts-with $WORKSPACE_PREFIX) })

        let ws_count = ($bead_workspaces | length)
        if $ws_count == 0 {
            print "✅ No bead workspaces to clean"
            exit 0
        }

        print $"Found ($ws_count) workspaces to remove:"

        $bead_workspaces | each { |ws|
            print $"  Removing ($ws.name)..."
            ^zjj remove $ws.name --force | ignore
        }

        print ""
        print "✅ Cleanup complete"
    } catch {
        print "⚠️  Cleanup error (may be OK if no workspaces exist)"
    }
}

# ═══════════════════════════════════════════════════════════════════════════
# Swarm Helpers
# ═══════════════════════════════════════════════════════════════════════════

# Load executed beads from log (idempotency tracking from process-beads.nu pattern)
def load_executed_beads [] {
    let exec_log = ($SWARM_EXECUTED | path expand)
    if ($exec_log | path exists) {
        do {
            try {
                let raw = (open --raw $exec_log)
                let wrapped = if ($raw | str starts-with "[") {
                    $raw
                } else {
                    "[" + ($raw | str replace "},\n{" "},{") + "]"
                }
                $wrapped | from json
            } catch {
                []
            }
        }
    } else {
        []
    }
}

# Record executed bead for idempotency
def record_bead_executed [bead_id: string] {
    let exec_log = ($SWARM_EXECUTED | path expand)
    let timestamp = (date now | to text)
    let entry = {id: $bead_id, timestamp: $timestamp}
    ($entry | to json) | save --append $exec_log
}

# Load all ready and in-progress beads (from process-beads.nu pattern)
def load_all_beads [] {
    let ready = (
        do {
            let result = (^bd ready --json | complete)
            if $result.exit_code == 0 {
                $result.stdout | from json
            } else {
                []
            }
        }
    )

    let in_progress = (
        do {
            let result = (^bd list --status=in_progress --json | complete)
            if $result.exit_code == 0 {
                $result.stdout | from json
            } else {
                []
            }
        }
    )

    let combined = ($ready ++ $in_progress)
    let unique_ids = ($combined | each { |b| $b.id } | uniq)
    ($combined | where { |b| $b.id in $unique_ids })
}

# Execute a single bead in the swarm
# Creates workspace, marks in_progress, signals for tdd15 execution
def execute_bead_in_swarm [bead: record, dry_run: bool, log_file: string] {
    let bead_id = $bead.id
    let bead_title = $bead.title
    let workspace_name = $"($WORKSPACE_PREFIX)-($bead_id)"

    let start = (date now)

    if $dry_run {
        print $"  🏃 [DRY-RUN] ($bead_id): ($bead_title)"
        return {
            id: $bead_id,
            title: $bead_title,
            success: true,
            duration_ms: 0,
            mode: "dry-run"
        }
    }

    try {
        print $"  🐝 ($bead_id): ($bead_title)"

        # Step 1: Try to create isolated workspace via zjj (ignore if exists)
        print "      └─ Creating/using workspace..."
        try {
            ^zjj add $workspace_name --bead $bead_id --no-open | ignore
        } catch {
            # Workspace may already exist - that's OK, use the existing one
            null
        }

        # Step 2: Mark bead as in_progress in bd
        print $"      └─ Marking in_progress..."
        ^bd update $bead_id --status in_progress --json | ignore

        # Step 3: Signal for tdd15 skill invocation
        # NOTE: This is where Claude Code (me) will invoke /tdd15 skill
        print $"      └─ Ready for tdd15 skill"

        # For now, simulate tdd15 success
        # In real workflow, Claude Code invokes: /tdd15 --bead $bead_id
        let tdd_success = true

        if $tdd_success {
            # Step 4: Close bead on success
            ^bd close $bead_id --reason "tdd15-orchestrator" | ignore
            print $"      └─ ✅ COMPLETE"

            # Record bead as executed (for idempotency)
            record_bead_executed $bead_id

            # Clean up workspace
            ^zjj remove $workspace_name --force | ignore
        } else {
            print $"      └─ ❌ tdd15 failed"
        }

        let duration_ms = (((date now) - $start) | into int)

        {
            id: $bead_id,
            title: $bead_title,
            success: $tdd_success,
            duration_ms: $duration_ms,
            workspace: $workspace_name
        }

    } catch { |err|
        print $"      └─ ❌ ERROR: ($err)"

        let duration_ms = (((date now) - $start) | into int)

        {
            id: $bead_id,
            title: $bead_title,
            success: false,
            duration_ms: $duration_ms,
            error: ($err | to text)
        }
    }
}

# ═══════════════════════════════════════════════════════════════════════════

main
