---
id: T01
parent: S05
milestone: M051
provides: []
requires: []
affects: []
key_files: ["scripts/verify-production-proof-surface.sh", "reference-backend/scripts/verify-production-proof-surface.sh", "website/docs/docs/production-backend-proof/index.md", "scripts/verify-m051-s02.sh", "compiler/meshc/tests/e2e_m051_s04.rs"]
key_decisions: ["D389: Canonicalize the public proof-page verifier at scripts/verify-production-proof-surface.sh and leave the reference-backend copy as a temporary wrapper until the compatibility tree is removed."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "Task-specific verification passed with bash scripts/verify-production-proof-surface.sh, cargo test -p meshc --test e2e_m050_s01 -- --nocapture, and a rerun of cargo test -p meshc --test e2e_m050_s03 -- --nocapture after fixing a stale source-contract expectation. I also ran cargo test -p meshc --test e2e_m051_s02 -- --nocapture to confirm one touched slice-level M051 rail. The remaining DB-backed slice-level replays were not rerun in this task because secure DATABASE_URL collection was skipped and the hard-timeout recovery cut execution before I could add the local Docker-backed DATABASE_URL harness the user requested."
completed_at: 2026-04-04T21:41:14.216Z
blocker_discovered: false
---

# T01: Moved the public proof-page verifier to scripts/ and retargeted the surviving docs and contract rails to the new canonical path.

> Moved the public proof-page verifier to scripts/ and retargeted the surviving docs and contract rails to the new canonical path.

## What Happened
---
id: T01
parent: S05
milestone: M051
key_files:
  - scripts/verify-production-proof-surface.sh
  - reference-backend/scripts/verify-production-proof-surface.sh
  - website/docs/docs/production-backend-proof/index.md
  - scripts/verify-m051-s02.sh
  - compiler/meshc/tests/e2e_m051_s04.rs
key_decisions:
  - D389: Canonicalize the public proof-page verifier at scripts/verify-production-proof-surface.sh and leave the reference-backend copy as a temporary wrapper until the compatibility tree is removed.
duration: ""
verification_result: mixed
completed_at: 2026-04-04T21:41:14.217Z
blocker_discovered: false
---

# T01: Moved the public proof-page verifier to scripts/ and retargeted the surviving docs and contract rails to the new canonical path.

**Moved the public proof-page verifier to scripts/ and retargeted the surviving docs and contract rails to the new canonical path.**

## What Happened

Created scripts/verify-production-proof-surface.sh as the canonical verifier, corrected its repo-root calculation for the top-level scripts directory, and converted reference-backend/scripts/verify-production-proof-surface.sh into a temporary wrapper so the compatibility tree can survive until the delete task without diverging. Updated the public Production Backend Proof page plus the direct historical callers named in the plan, then widened the retargeting to the later slice rails that still asserted the nested path: the secondary-surface contract, retained backend fixture/readme boundary, scripts/verify-m051-s02.sh, compiler/meshc/tests/e2e_m051_s02.rs, scripts/verify-m051-s04.sh, and compiler/meshc/tests/e2e_m051_s04.rs. One historical Rust rail (e2e_m050_s03) failed on the first rerun because its source contract still expected older proof-page wording; after updating that contract to the current proof-page sections, the rerun passed.

## Verification

Task-specific verification passed with bash scripts/verify-production-proof-surface.sh, cargo test -p meshc --test e2e_m050_s01 -- --nocapture, and a rerun of cargo test -p meshc --test e2e_m050_s03 -- --nocapture after fixing a stale source-contract expectation. I also ran cargo test -p meshc --test e2e_m051_s02 -- --nocapture to confirm one touched slice-level M051 rail. The remaining DB-backed slice-level replays were not rerun in this task because secure DATABASE_URL collection was skipped and the hard-timeout recovery cut execution before I could add the local Docker-backed DATABASE_URL harness the user requested.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `bash scripts/verify-production-proof-surface.sh` | 0 | ✅ pass | 2480ms |
| 2 | `cargo test -p meshc --test e2e_m050_s01 -- --nocapture` | 0 | ✅ pass | 8640ms |
| 3 | `cargo test -p meshc --test e2e_m050_s03 -- --nocapture` | 101 | ❌ fail | 4100ms |
| 4 | `cargo test -p meshc --test e2e_m050_s03 -- --nocapture` | 0 | ✅ pass | 5090ms |
| 5 | `cargo test -p meshc --test e2e_m051_s02 -- --nocapture` | 0 | ✅ pass | 69891ms |


## Deviations

Also updated scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs, scripts/fixtures/backend/reference-backend/README.md, reference-backend/README.md, scripts/verify-m051-s02.sh, compiler/meshc/tests/e2e_m051_s02.rs, scripts/verify-m051-s04.sh, and compiler/meshc/tests/e2e_m051_s04.rs because they still asserted the retiring nested verifier path and would have made later delete work fail closed for the wrong reason.

## Known Issues

The remaining DB-backed slice-level replays are still pending. The repo needs a local Docker-backed DATABASE_URL harness before bash scripts/verify-m051-s02.sh and bash scripts/verify-m051-s05.sh can be exercised without asking for secrets.

## Files Created/Modified

- `scripts/verify-production-proof-surface.sh`
- `reference-backend/scripts/verify-production-proof-surface.sh`
- `website/docs/docs/production-backend-proof/index.md`
- `scripts/verify-m051-s02.sh`
- `compiler/meshc/tests/e2e_m051_s04.rs`


## Deviations
Also updated scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs, scripts/fixtures/backend/reference-backend/README.md, reference-backend/README.md, scripts/verify-m051-s02.sh, compiler/meshc/tests/e2e_m051_s02.rs, scripts/verify-m051-s04.sh, and compiler/meshc/tests/e2e_m051_s04.rs because they still asserted the retiring nested verifier path and would have made later delete work fail closed for the wrong reason.

## Known Issues
The remaining DB-backed slice-level replays are still pending. The repo needs a local Docker-backed DATABASE_URL harness before bash scripts/verify-m051-s02.sh and bash scripts/verify-m051-s05.sh can be exercised without asking for secrets.
