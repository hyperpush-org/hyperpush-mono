---
phase: 112-mesher-rewrite-search-dashboard-and-alerts
plan: 02
subsystem: database
tags: [orm, query-builder, alerts, repo, query, join, cooldown, threshold]

# Dependency graph
requires:
  - phase: 112-mesher-rewrite-search-dashboard-and-alerts
    plan: 01
    provides: "12 search/dashboard/detail/team query rewrites, parse_limit helper, ORM boundary documentation pattern"
  - phase: 111-mesher-rewrite-issues-and-events
    provides: "ORM patterns for Query.from + pipe chains, Repo.update_where/delete_where, documented ORM boundaries"
provides:
  - "5 alert query functions rewritten from Repo.query_raw to ORM Query/Repo pipe chains"
  - "3 ORM boundary comments documenting why complex alert queries retain raw SQL"
  - "Complete raw SQL inventory by domain for Phase 113/114"
affects: [113-retention-cleanup, 114-verification, mesher-alerts]

# Tech tracking
tech-stack:
  added: []
  patterns: ["Query.join_as for JOIN with status filter passing status param twice for ($2 = '' OR column = $2) pattern", "ORM boundary documentation for INSERT...SELECT JSONB, cross-join derived tables, jsonb_build_object"]

key-files:
  created: []
  modified:
    - "mesher/storage/queries.mpl"

key-decisions:
  - "toggle_alert_rule and check_new_issue verified as already rewritten by Plan 01 -- skipped"
  - "list_alerts uses Query.join_as with status passed 3 times for project_id + optional status filter"

patterns-established:
  - "ORM boundary format consistently applied to all 5 complex alert queries (create_alert_rule, evaluate_threshold_rule, fire_alert, acknowledge_alert, resolve_fired_alert)"

requirements-completed: [REWR-05]

# Metrics
duration: 3min
completed: 2026-02-18
---

# Phase 112 Plan 02: Rewrite Alert System Queries to ORM Summary

**5 alert queries rewritten from raw SQL to ORM pipe chains with 3 new ORM boundary comments, completing REWR-05 and the Phase 112 alert domain**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-18T03:14:48Z
- **Completed:** 2026-02-18T03:17:26Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments
- Rewrote 5 alert query functions (list_alert_rules, get_event_alert_rules, should_fire_by_cooldown, get_threshold_rules, list_alerts) from Repo.query_raw to ORM Query/Repo pipe chains
- Added 3 ORM boundary comments for complex alert queries (create_alert_rule, evaluate_threshold_rule, fire_alert)
- Verified toggle_alert_rule and check_new_issue were already rewritten by Plan 01
- Mesher compiles successfully with all Phase 112 rewrites
- Documented complete raw SQL inventory: 28 calls in queries.mpl, 1 in writer.mpl, 2 in schema.mpl (DDL)

## Task Commits

Each task was committed atomically:

1. **Task 1: Rewrite alert system queries to ORM** - `30235cff` (feat)
2. **Task 2: Compile, verify, and document final Phase 112 raw SQL count** - `8522fb01` (chore)

## Files Created/Modified
- `mesher/storage/queries.mpl` - 5 alert queries rewritten to ORM, 3 boundary comments added
- `mesher/mesher` - Recompiled binary with all Phase 112 rewrites

## Decisions Made
- toggle_alert_rule and check_new_issue were already rewritten by Plan 01 -- verified and skipped (plan anticipated this possibility)
- list_alerts passes status parameter 3 times to bind both position in (? = '' OR alerts.status = ?) plus project_id

## Deviations from Plan

None - plan executed exactly as written.

## Raw SQL Inventory (Post-Phase 112)

Remaining Repo.query_raw/execute_raw calls by domain:

| Domain | Functions | Calls | Status |
|--------|-----------|-------|--------|
| Auth (PG crypto) | create_api_key, revoke_api_key, create_user, create_session | 4 | Two-step pattern for PG functions |
| Issue (Phase 111) | upsert_issue, assign_issue (NULL), check_volume_spikes | 3 | Documented ORM boundaries |
| Event (Phase 111) | extract_event_fields, insert_event (writer.mpl) | 2 | Documented ORM boundaries |
| Search (Phase 112) | list_issues_filtered, search_events_fulltext | 3 | Documented ORM boundaries |
| Dashboard (Phase 112) | event_breakdown_by_tag, project_health_summary | 2 | Documented ORM boundaries |
| Detail (Phase 112) | get_event_neighbors | 1 | Documented ORM boundaries |
| Alert (Phase 112) | create_alert_rule, evaluate_threshold_rule, fire_alert, acknowledge_alert, resolve_fired_alert | 6 | Documented ORM boundaries |
| Retention (Phase 113) | delete_expired_events, get_expired_partitions, drop_partition, get_all_project_retention, get_project_storage, update_project_settings, get_project_settings, check_sample_rate | 8 | Phase 113 scope |
| Schema (DDL) | schema migration, partition date | 2 | Excluded from data query count |

**Total: 31 raw SQL calls (28 queries.mpl + 1 writer.mpl + 2 schema.mpl DDL)**

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 112 complete: REWR-03, REWR-04, REWR-05 all fulfilled
- 8 retention queries remain for Phase 113 cleanup
- All remaining raw SQL is documented with ORM boundary rationale or is two-step PG crypto pattern

---
*Phase: 112-mesher-rewrite-search-dashboard-and-alerts*
*Completed: 2026-02-18*

## Self-Check: PASSED

All files and commits verified.
