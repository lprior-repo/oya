# Agent Instructions: Autonomous Development Loop

```jsonl
{"kind":"meta","project":"oya-frontend","version":"1.0.0","updated":"2026-02","format":"markdown-with-embedded-jsonl"}
{"kind":"workflow","name":"core_loop","steps":["bd ready --json → bd update <bead-id> --claim","git switch -c <branch>","tdd15 + red-queen + functional skills","moon run :ci --force","git push -u origin HEAD:<branch>"]}
{"kind":"build_system","tool":"moon","allowed":[":serve",":check",":test",":clippy",":fmt",":build-web",":ci --force"],"forbidden":["dx directly","cargo directly","npm directly"]}
{"kind":"framework","name":"dioxus","version":"0.7","state":"use_signal (atomic), use_store (nested)","props":"ReadOnlySignal<T>","styling":"Tailwind class: attribute","closures":"move |_|","assets":"asset!() macro","syntax":"rsx! only, no HTML tags"}
{"kind":"code_intelligence","tool":"codanna","required":"mcp__codanna__*","forbidden":["grep","rg","ripgrep","find","cat for search"],"workflow":["semantic_search_with_context → anchor","find_symbol/search_symbols → lock","get_calls/find_callers/analyze_impact → hints","Read tool → confirm"]}
{"kind":"zero_policy","rules":["no .unwrap() or .expect()","no panic!() or unsafe","Result<T,Error> with combinators"]}
{"kind":"ui_design","approach":"v0","principles":["high-quality Tailwind","ARIA accessibility","keyboard navigation","flat RSX trees","#[component] functions"]}
{"kind":"landing","requirements":["moon run :ci --force passes","bd close executed","git push succeeds"]}
{"kind":"tool","name":"semantic_search_with_context","purpose":"Search by concept with full context","when":"Start here for exploration"}
{"kind":"tool","name":"find_symbol","purpose":"Exact symbol lookup","when":"Know the exact name"}
{"kind":"tool","name":"search_symbols","purpose":"Fuzzy text search","when":"Partial name matches"}
{"kind":"tool","name":"get_calls","purpose":"What function calls","when":"Call graph outbound"}
{"kind":"tool","name":"find_callers","purpose":"What calls function","when":"Call graph inbound"}
{"kind":"tool","name":"analyze_impact","purpose":"Full dependency graph","when":"Change impact analysis"}
{"kind":"tool","name":"search_documents","purpose":"Search markdown/docs","when":"Find project docs"}
{"kind":"tool","name":"get_index_info","purpose":"Index statistics","when":"Verify index health"}
```

## Quick Reference

| Area | Rule |
|------|------|
| **Triage** | `bd ready --json` → `bd update <id> --claim` → Git branch/worktree isolation if needed |
| **Build** | `moon run :ci --force` (mandatory `--force`) |
| **State** | `use_signal` / `use_store`, NEVER `use_state` |
| **Code Search** | Codanna MCP tools ONLY, no grep/rg/find |
| **Errors** | `Result<T, Error>` with combinators, no unwrap |

---

## Restate Integration

The frontend connects to Restate via two HTTP layers:

| Port | Purpose | Key Files |
|------|---------|-----------|
| **9070** | Admin API (SQL queries, invocation control) | `src/restate_client/client.rs`, `src/restate_sync/poller.rs` |
| **8080** | Ingress API (service invocation) | `src/graph/execution_runtime/service_calls.rs` |

### Starting Restate

```bash
# Install Restate v1.6.2 (if not present)
curl -L "https://restate.gateway.scarf.sh/v1.6.2/restate-server-x86_64-unknown-linux-musl.tar.xz" -o /tmp/restate.tar.xz
tar -xJf /tmp/restate.tar.xz -C /tmp
cp /tmp/restate-server-x86_64-unknown-linux-musl/restate-server ~/bin/

# Start managed Oya/Restate runtime from the workspace root
moon run :run -- init

# Verify it's running
curl -s http://localhost:9070/deployments  # {"deployments":[]}
curl -s http://localhost:8080/restate/health
```

### Admin API (Port 9070)

All admin API calls need `Accept: application/json` and `Content-Type: application/json` headers.

```bash
# SQL query against sys tables
curl -s -X POST http://localhost:9070/query \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{"query": "SELECT * FROM sys_invocation LIMIT 5"}'

# List services/deployments
curl -s http://localhost:9070/services -H "Accept: application/json"
curl -s http://localhost:9070/deployments -H "Accept: application/json"
```

### Frontend Sync Architecture

```
Restate (:9070) → POST /query → InvocationPoller → use_restate_sync() → UI Panels
```

### Key Implementation Files

| File | Role |
|------|------|
| `src/restate_client/client.rs` | HTTP client for Admin API |
| `src/restate_client/queries.rs` | Pre-built SQL queries |
| `src/restate_sync/poller.rs` | Poll-based state machine |
| `src/hooks/use_restate_sync.rs` | Dioxus signal bridge |
| `src/graph/execution_runtime/service_calls.rs` | Ingress service calls |

### Test Status

- **Unit/Integration Tests**: ✅ Frontend CI passes through `moon run frontend:ci`
- **E2E Browser Tests**: ✅ Playwright flow editor suite passes through `moon run frontend:e2e`
- **Build**: ✅ Production web bundle passes through `moon run frontend:build-web`
- **Dioxus Version**: ✅ 0.7
