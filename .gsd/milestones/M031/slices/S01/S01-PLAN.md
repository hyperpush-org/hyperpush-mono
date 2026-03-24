# S01: Parser & Codegen Fixes

**Goal:** Fix three compiler bugs that block idiomatic Mesh code: trailing-closure disambiguation in control-flow conditions, `else if` chain value correctness, and multiline function call type resolution.
**Demo:** `if is_big(15) do ... end` compiles and runs; `else if` chains return correct Int/String/Bool values; multiline fn calls resolve correct return types. All proven by new e2e tests; all 303 existing tests still pass.

## Must-Haves

- `if fn_call() do`, `while fn_call() do`, `case fn_call() do`, `for x in fn_call() do` all parse and compile correctly
- `test("name") do ... end` and `describe("name") do ... end` still parse as trailing closures (regression gate)
- `else if` chains return correct values for Int, String, and Bool return types — no crashes, no garbage
- Multiline function calls (args on separate lines) resolve to the correct return type, not `()`
- All 303 existing e2e tests pass after all three fixes
- New e2e tests cover every fixed pattern

## Proof Level

- This slice proves: contract
- Real runtime required: yes (compiled Mesh binaries run and produce expected stdout)
- Human/UAT required: no

## Verification

```bash
# All existing tests pass (no regressions):
cargo test -p meshc --test e2e                    # 303+ tests
cargo test -p mesh-parser --lib                   # parser unit tests

# New e2e tests pass (added by this slice):
cargo test -p meshc --test e2e trailing_closure    # T01 regression tests
cargo test -p meshc --test e2e else_if             # T02 regression tests
cargo test -p meshc --test e2e multiline_call      # T03 regression tests

# Dogfood builds still work:
cargo run -p meshc -- build reference-backend
cargo run -p meshc -- build mesher
```

## Tasks

- [x] **T01: Fix trailing-closure disambiguation in control-flow conditions** `est:1h`
  - Why: R016 — `if fn_call() do` is misparsed because the Pratt parser's postfix loop eagerly treats `do` after `)` as a trailing closure, even inside control-flow conditions where `do` is the block opener.
  - Files: `compiler/mesh-parser/src/parser/mod.rs`, `compiler/mesh-parser/src/parser/expressions.rs`, `compiler/meshc/tests/e2e.rs`
  - Do: Add `suppress_trailing_closure: bool` to `Parser` struct. Set it `true` before `expr(p)` in the 4 control-flow condition sites (`parse_if_expr`, `parse_while_expr`, `parse_case_expr`, `parse_for_in_expr`), restore after. Guard the `DO_KW` trailing-closure check at line 111 of `expressions.rs` with `!p.suppress_trailing_closure`. Add e2e tests for all 4 control-flow forms and regression tests for `test("name") do ... end`.
  - Verify: `cargo test -p meshc --test e2e` passes 303+ tests; `cargo test -p mesh-parser --lib` passes; new trailing-closure tests pass
  - Done when: `if is_big(15) do ... end` compiles and runs correctly; `test("name") do ... end` still works

- [ ] **T02: Fix `else if` chain codegen to produce correct branch values** `est:45m`
  - Why: R015 — `else if` chains return garbage values or crash because `infer_if`'s recursive call bypasses `infer_expr`, so the inner if-expression's type is never stored in the `types` map. Codegen falls back to `MirType::Unit`.
  - Files: `compiler/mesh-typeck/src/infer.rs`, `compiler/meshc/tests/e2e.rs`
  - Do: In `infer_if` (line 6976), add a `types.insert(if_.syntax().text_range(), resolved_result)` before returning — matching what `infer_expr` does at line 5986. Add e2e tests for `else if` chains with Int, String, and Bool return types, including 3-level chains and `let` bindings.
  - Verify: `cargo test -p meshc --test e2e` passes; new `else_if` tests pass; no crashes or wrong values
  - Done when: `else if` chains return the correct branch value for all three tested types

- [ ] **T03: Fix multiline function call type resolution** `est:1h`
  - Why: R017 — function calls with args on separate lines resolve to `()` instead of the correct return type. The parser produces a correct CST, but the typechecker or codegen loses the type when the call spans multiple lines.
  - Files: `compiler/mesh-typeck/src/infer.rs`, `compiler/meshc/tests/e2e.rs`
  - Do: Diagnose the exact failure point by adding a test that compiles a multiline call and checking where the type is lost (compare `TextRange` storage vs lookup). Fix the root cause — likely in `infer_call` or the `types.insert` path. Add e2e tests for multiline user-defined function calls with Int and String return types, 2+ args on separate lines, and mixed single/multiline calls.
  - Verify: `cargo test -p meshc --test e2e` passes; new `multiline_call` tests pass
  - Done when: `add(\n  1,\n  2\n)` resolves to `Int` (not `()`); multiline calls work equivalently to single-line calls

## Files Likely Touched

- `compiler/mesh-parser/src/parser/mod.rs`
- `compiler/mesh-parser/src/parser/expressions.rs`
- `compiler/mesh-typeck/src/infer.rs`
- `compiler/meshc/tests/e2e.rs`
