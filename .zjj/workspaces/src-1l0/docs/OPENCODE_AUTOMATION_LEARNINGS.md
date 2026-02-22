# OpenCode Automation Learnings

## Purpose

Capture what was validated while prototyping a governance-first orchestration loop against OpenCode, with emphasis on practical API behavior, reliability traps, and what to standardize next.

## Prototype Scope

- Prototype script: `oya/spikes/go-template-cli/orchestrator.nu`
- Bead used: "Make me a basic Golang CLI for templating Backstage repos"
- Pipeline shape used during experimentation:
  - `contract`
  - `tdd15`
  - `qa`
  - `red_queen`
  - `gpt_review`
  - `ship_gate`
- Retry lane on failure: return to `tdd15`

## Confirmed OpenCode API Behaviors

1. **Model routing requires nested model object**
   - For `/session/{id}/message` and `/session/{id}/prompt_async`, model must be passed as:
   - `"model": { "providerID": "...", "modelID": "..." }`
   - Top-level provider/model fields are not reliable for API routing.

2. **`/session/status` is sparse**
   - Active sessions appear as keys with `{ "type": "busy" }`.
   - Idle sessions are not listed.
   - Missing key must be treated as idle.

3. **SSE is production-usable for orchestration**
   - Useful events observed: `session.status`, `session.idle`, `message.updated`, `message.part.updated`, `session.diff`.
   - SSE can replace tight polling loops once orchestration state is durable.

4. **Message payload shape is consistent for automation**
   - Assistant output is in `parts[]`, where `type == "text"` is the contract payload.
   - `info.tokens` and `info.time.completed` are usable for cost/completion tracking.

5. **Async calls can orphan work**
   - `prompt_async` continues server-side if orchestrator process dies.
   - Explicit abort (`/session/{id}/abort`) is required during timeout recovery.

## What Worked

- End-to-end governed loops reached `ship_gate` in repeated runs.
- Retry behavior correctly moved failed QA/review paths back to tdd15.
- Evidence enforcement prevented silent, low-quality pass-through.
- Stuck detection stopped repeated low-value loops.

## What Failed (and Why)

1. **Max-attempt exhaustion**
   - Some runs failed due to repeated QA/reliability concerns.
   - This is expected and preferable to false-positive success.

2. **Missing text part edge case**
   - Some assistant messages lacked a `text` part.
   - Guardrail added to classify and fail with explicit reason.

3. **Timeout orchestration mismatch**
   - External runner timeout killed orchestrator before async stage completion.
   - Mitigation: use sync `/message` for simpler prototype reliability.

## Governance Rules Added in Prototype

- Strict JSON output contract per stage.
- Evidence required for `qa`, `red_queen`, `gpt_review`.
- Evidence must include command entries with `cmd`, `exit_code`, and output text.
- Placeholder evidence (`not run`, `todo`, `n/a`) treated as invalid.
- Repeated same failure in tdd15 lane triggers `stuck_repeated_failure`.

## Context Strategy Decision

Current experiment mode sends:

- Full contract JSON context
- Full prior artifacts as JSONL in ephemeral prompt context

Rationale: maximize stage awareness while prototyping behavior.

Note: this improves continuity but increases token usage; production mode should support compact context packets.

## Recommended Defaults Going Forward

1. Use sync `/message` by default for prototype stability.
2. Keep async mode only when durable resume/reconcile state is implemented.
3. Persist each stage artifact with stable run/stage/attempt IDs.
4. Require hard evidence before QA/Review pass.
5. Add explicit `main_unhealthy` gate before admitting new runs.

## Immediate Next Steps

1. Add durable run store in Sled for resume after process death.
2. Wire real gate commands into evidence contract (`go test`, `moon run :quick`, etc.).
3. Add stage-specific timeout budgets and backoff policy.
4. Add one-click run report output per run ID.

## Useful Debug Commands

```bash
opencode debug config
opencode models --verbose
curl -u opencode:$PASSWORD http://127.0.0.1:4097/global/health
curl -u opencode:$PASSWORD http://127.0.0.1:4097/session/status
curl -u opencode:$PASSWORD http://127.0.0.1:4097/permission
curl -u opencode:$PASSWORD http://127.0.0.1:4097/question
curl -u opencode:$PASSWORD -N http://127.0.0.1:4097/event
```
