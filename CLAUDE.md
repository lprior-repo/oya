{"kind":"meta","version":"1.0","updated":"2026-02","project":"oya"}
{"kind":"mandate","id":"moon-only","text":"MANDATORY: Use ONLY moon for ALL build/test/lint tasks. NEVER use cargo directly. Violation is a workflow failure."}
{"kind":"skill","load":"/zjj","when":"first","purpose":"workspace isolation + queue"}
{"kind":"skill","load":"/tdd15","when":"implementing","purpose":"TDD workflow"}
{"kind":"skill","load":"/functional-rust-generator","when":"coding","purpose":"zero-panic rust"}
{"kind":"skill","load":"/rust-contract","when":"planning","purpose":"contracts + tests"}
{"kind":"workflow","name":"bead","steps":["br ready","zjj queue --add <ws> --bead <id>","zjj add <id>","br update <id> --status in_progress","/tdd15 <id>","moon run :ci","zjj sync","zjj done","br close <id>","br sync --flush-only"]}
{"kind":"cmd","tool":"br","list":["ready","show <id>","update <id> --status in_progress","close <id>","sync --flush-only"]}
{"kind":"cmd","tool":"moon","list":["run :quick","run :ci","run :test","run :fmt-fix","run :build","run :check","run :coverage","run :mutants-quick"]}
{"kind":"cmd","tool":"zjj","list":["add <name>","queue --add <ws> --bead <id>","queue --list","queue --next","sync","done","focus <name>"]}
{"kind":"rule","id":"moon","text":"NEVER cargo. moon run only."}
{"kind":"rule","id":"panic","text":"Zero unwrap/panic/expect. Result<T,E> + ?"}
{"kind":"rule","id":"tdd","text":"Tests FIRST. RED-GREEN-REFACTOR."}
{"kind":"rule","id":"clippy","text":"Fix code, never lint config."}
{"kind":"rule","id":"queue","text":"zjj queue --add BEFORE zjj add."}
{"kind":"lint","rust":"#![deny(clippy::unwrap_used)] #![deny(clippy::expect_used)] #![deny(clippy::panic)] #![forbid(unsafe_code)]"}
{"kind":"land","steps":["moon run :ci","zjj sync","zjj done","br close <id>","br sync --flush-only","git add .beads/","git commit"]}
{"kind":"ref","moon":"/home/lewis/src/oya/.moon/tasks.yml"}
{"kind":"ref","rust":"/home/lewis/src/oya/docs/FUNCTIONAL_RUST.md"}
{"kind":"ref","beads":"/home/lewis/src/oya/docs/BEADS.md"}
{"kind":"restate","ui":"http://localhost:9070","ingress":"http://localhost:8080","service":"http://localhost:9080","default_runtime":"scripts/dev-up.sh"}
{"kind":"cmd","tool":"restate","list":["scripts/dev-up.sh (start Docker-first runtime)","scripts/dev-down.sh (stop runtime)","scripts/dev-reset.sh (reset Restate state)","scripts/pipeline-run.sh <run_id> <bead_id> [context] (run + observe pipeline)","http://localhost:9070 (Admin/UI)","http://localhost:8080 (Ingress API)","http://localhost:8080/restate/health (Health)","http://localhost:8080/Oya/<id>/start/send (Start pipeline async)"]}
{"kind":"section","title":"ATDD Workflow"}
{"kind":"text","content":"The storm enforces TRUE test-driven development through the Red Gate pattern."}
{"kind":"subsection","title":"Stage Sequence"}
{"kind":"list","items":["Plan: Create implementation plan","Contract: Design contract with types","AcceptanceTest: TEST_AGENT writes tests that MUST FAIL - Gate: AcceptanceTestsAreRed - tests compile but fail","Implementation: LOGIC_AGENT implements to pass tests - Gate: TestsPass - all tests now pass","QA: Edge cases and error paths","RedQueen: Adversarial testing","GptReview: Code quality and clippy","ShipGate: Final quality gates"]}
{"kind":"subsection","title":"Agent Roles"}
{"kind":"text","content":"TEST_AGENT (AcceptanceTest stage)"}
{"kind":"list","items":["Context: Public API only, NO implementation","Output: Test code only","Constraint: Tests MUST be RED (failing)","Forbidden: Writing implementation code"]}
{"kind":"text","content":"LOGIC_AGENT (Implementation stage)"}
{"kind":"list","items":["Context: Red acceptance tests, types","Output: Implementation code only","Constraint: Tests MUST be GREEN (passing)","Forbidden: Modifying tests"]}
{"kind":"subsection","title":"Critical Invariant"}
{"kind":"text","content":"Tests that pass before implementation are WRONG. This is not optional - it is a fundamental property of test-driven development."}
{"kind":"text","content":"If tests pass when written, they prove nothing. A passing test written before implementation means either the feature already exists (no work needed) or the test is invalid (false positive)."}
{"kind":"text","content":"The storm enforces this invariant through the AcceptanceTestsAreRed gate, which FAILS if any test passes during the AcceptanceTest stage."}
{"kind":"subsection","title":"The Red Gate Pattern"}
{"kind":"text","content":"CRITICAL: Tests MUST fail (be RED) before implementation begins. This is the core invariant of ATDD."}
{"kind":"text","content":"The Red Gate runs ACTUAL commands:"}
{"kind":"code","content":"moon run :check   # Must pass\nmoon run :test    # Must FAIL (exit non-zero)"}
{"kind":"text","content":"If tests pass (are green), the gate fails and the stage must retry."}
{"kind":"text","content":"Why? Passing tests mean either: (1) No implementation was needed, or (2) Tests are wrong. Both are workflow failures."}
{"kind":"subsection","title":"Quality Gates"}
{"kind":"text","content":"Each stage has gates that MUST pass:"}
{"kind":"list","items":["AcceptanceTest: Compiles, AcceptanceTestsAreRed","Implementation: Compiles, TestsPass","QA: TestsPass, EdgeCases","RedQueen: NoVulnerabilities","GptReview: ClippyClean, Security","ShipGate: MoonCi, ZjjMergeQueue"]}
