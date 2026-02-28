---
phase: 138-testing-framework
plan: 04
subsystem: testing
tags: [mesh, test-runner, preprocessor, tokenizer, describe, setup, teardown]

# Dependency graph
requires:
  - phase: 138-testing-framework-plan-03
    provides: preprocess_test_source, tokenize_test_source, extract_test_blocks, emit_non_test_items, mesh_test_begin/pass/fail/summary builtins
provides:
  - Fixed emit_non_test_items using token-based depth tracking (no more count_do_in_line/count_end_in_line)
  - Fixed extract_tests_from_describe helper that correctly handles setup/teardown sub-blocks in describe bodies
  - New tests/e2e/test_setup_teardown.test.mpl fixture (5 tests in 2 describe groups + 1 top-level)
  - TEST-07 unblocked: shared setup/teardown for describe blocks now compiles and runs
affects: [139-package-manifest, 140-registry]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Token-based depth tracking in emit_non_test_items: skipping=true sets pending-skip state, skip_depth tracks nested do/end depth, zero means skip complete"
    - "extract_tests_from_describe helper: walks describe body with explicit setup/teardown skip logic, avoiding premature End detection"

key-files:
  created:
    - tests/e2e/test_setup_teardown.test.mpl
  modified:
    - compiler/meshc/src/test_runner.rs

key-decisions:
  - "emit_non_test_items rewrites to use tokenize_test_source() instead of character-level line scanning — token-based approach correctly handles nested setup/teardown sub-blocks inside describe blocks"
  - "extract_blocks_at uses new extract_tests_from_describe helper for describe bodies — avoids premature return when hitting setup/teardown End tokens"
  - "skip_depth=1 set when Do is encountered while skipping=true (after TestKw/DescribeKw), not on the keyword itself — allows skipping the label and parens before the opening Do"

patterns-established:
  - "Token-level skipping with two-phase state (skipping bool + skip_depth usize) for suppressing entire test/describe subtrees"

requirements-completed: [TEST-07]

# Metrics
duration: 3min
completed: 2026-02-28
---

# Phase 138 Plan 04: Fix describe+setup/teardown depth-tracking bugs Summary

**Token-based rewrite of emit_non_test_items and new extract_tests_from_describe helper that correctly suppress and extract setup/teardown sub-blocks inside describe groups**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-02-28T23:10:44Z
- **Completed:** 2026-02-28T23:13:51Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Replaced line-level character scanning (`count_do_in_line`/`count_end_in_line`) in `emit_non_test_items` with a clean token-based implementation using the existing `tokenize_test_source()` lexer — fixes the core describe+setup/teardown emit bug
- Discovered and fixed a second pre-existing extraction bug in `extract_blocks_at`: when it recursed into describe bodies, setup/teardown `End` tokens caused premature return, silently dropping all tests in describe groups with setup/teardown
- Added `extract_tests_from_describe` helper function that walks describe bodies while explicitly skipping setup/teardown sub-blocks
- Created `tests/e2e/test_setup_teardown.test.mpl` with 5 tests across 2 describe groups (each with setup and teardown blocks) plus 1 top-level test — all pass
- All 5 existing e2e test fixtures continue to pass (no regressions)

## Task Commits

Each task was committed atomically:

1. **Task 1: Rewrite emit_non_test_items using the token-based lexer** - `f46eff4b` (fix)
2. **Task 2: Add test_setup_teardown.test.mpl fixture and verify end-to-end** - `94571f01` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `compiler/meshc/src/test_runner.rs` - Replaced emit_non_test_items with token-based approach; added extract_tests_from_describe helper; removed count_do_in_line/count_end_in_line
- `tests/e2e/test_setup_teardown.test.mpl` - New e2e fixture: 2 describe groups each with setup+teardown sub-blocks, 4 tests inside describes, 1 top-level test

## Decisions Made
- Token-based depth tracking with two-phase state (`skipping: bool` + `skip_depth: usize`) cleanly separates "waiting for opening Do" from "counting nested block depth"
- `extract_tests_from_describe` introduced as a targeted helper rather than modifying `extract_blocks_at`'s recursion contract — minimizes change surface and avoids breaking existing describe-without-setup behavior
- `skip_depth` is incremented when `Do` is seen while `skipping=true`, not immediately when `TestKw`/`DescribeKw` is seen — this correctly skips the label string and parentheses between the keyword and its block

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed premature End detection in extract_blocks_at for describe+setup/teardown**
- **Found during:** Task 2 (fixture creation and verification)
- **Issue:** `extract_blocks_at` when recursing into a describe body would encounter the `End` token of a `setup do...end` or `teardown do...end` sub-block and interpret it as the end of the describe block, returning early and silently dropping all remaining tests in that describe group
- **Fix:** Added `extract_tests_from_describe` helper that walks describe bodies with explicit skip logic for setup/teardown sub-blocks (using the same two-phase `skip_depth` approach), called from `extract_blocks_at`'s `DescribeKw` branch
- **Files modified:** compiler/meshc/src/test_runner.rs
- **Verification:** `meshc test test_setup_teardown.test.mpl` exits 0 with all 5 tests passing; all existing fixtures pass
- **Committed in:** `94571f01` (part of Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug — Rule 1)
**Impact on plan:** Auto-fix essential for correctness — without it, no tests inside describe+setup groups would run. No scope creep.

## Issues Encountered
- First run of the setup/teardown fixture showed only 1 test passing (the top-level one) instead of expected 5 — triggered investigation of `extract_blocks_at` logic which revealed the premature End detection bug

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- TEST-07 fully unblocked: describe blocks with setup/teardown compile and all tests run
- All 5 e2e test fixtures pass cleanly
- Phase 139 (Package Manifest & meshpkg CLI) can proceed

---
*Phase: 138-testing-framework*
*Completed: 2026-02-28*

## Self-Check: PASSED

- FOUND: tests/e2e/test_setup_teardown.test.mpl
- FOUND: .planning/phases/138-testing-framework/138-04-SUMMARY.md
- FOUND commit f46eff4b (Task 1)
- FOUND commit 94571f01 (Task 2)
- OK: count_do_in_line removed from test_runner.rs
- OK: count_end_in_line removed from test_runner.rs
- OK: emit_non_test_items exists and uses tokenize_test_source
