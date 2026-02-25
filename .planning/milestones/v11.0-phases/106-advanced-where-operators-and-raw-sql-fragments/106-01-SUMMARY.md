---
phase: 106-advanced-where-operators-and-raw-sql-fragments
plan: 01
subsystem: database
requirements-completed: [WHERE-01, WHERE-02, WHERE-03, WHERE-04, WHERE-05, WHERE-06]
tags: [query-builder, sql, where-clause, not-in, between, ilike, or]

# Dependency graph
requires:
  - phase: 98-query-builder
    provides: "Query builder runtime, SQL generation, MIR/codegen pipeline"
  - phase: 103-raw-sql-support
    provides: "RAW: prefix encoding, where_raw, select_raw functions"
provides:
  - "Query.where_not_in(q, field, values) -- NOT IN clause with parameterized values"
  - "Query.where_between(q, field, low, high) -- BETWEEN clause with two params"
  - "Query.where_or(q, fields, values) -- grouped OR conditions"
  - "ILIKE operator via Query.where_op(q, field, :ilike, pattern)"
  - "NOT_IN, BETWEEN, OR encoding formats in WHERE clause system"
affects: [query-builder, repo, mesher-migration]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "NOT_IN:N encoding for NOT IN clauses (mirrors IN:N)"
    - "BETWEEN encoding for two-param range clauses"
    - "OR:field1,field2,...:N encoding for grouped OR conditions"

key-files:
  created: []
  modified:
    - "crates/mesh-rt/src/db/query.rs"
    - "crates/mesh-rt/src/db/repo.rs"
    - "crates/mesh-rt/src/lib.rs"
    - "crates/mesh-codegen/src/mir/lower.rs"
    - "crates/mesh-codegen/src/codegen/intrinsics.rs"
    - "crates/mesh-repl/src/jit.rs"
    - "crates/mesh-typeck/src/infer.rs"
    - "crates/meshc/tests/e2e.rs"

key-decisions:
  - "OR clause encoding uses OR:field1,field2,...:N format with field names embedded in clause string"
  - "ILIKE added as atom_to_sql_op mapping -- no new function needed, works via existing where_op"

patterns-established:
  - "Multi-param WHERE clauses use count-encoded format (NOT_IN:N, BETWEEN) for SQL builder parsing"

# Metrics
duration: 8min
completed: 2026-02-17
---

# Phase 106 Plan 01: Advanced WHERE Operators Summary

**NOT IN, BETWEEN, ILIKE, and OR operators added to Query builder with full pipeline (runtime, SQL gen, MIR, LLVM codegen, JIT, typechecker) and 10 new tests**

## Performance

- **Duration:** 8 min
- **Started:** 2026-02-17T20:22:45Z
- **Completed:** 2026-02-17T20:30:50Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- Four new WHERE clause types working end-to-end: NOT IN, BETWEEN, ILIKE (via where_op), and OR
- Correct parameter index sequencing across all clause types when combined in a single query
- Full pipeline registration across all 7 touch points (runtime, SQL gen x4, MIR, LLVM codegen, JIT, typechecker, lib exports)
- 5 new unit tests and 5 new E2E tests all passing

## Task Commits

Each task was committed atomically:

1. **Task 1: Add NOT IN, BETWEEN, ILIKE, and OR runtime + SQL generation** - `305f11ce` (feat)
2. **Task 2: Register new WHERE functions in MIR/codegen/JIT and add E2E tests** - `bd00f8ac` (feat)

## Files Created/Modified
- `crates/mesh-rt/src/db/query.rs` - Added ILIKE to atom_to_sql_op, plus mesh_query_where_not_in, mesh_query_where_between, mesh_query_where_or extern C functions
- `crates/mesh-rt/src/db/repo.rs` - Added NOT_IN, BETWEEN, OR clause handling in all 4 SQL builder WHERE parsers, plus 5 unit tests
- `crates/mesh-rt/src/lib.rs` - Re-exported three new functions
- `crates/mesh-codegen/src/mir/lower.rs` - Registered known_functions and map_builtin_name for 3 new functions
- `crates/mesh-codegen/src/codegen/intrinsics.rs` - Declared 3 LLVM intrinsics
- `crates/mesh-repl/src/jit.rs` - Registered 3 JIT symbols
- `crates/mesh-typeck/src/infer.rs` - Added type signatures for where_not_in, where_between, where_or in Query module
- `crates/meshc/tests/e2e.rs` - Added 5 E2E tests (where_not_in, where_between, where_ilike, where_or, advanced_where_combined)

## Decisions Made
- OR clause encoding uses `OR:field1,field2,...:N` format -- embeds field names directly in the clause string rather than requiring a separate encoding mechanism
- ILIKE implemented as an atom_to_sql_op mapping rather than a new function -- consistent with existing LIKE pattern and requires zero new infrastructure

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added typechecker registration for new Query functions**
- **Found during:** Task 2 (E2E tests)
- **Issue:** Plan did not mention `mesh-typeck/src/infer.rs` as a touch point, but the Query module functions must be registered in the typechecker for the compiler to recognize `Query.where_not_in`, `Query.where_between`, and `Query.where_or`
- **Fix:** Added type signatures for all three functions in the Query module type registration
- **Files modified:** `crates/mesh-typeck/src/infer.rs`
- **Verification:** All E2E tests pass after adding typechecker registrations
- **Committed in:** bd00f8ac (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Essential fix -- without typechecker registration, the compiler rejects all new function calls. No scope creep.

## Issues Encountered
- Stale build artifact caused linker errors ("Undefined symbols") even though symbols existed in libmesh_rt.a. Resolved by `cargo clean -p mesh-rt` followed by rebuild.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All four advanced WHERE operators working and tested
- Ready for Phase 106 Plan 02 (raw SQL fragments or remaining WHERE operator work)
- Parameter indexing verified correct across mixed clause types

---
*Phase: 106-advanced-where-operators-and-raw-sql-fragments*
*Completed: 2026-02-17*
