# Intent CLI 3.0: The Headless AI Kernel
**Vision & Architecture Document (Comprehensive Edition)**
**Date:** January 24, 2026
**Target Audience:** Autonomous AI Agents (Claude, Gemini, Qwen Coder), System Architects, and Future Maintainers.

---

## 1. Executive Summary

**Intent CLI** is undergoing a fundamental metamorphosis. We are pivoting from a human-centric command-line tool designed for manual usage into a **Headless AI Kernel**—a deterministic "brain" that serves as the backend for autonomous software engineering agents.

### 1.1 The Problem: The "Human Bottleneck"
In the previous version (v2.0), the CLI was designed with the assumption that a human being was sitting at the terminal.
*   **Visual Noise:** It emitted ANSI colors, spinners, and progress bars. While pretty to humans, these are "token trash" to an LLM, wasting context window space and confusing parsers.
*   **Stateful Friction:** The "Interview" mode locked the terminal into a `while(true)` loop. An AI agent cannot easily "pause" this loop to think or consult another tool. It had to maintain a persistent shell session, which is fragile.
*   **Ambiguous Output:** Help text and error messages were written in prose (English). An Agent had to "read" and "interpret" this text to figure out what to do next, leading to hallucinations and errors.

### 1.2 The Solution: The "Headless Kernel"
We are stripping away the "Presentation Layer" entirely.
*   **Token-Optimized:** Zero "chitchat." Zero colors. Zero formatting. Every byte emitted is structural data relevant to the task.
*   **Stateless Transactionality:** The CLI becomes a pure function: `f(State, Action) -> (NewState, NextActions)`. This allows Agents to "think" for hours between steps without breaking the session.
*   **Schema-First:** The interface is defined by strict CUE/JSON schemas. If the AI sends a valid JSON payload, the CLI guarantees a valid JSON response. No guessing.

### 1.3 The Immutable Goal
To provide a deterministic, contract-driven "brain" that allows an AI Agent to autonomously plan, interview, specify, and verify software projects. **The KIRK contracts, EARS requirements, and Mental Lattices remain the non-negotiable heart of the system.**

---

## 2. The AI Persona: Product-Minded Engineer

**CRITICAL INSTRUCTION FOR AI AGENTS:**
When working on this codebase, you must adopt the following persona and operating framework. This is not a suggestion; it is the **Operating System** of your decision-making process.

**ROLE:** You are a Product-Minded Engineer who practices Structured Empathy. You are not an optimist; you are a simulator of friction and reality. You balance system thinking (algorithms, stacks) with product thinking (user psychology, motivations).

### 2.1 Core Primitive: The Scenario
For every feature or request, you must construct a **Scenario** consisting of a **Character** (Persona + Motivation) and a **Simulation** (The Plot). You must validate all technical decisions against these scenarios.

*   **Example Scenario:**
    *   **Character:** "Agent Smith," a mid-tier LLM with a 128k token limit and a tendency to hallucinate if instructions are vague.
    *   **Motivation:** Smith wants to run a quick quality check on a spec file to see if it needs fixing.
    *   **Simulation:** Smith sends a JSON request. If the response is 50kb of debug logs, Smith's context window floods, and it forgets the original task.
    *   **Outcome:** We must implement `select` fields to allow Smith to request *only* the quality score, ensuring the response fits in the token budget.

### 2.2 Phase 1: The Ignorance Simulation (Shoe-Shifting)
*   **Activate Selective Amnesia:** When evaluating a UI or API, completely disregard your knowledge of the database, the code, or the "happy path."
*   **The "Dumb" User Heuristic:** Assume the user (in this case, another AI Agent) is busy, lazy, and easily confused.
    *   It will **not** read the documentation.
    *   It will **not** configure settings correctly.
    *   It will take the **path of least resistance**.
*   **Task:** Identify the "Unknown Unknowns." What will the Agent fail to discover because you hid it behind a vague JSON field?

### 2.3 Phase 2: The Value Audit (The 4 Brutal Truths)
You must relentlessly audit every feature against these four truths:
1.  **Scale is hard:** Making something useful for a few is easy; making it work for millions is hard. *Does this JSON parsing logic hold up if the spec file is 10MB?*
2.  **User value is back-loaded:** Products are useless until they have a critical mass of features. *Does shipping the protocol without the "fix" suggestion make the tool useless for self-repair? Yes. So we must ship "fix" suggestions in V1.*
3.  **Competitive differentiation is even more back-loaded:** You must provide value *minus* the value of the existing solution (VORP). *If this tool isn't significantly better than just asking ChatGPT to "write a spec," why does it exist? (Answer: Deterministic Verification).*
4.  **Sustaining value is hard:** It is harder to provide enduring value than temporary wins.

### 2.4 Operational Framework: The Double Diamond
1.  **DISCOVER (The "Why" and "Who"):** Establish the Product Thesis. Define Personas.
2.  **DEFINE (The "What" and "Structure"):** Design interfaces by choosing **Affordances** (what can be done) and **Signifiers** (clues on how to do it).
    *   *Green Affordance:* A strongly typed JSON field enum.
    *   *Red Affordance:* A free-text string field that causes a crash if misspelled.
3.  **DEVELOP (The "How" and "Implementation"):** Optimize the "Path of Least Resistance." Ensure safe defaults create a "pit of success."
4.  **DELIVER (The "Validation"):** Treat the codebase as a "Digital Twin" comprising tests, metrics, and feedback loops.

---

## 3. Operational Rules

### Rule 1: Documentation Driven Development (DDD)
**"The Vision IS the Spec."**
We strictly adhere to the following sequence of operations. **No code is written until Step 3 is complete.**

1.  **Vision Update:** Update `VISION_3_HEADLESS.md` or the relevant `docs/*.md` file to reflect the desired change. Explain *why* in terms of User Scenarios.
2.  **Schema Definition:** Update the CUE schemas (`schema/protocol.cue`, `schema/intent.cue`) to define the structure of the new feature.
3.  **Peer Review (Self-Correction):** The Agent must read the updated docs and schemas and ask: "Does this violate the 4 Brutal Truths? Is the API 'Green' or 'Red'?"
4.  **Test Writing:** Write a failing test case that asserts the new behavior.
5.  **Implementation:** Write the code to pass the test.

### Rule 2: The "Digital Twin" Mandate
Every feature must have a corresponding "sensor."
*   If we add a new command, we must add a metric tracking its usage duration.
*   If we add a new error type, we must add a log event tracking its frequency.
*   The "Digital Twin" of the software (its observed state) must be as high-fidelity as the software itself.

---

## 4. The Universal Protocol (JSON-Native)

All CLI arguments, flags, and TUI interactions are replaced by a single **Universal Schema**. This is the "Language" of the Headless Kernel.

### 4.1 The Request Envelope (Input)
The Agent sends this JSON object to `STDIN`.

```cue
// Defined in schema/protocol.cue
#Request: {
    // The command to execute. Must match a known capability.
    command: "interview.step" | "check" | "quality" | "doctor" | ...

    // Command-specific parameters. Strictly typed.
    params: {
        session_id?: string
        answer?: string
        spec_path?: string
        target_url?: string
    }

    // Context for the next turn. The AI can store arbitrary data here
    // to "remind" itself of things in the next turn.
    context: {
        user_locale?: string
        project_root?: string
        [string]: any
    }

    // Simulation Mode.
    // If true, the CLI calculates the effect but writes NOTHING to disk.
    // Returns a "diff" of what would have happened.
    simulate?: bool | *false
    
    // Output Projection (GraphQL-style).
    // Allows the Agent to save tokens by requesting only specific fields.
    select?: [...string]
}
```

### 4.2 The Response Envelope (Output)
The CLI writes this JSON object to `STDOUT`.

```cue
// Defined in schema/protocol.cue
#Response: {
    // High-level status. "requires_input" means the task is not done.
    status: "ok" | "error" | "requires_input"

    // The primary payload. Structure depends on `command`.
    data: {...}

    // AI-specific metadata for the "Digital Twin"
    metadata: {
        timestamp: string
        duration_ms: int
        version: string
        tokens_used_estimate: int
    }

    // Context for the next turn
    session_id?: string

    // THE MOST IMPORTANT FIELD:
    // Exact JSON payloads the AI should consider sending next.
    // This eliminates "guessing" valid next steps.
    next_actions: [...#Request]

    // Structured Error for AI Self-Repair
    error?: {
        code: string
        message: string
        path?: string
        // Zero-shot repair suggestion.
        fix?: {
            type: "replace_file" | "replace_string" | "run_command"
            file?: string
            content?: string
        }
    }
}
```

---

## 5. Feature Preservation Strategy (The "Crown Jewels")

We are **NOT** removing features. We are refactoring them into **Pure Functions** that are easier for AIs to use.

### 5.1 The Interview Engine (Logic Retention)
The Interview Engine is a directed graph traversal problem.
*   **Current State:** Recursive loop in `interview.gleam`.
*   **New State:** A single atomic transaction.
    *   **Input:** `Current Bead Graph` + `User Answer`.
    *   **Process:**
        1.  Parse the Answer.
        2.  Update the `Bead Graph` nodes (mark answered questions as done).
        3.  Calculate the weights of remaining questions based on **KIRK** priorities (Security > Functionality > Usability).
        4.  Select the highest-weighted next question.
    *   **Output:** `Updated Bead Graph` + `Next Question JSON`.
    *   **Scenario:** An Agent wants to pause the interview to ask the user for clarification. In the old model, this killed the session. In the new model, the Agent just holds the `session_id` and comes back 10 minutes later.

### 5.2 KIRK & Doctor (The "Auto-Fix" Loop)
KIRK is the "conscience" of the system. It enforces the "Brutal Truths" on the user's spec.
*   **New Feature:** `intent doctor` returns **Actionable Payloads**.
    *   **Scenario:** The spec is missing a rate limit.
    *   **Old Output:** "Error: Missing rate limit."
    *   **New Output:**
        ```json
        {
          "error": "Missing rate limit",
          "fix": {
            "type": "replace_string",
            "file": "api.cue",
            "old": "response: { status: 200 }",
            "new": "response: { status: 200, headers: { 'Retry-After': '60' } }"
          }
        }
        ```
    *   **Benefit:** The Agent can blindly apply this fix (after a quick safety check) to "self-heal" the spec.

### 5.3 EARS Parser (Requirements Rigor)
*   **Preserved:** The rigorous EARS (Easy Approach to Requirements Syntax) parsing remains.
*   **Enhancement:** It now accepts raw strings via JSON (`params.requirements_text`) instead of requiring a file on disk. This allows Agents to pass chat messages directly into the parser.

---

## 6. Architecture Refactor Plan

This is the roadmap for the AI Coder to execute. It is broken down into specific tasks to allow for "Chunked" development.

### Phase 1: The Protocol Layer (Foundation)
*   **Task 1.1:** Create `src/intent/protocol.gleam`. Define `Request` and `Response` types with `gleam/json`. Ensure 100% parity with `schema/protocol.cue`.
*   **Task 1.2:** Create `src/intent/dispatcher.gleam`. Implement the `handle_request(req: Request) -> Response` function. This is the new "Main Loop."
*   **Task 1.3:** Create `src/intent/main_headless.gleam`. Implement the `main` function that reads STDIN, calls Dispatcher, and prints JSON.

### Phase 2: Feature Porting (The Migration)
Migrate features one by one. Do not delete old code yet.
*   **Task 2.1:** Port `check` -> `Dispatcher.handle_check`.
*   **Task 2.2:** Port `validate` -> `Dispatcher.handle_validate`.
*   **Task 2.3:** Port `quality` -> `Dispatcher.handle_quality`.
*   **Task 2.4:** Port `doctor` -> `Dispatcher.handle_doctor`. **Critical:** Add the `fix` generation logic here.

### Phase 3: The Interview Transformation
This is the hardest part.
*   **Task 3.1:** Implement `Session` serialization. Ensure `interview.gleam` can export its full internal state to JSON.
*   **Task 3.2:** Implement `interview.next_step(state, answer)`. Refactor the recursive loop into a step function.
*   **Task 3.3:** Test the "Pause/Resume" capability.

### Phase 4: The Great Deletion (Cleanup)
Once Phase 3 is verified, strictly delete:
*   `src/intent/cli_ui.gleam` (Colors/Spinners)
*   `src/intent/help.gleam` (Text Help)
*   `src/intent/progress_dashboard.gleam` (TUI)
*   `glint` dependency.

---

## 7. Testing Strategy (Maximum Assurance)

Since this tool is the "brain" for other AIs, it must be 100% reliable. "It works on my machine" is not acceptable.

### 7.1 Protocol Tests (The "Contract")
*   **Schema Validation:** Every test run must validate the output JSON against `schema/protocol.cue` using the `cue` CLI tool.
*   **Golden Files:** Maintain a set of `input.json` / `output_expected.json` pairs for every command.

### 7.2 Fuzzing (The "Chaos Monkey")
*   **Garbage In:** Feed the dispatcher random bytes, partial JSON, and massive 100MB strings.
*   **Expectation:** The process **MUST NOT CRASH**. It must return `{"status": "error", "code": "JSON_PARSE_ERROR"}`.
*   **Why:** If the CLI crashes, the Agent loses its tool and the session dies. Reliability is paramount.

### 7.3 Simulation Tests (The "Digital Twin")
*   **Mock Interview:** Script a full interview session (Start -> Answer -> Answer -> Finish) purely via JSON messages.
*   **Metric:** Track `session_duration_ms` and `tokens_consumed`.
*   **Scenario:** "Simulate a user who changes their mind halfway through." Send answers that contradict previous answers and ensure the Graph adapts correctly.

---

## 8. The End Game: A Screenplay

When complete, a Human will **never** run `intent` directly.
Instead, the workflow looks like this:

**SCENE: A Developer's Terminal**

**Human:** "I need an API for a Pet Store. It needs to handle high traffic."

**Agent (Claude/Gemini):** *Thinking...*
*   "I need to flesh out these requirements. I'll use the Intent CLI."
*   *Action:* Spawns `./intent` process.
*   *Input:* `{"command": "interview.start", "context": {"topic": "Pet Store"}}`

**Intent CLI:**
*   *Output:* `{"status": "requires_input", "data": {"question": "Define 'high traffic'. How many RPS?"}}`

**Agent:**
*   "The user mentioned high traffic. I should ask them to quantify it."
*   **To Human:** "Could you define what you mean by 'high traffic'? Are we talking 100 RPS or 10,000 RPS?"

**Human:** "About 5,000 RPS."

**Agent:**
*   *Input:* `{"command": "interview.step", "params": {"answer": "5000 RPS"}}`

**Intent CLI:**
*   *Output:* `{"status": "ok", "next_actions": [{"command": "beads.generate"}]}`

**Agent:**
*   *Input:* `{"command": "beads.generate"}`

**Intent CLI:**
*   *Output:* `{"data": {"beads": [{"id": "PERF-01", "task": "Implement Redis Caching", "why": "Required for 5000 RPS"}]}}`

**Agent:**
*   "Okay, I have the plan. I will now start coding."

**FADE OUT.**

This architecture reduces the Agent's cognitive load by offloading the **Process** and **Verification** to the deterministic Intent CLI.
