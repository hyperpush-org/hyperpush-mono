---
phase: 110-mesher-rewrite-auth-and-users
plan: 01
subsystem: database
tags: [orm, query-builder, auth, sessions, pgcrypto, repo]

# Dependency graph
requires:
  - phase: 109-mesher-rewrite-subquery-upsert
    provides: "ORM Query/Repo APIs (Query.where, Query.where_raw, Query.select_raw, Repo.all, Repo.insert, Repo.delete_where)"
provides:
  - "5 user/session query functions rewritten to use ORM APIs"
  - "Repo.delete_where type signature fix (Ptr -> Result<Int, String>)"
affects: [110-02, mesher-auth, mesher-sessions]

# Tech tracking
tech-stack:
  added: []
  patterns: ["Two-step pattern: Repo.query_raw for PG function calls + Repo.insert for data INSERT", "Query.where_raw for inline SQL expressions in WHERE clause"]

key-files:
  created: []
  modified:
    - "mesher/storage/queries.mpl"
    - "crates/mesh-typeck/src/infer.rs"

key-decisions:
  - "Repo.delete_where type signature corrected from Ptr to Result<Int, String> to match runtime behavior"
  - "Two-step pattern for create_user/create_session: minimal Repo.query_raw SELECT for PG function, then Repo.insert for data"

patterns-established:
  - "Two-step ORM pattern: Use Repo.query_raw only for PG utility function calls (crypt, gen_random_bytes), then Repo.insert for actual data insertion"
  - "Query.where_raw with ? placeholder for inline crypt() expressions in WHERE clauses"

requirements-completed: [REWR-01]

# Metrics
duration: 6min
completed: 2026-02-18
---

# Phase 110 Plan 01: Rewrite Auth/Session Queries Summary

**5 user/session query functions rewritten from raw SQL to ORM Query/Repo APIs with two-step pattern for PG crypto functions**

## Performance

- **Duration:** 6 min
- **Started:** 2026-02-18T01:02:07Z
- **Completed:** 2026-02-18T01:08:35Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Rewrote authenticate_user and validate_session to use Query.where + Query.where_raw + Query.select_raw instead of Repo.query_raw
- Rewrote create_user and create_session to use two-step pattern: Repo.query_raw for PG crypto functions + Repo.insert for data INSERT
- Rewrote delete_session to use Repo.delete_where (zero raw SQL)
- Fixed Repo.delete_where type checker signature from Ptr to Result<Int, String> to match actual runtime return type

## Task Commits

Each task was committed atomically:

1. **Task 1: Rewrite authenticate_user and validate_session** - `e21dc5f6` (feat)
2. **Task 2: Rewrite create_user, create_session, and delete_session** - `739fab89` (feat)

## Files Created/Modified
- `mesher/storage/queries.mpl` - 5 user/session query functions rewritten to use ORM APIs
- `crates/mesh-typeck/src/infer.rs` - Fixed Repo.delete_where return type from Ptr to Result<Int, String>

## Decisions Made
- Fixed Repo.delete_where type checker signature: was returning Ptr but runtime actually returns Result<Int, String>. This is a correctness fix, not a behavioral change.
- Used two-step pattern for create_user and create_session: a minimal Repo.query_raw SELECT to compute PG crypto expressions (crypt, gen_random_bytes), then Repo.insert for the actual data insertion.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed Repo.delete_where type signature in type checker**
- **Found during:** Task 2 (delete_session rewrite)
- **Issue:** Repo.delete_where was typed as returning Ptr in the type checker, but the function signature of delete_session expects Result<Int, String>. The runtime actually returns Result<Int, String>.
- **Fix:** Changed type signature in infer.rs from `ptr_t.clone()` to `Ty::result(Ty::int(), Ty::string())`
- **Files modified:** crates/mesh-typeck/src/infer.rs
- **Verification:** `cargo build -p meshc` succeeds, `meshc build mesher` succeeds, 94/96 E2E tests pass (2 pre-existing failures unrelated)
- **Committed in:** 739fab89 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Type signature fix was necessary for correctness. The runtime already returned Result<Int, String>; the type checker was incorrectly using Ptr. No scope creep.

## Issues Encountered
None beyond the type signature fix documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All 5 user/session query functions now use ORM APIs
- Zero Repo.execute_raw in user/session domain
- Repo.query_raw remains only for PG utility function calls (crypt, gen_random_bytes)
- Ready for Plan 02 (remaining query rewrites in other domains)

## Self-Check: PASSED

All files and commits verified.

---
*Phase: 110-mesher-rewrite-auth-and-users*
*Completed: 2026-02-18*
