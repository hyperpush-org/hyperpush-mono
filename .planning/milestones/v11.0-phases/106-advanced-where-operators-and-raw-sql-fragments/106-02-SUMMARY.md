---
phase: 106-advanced-where-operators-and-raw-sql-fragments
plan: 02
subsystem: database
requirements-completed: [FRAG-01, FRAG-02, FRAG-03, FRAG-04]
tags: [query-builder, sql, fragments, raw-sql, parameter-renumbering, order-by, group-by]

# Dependency graph
requires:
  - phase: 106-advanced-where-operators-and-raw-sql-fragments
    plan: 01
    provides: "Advanced WHERE operators (NOT IN, BETWEEN, ILIKE, OR), full pipeline registration"
  - phase: 103-raw-sql-support
    provides: "RAW: prefix encoding, where_raw, select_raw functions"
provides:
  - "renumber_placeholders helper for $N and ? placeholder renumbering in SQL fragments"
  - "Query.order_by_raw(q, expression) -- raw ORDER BY expressions (random(), count DESC)"
  - "Query.group_by_raw(q, expression) -- raw GROUP BY expressions (date_trunc, level)"
  - "Fixed $N renumbering in where_raw and fragment SQL builders across all 3 code paths"
  - "RAW: prefix support in ORDER BY and GROUP BY SQL generation"
affects: [query-builder, repo, mesher-migration]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "renumber_placeholders for unified ? and $N placeholder handling"
    - "RAW: prefix in ORDER BY and GROUP BY fields for verbatim emission"

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
  - "Unified renumber_placeholders helper handles both ? and $N styles in a single pass"
  - "RAW: prefix reused for ORDER BY and GROUP BY raw expressions, consistent with existing select_raw/where_raw pattern"

patterns-established:
  - "All raw SQL clause positions (SELECT, WHERE, ORDER BY, GROUP BY) use RAW: prefix encoding"

# Metrics
duration: 8min
completed: 2026-02-17
---

# Phase 106 Plan 02: Fragment Renumbering and Raw ORDER BY/GROUP BY Summary

**Fixed $N placeholder renumbering in fragments/where_raw, added Query.order_by_raw and Query.group_by_raw with full pipeline registration and 14 new tests**

## Performance

- **Duration:** 8 min
- **Started:** 2026-02-17T20:33:32Z
- **Completed:** 2026-02-17T20:41:51Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- Fragment $N renumbering works correctly: `crypt($1, gen_salt('bf'))` after 2 WHERE params becomes `crypt($3, gen_salt('bf'))`
- ORDER BY raw and GROUP BY raw expressions emit verbatim SQL (random(), date_trunc, count DESC)
- All 4 SQL clause positions now support raw expressions: SELECT, WHERE, ORDER BY, GROUP BY
- JSONB operators (metadata @> $1::jsonb), PG functions (crypt, date_trunc) generate correct parameterized SQL
- 7 new unit tests and 7 new E2E tests, all 521 mesh-rt tests pass, all 20 query builder E2E tests pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Fix $N renumbering and add ORDER BY/GROUP BY raw support** - `90d67808` (feat)
2. **Task 2: Register new functions in MIR/codegen/JIT and add E2E tests** - `c7986a56` (feat)

## Files Created/Modified
- `crates/mesh-rt/src/db/repo.rs` - Added renumber_placeholders helper, updated where_raw and fragment injection in all 3 SQL builders, updated ORDER BY and GROUP BY generation for RAW: prefix, added 7 unit tests
- `crates/mesh-rt/src/db/query.rs` - Added mesh_query_order_by_raw and mesh_query_group_by_raw extern C functions
- `crates/mesh-rt/src/lib.rs` - Re-exported two new functions
- `crates/mesh-codegen/src/mir/lower.rs` - Registered known_functions and map_builtin_name for 2 new functions
- `crates/mesh-codegen/src/codegen/intrinsics.rs` - Declared 2 LLVM intrinsics, added 2 assertions to intrinsics test
- `crates/mesh-repl/src/jit.rs` - Registered 2 JIT symbols
- `crates/mesh-typeck/src/infer.rs` - Added type signatures for order_by_raw and group_by_raw, fixed fragment params type from Ptr to List<String>
- `crates/meshc/tests/e2e.rs` - Added 7 E2E tests (order_by_raw, group_by_raw, select_raw+group_by_raw, where_raw $1, fragment crypt, JSONB, all_positions)

## Decisions Made
- Unified renumber_placeholders helper handles both `?` and `$N` styles in a single char-by-char pass, returning (renumbered_sql, params_consumed)
- RAW: prefix reused for ORDER BY and GROUP BY raw expressions, keeping the established pattern from select_raw/where_raw

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added typechecker registration for new Query functions**
- **Found during:** Task 2 (E2E tests)
- **Issue:** Plan did not mention `mesh-typeck/src/infer.rs` as a touch point, but Query module functions must be registered in the typechecker for the compiler to recognize `Query.order_by_raw` and `Query.group_by_raw`
- **Fix:** Added type signatures for both functions in the Query module type registration
- **Files modified:** `crates/mesh-typeck/src/infer.rs`
- **Verification:** All E2E tests pass after adding typechecker registrations
- **Committed in:** c7986a56 (Task 2 commit)

**2. [Rule 1 - Bug] Fixed Query.fragment typechecker entry for params parameter**
- **Found during:** Task 2 (E2E fragment_crypt test)
- **Issue:** The typechecker had `fragment` typed as `(Ptr, String, Ptr)` but the params argument is actually a `List<String>` in Mesh source code. The `Ptr` type caused a type mismatch when passing `["secret"]` as the params list.
- **Fix:** Changed third parameter type from `Ptr` to `List<String>` to match the actual calling convention (consistent with `where_raw`)
- **Files modified:** `crates/mesh-typeck/src/infer.rs`
- **Verification:** e2e_query_builder_fragment_crypt test passes
- **Committed in:** c7986a56 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug)
**Impact on plan:** Both fixes essential for compiler acceptance of new function calls. No scope creep.

## Issues Encountered
- Stale build artifact caused linker errors ("Undefined symbols for _mesh_query_order_by_raw") even though symbols existed in source. Resolved by `cargo clean -p mesh-rt` followed by rebuild. Same issue as Plan 01.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 106 complete: all 5 success criteria verified
  - SC1: Comparison operators work (existing e2e_query_builder_where_op)
  - SC2: IN, IS NULL, IS NOT NULL, BETWEEN, LIKE, ILIKE all generate correct SQL (Plan 01)
  - SC3: OR conditions generate grouped parenthesized SQL (Plan 01)
  - SC4: Fragment with $1 placeholders renumbers correctly (this plan: unit + E2E tests)
  - SC5: Fragments work in WHERE, SELECT, ORDER BY, GROUP BY (this plan: e2e_query_builder_fragments_all_positions)
- Ready for Phase 107 (next phase in v11.0 roadmap)

## Self-Check: PASSED

- All 8 modified files verified present on disk
- Task 1 commit `90d67808` verified in git log
- Task 2 commit `c7986a56` verified in git log
- 521 mesh-rt unit tests pass (0 failures)
- 20 query builder E2E tests pass (0 failures)
- Intrinsics declaration test passes

---
*Phase: 106-advanced-where-operators-and-raw-sql-fragments*
*Completed: 2026-02-17*
