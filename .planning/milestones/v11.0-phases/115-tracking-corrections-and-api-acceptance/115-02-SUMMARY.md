---
phase: 115-tracking-corrections-and-api-acceptance
plan: 02
subsystem: documentation
tags: [roadmap, dead-code, queries, orm, mesh]

# Dependency graph
requires:
  - phase: 115-01
    provides: WHERE/FRAG/UPS requirements tracking corrections
  - phase: 109
    provides: positional API implementation (insert_or_update, delete_where_returning, where_sub)
provides:
  - ROADMAP.md Phase 109 canonical API acceptance note
  - mesher/storage/queries.mpl with dead code functions removed
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns: []

key-files:
  created: []
  modified:
    - .planning/ROADMAP.md
    - mesher/storage/queries.mpl

key-decisions:
  - "Phase 109 positional-arg API (Repo.insert_or_update, Repo.delete_where_returning, Query.where_sub) accepted as canonical v11.0 API shape; original keyword-option style was never implemented"
  - "get_project_id_by_key removed: superseded by get_project_by_api_key (returns full Project struct); zero import sites confirmed"
  - "get_user_orgs removed: simple ORM query never wired into any service; zero import sites confirmed"

patterns-established: []

requirements-completed: [UPS-01, UPS-02, UPS-03]

# Metrics
duration: 3min
completed: 2026-02-25
---

# Phase 115 Plan 02: Tracking Corrections and API Acceptance Summary

**Phase 109 positional API (insert_or_update, delete_where_returning, where_sub) formally accepted as canonical v11.0 API in ROADMAP; two dead-code query functions removed from mesher/storage/queries.mpl with zero import sites affected**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-25T22:16:17Z
- **Completed:** 2026-02-25T22:19:00Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Added Phase 109 API acceptance note to ROADMAP.md confirming positional-arg style is the canonical v11.0 API (Phase 109 SC already used positional names; acceptance note explicitly documents this as the accepted replacement for the never-implemented keyword-option style)
- Removed `get_project_id_by_key` function (lines 128-144) from mesher/storage/queries.mpl — superseded by `get_project_by_api_key` which returns the full Project struct; zero import sites across all 32+ mesher .mpl files
- Removed `get_user_orgs` function (lines 266-274) from mesher/storage/queries.mpl — ORM query never wired into any service; zero import sites confirmed
- Verified `meshc build mesher` compiles with zero errors after dead code removal

## Task Commits

Each task was committed atomically:

1. **Task 1: Update ROADMAP Phase 109 success criteria to reflect positional API** - `75415abf` (feat)
2. **Task 2: Remove dead code functions from queries.mpl** - `a1546695` (fix)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `.planning/ROADMAP.md` - Added API acceptance note to Phase 109 section clarifying positional-arg style as canonical v11.0 API shape
- `mesher/storage/queries.mpl` - Removed get_project_id_by_key (17 lines) and get_user_orgs (11 lines); 28 lines deleted total

## Decisions Made

- Phase 109 positional API style (insert_or_update, delete_where_returning, where_sub) accepted as canonical v11.0 API. The original ROADMAP proposed keyword-option style (`Repo.insert(on_conflict: :update)`, `returning: true`, `Query.where(sub: ...)`) was never implemented. The positional functions are implemented, tested, and verified end-to-end in Phase 109-02.
- Dead code removal is safe: `grep -r` across all mesher .mpl files confirmed zero import sites for both functions before removal. `meshc build mesher` confirms zero compilation errors after removal.

## Deviations from Plan

None - plan executed exactly as written.

Note: Phase 109 success criteria in ROADMAP.md were already using the positional API names (they were updated when Phase 109 was executed and completed). Task 1 added the explicit acceptance note clarifying the canonical status, which was the primary intent of the task.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 115 complete: all 2 plans executed
- v11.0 Query Builder milestone complete: Phase 115 was the final phase
- Requirements UPS-01, UPS-02, UPS-03 accepted and complete (combined with 115-01's WHERE/FRAG completions)
- ROADMAP.md correctly documents the canonical API shape for all v11.0 features
- mesher/storage/queries.mpl is clean with zero dead code functions

---
*Phase: 115-tracking-corrections-and-api-acceptance*
*Completed: 2026-02-25*
