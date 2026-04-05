# S02: Extract retained backend-only proof out of reference-backend

**Goal:** Move the backend-only deploy, migration, health, and recovery proof into an internal retained fixture plus slice-owned verifier surface, while keeping the current top-level `reference-backend/` compatibility path intact until later slices can retarget or delete it.
**Demo:** After this: Maintainers can still replay the backend-specific deploy, recovery, and health-style proof that matters after retirement, but it now lives in retained harnesses or maintainer material instead of a public top-level app.

## Tasks
- [x] **T01: Materialized the internal `reference-backend` fixture under `scripts/fixtures/backend/` with artifact-local stage/smoke scripts and fixture-local contract tests.** — — Create the internal package-local home for the backend-only proof before changing the harnesses that consume it.

- Why: `reference-backend/` currently doubles as a public/docs anchor and the only backend proof fixture. S02 needs an internal home under `scripts/fixtures/` so the retained backend contract can outlive the top-level app path without breaking later S03/S04/S05 compatibility work.
- Files: `reference-backend/mesh.toml`, `reference-backend/main.mpl`, `reference-backend/jobs/worker.mpl`, `reference-backend/storage/jobs.mpl`, `reference-backend/migrations/20260323010000_create_jobs.mpl`, `reference-backend/scripts/stage-deploy.sh`, `reference-backend/README.md`, `scripts/fixtures/backend/reference-backend/mesh.toml`
- Do: copy the repo-root backend package into `scripts/fixtures/backend/reference-backend/`, preserve `name = "reference-backend"` plus the `/health`/jobs/recovery/deploy SQL contract, add a maintainer-only README at the new path, and make the retained fixture’s staging/build flow use artifact-local output so the new fixture stays source-only instead of accumulating a tracked binary.
- Verify: `cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests` and `tmp_dir="$(mktemp -d)" && bash scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh "$tmp_dir" && test -x "$tmp_dir/reference-backend" && test -f "$tmp_dir/reference-backend.up.sql"`
- Done when: the internal fixture exists under `scripts/fixtures/backend/reference-backend/`, builds/tests from its own path, stages a runnable deploy bundle, and does not write a tracked binary back into the retained fixture tree.

## Steps

1. Copy the Mesh source, tests, migration SQL, deploy SQL, and package-local scripts from `reference-backend/` into `scripts/fixtures/backend/reference-backend/` without copying the built `reference-backend/reference-backend` binary.
2. Preserve the manifest identity and backend behavior at the new path so `/health`, `POST /jobs`, `GET /jobs/:id`, and worker recovery semantics still belong to package `reference-backend`.
3. Add a retained fixture README that clearly marks the new path as maintainer-only/internal rather than a public first-contact app.
4. Rework the retained fixture’s stage-deploy/build flow to emit executable output into the bundle or `.tmp/m051-s02/` instead of the tracked source tree.

## Must-Haves

- [ ] `scripts/fixtures/backend/reference-backend/mesh.toml` still declares package `reference-backend`.
- [ ] The retained fixture keeps the migration/deploy SQL, worker, storage, and package-test surfaces intact.
- [ ] The retained fixture can be built/staged without creating a checked-in binary under `scripts/fixtures/backend/reference-backend/`.
- [ ] Repo-root `reference-backend/` remains available for compatibility consumers until later slices.
  - Estimate: 2h
  - Files: scripts/fixtures/backend/reference-backend/mesh.toml, scripts/fixtures/backend/reference-backend/main.mpl, scripts/fixtures/backend/reference-backend/jobs/worker.mpl, scripts/fixtures/backend/reference-backend/storage/jobs.mpl, scripts/fixtures/backend/reference-backend/migrations/20260323010000_create_jobs.mpl, scripts/fixtures/backend/reference-backend/deploy/reference-backend.up.sql, scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh, scripts/fixtures/backend/reference-backend/README.md
  - Verify: `cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests` and `tmp_dir="$(mktemp -d)" && bash scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh "$tmp_dir" && test -x "$tmp_dir/reference-backend" && test -f "$tmp_dir/reference-backend.up.sql"`
- [x] **T02: Rebound the retained backend e2e rails to the internal fixture and added M051/S02 contract tests.** — — Replace the monolithic path-hardcoded backend test target with a reusable retained-proof harness and a slice-owned contract rail.

- Why: `compiler/meshc/tests/e2e_reference_backend.rs` still hardcodes repo-root paths and in-place binaries, which makes the backend proof fragile and blocks later retirement of the top-level app path.
- Files: `compiler/meshc/tests/e2e_reference_backend.rs`, `compiler/meshc/tests/support/mod.rs`, `scripts/fixtures/backend/reference-backend/mesh.toml`, `scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh`
- Do: extract the shared build/migrate/spawn/http/db/recovery helpers into `compiler/meshc/tests/support/m051_reference_backend.rs`, add explicit helpers for the retained fixture path and artifact-local build outputs, rebind the retained backend scenarios to the new fixture home, and add `compiler/meshc/tests/e2e_m051_s02.rs` to pin the retained README/scripts/bundle-shape contract without forcing public-doc retargeting in this slice.
- Verify: `cargo test -p meshc --test e2e_m051_s02 -- --nocapture` and `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_stage_deploy_bundle -- --nocapture`
- Done when: one shared support module owns the retained backend proof helpers, the old runtime target no longer depends on `reference-backend/reference-backend`, and the new S02 e2e target fails closed on retained-surface drift.

## Steps

1. Create `compiler/meshc/tests/support/m051_reference_backend.rs` with canonical repo-root and retained-fixture path helpers plus artifact-local build/stage helpers.
2. Move the reusable HTTP/DB/process/recovery helpers out of `e2e_reference_backend.rs` and keep the legacy target delegating to the new support module.
3. Rebind at least the stage-deploy/bundle path to the retained fixture so proof commands stop depending on the tracked repo-root binary.
4. Add `compiler/meshc/tests/e2e_m051_s02.rs` contract tests for the retained README/scripts and copied artifact bundle shape under `.tmp/m051-s02/`.

## Must-Haves

- [ ] Shared retained backend helpers live in `compiler/meshc/tests/support/m051_reference_backend.rs`.
- [ ] `e2e_reference_backend` no longer hardcodes the repo-root built binary path for retained scenarios.
- [ ] `compiler/meshc/tests/e2e_m051_s02.rs` exists and runs >0 real tests.
- [ ] Retained artifacts and bundle pointers land under `.tmp/m051-s02/` instead of being mixed into older roots.
  - Estimate: 2h30m
  - Files: compiler/meshc/tests/support/m051_reference_backend.rs, compiler/meshc/tests/support/mod.rs, compiler/meshc/tests/e2e_reference_backend.rs, compiler/meshc/tests/e2e_m051_s02.rs, scripts/fixtures/backend/reference-backend/mesh.toml, scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh
  - Verify: `cargo test -p meshc --test e2e_m051_s02 -- --nocapture` and `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_stage_deploy_bundle -- --nocapture`
- [x] **T03: Added the retained backend runbook and fail-closed S02 verifier shell, but the final process-restart recovery rail is still red.** — — Close the slice with one internal maintainer runbook and one authoritative replay that keep the backend-only proof serial, DB-backed, and inspectable.

- Why: the slice is not done when the retained fixture and support helpers exist; maintainers need one slice-owned command that replays migration, staged deploy, `/health`, and restart recovery truth against the internal surface and keeps enough evidence for later deletion work.
- Files: `scripts/fixtures/backend/reference-backend/README.md`, `scripts/fixtures/backend/reference-backend/scripts/apply-deploy-migrations.sh`, `scripts/fixtures/backend/reference-backend/scripts/deploy-smoke.sh`, `scripts/fixtures/backend/reference-backend/scripts/smoke.sh`, `scripts/verify-m051-s02.sh`, `compiler/meshc/tests/e2e_m051_s02.rs`
- Do: expand the retained fixture README into the maintainer/backend-proof runbook, keep the package-local scripts pointed at the internal fixture and redaction-safe failure surfaces, add `scripts/verify-m051-s02.sh` as the authoritative S02 replay with phase/status/bundle markers plus 0-test guards, and make that wrapper replay the slice contract test together with the named DB-backed backend rails while leaving repo-root `reference-backend` compatibility files available for later S04/S05 retargeting.
- Verify: `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} bash scripts/verify-m051-s02.sh`
- Done when: the internal retained README/verifier are the maintainer authority for backend-only proof, `.tmp/m051-s02/verify/` preserves fresh phase logs and bundle pointers, and the retained replay still proves migrations, staged deploy, worker crash, restart visibility, and whole-process restart recovery.

## Steps

1. Write `scripts/fixtures/backend/reference-backend/README.md` as the retained maintainer runbook for staged deploy, migration apply, live smoke, `/health`, and recovery interpretation.
2. Keep the retained package-local scripts fail-closed on missing artifacts/env and aligned with the internal fixture path plus artifact-local binaries.
3. Implement `scripts/verify-m051-s02.sh` to run cheap contract checks first, then the slice e2e target, then the named DB-backed migration/deploy/recovery rails serially with retained artifact copying under `.tmp/m051-s02/verify/`.
4. Preserve repo-root `reference-backend/` compatibility surfaces for later slices instead of deleting or rewriting first-contact docs here.

## Must-Haves

- [ ] `scripts/fixtures/backend/reference-backend/README.md` is the canonical retained maintainer runbook for backend-only proof.
- [ ] `scripts/verify-m051-s02.sh` is the authoritative S02 acceptance rail and fails closed on skipped tests, missing artifacts, and secret leakage.
- [ ] Fresh `.tmp/m051-s02/verify/` runs keep status, current phase, phase report, bundle pointer, and copied runtime artifacts.
- [ ] The retained replay covers migration status/apply, staged deploy smoke, worker crash recovery, restart visibility, and process-restart recovery.
  - Estimate: 2h
  - Files: scripts/fixtures/backend/reference-backend/README.md, scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh, scripts/fixtures/backend/reference-backend/scripts/apply-deploy-migrations.sh, scripts/fixtures/backend/reference-backend/scripts/deploy-smoke.sh, scripts/fixtures/backend/reference-backend/scripts/smoke.sh, scripts/verify-m051-s02.sh, compiler/meshc/tests/e2e_m051_s02.rs
  - Verify: `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} bash scripts/verify-m051-s02.sh`
  - Blocker: `bash scripts/verify-m051-s02.sh` still fails at `m051-s02-process-restart-recovery`. The wrapper plumbing, retained README, retained package tests, migration rail, staged deploy rail, worker-crash rail, and worker-restart-visibility rail are in place, but the slice cannot close until `e2e_reference_backend_process_restart_recovers_inflight_job` is green again or the plan is explicitly changed.
