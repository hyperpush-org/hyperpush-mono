---
phase: 114-compile-run-and-end-to-end-verification
plan: 02
subsystem: infra
tags: [mesher, postgres, e2e, sigsegv, event-ingestion, websocket, smoke-test, orm]

# Dependency graph
requires:
  - phase: 114-compile-run-and-end-to-end-verification
    plan: 01
    provides: Compiled mesher/mesher binary confirmed startable against PostgreSQL
  - phase: 113-mesher-rewrite-retention-and-final-cleanup
    provides: Fully rewritten ORM query layer (all domains)
provides:
  - Live end-to-end HTTP API smoke test results for all 8 domains (2xx responses confirmed)
  - WebSocket upgrade verification (101 Switching Protocols confirmed)
  - EventProcessor SIGSEGV confirmed resolved under real authenticated load
  - SERVICE_CALL_SEGFAULT.md updated with live verification results and final RESOLVED status
affects: [115-tracking-corrections]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Event ingestion auth uses x-sentry-auth header, not X-Api-Key (mesher/ingestion/auth.mpl)"
    - "Test API key stored as plaintext in api_keys.key_hash (not hashed); authenticate uses crypt comparison"

key-files:
  created: []
  modified:
    - SERVICE_CALL_SEGFAULT.md

key-decisions:
  - "Event ingestion endpoint uses x-sentry-auth header (not X-Api-Key as listed in plan interface section) -- confirmed by mesher/ingestion/auth.mpl source"
  - "POST /api/v1/events returns 202 Accepted (not 200) for valid event ingestion -- consistent with async processing"
  - "SIGSEGV root cause (MirType::Tuple by-value struct vs ptr) confirmed resolved: all endpoints 2xx, process alive after all requests"

patterns-established:
  - "Smoke test pattern: seed test data via psql, test each HTTP domain with curl, verify process alive with kill -0"
  - "Auth header for event ingestion: x-sentry-auth (not X-Api-Key)"

requirements-completed: [VER-02, VER-03]

# Metrics
duration: 15min
completed: 2026-02-25
---

# Phase 114 Plan 02: HTTP API Endpoint Smoke Test and WebSocket Upgrade Verification Summary

**All 8 HTTP API domains return 2xx and WebSocket upgrade returns 101; EventProcessor SIGSEGV confirmed resolved against live PostgreSQL**

## Performance

- **Duration:** ~15 min (human checkpoint approval + documentation)
- **Started:** 2026-02-25T21:50:42Z
- **Completed:** 2026-02-25T21:55:00Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- Confirmed POST /api/v1/events returns 202 with valid x-sentry-auth header and Mesher process remains alive -- the MirType::Tuple SIGSEGV is resolved
- Verified all 8 HTTP domain endpoints return 2xx: event_ingest (202), search_issues (200), dashboard_volume (200), dashboard_health (200), alert_rules (200), alerts (200), settings (200), storage (200)
- Confirmed WebSocket upgrade on :8081 returns 101 Switching Protocols
- Updated SERVICE_CALL_SEGFAULT.md with live verification results table and final RESOLVED status

## Task Commits

1. **Task 1: HTTP API endpoint smoke test and WebSocket upgrade verification** - `783e5882` (docs)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `SERVICE_CALL_SEGFAULT.md` - Added "## Live Verification (Phase 114)" section with smoke test results table, auth header note, and RESOLVED status

## Decisions Made

- Event ingestion uses `x-sentry-auth` header, not `X-Api-Key` as listed in the plan's interface section. The plan's test command used the wrong header; the human verified with the correct header from mesher/ingestion/auth.mpl.
- POST /api/v1/events returns 202 Accepted (async processing accepted), not 200 OK.

## Deviations from Plan

### Auth Header Discovery

**1. [Rule 1 - Bug in plan spec] Event ingestion uses x-sentry-auth header, not X-Api-Key**
- **Found during:** Task 1 (human verification step)
- **Issue:** Plan interface section documented `X-Api-Key` header for event ingestion. Actual auth implementation in mesher/ingestion/auth.mpl uses `x-sentry-auth` header.
- **Fix:** Human re-ran the curl command with the correct `x-sentry-auth: testkey123` header. Documentation updated in SERVICE_CALL_SEGFAULT.md to note this.
- **Impact:** No code change needed. This was a plan documentation error, not a code defect.

---

**Total deviations:** 1 (plan spec had wrong header name; caught and corrected during verification)
**Impact on plan:** No code changes required. Verification succeeded with corrected header.

## Issues Encountered

None beyond the header name mismatch noted above. All endpoints responded correctly on first attempt with correct auth header.

## User Setup Required

None - test data was seeded via psql during verification, no ongoing setup required.

## Next Phase Readiness

- Phase 114 complete: compile, startup, and full HTTP/WS smoke test all verified
- MirType::Tuple SIGSEGV is confirmed resolved in live Mesher against PostgreSQL
- SERVICE_CALL_SEGFAULT.md fully documents root cause, fix, and live verification -- document is complete
- Phase 115 (tracking corrections) can proceed; one known latent issue remains: service loop arg loading only distinguishes ptr vs i64 (Hypothesis C in SERVICE_CALL_SEGFAULT.md) -- not triggered by current Mesher services

## Self-Check

- FOUND: SERVICE_CALL_SEGFAULT.md contains "## Live Verification (Phase 114)" section
- FOUND: commit 783e5882 (docs(114-02): document live verification results in SERVICE_CALL_SEGFAULT.md)

---
*Phase: 114-compile-run-and-end-to-end-verification*
*Completed: 2026-02-25*
