# Mutation Testing Policy

`moon run :mutants-quick` is the explicit hardening ratchet for mutation testing.
It builds a Git diff file against `main` and runs
`cargo mutants --in-diff target/mutants-quick.diff --baseline run`, so local
operators can exercise mutation testing on changed Rust code without turning it
into part of the fast CI baseline.

Policy:

- Keep `mutants-quick` opt-in with `runInCI: false`.
- Keep `root-ci` and `quick` free of mutation-testing dependencies.
- Use `moon run :mutants-quick` before high-risk PRs or after test-suite changes.
- Use `moon run :mutants` only for full, intentionally slow mutation sweeps.
