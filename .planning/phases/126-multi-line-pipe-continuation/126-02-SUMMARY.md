---
phase: 126-multi-line-pipe-continuation
plan: 02
subsystem: e2e-tests
tags: [e2e, pipe, multi-line, fixture, PIPE-01, PIPE-02]
dependency_graph:
  requires: [126-01]
  provides: [PIPE-01-e2e, PIPE-02-equivalence]
  affects: [compiler/meshc/tests/e2e.rs, tests/e2e/]
tech_stack:
  added: []
  patterns: [read_fixture, compile_and_run, equivalence-assertion]
key_files:
  created:
    - tests/e2e/pipe_multiline_trailing.mpl
    - tests/e2e/pipe_multiline_slot.mpl
  modified:
    - compiler/meshc/tests/e2e.rs
key_decisions:
  - Named regression test e2e_pipe_126_regression (not e2e_pipe_regression_single_line) because e2e_pipe already exists with identical logic
metrics:
  duration: 3m
  completed: "2026-02-27"
  tasks_completed: 2
  files_modified: 3
requirements_satisfied:
  - PIPE-01
  - PIPE-02
---

# Phase 126 Plan 02: Multi-line Pipe Continuation (E2E Tests) Summary

E2E fixtures and test functions proving all four multi-line pipe forms produce correct output through the full compiler pipeline, with explicit byte-for-byte equivalence assertions for PIPE-02.

## What Was Built

Two fixture files and five E2E test functions covering every multi-line pipe variant introduced in Phase 126, plus explicit equivalence verification that multi-line forms produce identical output to their single-line counterparts.

### Fixture Files

**tests/e2e/pipe_multiline_trailing.mpl** — Tests the trailing-pipe form with a 3-stage chain plus a mixed trailing/leading chain:
- `a`: `5 |> double |> add_one` (trailing form) = 11
- `b`: `10 |> add(5) |> double` (trailing then leading) = 30
- Expected output: `"11\n30\n"`

**tests/e2e/pipe_multiline_slot.mpl** — Tests multi-line slot pipe in both leading and trailing forms:
- `a`: `5 |2> add(10)` (leading form) = 15
- `b`: `5 |2> add(10)` (trailing form) = 15 (equivalence proof: `a == b`)
- `c`: `3 |2> add(2) |2> mul(4)` (multi-stage trailing) = 20
- Expected output: `"15\n15\n20\n"`

### Test Functions (5 new)

| Test | Purpose | Expected |
|------|---------|----------|
| `e2e_pipe_multiline_trailing` | PIPE-01: trailing `|>` fixture | `"11\n30\n"` |
| `e2e_pipe_multiline_slot` | PIPE-01: trailing `|N>` fixture | `"15\n15\n20\n"` |
| `e2e_pipe_multiline_equivalence` | PIPE-02: single-line == multi-line `|>` | identical output |
| `e2e_pipe_slot_multiline_equivalence` | PIPE-02: single-line == multi-line `|N>` | identical output |
| `e2e_pipe_126_regression` | Regression: existing `pipe.mpl` unchanged | `"11\n"` |

## Verification

- Full pipe test suite: 24 passed, 0 failed
- Parser tests: 237 passed, 0 failed
- All four PIPE-01 forms covered (trailing-`|>`, leading-`|>`, trailing-`|N>`, leading-`|N>`)
- PIPE-02 explicitly verified by `assert_eq!(single_line, multi_line, ...)` with message

## Deviations from Plan

**1. [Rule 1 - Minor] Named regression test e2e_pipe_126_regression instead of e2e_pipe_regression_single_line**
- **Found during:** Task 2
- **Issue:** Plan noted to use `e2e_pipe_126_regression` if `e2e_pipe` already existed — it does (line 180)
- **Fix:** Used the plan's specified fallback name
- **Files modified:** compiler/meshc/tests/e2e.rs

## Self-Check: PASSED

Files verified:
- tests/e2e/pipe_multiline_trailing.mpl: FOUND
- tests/e2e/pipe_multiline_slot.mpl: FOUND
- compiler/meshc/tests/e2e.rs: FOUND (with 5 new test functions)

Commits verified:
- 459ba641 (Task 1 - fixture files)
- e9276cfb (Task 2 - E2E test functions)
