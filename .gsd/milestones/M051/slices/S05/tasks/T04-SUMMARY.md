---
id: T04
parent: S05
milestone: M051
provides: []
requires: []
affects: []
key_files: ["compiler/meshc/tests/e2e_m051_s05.rs", "scripts/verify-m051-s05.sh", "compiler/meshc/tests/e2e_reference_backend.rs", ".gsd/KNOWLEDGE.md"]
key_decisions: ["The S05 retained bundle should copy each delegated S01-S04 verify tree and its pointed proof bundle, then rewrite the copied child `latest-proof-bundle.txt` pointer to the copied bundle path so the final bundle stays self-contained.", "Resume the remaining red state in `scripts/fixtures/backend/reference-backend/scripts/smoke.sh` and its startup readiness gate rather than reopening the new S05 wrapper or the staged deploy-artifact ownership fix."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "`bash -n scripts/verify-m051-s05.sh` passed. `cargo test -p meshc --test e2e_m051_s05 -- --nocapture` passed. The previously red targeted rail `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_deploy_artifact_smoke -- --ignored --nocapture` passed against the live local Docker-backed Postgres URL after the staged-worker ownership fix. The full `bash scripts/verify-m051-s05.sh` replay failed in the delegated S02 fixture-smoke phase; inspect `.tmp/m051-s05/verify/m051-s02-wrapper.log`, `.tmp/m051-s02/verify/m051-s02-fixture-smoke.log`, and `.tmp/m051-s02/fixture-smoke/reference-backend.log`. The slice-level verification excerpt commands were not re-run because the authoritative S05 assembled rail failed earlier in the child wrapper."
completed_at: 2026-04-04T22:55:58.718Z
blocker_discovered: false
---

# T04: Added the M051 S05 post-deletion contract and wrapper, but the final replay still fails in the retained S02 fixture-smoke handoff.

> Added the M051 S05 post-deletion contract and wrapper, but the final replay still fails in the retained S02 fixture-smoke handoff.

## What Happened
---
id: T04
parent: S05
milestone: M051
key_files:
  - compiler/meshc/tests/e2e_m051_s05.rs
  - scripts/verify-m051-s05.sh
  - compiler/meshc/tests/e2e_reference_backend.rs
  - .gsd/KNOWLEDGE.md
key_decisions:
  - The S05 retained bundle should copy each delegated S01-S04 verify tree and its pointed proof bundle, then rewrite the copied child `latest-proof-bundle.txt` pointer to the copied bundle path so the final bundle stays self-contained.
  - Resume the remaining red state in `scripts/fixtures/backend/reference-backend/scripts/smoke.sh` and its startup readiness gate rather than reopening the new S05 wrapper or the staged deploy-artifact ownership fix.
duration: ""
verification_result: mixed
completed_at: 2026-04-04T22:55:58.721Z
blocker_discovered: false
---

# T04: Added the M051 S05 post-deletion contract and wrapper, but the final replay still fails in the retained S02 fixture-smoke handoff.

**Added the M051 S05 post-deletion contract and wrapper, but the final replay still fails in the retained S02 fixture-smoke handoff.**

## What Happened

Added `compiler/meshc/tests/e2e_m051_s05.rs` as the slice-owned post-deletion source contract and `scripts/verify-m051-s05.sh` as the assembled post-deletion replay with delegated S01-S04 wrapper execution plus copied child verify trees and proof bundles. While reproducing the existing child-rail state, I also fixed `compiler/meshc/tests/e2e_reference_backend.rs` so the staged deploy-artifact test no longer reuses the already-processed smoke job when it waits for a staged worker-owned job. The remaining failure is still in the delegated S02 wrapper: `scripts/fixtures/backend/reference-backend/scripts/smoke.sh` hands off to `deploy-smoke.sh` on the first 200 `/health` response even when the worker still reports `status=degraded`, `liveness=recovering`, and `recovery_active=true`, so the created job stays `pending` and the assembled S05 replay stops in `m051-s02-wrapper` before the final retain/copy phases run.

## Verification

`bash -n scripts/verify-m051-s05.sh` passed. `cargo test -p meshc --test e2e_m051_s05 -- --nocapture` passed. The previously red targeted rail `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_deploy_artifact_smoke -- --ignored --nocapture` passed against the live local Docker-backed Postgres URL after the staged-worker ownership fix. The full `bash scripts/verify-m051-s05.sh` replay failed in the delegated S02 fixture-smoke phase; inspect `.tmp/m051-s05/verify/m051-s02-wrapper.log`, `.tmp/m051-s02/verify/m051-s02-fixture-smoke.log`, and `.tmp/m051-s02/fixture-smoke/reference-backend.log`. The slice-level verification excerpt commands were not re-run because the authoritative S05 assembled rail failed earlier in the child wrapper.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `bash -n scripts/verify-m051-s05.sh` | 0 | ✅ pass | 0ms |
| 2 | `cargo test -p meshc --test e2e_m051_s05 -- --nocapture` | 0 | ✅ pass | 5200ms |
| 3 | `DATABASE_URL=<local-docker-url> cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_deploy_artifact_smoke -- --ignored --nocapture` | 0 | ✅ pass | 52690ms |
| 4 | `DATABASE_URL=<local-docker-url> bash scripts/verify-m051-s05.sh` | 1 | ❌ fail | 284000ms |


## Deviations

Touched `compiler/meshc/tests/e2e_reference_backend.rs` even though the written task plan only named the new S05 contract and wrapper files. That existing S02 child rail was still red from a stale staged-job ownership assumption, and the S05 assembled verifier could not become honest without fixing it first.

## Known Issues

`bash scripts/verify-m051-s05.sh` is still red in the delegated S02 fixture-smoke phase. `scripts/fixtures/backend/reference-backend/scripts/smoke.sh` currently treats the first successful `/health` response as ready even when the worker is still in the degraded startup window, so `deploy-smoke.sh` can create a job before the worker is truthfully healthy and leave it stuck at `pending`. Because the assembled S05 replay did not reach its retain/copy phases, `.tmp/m051-s05/verify/latest-proof-bundle.txt` still points at the in-progress verify directory rather than a completed retained proof bundle.

## Files Created/Modified

- `compiler/meshc/tests/e2e_m051_s05.rs`
- `scripts/verify-m051-s05.sh`
- `compiler/meshc/tests/e2e_reference_backend.rs`
- `.gsd/KNOWLEDGE.md`


## Deviations
Touched `compiler/meshc/tests/e2e_reference_backend.rs` even though the written task plan only named the new S05 contract and wrapper files. That existing S02 child rail was still red from a stale staged-job ownership assumption, and the S05 assembled verifier could not become honest without fixing it first.

## Known Issues
`bash scripts/verify-m051-s05.sh` is still red in the delegated S02 fixture-smoke phase. `scripts/fixtures/backend/reference-backend/scripts/smoke.sh` currently treats the first successful `/health` response as ready even when the worker is still in the degraded startup window, so `deploy-smoke.sh` can create a job before the worker is truthfully healthy and leave it stuck at `pending`. Because the assembled S05 replay did not reach its retain/copy phases, `.tmp/m051-s05/verify/latest-proof-bundle.txt` still points at the in-progress verify directory rather than a completed retained proof bundle.
