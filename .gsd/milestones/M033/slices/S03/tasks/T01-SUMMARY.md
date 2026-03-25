---
id: T01
parent: S03
milestone: M033
key_files:
  - compiler/meshc/tests/e2e_m033_s03.rs
  - mesher/storage/queries.mpl
  - .gsd/KNOWLEDGE.md
key_decisions:
  - Kept the T01 caller contracts stable by aliasing every rewritten read projection back to the existing row keys instead of changing consumers.
  - Used the S02 live-Postgres harness pattern for the new S03 proof target so later S03 tasks can extend one durable integration test file instead of inventing separate one-off probes.
duration: ""
verification_result: mixed
completed_at: 2026-03-25T18:43:06.413Z
blocker_discovered: false
---

# T01: Seeded the S03 harness and rewrote the basic read helpers, but the new Mesh probes still need quote cleanup

**Seeded the S03 harness and rewrote the basic read helpers, but the new Mesh probes still need quote cleanup**

## What Happened

I created `compiler/meshc/tests/e2e_m033_s03.rs` as the first real S03 live-Postgres harness, patterned after the S02 Docker/Postgres helper stack, and added three named `e2e_m033_s03_basic_reads_*` proofs covering the T01 helper families: issue count/project lookup, session/settings/storage reads, and API-key/alert-rule list reads. I rewrote the targeted helpers in `mesher/storage/queries.mpl` off raw projection strings and trivial raw whole-query reads onto `Query.where_expr`, `Query.select_expr` / `Query.select_exprs`, `Expr.label`, `Expr.coalesce`, and explicit `Pg.*` casts while preserving the caller-visible row keys (`cnt`, `project_id`, `token`, `revoked_at`, `retention_days`, `sample_rate`, `event_count`, `estimated_bytes`). No caller file needed changes because the rewritten queries kept the existing map keys stable. The first task-level verification run then failed in the new test target before exercising the rewritten reads: the Rust-authored raw-string Mesh probe templates still contain escaped quotes like `\"cnt\"` and `\"14\"` inside `#{...}` interpolation expressions, so the temporary Mesh programs fail to parse. I recorded that resume-critical gotcha in `.gsd/KNOWLEDGE.md` so the next agent can resume by removing those literal backslashes in the probe templates and rerunning the same gate.

## Verification

Ran the task-level gate `cargo test -p meshc --test e2e_m033_s03 basic_reads -- --nocapture`. Cargo built the Rust test target successfully, but all three `basic_reads` tests failed when their temporary Mesh storage probes were compiled. The failure is a Mesh parse error inside the generated probe source, caused by literal `\"...\"` quote escapes embedded in interpolation expressions within the Rust raw-string templates. Because the wrap-up warning arrived at that point, I stopped after the first failing gate and did not start the remaining build/fmt/slice-level checks in this unit.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test -p meshc --test e2e_m033_s03 basic_reads -- --nocapture` | 101 | ❌ fail | 58800ms |


## Deviations

Stopped after the first failing task-level verification command because of the context-budget wrap-up warning. I did not proceed to `cargo run -q -p meshc -- build mesher`, the full slice test target, the fmt check, or the slice verify script in this unit.

## Known Issues

`compiler/meshc/tests/e2e_m033_s03.rs` still contains literal escaped quotes (`\"...\"`) inside the Rust raw-string Mesh probe templates. Those backslashes are emitted into the temporary Mesh source and cause parse errors in all three `e2e_m033_s03_basic_reads_*` tests before the rewritten helper logic is exercised. Resume by replacing those escaped quotes with plain quotes inside the probe templates, then rerun `cargo test -p meshc --test e2e_m033_s03 basic_reads -- --nocapture` before moving on to the remaining verification commands.

## Files Created/Modified

- `compiler/meshc/tests/e2e_m033_s03.rs`
- `mesher/storage/queries.mpl`
- `.gsd/KNOWLEDGE.md`
