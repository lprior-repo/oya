{"kind":"meta","version":"1.0","updated":"2026-02","project":"oya"}
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
