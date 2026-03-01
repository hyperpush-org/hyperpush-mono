---
phase: 141-dogfeed-v14-changes-to-mesher
plan: 01
subsystem: storage
tags: [crypto, stdlib, mesh, mesher, pgcrypto, tokens, package-manifest]

# Dependency graph
requires:
  - phase: 135-encoding-crypto-stdlib
    provides: "Crypto.uuid4() implementation in Mesh stdlib"
  - phase: 139-package-manifest-meshpkg-cli
    provides: "mesh.toml package manifest format specification"
provides:
  - "Mesher storage layer uses Crypto.uuid4() for API key and session token generation"
  - "mesher/mesh.toml package manifest declaring Mesher as a Mesh package"
affects: [141-02, 141-03, deploy-mesher]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Crypto.uuid4() for random token generation — eliminates DB round-trip pattern used for gen_random_bytes"
    - "Slot pipe |2> String.replace for hyphen stripping from UUID to produce raw hex"

key-files:
  created:
    - mesher/mesh.toml
  modified:
    - mesher/storage/queries.mpl

key-decisions:
  - "Crypto.uuid4() for API key generation: 'mshr_' + UUID4 = 41-char key (changed from 53 chars mshr_ + 48 hex); length change is acceptable — keys are stored and compared as-is without length validation"
  - "create_session uses two UUID4s with hyphens stripped via slot pipe |2> String.replace('-', '') to maintain 64-char hex token format"
  - "bcrypt (crypt/gen_salt via pgcrypto) deliberately preserved in create_user and authenticate_user — no Mesh stdlib equivalent for bcrypt"

patterns-established:
  - "Slot pipe for transform: Crypto.uuid4() |2> String.replace('-', '') — inserts piped value as second arg of String.replace"

requirements-completed: [DOGFEED-141]

# Metrics
duration: 1min
completed: 2026-03-01
---

# Phase 141 Plan 01: Dogfeed v14 Changes to Mesher Summary

**Mesher storage layer updated to use Crypto.uuid4() for token generation (no DB round-trips), and mesher/mesh.toml added to dogfeed Phase 139's package manifest format**

## Performance

- **Duration:** 1 min
- **Started:** 2026-03-01T05:58:23Z
- **Completed:** 2026-03-01T05:59:21Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Replaced pgcrypto-backed `gen_random_bytes` in `create_api_key` and `create_session` with `Crypto.uuid4()` stdlib calls — eliminates two unnecessary DB round-trips per token generation
- `create_session` maintains the 64-char hex token format by stripping hyphens from two concatenated UUIDs via slot pipe `|2> String.replace`
- `create_user` and `authenticate_user` bcrypt logic via pgcrypto left intact (no stdlib equivalent)
- Added `mesher/mesh.toml` declaring Mesher as a first-class Mesh package, dogfeeding Phase 139 PKG-01 manifest format

## Task Commits

Each task was committed atomically:

1. **Task 1: Replace pgcrypto token generation with Crypto stdlib** - `638ef671` (feat)
2. **Task 2: Create mesh.toml package manifest** - `d84e6982` (feat)

## Files Created/Modified
- `mesher/storage/queries.mpl` - `create_api_key` and `create_session` updated to use `Crypto.uuid4()`; `create_user`/`authenticate_user` unchanged
- `mesher/mesh.toml` - New package manifest declaring Mesher as a Mesh package with version, description, license, and empty dependencies section

## Decisions Made
- API key format changes from 53 chars (`mshr_` + 48 hex) to 41 chars (`mshr_` + UUID4 with hyphens). Acceptable because key_value is stored and compared verbatim with no length constraint.
- Session token format preserved at 64 hex chars by using two UUID4s with hyphens stripped — maintaining backward compatibility with existing session validation logic.
- bcrypt (pgcrypto `crypt`/`gen_salt`) intentionally kept — Mesh stdlib has no bcrypt equivalent; password security must stay at DB layer.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Crypto stdlib dogfeeding (CRYPTO-03) and package manifest dogfeeding (PKG-01) complete
- Ready for Phase 141 Plan 02 (next dogfeed task in the phase)

## Self-Check: PASSED

All expected files exist and commits are verified in git history.

---
*Phase: 141-dogfeed-v14-changes-to-mesher*
*Completed: 2026-03-01*
