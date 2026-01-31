#!/usr/bin/env python3
"""
Intent CLI JSON Integration Examples (Python)

Practical examples showing how to integrate Intent CLI JSON output
into Python applications, CI/CD pipelines, and automation scripts.
"""

import json
import subprocess
import sys
from dataclasses import dataclass
from typing import Any, Dict, List, Optional


# ============================================================================
# Type Definitions
# ============================================================================

@dataclass
class JsonError:
    code: str
    message: str
    location: Optional[str] = None
    fix_hint: Optional[str] = None
    fix_command: Optional[str] = None


@dataclass
class NextAction:
    command: str
    reason: str


@dataclass
class JsonMetadata:
    timestamp: str
    version: str
    exit_code: int
    correlation_id: str
    duration_ms: int


@dataclass
class JsonResponse:
    success: bool
    action: str
    command: str
    data: Dict[str, Any]
    errors: List[JsonError]
    next_actions: List[NextAction]
    metadata: JsonMetadata
    spec_path: Optional[str]


# ============================================================================
# Helper Functions
# ============================================================================

def run_intent_command(command: str, args: List[str]) -> JsonResponse:
    """Execute an Intent CLI command and parse JSON output"""
    full_command = ["intent", command] + args + ["--json=true"]

    result = subprocess.run(
        full_command,
        capture_output=True,
        text=True,
        check=False
    )

    response_data = json.loads(result.stdout)

    # Convert to dataclass
    return JsonResponse(
        success=response_data["success"],
        action=response_data["action"],
        command=response_data["command"],
        data=response_data["data"],
        errors=[JsonError(**e) for e in response_data["errors"]],
        next_actions=[NextAction(**a) for a in response_data["next_actions"]],
        metadata=JsonMetadata(**response_data["metadata"]),
        spec_path=response_data.get("spec_path")
    )


def is_success(response: JsonResponse) -> bool:
    """Check if response indicates success"""
    return response.success and response.metadata.exit_code == 0


def get_error_messages(response: JsonResponse) -> List[str]:
    """Extract error messages from response"""
    return [error.message for error in response.errors]


def get_next_actions(response: JsonResponse) -> List[str]:
    """Get next action commands"""
    return [action.command for action in response.next_actions]


# ============================================================================
# Quality Gate Example
# ============================================================================

def quality_gate(spec_path: str, threshold: int = 80) -> None:
    """Quality gate that fails if spec score is below threshold"""
    print(f"Running quality gate (threshold: {threshold})...")

    response = run_intent_command("quality", [spec_path])

    if not response.success:
        errors = "\n".join(get_error_messages(response))
        raise RuntimeError(f"Quality check failed:\n{errors}")

    score = response.data["overall_score"]
    print(f"Quality score: {score}/100")

    if score < threshold:
        print(f"\n❌ Quality gate failed: {score} < {threshold}")
        print("\nIssues:")
        for issue in response.data["issues"]:
            print(f"  • {issue}")

        print("\nSuggestions:")
        for suggestion in response.data["suggestions"]:
            print(f"  • {suggestion}")

        sys.exit(1)

    print(f"✅ Quality gate passed: {score} >= {threshold}")

    # Follow next actions
    if response.next_actions:
        print("\nSuggested next steps:")
        for action in response.next_actions:
            print(f"  {action.command}")
            print(f"    → {action.reason}")


# ============================================================================
# KIRK Analysis Example
# ============================================================================

def kirk_analysis(spec_path: str) -> None:
    """Complete KIRK analysis workflow with automated reporting"""
    print("Starting KIRK analysis...\n")

    # Run all KIRK commands
    quality = run_intent_command("quality", [spec_path])
    coverage = run_intent_command("coverage", [spec_path])
    gaps = run_intent_command("gaps", [spec_path])
    invert = run_intent_command("invert", [spec_path])
    effects = run_intent_command("effects", [spec_path])

    # Generate report
    print("=== KIRK Analysis Report ===\n")

    # Quality scores
    print("Quality Scores:")
    print(f"  Overall:       {quality.data['overall_score']}/100")
    print(f"  Coverage:      {quality.data['coverage_score']}/100")
    print(f"  Clarity:       {quality.data['clarity_score']}/100")
    print(f"  Testability:   {quality.data['testability_score']}/100")
    print(f"  AI Readiness:  {quality.data['ai_readiness_score']}/100")

    # Coverage analysis
    print("\nCoverage:")
    print(f"  Overall:       {coverage.data['overall_score']:.1f}%")
    print(f"  OWASP:         {coverage.data['owasp']['score']:.1f}%")
    print(f"  Edge Cases:    {len(coverage.data['edge_cases']['tested'])} tested")
    print(f"                 {len(coverage.data['edge_cases']['suggested'])} suggested")

    # Gaps summary
    print("\nGaps Detected:")
    print(f"  Total:         {gaps.data['total_gaps']}")
    breakdown = gaps.data['severity_breakdown']
    print(f"  Critical:      {breakdown['critical']}")
    print(f"  High:          {breakdown['high']}")
    print(f"  Medium:        {breakdown['medium']}")
    print(f"  Low:           {breakdown['low']}")

    # Inversion analysis
    print("\nFailure Mode Analysis:")
    print(f"  Score:         {invert.data['score']:.1f}/100")
    print(f"  Security:      {len(invert.data['security_gaps'])} gaps")
    print(f"  Usability:     {len(invert.data['usability_gaps'])} gaps")
    print(f"  Integration:   {len(invert.data['integration_gaps'])} gaps")

    # Effects analysis
    print("\nSecond-Order Effects:")
    print(f"  Total:         {effects.data['total_second_order_effects']}")
    print(f"  Coverage:      {effects.data['coverage_score']:.1f}%")
    print(f"  Orphans:       {len(effects.data['orphaned_resources'])}")

    # Critical issues
    critical_gaps = [
        g for g in gaps.data['security_gaps']
        if g['severity'] == 'critical'
    ]
    if critical_gaps:
        print("\n⚠️  Critical Security Gaps:")
        for gap in critical_gaps:
            print(f"  • {gap['description']}")
            print(f"    Fix: {gap['suggestion']}")

    print("\n=== End Report ===")


# ============================================================================
# Test Execution with Feedback
# ============================================================================

def test_with_feedback(spec_path: str) -> None:
    """Run tests and generate fix beads for failures"""
    print("Running tests...\n")

    check_response = run_intent_command("check", [spec_path])

    print("Test Results:")
    print(f"  Total:    {check_response.data['total']}")
    print(f"  Passed:   {check_response.data['passed']}")
    print(f"  Failed:   {check_response.data['failed']}")
    print(f"  Skipped:  {check_response.data['skipped']}")
    print(f"  Duration: {check_response.data['duration_ms']}ms")

    if check_response.data['failed'] > 0:
        print("\n❌ Tests failed. Generating fix beads...\n")

        # Save check results to file
        results_file = "check-results.json"
        with open(results_file, 'w') as f:
            json.dump(check_response.data, f, indent=2)

        # Generate feedback
        feedback = run_intent_command("feedback", ["--results", results_file])

        print("Fix Beads Generated:")
        for i, bead in enumerate(feedback.data['fix_beads'], 1):
            print(f"\n{i}. {bead['behavior_name']} (Priority: {bead['priority']})")
            print(f"   Feature: {bead['feature']}")
            print(f"   Type: {bead['failure_type']}")
            print(f"   Issue: {bead['description']}")
            print(f"   Fix: {bead['fix_suggestion']}")

        sys.exit(1)

    print("\n✅ All tests passed!")


# ============================================================================
# CI/CD Integration
# ============================================================================

def cicd_check(
    spec_path: str,
    quality_threshold: int = 80,
    coverage_threshold: float = 70.0,
    allow_critical_gaps: bool = False
) -> None:
    """Complete CI/CD check suitable for GitHub Actions, GitLab CI, etc."""
    print("=== CI/CD Quality Check ===\n")

    failed = False

    # Quality check
    quality = run_intent_command("quality", [spec_path])
    quality_score = quality.data['overall_score']
    print(f"Quality: {quality_score}/100 (threshold: {quality_threshold})")

    if quality_score < quality_threshold:
        print("  ❌ Below threshold")
        failed = True
    else:
        print("  ✅ Passed")

    # Coverage check
    coverage = run_intent_command("coverage", [spec_path])
    coverage_score = coverage.data['overall_score']
    print(f"\nCoverage: {coverage_score:.1f}% (threshold: {coverage_threshold})")

    if coverage_score < coverage_threshold:
        print("  ❌ Below threshold")
        failed = True
    else:
        print("  ✅ Passed")

    # Critical gaps check
    gaps = run_intent_command("gaps", [spec_path])
    critical_count = gaps.data['severity_breakdown']['critical']
    print(f"\nCritical Gaps: {critical_count}")

    if critical_count > 0 and not allow_critical_gaps:
        print("  ❌ Critical gaps not allowed")
        for gap in gaps.data['security_gaps']:
            if gap['severity'] == 'critical':
                print(f"    • {gap['description']}")
        failed = True
    else:
        print("  ✅ Passed")

    # Tests
    check = run_intent_command("check", [spec_path])
    print(f"\nTests: {check.data['passed']}/{check.data['total']} passed")

    if check.data['failed'] > 0:
        print(f"  ❌ {check.data['failed']} tests failed")
        failed = True
    else:
        print("  ✅ All tests passed")

    if failed:
        print("\n❌ CI/CD check failed")
        sys.exit(1)

    print("\n✅ CI/CD check passed")


# ============================================================================
# Workflow Automation
# ============================================================================

def auto_workflow(spec_path: str, max_depth: int = 3) -> None:
    """Execute a workflow by following next_actions suggestions"""
    print("Starting automated workflow...\n")

    response = run_intent_command("quality", [spec_path])
    depth = 0

    while response.next_actions and depth < max_depth:
        print(f"\n=== Step {depth + 1} ===")
        print(f"Command: {response.command}")
        print(f"Action: {response.action}")

        next_action = response.next_actions[0]
        print(f"\nNext: {next_action.command}")
        print(f"Reason: {next_action.reason}")

        # Parse command (simplified)
        parts = next_action.command.split()
        command = parts[1]  # Skip 'intent'
        args = [arg for arg in parts[2:] if not arg.startswith('--')]

        response = run_intent_command(command, args)
        depth += 1

    print("\n=== Workflow Complete ===")


# ============================================================================
# Error Handling
# ============================================================================

def robust_execution(spec_path: str) -> None:
    """Robust error handling for Intent CLI commands"""
    try:
        response = run_intent_command("quality", [spec_path])

        if not response.success:
            print("Command failed:")
            for error in response.errors:
                print(f"  [{error.code}] {error.message}")
                if error.location:
                    print(f"    at {error.location}")
                if error.fix_hint:
                    print(f"    hint: {error.fix_hint}")
                if error.fix_command:
                    print(f"    fix: {error.fix_command}")

            # Attempt auto-fix if suggested
            fixable_errors = [e for e in response.errors if e.fix_command]
            if fixable_errors:
                print("\nAttempting auto-fix...")
                for error in fixable_errors:
                    print(f"Running: {error.fix_command}")
                    # Execute fix command (simplified)

            sys.exit(response.metadata.exit_code)

        print("✅ Success")
        print(f"Correlation ID: {response.metadata.correlation_id}")

    except Exception as error:
        print(f"Execution error: {error}", file=sys.stderr)
        sys.exit(4)


# ============================================================================
# Main
# ============================================================================

def main():
    spec_path = sys.argv[1] if len(sys.argv) > 1 else "examples/user-api.cue"

    # Uncomment to run different examples:

    # quality_gate(spec_path, 80)
    # kirk_analysis(spec_path)
    # test_with_feedback(spec_path)
    # auto_workflow(spec_path)
    # cicd_check(spec_path)
    robust_execution(spec_path)


if __name__ == "__main__":
    main()
