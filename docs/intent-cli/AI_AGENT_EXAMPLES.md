# AI Agent Examples

This guide provides practical examples of how AI agents can interact with the Intent CLI programmatically. All examples use the `--cue` flag for machine-readable output and the `--json` flag where applicable.

## Table of Contents

1. [Basic Interview Workflow](#basic-interview-workflow)
2. [Automated Testing Workflow](#automated-testing-workflow)
3. [KIRK Analysis Workflow](#kirk-analysis-workflow)
4. [Beads Generation Workflow](#beads-generation-workflow)
5. [Error Handling](#error-handling)

---

## Basic Interview Workflow

### Starting a New Interview

**Command:**
```bash
intent interview --cue --profile api
```

**Response:**
```cue
{
	action: "ask_question"

	question: {
		text: "What is the primary purpose of this API?"
		pattern: "ubiquitous"
		examples: ["User authentication", "Payment processing", "Data analytics"]
		hint: "Describe the core capability this API provides"
	}

	progress: {
		current_step: 1
		total_steps: 25
		percent_complete: 0
		category: "basic_info"
	}

	session: {
		id: "interview-abc123def456"
		profile: "api"
		started_at: "2026-01-17T14:30:00Z"
	}
}
```

**Python Example:**
```python
import subprocess
import json

def start_interview(profile: str = "api") -> dict:
    """Start a new interview session"""
    result = subprocess.run(
        ["intent", "interview", "--cue", "--profile", profile],
        capture_output=True,
        text=True,
        check=True
    )

    # Parse CUE output (convert to JSON for simplicity)
    # In production, use a proper CUE parser
    response = parse_cue_response(result.stdout)

    if response["action"] == "ask_question":
        session_id = response["session"]["id"]
        question = response["question"]["text"]
        return {
            "session_id": session_id,
            "question": question,
            "progress": response["progress"]
        }
    else:
        raise ValueError(f"Unexpected action: {response['action']}")

# Usage
interview = start_interview("api")
print(f"Session ID: {interview['session_id']}")
print(f"Question: {interview['question']}")
```

### Answering Questions

**Command:**
```bash
intent interview --cue --session interview-abc123def456 --answer "THE SYSTEM SHALL provide user authentication via JWT tokens"
```

**Response:**
```cue
{
	action: "ask_question"

	question: {
		text: "What authentication method does this API use?"
		pattern: "ubiquitous"
		examples: ["JWT", "OAuth2", "API Keys", "Session cookies"]
		hint: "Specify how clients prove their identity"
	}

	progress: {
		current_step: 2
		total_steps: 25
		percent_complete: 4
		category: "basic_info"
	}

	session: {
		id: "interview-abc123def456"
		profile: "api"
		started_at: "2026-01-17T14:30:00Z"
	}
}
```

**Python Example:**
```python
def answer_question(session_id: str, answer: str) -> dict:
    """Submit an answer to the current question"""
    result = subprocess.run(
        [
            "intent", "interview", "--cue",
            "--session", session_id,
            "--answer", answer
        ],
        capture_output=True,
        text=True,
        check=True
    )

    response = parse_cue_response(result.stdout)

    if response["action"] == "ask_question":
        return {
            "next_question": response["question"]["text"],
            "progress": response["progress"]
        }
    elif response["action"] == "interview_complete":
        return {
            "completed": True,
            "spec_path": response["output"]["spec_path"]
        }
    else:
        raise ValueError(f"Unexpected action: {response['action']}")

# Usage
next_step = answer_question(
    "interview-abc123def456",
    "THE SYSTEM SHALL use JWT tokens for authentication"
)
```

### Resuming an Interview

**Command:**
```bash
intent interview --cue --session interview-abc123def456
```

This returns the next unanswered question in the same format as above.

### Complete Interview Loop

**Python Example:**
```python
def run_complete_interview(profile: str, answers_generator) -> str:
    """
    Run a complete interview session with an AI-generated answer provider

    Args:
        profile: Interview profile (api, cli, event, etc.)
        answers_generator: Function that takes a question and returns an answer

    Returns:
        Path to the generated spec file
    """
    # Start interview
    interview = start_interview(profile)
    session_id = interview["session_id"]

    while True:
        question = interview.get("question")

        if not question:
            # Interview complete
            if interview.get("completed"):
                return interview["spec_path"]
            break

        # Generate answer using AI/LLM
        answer = answers_generator(question)

        # Submit answer
        interview = answer_question(session_id, answer)

    return interview.get("spec_path", "")

# Usage with an LLM
def llm_answer_generator(question: str) -> str:
    """Use an LLM to generate answers"""
    prompt = f"""
    You are helping define requirements for an API.
    Answer this question using EARS syntax:

    {question}

    Use patterns like:
    - THE SYSTEM SHALL [behavior]
    - WHEN [trigger], THE SYSTEM SHALL [response]
    - IF [condition], THEN THE SYSTEM SHALL [behavior]
    """
    return call_llm(prompt)

spec_path = run_complete_interview("api", llm_answer_generator)
print(f"Generated spec: {spec_path}")
```

### Dry-Run Mode (Preview Without Saving)

**Command:**
```bash
intent interview --cue --profile api --dry-run
```

**Response:**
```cue
{
	action: "ask_question"

	question: {
		text: "What is the primary purpose of this API?"
		pattern: "ubiquitous"
		examples: ["User authentication", "Payment processing"]
		hint: "Describe the core capability"
	}

	progress: {
		current_step: 1
		total_steps: 25
		percent_complete: 0
		category: "basic_info"
	}

	session: {
		id: "dry-run-xyz789"
		profile: "api"
		started_at: "2026-01-17T14:30:00Z"
		dry_run: true
	}
}
```

**Note:** Dry-run sessions are not saved to `.interview/sessions.jsonl` and cannot be resumed. They are useful for previewing the interview flow.

---

## Automated Testing Workflow

### Running Tests with JSON Output

**Command:**
```bash
intent check examples/petstore.cue --target http://localhost:8080 --json
```

**Response:**
```json
{
  "summary": {
    "passed": 12,
    "failed": 3,
    "blocked": 2,
    "total": 17
  },
  "results": [
    {
      "feature": "User Management",
      "behavior": "create-user",
      "status": "passed",
      "duration_ms": 145
    },
    {
      "feature": "User Management",
      "behavior": "get-user",
      "status": "failed",
      "error": "Expected status 200, got 404",
      "duration_ms": 89
    },
    {
      "feature": "Authentication",
      "behavior": "login",
      "status": "blocked",
      "reason": "Depends on create-user which failed",
      "duration_ms": 0
    }
  ]
}
```

**Python Example:**
```python
def run_api_tests(spec_path: str, target_url: str) -> dict:
    """Run API tests and return results"""
    result = subprocess.run(
        [
            "intent", "check", spec_path,
            "--target", target_url,
            "--json"
        ],
        capture_output=True,
        text=True
    )

    results = json.loads(result.stdout)

    return {
        "exit_code": result.returncode,
        "passed": results["summary"]["passed"],
        "failed": results["summary"]["failed"],
        "blocked": results["summary"]["blocked"],
        "details": results["results"]
    }

# Usage
test_results = run_api_tests(
    ".interview/spec-abc123.cue",
    "http://localhost:8080"
)

if test_results["exit_code"] == 0:
    print("All tests passed!")
elif test_results["exit_code"] == 1:
    print(f"{test_results['failed']} tests failed")
    for result in test_results["details"]:
        if result["status"] == "failed":
            print(f"  - {result['behavior']}: {result['error']}")
```

### Re-running Specific Tests

**Command:**
```bash
intent check spec.cue --target http://localhost:8080 --behavior create-user --json
```

**TypeScript Example:**
```typescript
interface TestResult {
  feature: string;
  behavior: string;
  status: 'passed' | 'failed' | 'blocked';
  error?: string;
  duration_ms: number;
}

interface TestSummary {
  summary: {
    passed: number;
    failed: number;
    blocked: number;
    total: number;
  };
  results: TestResult[];
}

async function runBehaviorTest(
  specPath: string,
  targetUrl: string,
  behaviorName: string
): Promise<TestResult> {
  const { stdout } = await execAsync(
    `intent check ${specPath} --target ${targetUrl} --behavior ${behaviorName} --json`
  );

  const results: TestSummary = JSON.parse(stdout);
  return results.results[0];
}

// Usage
const result = await runBehaviorTest(
  '.interview/spec-abc123.cue',
  'http://localhost:8080',
  'create-user'
);

console.log(`${result.behavior}: ${result.status}`);
```

### Continuous Testing Loop

**Python Example:**
```python
import time

def continuous_test_loop(spec_path: str, target_url: str, interval: int = 60):
    """Run tests continuously and report failures"""
    while True:
        results = run_api_tests(spec_path, target_url)

        if results["exit_code"] != 0:
            print(f"[{time.strftime('%H:%M:%S')}] Tests failed!")
            for result in results["details"]:
                if result["status"] == "failed":
                    send_alert(f"Test failed: {result['behavior']}")
        else:
            print(f"[{time.strftime('%H:%M:%S')}] All tests passed")

        time.sleep(interval)

# Usage
continuous_test_loop(".interview/spec-abc123.cue", "http://localhost:8080")
```

---

## KIRK Analysis Workflow

### Quality Analysis

**Command:**
```bash
intent quality .interview/spec-abc123.cue
```

**Output:**
```
Quality Analysis for spec-abc123.cue
====================================

Overall Score: 78/100

Dimensions:
  Completeness:  85/100 - Most critical paths covered
  Clarity:       80/100 - Requirements are clear and unambiguous
  Testability:   75/100 - Most behaviors are testable
  Coverage:      70/100 - Good HTTP method coverage
  Correctness:   90/100 - No logical contradictions found

Recommendations:
  1. Add error handling for edge cases
  2. Specify timeout behaviors
  3. Add more authentication failure scenarios
```

**Python Example:**
```python
def analyze_quality(spec_path: str) -> dict:
    """Analyze spec quality and return scores"""
    result = subprocess.run(
        ["intent", "quality", spec_path],
        capture_output=True,
        text=True,
        check=True
    )

    # Parse quality scores from output
    scores = parse_quality_output(result.stdout)

    return {
        "overall": scores["overall"],
        "completeness": scores["completeness"],
        "clarity": scores["clarity"],
        "testability": scores["testability"],
        "coverage": scores["coverage"],
        "correctness": scores["correctness"],
        "recommendations": scores["recommendations"]
    }

# Usage
quality = analyze_quality(".interview/spec-abc123.cue")
if quality["overall"] < 70:
    print("Quality too low, regenerating spec...")
```

### Gap Detection

**Command:**
```bash
intent gaps .interview/spec-abc123.cue
```

**Output:**
```
Gap Analysis
============

Blocking Gaps (3):
  1. No authentication failure handling specified
     Why: Critical for security
     Suggestion: Add UNWANTED behavior for invalid tokens

  2. Missing timeout configuration
     Why: Required for reliability
     Suggestion: Specify timeout values in config

  3. No rate limiting defined
     Why: Prevents abuse
     Suggestion: Add rate limit behaviors

Nice-to-Have Gaps (2):
  1. No pagination details
  2. Missing cache headers
```

**Python Example:**
```python
def detect_gaps(spec_path: str) -> dict:
    """Find missing requirements in spec"""
    result = subprocess.run(
        ["intent", "gaps", spec_path],
        capture_output=True,
        text=True,
        check=True
    )

    gaps = parse_gaps_output(result.stdout)

    return {
        "blocking": gaps["blocking"],
        "nice_to_have": gaps["nice_to_have"],
        "total": len(gaps["blocking"]) + len(gaps["nice_to_have"])
    }

# Usage
gaps = detect_gaps(".interview/spec-abc123.cue")
if gaps["blocking"]:
    print(f"Found {len(gaps['blocking'])} blocking gaps!")
    for gap in gaps["blocking"]:
        print(f"  - {gap['description']}")
```

### Inversion Analysis (Second-Order Thinking)

**Command:**
```bash
intent invert .interview/spec-abc123.cue
```

**Output:**
```
Inversion Analysis
==================

What Could Go Wrong:

1. Authentication bypass
   Risk: High
   Scenario: Client sends request without token
   Mitigation: Add explicit authentication check behavior

2. Data race conditions
   Risk: Medium
   Scenario: Concurrent updates to same resource
   Mitigation: Specify optimistic locking or versioning

3. Memory exhaustion
   Risk: Medium
   Scenario: Large payload uploads
   Mitigation: Add max payload size limit
```

**Python Example:**
```python
def inversion_check(spec_path: str) -> list[dict]:
    """Identify potential failure modes"""
    result = subprocess.run(
        ["intent", "invert", spec_path],
        capture_output=True,
        text=True,
        check=True
    )

    risks = parse_inversion_output(result.stdout)

    return [
        {
            "scenario": risk["scenario"],
            "severity": risk["risk"],
            "mitigation": risk["mitigation"]
        }
        for risk in risks
    ]

# Usage
risks = inversion_check(".interview/spec-abc123.cue")
high_risks = [r for r in risks if r["severity"] == "High"]
if high_risks:
    print("Critical risks found:")
    for risk in high_risks:
        print(f"  - {risk['scenario']}")
```

### Complete KIRK Analysis

**Python Example:**
```python
def complete_kirk_analysis(spec_path: str) -> dict:
    """Run all KIRK analysis tools"""
    quality = analyze_quality(spec_path)
    gaps = detect_gaps(spec_path)
    risks = inversion_check(spec_path)

    # Aggregate insights
    analysis = {
        "quality_score": quality["overall"],
        "blocking_gaps": len(gaps["blocking"]),
        "high_risks": len([r for r in risks if r["severity"] == "High"]),
        "ready_for_implementation": (
            quality["overall"] >= 70 and
            len(gaps["blocking"]) == 0 and
            len([r for r in risks if r["severity"] == "High"]) == 0
        )
    }

    return analysis

# Usage
analysis = complete_kirk_analysis(".interview/spec-abc123.cue")
if analysis["ready_for_implementation"]:
    print("Spec is ready for implementation!")
else:
    print("Spec needs refinement:")
    if analysis["quality_score"] < 70:
        print(f"  - Quality too low: {analysis['quality_score']}")
    if analysis["blocking_gaps"] > 0:
        print(f"  - Blocking gaps: {analysis['blocking_gaps']}")
    if analysis["high_risks"] > 0:
        print(f"  - High risks: {analysis['high_risks']}")
```

---

## Beads Generation Workflow

### Generating Work Items

**Command:**
```bash
intent beads interview-abc123def456
```

**Output (text mode):**
```
Generated 8 beads for session interview-abc123def456

Beads saved to: .beads/issues.jsonl

Next steps:
  1. Review beads: bd ready
  2. Claim work: bd update <id> --status in_progress
  3. Complete: bd close <id> --reason 'Done'
```

**Python Example:**
```python
def generate_beads(session_id: str) -> list[str]:
    """Generate work items from completed interview"""
    result = subprocess.run(
        ["intent", "beads", session_id],
        capture_output=True,
        text=True,
        check=True
    )

    # Parse bead IDs from output
    bead_ids = parse_bead_ids(result.stdout)

    return bead_ids

# Usage
session_id = "interview-abc123def456"
bead_ids = generate_beads(session_id)
print(f"Generated {len(bead_ids)} work items")
```

### Executing Beads in Order

**Python Example:**
```python
import subprocess
import json

def list_ready_beads() -> list[dict]:
    """Get ready beads from bd"""
    result = subprocess.run(
        ["bd", "ready", "--json"],
        capture_output=True,
        text=True,
        check=True
    )
    return json.loads(result.stdout)

def claim_bead(bead_id: str) -> None:
    """Mark bead as in progress"""
    subprocess.run(
        ["bd", "update", bead_id, "--status", "in_progress", "--json"],
        check=True
    )

def complete_bead(bead_id: str, reason: str) -> None:
    """Mark bead as complete"""
    subprocess.run(
        ["bd", "close", bead_id, "--reason", reason, "--json"],
        check=True
    )

def execute_beads_workflow():
    """Execute all ready beads in dependency order"""
    while True:
        beads = list_ready_beads()

        if not beads:
            print("No more ready beads")
            break

        # Get first ready bead (bd orders by dependencies)
        bead = beads[0]
        bead_id = bead["id"]

        print(f"Working on: {bead['title']}")

        # Claim the bead
        claim_bead(bead_id)

        # Execute the work (AI agent implementation)
        success = execute_bead_implementation(bead)

        if success:
            complete_bead(bead_id, "Implementation complete")
        else:
            # Mark as blocked or failed
            subprocess.run(
                ["bd", "update", bead_id, "--status", "blocked"],
                check=True
            )
            break

# Usage
execute_beads_workflow()
```

---

## Error Handling

### Structured Error Parsing

**Command (with error):**
```bash
intent check missing.cue --target http://localhost:8080 --json
```

**Exit Code:** 4 (file not found)

**Response:**
```json
{
  "action": "error",
  "error": {
    "type": "file_not_found",
    "message": "File not found: missing.cue",
    "context": {
      "path": "missing.cue",
      "expected_location": "Spec files should be in project root or .interview/"
    }
  },
  "suggestion": "Check that the file path is correct and the file exists",
  "recovery": [
    "Verify the file exists: ls missing.cue",
    "Check file permissions: ls -la missing.cue",
    "Use absolute path if relative path fails",
    "Ensure you're in the correct directory"
  ],
  "retry_allowed": true,
  "exit_code": 4
}
```

**Python Example:**
```python
class IntentError(Exception):
    """Structured error from Intent CLI"""
    def __init__(self, error_data: dict):
        self.error_type = error_data["error"]["type"]
        self.message = error_data["error"]["message"]
        self.context = error_data["error"].get("context", {})
        self.suggestion = error_data["suggestion"]
        self.recovery = error_data["recovery"]
        self.retry_allowed = error_data["retry_allowed"]
        self.exit_code = error_data["exit_code"]
        super().__init__(self.message)

def run_intent_command(args: list[str]) -> dict:
    """Run intent command and handle errors"""
    result = subprocess.run(
        ["intent"] + args,
        capture_output=True,
        text=True
    )

    # Parse response
    try:
        response = json.loads(result.stdout)
    except json.JSONDecodeError:
        # Not JSON, might be text error
        raise RuntimeError(result.stderr or result.stdout)

    # Check for errors
    if result.returncode != 0:
        if response.get("action") == "error":
            raise IntentError(response)
        else:
            raise RuntimeError(f"Command failed with exit code {result.returncode}")

    return response

# Usage with retry
def run_with_retry(args: list[str], max_retries: int = 3) -> dict:
    """Run command with automatic retry on retriable errors"""
    for attempt in range(max_retries):
        try:
            return run_intent_command(args)
        except IntentError as e:
            if not e.retry_allowed or attempt == max_retries - 1:
                print(f"Error: {e.message}")
                print(f"Suggestion: {e.suggestion}")
                print("Recovery steps:")
                for step in e.recovery:
                    print(f"  - {step}")
                raise
            else:
                print(f"Retrying ({attempt + 1}/{max_retries})...")
                time.sleep(2 ** attempt)  # Exponential backoff

# Usage
try:
    result = run_with_retry(["check", "spec.cue", "--target", "http://localhost:8080"])
except IntentError as e:
    if e.error_type == "file_not_found":
        # Handle file not found specifically
        print("Creating spec from interview...")
    elif e.error_type == "http_connection_error":
        # Handle connection error
        print("Starting local server...")
```

### Exit Code Handling

**Exit Codes:**
- `0`: Success
- `1`: Test failures (some behaviors failed)
- `2`: Blocked behaviors (dependencies failed)
- `3`: Invalid specification (CUE validation error)
- `4`: General error (file not found, network error, etc.)

**Python Example:**
```python
def handle_exit_code(exit_code: int, stdout: str, stderr: str) -> str:
    """Handle different exit codes appropriately"""
    if exit_code == 0:
        return "success"
    elif exit_code == 1:
        # Test failures - parse results and retry failed tests
        results = json.loads(stdout)
        failed = [r for r in results["results"] if r["status"] == "failed"]
        return f"tests_failed: {len(failed)}"
    elif exit_code == 2:
        # Blocked behaviors - check dependencies
        return "blocked_dependencies"
    elif exit_code == 3:
        # Invalid spec - regenerate or fix
        return "invalid_spec"
    elif exit_code == 4:
        # General error - check error message
        return "general_error"
    else:
        return f"unknown_exit_code: {exit_code}"

# Usage
result = subprocess.run(
    ["intent", "check", "spec.cue", "--target", "http://localhost:8080"],
    capture_output=True,
    text=True
)

status = handle_exit_code(result.returncode, result.stdout, result.stderr)
print(f"Status: {status}")
```

### Auto-Recovery Implementation

**Python Example:**
```python
def auto_recover(error: IntentError) -> bool:
    """Attempt automatic recovery from errors"""
    if error.error_type == "file_not_found":
        path = error.context["path"]
        # Try to find the file
        if os.path.exists(f".interview/{path}"):
            print(f"Found file in .interview/ directory")
            return True

    elif error.error_type == "http_connection_error":
        # Try to start local service
        print("Attempting to start local service...")
        subprocess.run(["docker", "compose", "up", "-d"], check=False)
        time.sleep(5)
        return True

    elif error.error_type == "cue_validation_error":
        # Show CUE error and suggest fix
        print(f"CUE validation error: {error.message}")
        print("Run: cue vet <file> to see details")
        return False

    return False

# Usage
try:
    result = run_intent_command(["check", "spec.cue", "--target", "http://localhost:8080"])
except IntentError as e:
    if auto_recover(e):
        # Retry after recovery
        result = run_intent_command(["check", "spec.cue", "--target", "http://localhost:8080"])
    else:
        raise
```

---

## Best Practices for AI Agents

1. **Always use `--cue` or `--json` flags** for machine-readable output
2. **Parse the `action` field first** to route logic appropriately
3. **Check exit codes** to determine success/failure/blocked states
4. **Save session IDs** for resumability across context resets
5. **Parse recovery arrays** from errors for automatic fixes
6. **Store `.interview/` directory** in version control for audit trail
7. **Run KIRK analysis** before generating beads to catch gaps early
8. **Execute beads in dependency order** using the `bd ready` command
9. **Mark beads as failed with reasons** to improve regeneration
10. **Use dry-run mode** (`--dry-run`) to preview without committing

---

## Additional Resources

- [API Reference](API_REFERENCE.md) - Complete CLI command reference
- [EARS Syntax Guide](EARS_KIRK_WORKFLOW.md) - Writing requirements with EARS
- [Spec Format](SPEC_FORMAT.md) - CUE specification schema
- [AI Protocol Quickstart](AI_PROTOCOL_QUICKSTART.md) - Quick start for AI agents
