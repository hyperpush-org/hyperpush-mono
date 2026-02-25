---
phase: 113-mesher-rewrite-retention-and-final-cleanup
plan: 01
subsystem: database
tags: [orm, query-builder, repo, mesh, retention, storage, raw-sql-rewrite]

# Dependency graph
requires:
  - phase: 112-mesher-rewrite-search-dashboard-and-alerts
    provides: Alert system queries rewritten to ORM; established ORM boundary documentation pattern
provides:
  - "4 retention/storage data queries rewritten to ORM (delete_expired_events, get_all_project_retention, get_project_storage, get_project_settings)"
  - "ORM boundary documentation for update_project_settings and check_sample_rate"
  - "DDL exclusion comments for get_expired_partitions and drop_partition"
  - "Zero unaccounted-for data queries remaining in mesher/ -- REWR-08 satisfied"
  - "Updated file header documenting all data queries now use ORM"
affects: [phase-114]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Repo.delete_where + Query.where_raw for DELETE with complex WHERE (interval expression)"
    - "Query.from + Query.select_raw + Repo.all for simple projection queries (no WHERE)"
    - "Query.from + Query.where_raw + Query.select_raw + Repo.all for filtered projection queries"

key-files:
  created: []
  modified:
    - mesher/storage/queries.mpl

key-decisions:
  - "delete_expired_events uses Repo.delete_where + Query.where_raw for interval expression -- the interval arithmetic (? || ' days')::interval is expressible via where_raw"
  - "update_project_settings retains raw SQL: COALESCE with server-side JSONB extraction cannot be expressed via Repo.update_where Map<String,String>"
  - "check_sample_rate retains raw SQL: random() comparison with scalar subquery and COALESCE default not expressible via ORM query builder"
  - "get_expired_partitions and drop_partition excluded from data query count as DDL/catalog operations"
  - "All 24+ remaining raw SQL calls in queries.mpl categorized: 4 PG crypto two-step, 18 documented ORM boundaries, 2 DDL/partition"

patterns-established:
  - "ORM boundary comment format: multi-line before function explaining WHAT cannot be expressed and WHY, ending in 'Intentional raw SQL.'"
  - "DDL exclusion comment format: single-line in function comment noting DDL/catalog type and scope exclusion"

requirements-completed: [REWR-06, REWR-08]

# Metrics
duration: 5min
completed: 2026-02-25
---

# Phase 113 Plan 01: Retention/Storage ORM Rewrite Summary

**4 retention/storage queries rewritten to ORM (delete_expired_events, get_all_project_retention, get_project_storage, get_project_settings) + 2 ORM boundaries + 2 DDL exclusions documented, satisfying REWR-06 and REWR-08 with zero compilation errors**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-02-25T20:50:00Z
- **Completed:** 2026-02-25T20:55:00Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- Rewrote 4 retention/storage data query functions from raw SQL to ORM Query/Repo pipe chains
- Added ORM boundary documentation to 2 functions that must retain raw SQL (update_project_settings, check_sample_rate)
- Added DDL exclusion comments to 2 partition/catalog functions (get_expired_partitions, drop_partition)
- Updated file header to reflect all data queries now use ORM
- Verified mesher compiles with zero errors after all changes
- Audited all remaining Repo.query_raw/execute_raw calls: 24 in queries.mpl (4 PG crypto, 18 ORM boundaries, 2 DDL), 1 in writer.mpl (JSONB extraction), 2 in schema.mpl (DDL) -- all accounted for

## Task Commits

Each task was committed atomically:

1. **Task 1: Rewrite 4 retention/storage queries to ORM and document 2 boundaries** - `f63b151d` (feat)
2. **Task 2: Compile Mesher and audit remaining raw SQL calls** - no file changes (compilation + audit verification)

**Plan metadata:** (docs commit to follow)

## Files Created/Modified

- `/Users/sn0w/Documents/dev/snow/mesher/storage/queries.mpl` - Header updated; 4 functions rewritten to ORM; 2 ORM boundary comments added; 2 DDL exclusion comments added; section header updated with Phase 113 note

## Decisions Made

- `delete_expired_events` uses `Repo.delete_where` + `Query.where_raw`: the interval expression `(? || ' days')::interval` is expressible via `where_raw`, making this a clean ORM rewrite
- `update_project_settings` retains raw SQL: `COALESCE(($2::jsonb->>'retention_days')::int, retention_days)` requires server-side JSONB extraction and fallback to current column value -- `Repo.update_where` only accepts `Map<String,String>` literal values
- `check_sample_rate` retains raw SQL: `random() < COALESCE((SELECT ...), 1.0)` uses a server-side random function comparison with a scalar subquery -- not expressible via ORM
- DDL/partition functions excluded per established ORM rewrite scope rules

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- `meshc` installed at `/Users/sn0w/.local/bin/meshc` could not locate `libmesh_rt.a` -- used the locally built binary at `/Users/sn0w/Documents/dev/snow/target/debug/meshc` instead (consistent with prior phases). Zero compilation errors confirmed.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- REWR-06 and REWR-08 requirements satisfied
- All data queries across Mesher now use ORM or have documented ORM boundary rationale
- mesher compiles with zero errors
- Phase 113 cleanup complete; ready for any remaining Phase 113 plans or Phase 114

---
*Phase: 113-mesher-rewrite-retention-and-final-cleanup*
*Completed: 2026-02-25*
