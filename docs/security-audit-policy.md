# Security Audit Policy

`moon run :security` runs `cargo audit` for each Rust project in the Moon workspace.

The audit task intentionally ignores only the advisory IDs listed below. Any new vulnerability or warning not listed here remains visible and fails the task.

## Accepted Transitive Advisories

| Advisory | Package | Path | Policy |
| --- | --- | --- | --- |
| RUSTSEC-2026-0037 | `quinn-proto 0.11.13` | `oya-frontend -> dioxus-fullstack -> reqwest -> quinn` | Accepted temporarily as a transitive Dioxus dependency until Dioxus/Reqwest updates expose `quinn-proto >=0.11.14`. |
| RUSTSEC-2026-0049 | `rustls-webpki 0.103.9` | `oya-frontend -> dioxus-fullstack -> reqwest -> rustls` | Accepted temporarily as a transitive Dioxus dependency until `rustls-webpki >=0.103.10` is available through the frontend stack. |
| RUSTSEC-2026-0098 | `rustls-webpki 0.103.9` | `oya-frontend -> dioxus-fullstack -> reqwest -> rustls` | Accepted temporarily as a transitive Dioxus dependency until `rustls-webpki >=0.103.12` is available through the frontend stack. |
| RUSTSEC-2026-0099 | `rustls-webpki 0.103.9` | `oya-frontend -> dioxus-fullstack -> reqwest -> rustls` | Accepted temporarily as a transitive Dioxus dependency until `rustls-webpki >=0.103.12` is available through the frontend stack. |
| RUSTSEC-2026-0104 | `rustls-webpki 0.103.9` | `oya-frontend -> dioxus-fullstack -> reqwest -> rustls` | Accepted temporarily as a transitive Dioxus dependency until `rustls-webpki >=0.103.13` is available through the frontend stack. |
| RUSTSEC-2020-0071 | `time 0.1.45` | `oya-frontend -> playwright -> zip` | Accepted temporarily for test-only Playwright tooling. Production code pins modern `time` in the backend workspace. |

## Visible Warnings

The following audit warnings are intentionally not hidden. They currently do not fail `cargo audit`, but they must remain visible in `moon run :security` output:

| Advisory | Package | Current path |
| --- | --- | --- |
| RUSTSEC-2025-0057 | `fxhash 0.2.1` | `oya -> sled` |
| RUSTSEC-2024-0384 | `instant 0.1.13` | `oya -> sled -> parking_lot` |
| RUSTSEC-2024-0436 | `paste 1.0.15` | `oya -> restate-sdk`, `oya -> ratatui`, `oya-frontend -> playwright` |
| RUSTSEC-2026-0002 | `lru 0.12.5` | `oya -> ratatui` |
| RUSTSEC-2025-0134 | `rustls-pemfile 1.0.4` | `oya-frontend -> reqwest 0.11`, `oya-frontend -> playwright` |
| RUSTSEC-2026-0097 | `rand 0.9.2` | `oya-frontend -> dioxus-devtools`, `oya-frontend -> dioxus-fullstack`, `oya-frontend -> proptest` |

## Update Rule

When `moon run :security` fails, either upgrade the dependency tree or add a narrowly documented advisory entry in this file and the exact `cargo audit --ignore` entry in `.moon/tasks/all.yml`.
