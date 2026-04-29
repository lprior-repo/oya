# Frontend Test Summary

This file records the current release-facing frontend gates. Older red-phase
cycle-detection notes were superseded by the committed test suite and should not
be read as current release status.

## Current Verified Gates

- `moon run frontend:e2e` passes the Playwright browser suite: 8/8 tests.
- `moon run frontend:build-web` builds the Dioxus web bundle in release mode.
- `moon run frontend:ci` passes the frontend unit/integration/lint gate.
- `moon run :ci --force` passes the full workspace gate.

## Browser Coverage

The E2E suite covers:

- Core editor shell load.
- Sidebar search/filtering.
- Context-menu palette open/close.
- Node add/select/delete.
- Node drag visibility.
- Extend-flow panel actions.
- Seeded adversarial interaction loop invariants.

## Command Policy

Run all frontend gates through Moon only. Do not invoke `dx`, `cargo`, or `npm`
directly for release verification.
