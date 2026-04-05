---
id: S02
parent: M051
milestone: M051
provides:
  - Internal retained backend fixture at `scripts/fixtures/backend/reference-backend/` with maintainer-only runbook, deploy SQL, package tests, and source-only build/stage scripts.
  - Shared retained-backend harness support in `compiler/meshc/tests/support/m051_reference_backend.rs` plus rebinding of retained backend e2e rails away from `reference-backend/reference-backend`.
  - Authoritative maintained replay in `bash scripts/verify-m051-s02.sh` with retained runtime, fixture-smoke, contract-artifact, and proof-bundle markers under `.tmp/m051-s02/verify/`.
requires:
  - slice: S01
    provides: S01 already moved the maintained deeper real-app contract onto `mesher/`, so S02 could turn `reference-backend` into an internal retained proof surface without re-promoting it.
affects:
  - S03
  - S04
  - S05
key_files:
  - scripts/fixtures/backend/reference-backend/README.md
  - scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh
  - scripts/fixtures/backend/reference-backend/scripts/smoke.sh
  - compiler/meshc/tests/support/m051_reference_backend.rs
  - compiler/meshc/tests/e2e_reference_backend.rs
  - compiler/meshc/tests/e2e_m051_s02.rs
  - scripts/verify-m051-s02.sh
  - .gsd/PROJECT.md
key_decisions:
  - Keep the retained backend fixture source-only by building it with explicit `--output` artifact paths and leave repo-root `reference-backend/` in place as a compatibility copy until later slices retarget or delete it.
  - D382: Build the retained backend runtime binary into `.tmp/m051-s02/reference-backend-runtime/`, but keep staged deploy bundles outside the repo root and publish only bundle pointers/manifests back under `.tmp/m051-s02/`.
  - D383: Run `fixture-smoke` as its own phase and kill stale `.tmp/m051-s02/fixture-smoke/build/reference-backend` workers before reusing the shared DB-backed deploy/recovery rails.
patterns_established:
  - Retained proof fixtures should stay source-only and build with explicit `meshc build <fixture> --output <artifact>` paths; a tracked in-place binary is verifier drift, not an acceptable artifact.
  - Slice-owned assembled verifiers should publish `status.txt`, `current-phase.txt`, `phase-report.txt`, `full-contract.log`, and `latest-proof-bundle.txt` under `.tmp/<slice>/verify/` so downstream slices can consume one deterministic proof surface.
  - When a package-local smoke phase and later deploy/recovery phases share one Postgres database, stale worker cleanup belongs in the verifier contract itself rather than as ad hoc debugging.
observability_surfaces:
  - `.tmp/m051-s02/verify/status.txt`, `current-phase.txt`, `phase-report.txt`, `full-contract.log`, and `latest-proof-bundle.txt` from `bash scripts/verify-m051-s02.sh`
  - `reference-backend` runtime `/health` and `/jobs/:id` state during staged deploy, worker-crash recovery, worker-restart visibility, and process-restart recovery rails
  - Retained runtime artifacts under `.tmp/m051-s02/reference-backend-runtime/` and package-local smoke logs under `.tmp/m051-s02/fixture-smoke/reference-backend.log`
drill_down_paths:
  - .gsd/milestones/M051/slices/S02/tasks/T01-SUMMARY.md
  - .gsd/milestones/M051/slices/S02/tasks/T02-SUMMARY.md
  - .gsd/milestones/M051/slices/S02/tasks/T03-SUMMARY.md
duration: ""
verification_result: passed
completed_at: 2026-04-04T15:08:26.646Z
blocker_discovered: false
---

# S02: Extract retained backend-only proof out of reference-backend

**Internalized the retained backend-only `reference-backend` proof into a source-only fixture, shared `compiler/meshc` harness, and green slice-owned verifier while leaving repo-root `reference-backend/` only as a compatibility copy.**

## What Happened

S02 pulled the backend-only migration/deploy/health/recovery proof out of the repo-root `reference-backend/` app path and made it an internal retained surface. T01 copied the backend package into `scripts/fixtures/backend/reference-backend/`, preserved package identity and the backend-specific HTTP/jobs/recovery contract, added a maintainer-only README, and made the fixture-local `stage-deploy.sh` / `smoke.sh` build to explicit artifact paths so the retained fixture stays source-only. T02 added `compiler/meshc/tests/support/m051_reference_backend.rs` as the canonical retained-backend harness, rebound the retained `e2e_reference_backend` build/migrate/stage seams to the internal fixture, and added `compiler/meshc/tests/e2e_m051_s02.rs` so the slice owns fixture/readme/verifier/bundle-shape drift checks directly. T03 expanded the retained README into the maintainer runbook, added `scripts/verify-m051-s02.sh` as the assembled acceptance rail with `status.txt` / `current-phase.txt` / `phase-report.txt` / `full-contract.log` / `latest-proof-bundle.txt`, and retained the runtime, fixture-smoke, and contract-artifact bundles under `.tmp/m051-s02/verify/`.

During closeout, the remaining false-red came from the new verifier surface rather than the retained backend runtime itself: a leftover `.tmp/m051-s02/fixture-smoke/build/reference-backend` worker could keep polling the shared Postgres database after the package-local smoke phase and steal jobs from the later staged deploy smoke rail, leaving the staged worker's `/health.processed_jobs` counter at zero even though the staged binary and `deploy-smoke.sh` had both done the right thing. Cleaning those stale fixture-smoke workers before and after the verifier's DB-backed phases made the full retained replay green again. The result is one internal retained backend fixture, one shared `compiler/meshc` support module, and one maintainer-owned `bash scripts/verify-m051-s02.sh` replay that keep backend-only proof alive while leaving repo-root `reference-backend/` untouched for the later tooling/docs/deletion slices.

## Verification

Verified the retained fixture package tests with `cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests`; verified source-only staging with `tmp_dir="$(mktemp -d)" && bash scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh "$tmp_dir" && test -x "$tmp_dir/reference-backend" && test -f "$tmp_dir/reference-backend.up.sql"`; verified the slice-owned contract target with `cargo test -p meshc --test e2e_m051_s02 -- --nocapture`; verified the rebased legacy stage-deploy rail with `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_stage_deploy_bundle -- --nocapture`; and verified the full maintained replay with `DATABASE_URL=<disposable local Docker Postgres db> bash scripts/verify-m051-s02.sh`. The assembled verifier finished with `.tmp/m051-s02/verify/status.txt=ok`, `.tmp/m051-s02/verify/current-phase.txt=complete`, all required phases passed in `.tmp/m051-s02/verify/phase-report.txt`, and a retained proof bundle pointer at `.tmp/m051-s02/verify/latest-proof-bundle.txt`. A post-closeout process check also confirmed the new stale-worker cleanup left no `.tmp/m051-s02/fixture-smoke/build/reference-backend` process behind.

## Requirements Advanced

- R119 — S02 removes backend-only migration/deploy/health/recovery proof from the public repo-root app path and keeps that proof alive through an internal retained fixture plus maintainer-only verifier, which lets `mesher/` stay the maintained deeper reference app while later slices retarget tooling/docs and prepare final deletion of the legacy copy.

## Requirements Validated

None.

## New Requirements Surfaced

None.

## Requirements Invalidated or Re-scoped

None.

## Deviations

- Used a disposable local Docker Postgres database for the final DB-backed replay because this worktree had no repo-local `DATABASE_URL` configured.
- Added explicit stale `fixture-smoke` worker cleanup around the assembled verifier's shared-DB phases after a false-red `m051-s02-deploy-artifact-smoke` failure showed that leftover `.tmp/m051-s02/fixture-smoke/build/reference-backend` workers could steal later jobs.

## Known Limitations

- Repo-root `reference-backend/` still exists as a compatibility copy; S02 intentionally does not delete or public-retarget it yet.
- Tooling/editor/formatter/LSP rails still need the S03 cutover to the retained backend fixture.
- Public docs, scaffold guidance, and skills still need the S04 retarget away from repo-root `reference-backend/`.
- `bash scripts/verify-m051-s02.sh` proves the retained backend contract only when a disposable Postgres `DATABASE_URL` is available; database provisioning itself is still outside the verifier.

## Follow-ups

- S03 should rebind tooling/editor/LSP/formatter rails from repo-root `reference-backend/` to the retained backend fixture and shared `m051_reference_backend` support module.
- S04 should retarget public docs, scaffold output, and skill guidance away from repo-root `reference-backend/` while keeping the maintainer-only retained backend runbook discoverable where appropriate.
- S05 should delete the repo-root compatibility copy only after the S02 verifier, S03 tooling rails, and S04 docs retarget all point at the retained/internal surfaces.

## Files Created/Modified

- `scripts/fixtures/backend/reference-backend/` — Copied the backend package into an internal retained fixture with maintainer-only README, deploy SQL, tests, and fixture-local scripts that build to artifact paths instead of writing in place.
- `compiler/meshc/tests/support/m051_reference_backend.rs` — Added the shared retained-backend harness for canonical fixture paths, artifact-local runtime builds, migrate/stage helpers, and bundle-pointer utilities.
- `compiler/meshc/tests/e2e_reference_backend.rs` — Rebound retained backend rails to the internal fixture and kept the deeper DB-backed runtime/recovery scenarios on the legacy target through the new support module.
- `compiler/meshc/tests/e2e_m051_s02.rs` — Added slice-owned contract tests for retained fixture path resolution, source-only staging, bundle shape, and verifier/readme drift.
- `scripts/verify-m051-s02.sh` — Added the assembled retained-backend verifier with phase markers, retained-artifact copying, bundle-shape checks, and stale fixture-smoke cleanup around shared-DB phases.
- `.gsd/PROJECT.md` — Refreshed project current-state documentation to reflect that M051/S02 is complete and the retained backend proof now lives under the internal fixture plus `verify-m051-s02.sh`.
- `.gsd/KNOWLEDGE.md` — Appended the retained-backend verifier gotcha about stale fixture-smoke workers stealing later shared-DB jobs and how the assembled verifier now cleans them.
