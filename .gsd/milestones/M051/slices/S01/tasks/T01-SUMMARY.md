---
id: T01
parent: S01
milestone: M051
provides: []
requires: []
affects: []
key_files: ["mesher/config.mpl", "mesher/tests/config.test.mpl", "mesher/main.mpl", "mesher/ingestion/pipeline.mpl", "mesher/.env.example"]
key_decisions: ["Validate Mesher env config and open the Postgres pool before calling Node.start_from_env(), and use Node.start_from_env() as the only clustered bootstrap path."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "Ran `cargo run -q -p meshc -- test mesher/tests` and `cargo run -q -p meshc -- build mesher`. The package tests passed with the new config-helper coverage, the Mesher build completed successfully, and no extra slice-level verification was defined beyond those T01 rails."
completed_at: 2026-04-04T07:28:52.804Z
blocker_discovered: false
---

# T01: Moved Mesher startup to the scaffold-style env contract and replaced app-owned cluster bootstrap with Node.start_from_env().

> Moved Mesher startup to the scaffold-style env contract and replaced app-owned cluster bootstrap with Node.start_from_env().

## What Happened
---
id: T01
parent: S01
milestone: M051
key_files:
  - mesher/config.mpl
  - mesher/tests/config.test.mpl
  - mesher/main.mpl
  - mesher/ingestion/pipeline.mpl
  - mesher/.env.example
key_decisions:
  - Validate Mesher env config and open the Postgres pool before calling Node.start_from_env(), and use Node.start_from_env() as the only clustered bootstrap path.
duration: ""
verification_result: passed
completed_at: 2026-04-04T07:28:52.805Z
blocker_discovered: false
---

# T01: Moved Mesher startup to the scaffold-style env contract and replaced app-owned cluster bootstrap with Node.start_from_env().

**Moved Mesher startup to the scaffold-style env contract and replaced app-owned cluster bootstrap with Node.start_from_env().**

## What Happened

Added a dedicated Config module for Mesher's startup env contract, package tests that pin the key names/defaults/error text, and rewrote mesher/main.mpl around config validation, Postgres pool open, and runtime-owned bootstrap through Node.start_from_env(). Updated pipeline startup to consume validated rate-limit values and added a maintainer-facing .env.example matching the new DATABASE_URL/PORT/MESHER_WS_PORT/MESH_* contract.

## Verification

Ran `cargo run -q -p meshc -- test mesher/tests` and `cargo run -q -p meshc -- build mesher`. The package tests passed with the new config-helper coverage, the Mesher build completed successfully, and no extra slice-level verification was defined beyond those T01 rails.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo run -q -p meshc -- test mesher/tests` | 0 | ✅ pass | 19600ms |
| 2 | `cargo run -q -p meshc -- build mesher` | 0 | ✅ pass | 9900ms |


## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `mesher/config.mpl`
- `mesher/tests/config.test.mpl`
- `mesher/main.mpl`
- `mesher/ingestion/pipeline.mpl`
- `mesher/.env.example`


## Deviations
None.

## Known Issues
None.
