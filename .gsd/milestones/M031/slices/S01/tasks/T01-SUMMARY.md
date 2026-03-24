---
id: T01
parent: S01
milestone: M031
provides:
  - suppress_trailing_closure flag in Parser struct
  - Trailing-closure guard in Pratt postfix loop
  - Control-flow condition sites set/restore flag
  - E2e tests for all 4 control-flow fn-call patterns plus trailing-closure regression
key_files:
  - compiler/mesh-parser/src/parser/mod.rs
  - compiler/mesh-parser/src/parser/expressions.rs
  - compiler/meshc/tests/e2e.rs
key_decisions:
  - Used save/restore pattern for suppress_trailing_closure to handle nested expressions correctly
  - Trailing closure regression test uses meshc test with test()/describe() syntax since fn() type annotations are not supported in Mesh parameter positions
patterns_established:
  - Parser flag save/restore around expr(p) calls in control-flow parsers
observability_surfaces:
  - Parse errors at the DO_KW position indicate trailing-closure disambiguation failure
duration: 25m
verification_result: passed
completed_at: 2026-03-24
blocker_discovered: false
---

# T01: Fix trailing-closure disambiguation in control-flow conditions

**Added suppress_trailing_closure flag to parser so `if fn_call() do`, `while fn_call() do`, `case fn_call() do`, and `for x in fn_call() do` correctly treat `do` as the block opener instead of a trailing closure**

## What Happened

The Pratt parser's postfix loop eagerly parsed a trailing closure when it saw `DO_KW` after a call expression's `)`. This broke `if fn_call() do ... end` and the other three control-flow forms where `do` is the block opener, not a trailing closure.

Added a `suppress_trailing_closure: bool` field to the `Parser` struct, initialized to `false`. The trailing-closure parse at the postfix `DO_KW` check is now guarded by `!p.suppress_trailing_closure()`. Each of the 4 control-flow parsers (`parse_if_expr`, `parse_while_expr`, `parse_case_expr`, `parse_for_in_expr`) saves the flag, sets it to `true` before `expr(p)`, and restores it afterward.

Added 5 e2e tests: one for each control-flow form with a function call in the condition/scrutinee/iterable, plus a regression test using `meshc test` with `test()` and `describe()` trailing closures.

## Verification

- `cargo test -p meshc --test e2e trailing_closure` — 5/5 pass
- `cargo test -p mesh-parser --lib` — 17/17 pass
- `cargo test -p meshc --test e2e` — 298/308 pass (10 failures are pre-existing `try_*` tests, confirmed by checking on clean main)
- `cargo run -p meshc -- build reference-backend` — success
- `cargo run -p meshc -- build mesher` — success

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test -p meshc --test e2e trailing_closure` | 0 | ✅ pass | 14.9s |
| 2 | `cargo test -p mesh-parser --lib` | 0 | ✅ pass | 8.6s |
| 3 | `cargo test -p meshc --test e2e` | 101 | ✅ pass (10 pre-existing failures, 0 regressions) | 213.5s |
| 4 | `cargo run -p meshc -- build reference-backend` | 0 | ✅ pass | 25.0s |
| 5 | `cargo run -p meshc -- build mesher` | 0 | ✅ pass | 29.3s |

## Diagnostics

When `if fn_call() do` or similar patterns fail to parse, the error will appear at the `do` token position as "expected expression" (the parser tried to continue past the call site without consuming `do`). The fix makes these parse correctly instead.

## Deviations

- Test source code initially used `:` for type annotations instead of `::` (Mesh's actual syntax). Fixed to match Mesh idiom.
- `while` test simplified from mutable-counter to `always_false()` since Mesh has no mutable assignment.
- Trailing-closure regression test uses `meshc test` with `test()/describe()` trailing closures instead of a user-defined higher-order function, because `fn()` type annotations are not supported in Mesh parameter positions.

## Known Issues

- `fn()` type annotations (e.g., `f :: fn() -> String`) in function parameter positions are not supported by the type parser — this is a pre-existing language limitation, not related to this fix.
- 10 pre-existing `try_*`/`from_try_*` e2e test failures (runtime crashes with exit code None) exist on clean main.

## Files Created/Modified

- `compiler/mesh-parser/src/parser/mod.rs` — Added `suppress_trailing_closure: bool` field and getter to Parser struct
- `compiler/mesh-parser/src/parser/expressions.rs` — Guarded trailing-closure parse with `!p.suppress_trailing_closure()`; set/restore flag in 4 control-flow parsers
- `compiler/meshc/tests/e2e.rs` — Added 5 e2e tests: if/while/case/for with fn-call conditions, plus trailing-closure regression
