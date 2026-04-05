---
id: T03
parent: S02
milestone: M051
provides: []
requires: []
affects: []
key_files: ["scripts/fixtures/backend/reference-backend/README.md", "scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh", "scripts/fixtures/backend/reference-backend/scripts/deploy-smoke.sh", "scripts/fixtures/backend/reference-backend/scripts/smoke.sh", "scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl", "scripts/verify-m051-s02.sh", "compiler/meshc/tests/e2e_m051_s02.rs", ".gsd/KNOWLEDGE.md"]
key_decisions: ["Keep the retained proof bundle itself outside the repo root and publish only a resolved pointer through `.tmp/m051-s02/verify/latest-proof-bundle.txt` when the assembled verifier reaches its retention phases.", "Run the package-local fixture smoke as its own verifier phase so `.tmp/m051-s02/fixture-smoke/` becomes a stable retained artifact root alongside the shared runtime build output."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "Passed `bash -n` on the updated retained scripts and verifier, `cargo test -p meshc --test e2e_m051_s02 -- --nocapture`, and `cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests`. In the assembled retained wrapper, migration-status/apply, package-local smoke, staged deploy smoke, worker-crash recovery, and worker-restart visibility all passed and were recorded in `.tmp/m051-s02/verify/phase-report.txt`. The authoritative replay still fails at `m051-s02-process-restart-recovery`; `.tmp/m051-s02/verify/m051-s02-process-restart-recovery.log` shows `e2e_reference_backend_process_restart_recovers_inflight_job` failing with `backend never exposed boot-recovery window with pending job visibility`."
completed_at: 2026-04-04T09:25:27.455Z
blocker_discovered: true
---

# T03: Added the retained backend runbook and fail-closed S02 verifier shell, but the final process-restart recovery rail is still red.

> Added the retained backend runbook and fail-closed S02 verifier shell, but the final process-restart recovery rail is still red.

## What Happened
---
id: T03
parent: S02
milestone: M051
key_files:
  - scripts/fixtures/backend/reference-backend/README.md
  - scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh
  - scripts/fixtures/backend/reference-backend/scripts/deploy-smoke.sh
  - scripts/fixtures/backend/reference-backend/scripts/smoke.sh
  - scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl
  - scripts/verify-m051-s02.sh
  - compiler/meshc/tests/e2e_m051_s02.rs
  - .gsd/KNOWLEDGE.md
key_decisions:
  - Keep the retained proof bundle itself outside the repo root and publish only a resolved pointer through `.tmp/m051-s02/verify/latest-proof-bundle.txt` when the assembled verifier reaches its retention phases.
  - Run the package-local fixture smoke as its own verifier phase so `.tmp/m051-s02/fixture-smoke/` becomes a stable retained artifact root alongside the shared runtime build output.
duration: ""
verification_result: mixed
completed_at: 2026-04-04T09:25:27.457Z
blocker_discovered: true
---

# T03: Added the retained backend runbook and fail-closed S02 verifier shell, but the final process-restart recovery rail is still red.

**Added the retained backend runbook and fail-closed S02 verifier shell, but the final process-restart recovery rail is still red.**

## What Happened

Expanded `scripts/fixtures/backend/reference-backend/README.md` into the maintainer-facing retained backend runbook, hardened the retained fixture scripts so they fail closed on missing commands/artifacts and stay pointed at the internal fixture path, added `scripts/verify-m051-s02.sh` as the slice-owned serial acceptance rail with `status.txt` / `current-phase.txt` / `phase-report.txt` / `full-contract.log`, and rewrote `compiler/meshc/tests/e2e_m051_s02.rs` to pin that README/verifier/package-script surface. The assembled verifier now gets through the cheap contract gate, the retained fixture package tests, the slice-owned Rust contract target, the migration-status/apply rail, the package-local smoke rail, staged deploy smoke, worker-crash recovery, and worker-restart visibility. The task remains blocked because the final required backend rail `e2e_reference_backend_process_restart_recovers_inflight_job` is red in the authoritative wrapper run: the backend no longer exposes the expected boot-recovery pending window before the recovered job is already `processed` and `/health.worker.recovery_active` is `false`.

## Verification

Passed `bash -n` on the updated retained scripts and verifier, `cargo test -p meshc --test e2e_m051_s02 -- --nocapture`, and `cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests`. In the assembled retained wrapper, migration-status/apply, package-local smoke, staged deploy smoke, worker-crash recovery, and worker-restart visibility all passed and were recorded in `.tmp/m051-s02/verify/phase-report.txt`. The authoritative replay still fails at `m051-s02-process-restart-recovery`; `.tmp/m051-s02/verify/m051-s02-process-restart-recovery.log` shows `e2e_reference_backend_process_restart_recovers_inflight_job` failing with `backend never exposed boot-recovery window with pending job visibility`.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `bash -n scripts/verify-m051-s02.sh && bash -n scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh && bash -n scripts/fixtures/backend/reference-backend/scripts/apply-deploy-migrations.sh && bash -n scripts/fixtures/backend/reference-backend/scripts/deploy-smoke.sh && bash -n scripts/fixtures/backend/reference-backend/scripts/smoke.sh` | 0 | ✅ pass | 120ms |
| 2 | `cargo test -p meshc --test e2e_m051_s02 -- --nocapture` | 0 | ✅ pass | 26260ms |
| 3 | `cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests` | 0 | ✅ pass | 2530ms |
| 4 | `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_migration_status_and_apply -- --ignored --nocapture` | 0 | ✅ pass | 7560ms |
| 5 | `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_deploy_artifact_smoke -- --ignored --nocapture` | 0 | ✅ pass | 31900ms |
| 6 | `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture` | 0 | ✅ pass | 12380ms |
| 7 | `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_restart_is_visible_in_health -- --ignored --nocapture` | 0 | ✅ pass | 17870ms |
| 8 | `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_process_restart_recovers_inflight_job -- --ignored --nocapture` | 1 | ❌ fail | 67240ms |


## Deviations

Used a disposable local Docker Postgres instance for the final DB-backed replay because neither the process environment nor the available repo-local env files supplied `DATABASE_URL` in this unit.

## Known Issues

`bash scripts/verify-m051-s02.sh` still fails at `m051-s02-process-restart-recovery`. The wrapper plumbing, retained README, retained package tests, migration rail, staged deploy rail, worker-crash rail, and worker-restart-visibility rail are in place, but the slice cannot close until `e2e_reference_backend_process_restart_recovers_inflight_job` is green again or the plan is explicitly changed.

## Files Created/Modified

- `scripts/fixtures/backend/reference-backend/README.md`
- `scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh`
- `scripts/fixtures/backend/reference-backend/scripts/deploy-smoke.sh`
- `scripts/fixtures/backend/reference-backend/scripts/smoke.sh`
- `scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl`
- `scripts/verify-m051-s02.sh`
- `compiler/meshc/tests/e2e_m051_s02.rs`
- `.gsd/KNOWLEDGE.md`


## Deviations
Used a disposable local Docker Postgres instance for the final DB-backed replay because neither the process environment nor the available repo-local env files supplied `DATABASE_URL` in this unit.

## Known Issues
`bash scripts/verify-m051-s02.sh` still fails at `m051-s02-process-restart-recovery`. The wrapper plumbing, retained README, retained package tests, migration rail, staged deploy rail, worker-crash rail, and worker-restart-visibility rail are in place, but the slice cannot close until `e2e_reference_backend_process_restart_recovers_inflight_job` is green again or the plan is explicitly changed.
