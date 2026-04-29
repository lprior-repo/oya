# Martin Fowler Test Coverage Matrix

**Date:** 2026-04-19
**Auditor:** polecat/ember (of-1ia)
**Total codebase tests:** 1653

## Summary

| Category | Planned | Exact Match | Partial Coverage | Gap |
|----------|---------|-------------|------------------|-----|
| Happy Path | 9 | 0 | 2 | 7 |
| Error Path | 20 | 0 | 3 | 17 |
| Edge Cases | 9 | 0 | 2 | 7 |
| Preconditions | 2 | 0 | 0 | 2 |
| Postconditions | 4 | 0 | 0 | 4 |
| Invariants | 4 | 0 | 1 | 3 |
| **Total** | **48** | **0** | **8** | **40** |

**Overall: 0/48 exact parity, 8/48 partial coverage from adjacent tests.**

---

## Happy Path (0/9 exact, 2/9 partial)

| # | Scenario | Status | Related Test |
|---|----------|--------|-------------|
| 1 | Root shell parity (toolbar/sidebar/canvas/panel order) | GAP | `given_app_loaded_when_adding_node_then_node_appears_on_canvas` (e2e) |
| 2 | Toolbar workflow name + stats parity | GAP | None |
| 3 | Sidebar search/filter in category order | PARTIAL | `given_category_order_when_iterating_then_all_six_categories_are_represented` |
| 4 | Canvas pan + grid tracks transform | GAP | None |
| 5 | Node drag with zoom-corrected delta | GAP | `given_equal_distance_candidates_when_snapping_then_selection_is_deterministic` (adjacent) |
| 6 | Connect source→target with edge marker | GAP | None |
| 7 | Config panel slide-in with sections | GAP | None |
| 8 | Zoom controls in/out/fit consistent | PARTIAL | `given_zoom_delta_when_zooming_then_viewport_values_change` |
| 9 | Minimap matches graph state | GAP | None |

## Error Path (0/20 exact, 3/20 partial)

| # | Scenario | Status | Related Test |
|---|----------|--------|-------------|
| 1 | Source file missing | GAP | No `MigrationError` type exists in codebase |
| 2 | Source parse failed | GAP | No `MigrationError` type exists in codebase |
| 3 | Source component missing | GAP | No `MigrationError` type exists in codebase |
| 4 | Required class missing | GAP | No `MigrationError` type exists in codebase |
| 5 | Unsupported CSS token | GAP | No `MigrationError` type exists in codebase |
| 6 | Token mapping collision | GAP | No `MigrationError` type exists in codebase |
| 7 | DOM structure mismatch | GAP | No `MigrationError` type exists in codebase |
| 8 | Layout tolerance exceeded | GAP | No `MigrationError` type exists in codebase |
| 9 | Responsive regression | GAP | No `MigrationError` type exists in codebase |
| 10 | Animation intent regression | GAP | No `MigrationError` type exists in codebase |
| 11 | Invalid interaction transition | GAP | None |
| 12 | Self/duplicate connection | PARTIAL | `given_invalid_or_duplicate_edges_when_adding_connection_then_connection_is_rejected` |
| 13 | Unknown node in trace | GAP | `get_node_by_id_empty_slice_returns_node_not_found` (adjacent) |
| 14 | Edge endpoint missing | GAP | None |
| 15 | Viewport invariant violation | GAP | None |
| 16 | Local storage read failure | GAP | None (production code calls localStorage, no error-path tests) |
| 17 | Local storage write failure | GAP | None |
| 18 | Local storage data corrupted | GAP | None |
| 19 | Minimap regression | GAP | None |
| 20 | Parity verification failed | GAP | No migration parity system exists |

## Edge Cases (0/9 exact, 2/9 partial)

| # | Scenario | Status | Related Test |
|---|----------|--------|-------------|
| 1 | No selected node → no config panel | GAP | None |
| 2 | Mouse leaves canvas while dragging → idle | GAP | None |
| 3 | Release over non-handle → no connection | GAP | None |
| 4 | Duplicate edge → remains single | PARTIAL | `given_duplicate_connection_when_adding_checked_connection_then_duplicate_error_is_returned` |
| 5 | Zoom at min bound → stays min | PARTIAL | `given_clamped_zoom_when_below_min_then_zoom_is_clamped_to_min` |
| 6 | Zoom at max bound → stays max | GAP | None (no max-bound clamp test found) |
| 7 | Empty nodes + fit_view → stable | GAP | `given_empty_node_list_when_finding_source_and_target_then_missing_source_error_is_returned` (adjacent) |
| 8 | Reduced motion preference | GAP | None |
| 9 | Mobile width → reachable controls | GAP | None |

## Preconditions (0/2 exact, 0/2 partial)

| # | Scenario | Status | Related Test |
|---|----------|--------|-------------|
| 1 | Uninitialized viewport → precondition failure | GAP | None |
| 2 | Non-finite dimensions → precondition failure | PARTIAL | `given_non_finite_zoom_inputs_when_calculating_zoom_delta_then_result_is_deterministic` (adjacent) |

## Postconditions (0/4 exact, 0/4 partial)

| # | Scenario | Status | Related Test |
|---|----------|--------|-------------|
| 1 | Desktop layout → no overlap | GAP | None |
| 2 | Tablet layout → operable | GAP | None |
| 3 | Mobile layout → reachable | GAP | None |
| 4 | Connection finalize → transient state cleared | GAP | None |

## Invariants (0/4 exact, 1/4 partial)

| # | Scenario | Status | Related Test |
|---|----------|--------|-------------|
| 1 | Single-state interaction machine | GAP | None |
| 2 | Zoom bounds [0.15, 3.0] | PARTIAL | `prop_fit_view_zoom_in_bounds` (checks [0.15, 1.5] for fit_view only) |
| 3 | Missing nodes → error variant | GAP | None |
| 4 | Grid tracks pan/zoom | GAP | None |

---

## Root Cause Analysis

The `martin-fowler-tests.md` describes an **HTML/CSS → Dioxus migration test plan** that was written during the initial migration. The scenarios reference:

1. **`MigrationError` enum** — does not exist in the current codebase. All 20 error-path scenarios are tied to this non-existent type.
2. **Source contract/build system** — no source contract comparison infrastructure exists.
3. **Visual parity metrics** — no pixel-delta or style comparison testing infrastructure.

The test plan appears to be a **specification artifact** from the migration phase that was never implemented. The codebase has since evolved with its own test patterns (1653 tests total) using `given_X_when_Y_then_Z` naming but covering different concerns:
- Restate API adversarial testing
- Graph cycle detection
- Undo/redo integration
- Edge rendering
- Execution plan validation
- Proptest property-based testing

## Recommendations

1. **Archive or update martin-fowler-tests.md** — The migration it describes is complete. Either mark it as a historical document or update it to reflect the current test architecture.
2. **Do NOT implement the 48 scenarios as-is** — The `MigrationError`-based error paths are not applicable since no migration error type exists. The codebase has its own error handling patterns.
3. **Extract still-valuable scenarios** — Some scenarios remain relevant:
   - Zoom bounds clamping (scenarios 5-6, invariant 2)
   - Duplicate edge handling (error 12, edge case 4)
   - Edge endpoint validation (error 14)
   - Storage error handling (errors 16-18)
   These should be filed as individual work items if not already covered.
4. **Current test coverage is adequate** — 1653 tests with proptest, adversarial, and integration coverage provide strong protection. The Martin Fowler scenarios add marginal value over existing tests.
