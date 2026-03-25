---
id: T01
parent: S01
milestone: M033
provides:
  - Partial neutral SQL expression runtime scaffolding; compiler/runtime integration and e2e coverage are still unfinished
key_files:
  - compiler/mesh-rt/src/db/expr.rs
  - compiler/mesh-rt/src/db/query.rs
  - compiler/mesh-rt/src/db/mod.rs
  - compiler/mesh-rt/src/lib.rs
  - .gsd/milestones/M033/slices/S01/S01-PLAN.md
key_decisions:
  - Use a dedicated Box-backed SqlExpr tree with local placeholder numbering so outer Query/Repo serializers can renumber fragments safely later
patterns_established:
  - Expression fragments should serialize with per-fragment $1..$N numbering; the surrounding SQL builder is responsible for final placeholder renumbering
observability_surfaces:
  - none yet; the planned e2e surface in compiler/meshc/tests/e2e_m033_s01.rs was not created before the context wrap-up
duration: 1h
verification_result: failed
completed_at: 2026-03-24 14:39 EDT
blocker_discovered: false
---

# T01: Ship the neutral expression contract through compiler and runtime

**Started the neutral SQL expression runtime skeleton and added the slice’s explicit failure-path verification command, but T01 is still incomplete.**

## What Happened

I completed the pre-flight artifact fix in `.gsd/milestones/M033/slices/S01/S01-PLAN.md` by adding an explicit `expr_error_` verification command so the slice has a named diagnostic/failure-path gate.

On the implementation side, I added `compiler/mesh-rt/src/db/expr.rs` with a dedicated `SqlExpr` tree plus portable serializer support for column refs, parameter values, `NULL`, function calls, arithmetic/comparison, `CASE`, `COALESCE`, `EXCLUDED.<field>`, and aliases. I also registered the new runtime module in `compiler/mesh-rt/src/db/mod.rs` and exported the expression builders from `compiler/mesh-rt/src/lib.rs`.

I started the Query-side plumbing in `compiler/mesh-rt/src/db/query.rs` by extending the internal query layout with a `select_params` slot so expression-valued `SELECT` clauses can eventually carry their own parameters without colliding with WHERE/HAVING placeholders.

I stopped there because the context-budget warning hit mid-task. The unfinished parts are still the main body of T01: Query/Repo expression entrypoints, type inference wiring, MIR/intrinsic wiring, and `compiler/meshc/tests/e2e_m033_s01.rs`.

## Verification

I did not run the task or slice verification commands before wrap-up. The repo was left in a coherent partial state by reverting the incomplete `mesh_query_select_expr` / `mesh_query_where_expr` / `mesh_repo_*_expr` export additions from `compiler/mesh-rt/src/lib.rs` after the context warning arrived.

## Verification Evidence

No verification commands were run before the context wrap-up.

## Diagnostics

Resume from these files first:

1. `compiler/mesh-rt/src/db/expr.rs` — expression tree/serializer already exists and is the intended core
2. `compiler/mesh-rt/src/db/query.rs` — `select_params` slot is already added; next work should add `mesh_query_select_expr` and `mesh_query_where_expr`
3. `compiler/mesh-rt/src/db/repo.rs` — add `update_where_expr` / `insert_or_update_expr` and any supporting SQL builders
4. `compiler/mesh-typeck/src/infer.rs`, `compiler/mesh-codegen/src/mir/lower.rs`, `compiler/mesh-codegen/src/codegen/intrinsics.rs` — add the `Expr` module surface plus the new Query/Repo entrypoints
5. `compiler/meshc/tests/e2e_m033_s01.rs` — create the named `e2e_m033_expr_*` and `expr_error_*` proofs that the slice plan now expects

The safest next move is to finish the runtime/compiler wiring before touching Mesher code. Do not assume `expr.rs` is wired through yet; it is only the runtime-side core.

## Deviations

I wrapped the unit early because of the explicit context-budget warning. That left T01 incomplete instead of forcing through unverified compiler/runtime changes.

## Known Issues

- `compiler/mesh-rt/src/db/query.rs` contains the new `select_params` slot, but the expression-aware Query functions were not implemented yet.
- `compiler/mesh-rt/src/db/repo.rs` still has only the legacy Map<String,String>-based write surface.
- `compiler/mesh-typeck/src/infer.rs`, `compiler/mesh-codegen/src/mir/lower.rs`, and `compiler/mesh-codegen/src/codegen/intrinsics.rs` were not updated.
- `compiler/meshc/tests/e2e_m033_s01.rs` does not exist yet.
- T01 verification was not run.

## Files Created/Modified

- `compiler/mesh-rt/src/db/expr.rs` — added the new neutral expression tree plus serializer unit tests
- `compiler/mesh-rt/src/db/query.rs` — added a `select_params` query slot in preparation for expression-valued SELECT support
- `compiler/mesh-rt/src/db/mod.rs` — registered the new `expr` runtime module
- `compiler/mesh-rt/src/lib.rs` — exported the new expression builder runtime functions and reverted incomplete Query/Repo expression exports
- `.gsd/milestones/M033/slices/S01/S01-PLAN.md` — added an explicit `expr_error_` verification command for the slice failure-path gate
