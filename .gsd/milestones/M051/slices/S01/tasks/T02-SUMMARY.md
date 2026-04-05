---
id: T02
parent: S01
milestone: M051
provides: []
requires: []
affects: []
key_files: ["compiler/meshc/tests/support/m051_mesher.rs", "compiler/meshc/tests/e2e_m051_s01.rs", "compiler/meshc/tests/support/mod.rs", ".gsd/KNOWLEDGE.md", ".gsd/DECISIONS.md"]
key_decisions: ["Use a dedicated Mesher Rust support harness with Docker Postgres and redacted `.tmp/m051-s01` artifacts instead of reusing the older M033 hardcoded Mesher helpers.", "Build Mesher to an artifact-local binary with `meshc build mesher --output ...` so the proof rail does not churn the tracked `mesher/mesher` package output."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "Ran `cargo test -p meshc --test e2e_m051_s01 -- --nocapture`, which executed 2 tests and passed both the missing-`DATABASE_URL` fail-closed scenario and the live Docker/Postgres migrate/build/run/ingest/readback scenario. Ran a direct `python3` artifact-contract check to confirm the expected retained files exist under `.tmp/m051-s01/` and that no artifact contains the raw `postgres://mesh:mesh@127.0.0.1:...` DSN prefix. There was no additional slice-level Verification section beyond this task-owned rail."
completed_at: 2026-04-04T07:40:44.533Z
blocker_discovered: false
---

# T02: Added a dedicated Mesher Postgres runtime proof rail with redacted `.tmp/m051-s01` artifacts.

> Added a dedicated Mesher Postgres runtime proof rail with redacted `.tmp/m051-s01` artifacts.

## What Happened
---
id: T02
parent: S01
milestone: M051
key_files:
  - compiler/meshc/tests/support/m051_mesher.rs
  - compiler/meshc/tests/e2e_m051_s01.rs
  - compiler/meshc/tests/support/mod.rs
  - .gsd/KNOWLEDGE.md
  - .gsd/DECISIONS.md
key_decisions:
  - Use a dedicated Mesher Rust support harness with Docker Postgres and redacted `.tmp/m051-s01` artifacts instead of reusing the older M033 hardcoded Mesher helpers.
  - Build Mesher to an artifact-local binary with `meshc build mesher --output ...` so the proof rail does not churn the tracked `mesher/mesher` package output.
duration: ""
verification_result: passed
completed_at: 2026-04-04T07:40:44.534Z
blocker_discovered: false
---

# T02: Added a dedicated Mesher Postgres runtime proof rail with redacted `.tmp/m051-s01` artifacts.

**Added a dedicated Mesher Postgres runtime proof rail with redacted `.tmp/m051-s01` artifacts.**

## What Happened

Added a dedicated Mesher Postgres runtime proof rail for S01. The new `m051_mesher` support module starts a fresh Docker `postgres:16` container on a random host port, runs `meshc migrate mesher up`, builds Mesher to an artifact-local binary, spawns the runtime with the current T01 env contract, and captures redacted command/log/HTTP/DB artifacts under `.tmp/m051-s01/`. The new `e2e_m051_s01` target proves both missing `DATABASE_URL` fail-closed behavior and the real Postgres-backed maintainer path: seeded default project/API key, missing and invalid auth rejection, malformed event JSON rejection, accepted event ingest, persisted DB state, and readback through Mesher’s real settings/storage/issues/issue-events surfaces. No Mesher runtime or seed migration changes were needed once the new rail exercised the existing T01 contract directly.

## Verification

Ran `cargo test -p meshc --test e2e_m051_s01 -- --nocapture`, which executed 2 tests and passed both the missing-`DATABASE_URL` fail-closed scenario and the live Docker/Postgres migrate/build/run/ingest/readback scenario. Ran a direct `python3` artifact-contract check to confirm the expected retained files exist under `.tmp/m051-s01/` and that no artifact contains the raw `postgres://mesh:mesh@127.0.0.1:...` DSN prefix. There was no additional slice-level Verification section beyond this task-owned rail.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `/usr/bin/time -p cargo test -p meshc --test e2e_m051_s01 -- --nocapture` | 0 | ✅ pass | 22220ms |
| 2 | `/usr/bin/time -p python3 -c "from pathlib import Path; import sys; root=Path('.tmp/m051-s01'); required=['project-settings-ready.json','events-ingest-accepted.json','project-issues-readback.json','issue-events-readback.json','runtime.stdout.log','postgres.logs.txt']; files=[str(p) for p in root.rglob('*') if p.is_file()]; missing=[name for name in required if not any(path.endswith(name) for path in files)]; leaked=[str(p) for p in root.rglob('*') if p.is_file() and 'postgres://mesh:mesh@127.0.0.1:' in p.read_text(errors='ignore')]; print('missing=', missing); print('leaked=', leaked); sys.exit(0 if not missing and not leaked else 1)"` | 0 | ✅ pass | 780ms |


## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `compiler/meshc/tests/support/m051_mesher.rs`
- `compiler/meshc/tests/e2e_m051_s01.rs`
- `compiler/meshc/tests/support/mod.rs`
- `.gsd/KNOWLEDGE.md`
- `.gsd/DECISIONS.md`


## Deviations
None.

## Known Issues
None.
