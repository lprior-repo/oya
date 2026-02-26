{"kind":"meta","version":"1.0","updated":"2026-02","project":"oya"}
{"kind":"mandate","id":"moon-only","text":"MANDATORY: Use ONLY moon for ALL build/test/lint tasks. NEVER use cargo directly. Violation is a workflow failure."}
{"kind":"mandate","id":"codanna-only-discovery","text":"MANDATORY: Use ONLY Codanna MCP for code discovery (symbols, callers, calls, impact, dependency tracing)."}
{"kind":"rule","id":"no-glob-read-grep-explore","text":"FORBIDDEN for exploration: glob/read/grep/find/rg. Use only after Codanna identifies exact path/symbol, or for non-indexed artifacts."}
{"kind":"rule","id":"exploration-ladder","text":"Use this order: codanna_search_symbols -> codanna_find_symbol -> codanna_analyze_impact -> targeted read."}
{"kind":"rule","id":"cheap-defaults","text":"Default limits: search limit <= 5, impact depth <= 2, single-pass batched queries, no repeated re-query of same symbol."}
{"kind":"rule","id":"response-budget","text":"Default response budget: <= 8 lines, no redundant recap, no chain-of-thought, include only decision/files/next-action."}
{"kind":"policy","id":"non-codanna-explore-failure","text":"Exploration with glob/read/grep before a Codanna attempt is a workflow failure unless user explicitly requests it."}
{"kind":"cmd","tool":"codanna_mcp","list":["codanna_search_symbols","codanna_find_symbol","codanna_get_calls","codanna_find_callers","codanna_analyze_impact","codanna_semantic_search_with_context","codanna_get_index_info"]}
{"kind":"skill","load":"/jj","when":"first","purpose":"workspace isolation"}
{"kind":"skill","load":"/functional-rust-generator","when":"coding","purpose":"zero-panic rust"}
{"kind":"skill","load":"/rust-contract","when":"planning","purpose":"contracts + tests"}
{"kind":"workflow","name":"bead","steps":["br ready","jj workspace add ../<id> --name <id>","br update <id> --status in_progress","oya lifecycle --bead <id> --repo <owner/repo>","moon run :ci","jj git fetch","jj rebase -d main@origin","jj bookmark set <name> -r @","jj git push --bookmark <name>","jj workspace forget <name>","br close <id>","br sync --flush-only"]}
{"kind":"cmd","tool":"br","list":["ready","show <id>","update <id> --status in_progress","close <id>","sync --flush-only"]}
{"kind":"cmd","tool":"moon","list":["run :quick","run :ci","run :test","run :fmt-fix","run :build","run :check","run :coverage","run :mutants-quick"]}
{"kind":"cmd","tool":"jj","list":["workspace add <destination> [--name <name>]","workspace forget <name>","git fetch","rebase -d <revision>","bookmark set <name> -r <revision>","bookmark list","log"]}
{"kind":"rule","id":"moon","text":"NEVER cargo. moon run only."}
{"kind":"rule","id":"panic","text":"Zero unwrap/panic/expect. Result<T,E> + ?"}
{"kind":"rule","id":"tdd","text":"Tests FIRST. RED-GREEN-REFACTOR."}
{"kind":"rule","id":"clippy","text":"Fix code, never lint config."}
{"kind":"rule","id":"fn-lines","text":"Source functions must be <= 40 lines (clippy::too_many_lines)."}
{"kind":"rule","id":"fn-args","text":"Source functions must take <= 5 inputs (clippy::too_many_arguments)."}
{"kind":"rule","id":"workspace","text":"jj workspace add before starting work."}
{"kind":"lint","rust":"#![deny(clippy::unwrap_used)] #![deny(clippy::expect_used)] #![deny(clippy::panic)] #![deny(clippy::too_many_lines)] #![deny(clippy::too_many_arguments)] #![forbid(unsafe_code)]"}
{"kind":"land","steps":["moon run :ci","jj git fetch","jj rebase -d main@origin","jj bookmark set <name> -r @","jj git push --bookmark <name>","jj workspace forget <name>","br close <id>","br sync --flush-only","git add .beads/","git commit"]}
{"kind":"ref","moon":"/home/lewis/src/oya/.moon/tasks.yml"}
{"kind":"ref","rust":"/home/lewis/src/oya/docs/FUNCTIONAL_RUST.md"}
{"kind":"ref","beads":"/home/lewis/src/oya/docs/BEADS.md"}
{"kind":"restate","ui":"http://localhost:9070","ingress":"http://localhost:909","service":"http://localhost:9180","default_runtime":"oya init"}
{"kind":"rule","id":"runtime-init","text":"Please use `oya init` to bootstrap local runtime. Use `oya init --down` to stop Docker Restate."}
{"kind":"cmd","tool":"restate","list":["oya init (fresh Docker Restate + handler registration + validations)","oya init --down (stop Docker Restate)","http://localhost:9070 (Admin/UI)","http://localhost:909 (Ingress API)","http://localhost:909/restate/health (Health)","http://localhost:9180/discover (Oya discovery endpoint)","http://localhost:909/Oya/<key>/run (workflow run endpoint)","http://localhost:909/OyaService/get_lifecycle (status endpoint)","http://localhost:909/OyaService/cancel (cancel endpoint)","http://localhost:909/OyaMemory/<id>/start (memory start endpoint)","http://localhost:909/OyaMemory/<id>/run_pipeline (memory pipeline endpoint)"]}
{"kind":"observability","name":"OpenObserve","ui":"http://localhost:5080","otlp_grpc":"localhost:4317","otlp_http":"http://localhost:4318","credentials":"~/.local/share/observability/.env"}
{"kind":"cmd","tool":"observability","list":["systemctl --user start observability.service (start stack)","systemctl --user stop observability.service (stop stack)","systemctl --user status observability.service (check status)","~/.local/share/observability/observability.sh start (alt start)","~/.local/share/observability/observability.sh stop (alt stop)","~/.local/share/observability/observability.sh logs [service] (view logs)","~/.local/share/observability/observability.sh ui (open browser)","~/.local/share/observability/observability.sh creds (show credentials)"]}
{"kind":"env","name":"OTEL_EXPORTER_OTLP_ENDPOINT","value":"http://localhost:4318","purpose":"OTLP exporter endpoint for traces/metrics/logs"}
{"kind":"env","name":"OTEL_SERVICE_NAME","value":"oya-orchestrator","purpose":"Service name in OpenObserve"}
{"kind":"ref","observability":"~/.local/share/observability/README.md"}
{"kind":"rule","id":"clippy-test-exemption","text":"Test files (tests/*.rs, src/lib_tests.rs) are EXEMPT from clippy::unwrap_used for brevity in assertions. CLIPPY task excludes tests."}
