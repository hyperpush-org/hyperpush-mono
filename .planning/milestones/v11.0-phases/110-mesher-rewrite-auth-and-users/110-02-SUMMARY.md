---
phase: 110-mesher-rewrite-auth-and-users
plan: 02
subsystem: database
tags: [orm, query-builder, api-keys, join, repo]

# Dependency graph
requires:
  - phase: 110-mesher-rewrite-auth-and-users
    plan: 01
    provides: "5 user/session query functions rewritten to ORM, two-step pattern established"
  - phase: 107-mesher-rewrite-join
    provides: "Query.join_as ORM API for aliased JOIN queries"
provides:
  - "4 API key query functions rewritten to use ORM APIs"
  - "Repo.update_where type signature fix (Ptr -> Map<String,String> for fields, Ptr -> Result<Map,String> for return)"
  - "All 9 auth/user/session/API-key functions now use ORM instead of raw SQL"
affects: [mesher-auth, mesher-settings, mesher-ingestion]

# Tech tracking
tech-stack:
  added: []
  patterns: ["Query.join_as with aliased tables for multi-table ORM queries", "Two-step pattern for revoke operations: Repo.query_raw for now() + Repo.update_where for data"]

key-files:
  created: []
  modified:
    - "mesher/storage/queries.mpl"
    - "crates/mesh-typeck/src/infer.rs"

key-decisions:
  - "Repo.update_where type signature corrected: fields_map from Ptr to Map<String,String>, return from Ptr to Result<Map<String,String>, String>"
  - "revoke_api_key uses two-step pattern (query now() then Repo.update_where) rather than single Repo.execute_raw"

patterns-established:
  - "Query.join_as for ORM JOIN queries with table aliases (ak.key_value, ak.revoked_at)"
  - "Query.where_raw for IS NULL checks on aliased joined tables"

requirements-completed: [REWR-01]

# Metrics
duration: 4min
completed: 2026-02-18
---

# Phase 110 Plan 02: Rewrite API Key Queries Summary

**4 API key query functions rewritten from raw SQL to ORM Query.join_as/Repo.insert/Repo.update_where with type checker fix for update_where signature**

## Performance

- **Duration:** 4 min
- **Started:** 2026-02-18T01:11:23Z
- **Completed:** 2026-02-18T01:16:12Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Rewrote get_project_by_api_key and get_project_id_by_key to use Query.join_as + Query.where_raw + Query.select_raw instead of Repo.query_raw with raw SQL JOINs
- Rewrote create_api_key to use two-step pattern: Repo.query_raw for gen_random_bytes + Repo.insert for data INSERT
- Rewrote revoke_api_key to use two-step pattern: Repo.query_raw for now() timestamp + Repo.update_where for the UPDATE
- Fixed Repo.update_where type checker signature: fields_map from Ptr to Map<String,String>, return from Ptr to Result<Map<String,String>, String>
- Combined with Plan 01: all 9 auth/user/session/API-key functions now use ORM APIs

## Task Commits

Each task was committed atomically:

1. **Task 1: Rewrite get_project_by_api_key and get_project_id_by_key with ORM JOIN** - `567e1c23` (feat)
2. **Task 2: Rewrite create_api_key and revoke_api_key** - `7360b290` (feat)

## Files Created/Modified
- `mesher/storage/queries.mpl` - 4 API key query functions rewritten to use ORM APIs (Query.join_as, Repo.insert, Repo.update_where)
- `crates/mesh-typeck/src/infer.rs` - Fixed Repo.update_where type signature from (Ptr, Ptr) to (Map<String,String>, Ptr) -> Result<Map, String>

## Decisions Made
- Fixed Repo.update_where type checker signature: fields_map was Ptr but runtime expects Map<String,String>; return was Ptr but runtime returns Result<Map, String>. Same class of fix as Repo.delete_where in Plan 01.
- Used two-step pattern for revoke_api_key: Repo.query_raw for SELECT now()::text, then Repo.update_where for the actual UPDATE. This mirrors the two-step pattern established for create_user/create_session.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed Repo.update_where type signature in type checker**
- **Found during:** Task 2 (revoke_api_key rewrite)
- **Issue:** Repo.update_where was typed with Ptr for both fields_map argument and return type. The fields_map argument didn't unify with Map<String,String> literals, and the Ptr return type prevented using the ? operator.
- **Fix:** Changed fields_map type from ptr_t to Ty::map(Ty::string(), Ty::string()), and return type from ptr_t to Ty::result(Ty::map(Ty::string(), Ty::string()), Ty::string())
- **Files modified:** crates/mesh-typeck/src/infer.rs
- **Verification:** `cargo build -p meshc` succeeds, `meshc build mesher` succeeds, 255/255 E2E tests pass
- **Committed in:** 7360b290 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Type signature fix was necessary for correctness. The runtime already expected Map<String,String> and returned Result; the type checker was incorrectly using Ptr. No scope creep.

## Issues Encountered
None beyond the type signature fix documented above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All 9 auth/user/session/API-key functions now use ORM APIs
- Zero Repo.execute_raw calls in the auth/user/session/API-key domain
- Repo.query_raw remains only for PG utility function calls (crypt, gen_random_bytes, now())
- Phase 110 (mesher-rewrite-auth-and-users) is fully complete
- Remaining Repo.execute_raw and Repo.query_raw calls are in other domains (issues, alerts, retention, search) -- out of scope for this phase

## Self-Check: PASSED

All files and commits verified.

---
*Phase: 110-mesher-rewrite-auth-and-users*
*Completed: 2026-02-18*
