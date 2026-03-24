# S07: Recovery Proof Closure

**Goal:** Close Mesh’s unfinished recovery-trust contract on `reference-backend/` by moving worker crash recovery onto a real supervisor restart boundary, making abandoned-job reclaim concurrency-safe, and proving both worker-level and whole-process recovery through the canonical backend harness.
**Demo:** `compiler/meshc/tests/e2e_reference_backend.rs` can drive `reference-backend/` into a real degraded/recovering window, observe trustworthy `/health` restart metadata, and prove that both a crashed worker and a killed backend process reclaim and finish the same in-flight job exactly once without breaking the staged deployment/migration contract.

## Decomposition Rationale

S07 is a root-cause slice, not a docs slice. The research shows the current failure comes from fake restart behavior inside `reference-backend/jobs/worker.mpl`: the crashing actor is mutating post-restart state before any real child exit happens, and the storage layer still requeues every `processing` row with no staleness guard. That means the first task has to reestablish a real restart boundary and a safe reclaim contract before anything about health or process-restart proof can become trustworthy.

Once recovery is owned by the restarted worker instead of the crashing one, the next risk is observability drift. The existing `/health` surface already has the right rough shape, but it still derives liveness mostly from status strings and has already shown corrupted or misleading metadata under the slower restart-visibility proof. The second task therefore hardens the health contract around real lifecycle evidence so future failures are diagnosable without guessing from logs.

The final task adds the missing end-to-end closure that S06 prematurely advertised: a deterministic whole-process restart proof in the same `compiler/meshc/tests/e2e_reference_backend.rs` harness. Keeping all recovery proof in that one file preserves the milestone pattern established in S02/S04 and gives S08 one truthful runtime surface to reconcile publicly.

## Must-Haves

- S07 must directly close **R004** by making worker crash/restart recovery real: the crashing worker records exit intent and exits, while only the restarted child performs boot bookkeeping and abandoned-job reclaim.
- S07 must close the remaining concurrency-trust hole by replacing blanket `processing`-row reclaim with a stale/lease-style recovery contract in `reference-backend/storage/jobs.mpl`, keeping `reference-backend/migrations/20260323010000_create_jobs.mpl` and `reference-backend/deploy/reference-backend.up.sql` aligned if SQL/index changes are needed.
- S07 must make `/health` a trustworthy recovery surface by exposing a real degraded/recovering window, stable restart metadata, and liveness derived from lifecycle evidence such as `tick_age_ms`, not just raw status strings.
- S07 must end with passing worker-crash, restart-visibility, and whole-process-restart proofs in `compiler/meshc/tests/e2e_reference_backend.rs`, plus the migration/deploy regressions needed to keep S04’s artifact-first deployment path honest.

## Proof Level

- This slice proves: operational
- Real runtime required: yes
- Human/UAT required: no

## Verification

- `cargo run -p meshc -- build reference-backend`
- `cargo run -p meshc -- fmt --check reference-backend`
- `cargo run -p meshc -- test reference-backend`
- `set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture`
- `set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_restart_is_visible_in_health -- --ignored --nocapture`
- `set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_process_restart_recovers_inflight_job -- --ignored --nocapture`
- `set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_migration_status_and_apply -- --ignored --nocapture`
- `set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_deploy_artifact_smoke -- --ignored --nocapture`

## Observability / Diagnostics

- Runtime signals: `GET /health` must show coherent worker `status`, `liveness`, `tick_age_ms`, `restart_count`, `last_exit_reason`, `recovered_jobs`, and `recovery_active` transitions across crash and restart proofs.
- Inspection surfaces: `compiler/meshc/tests/e2e_reference_backend.rs`, `GET /health`, `GET /jobs/:id`, direct Postgres reads from `jobs` and `_mesh_migrations`, and named worker logs such as `Job worker boot`, `Job worker recovered`, and `Job worker processed`.
- Failure visibility: future agents must be able to tell whether breakage is in restart ownership, stale-row reclaim, health-state transitions, or process-restart closure by rerunning one named ignored test and inspecting the last health payload plus durable DB row state.
- Redaction constraints: proof output must not echo `DATABASE_URL`; diagnostics stay on health JSON, job ids, timestamps, counters, SQL artifact paths, and named runtime phases.

## Integration Closure

- Upstream surfaces consumed: `reference-backend/jobs/worker.mpl`, `reference-backend/storage/jobs.mpl`, `reference-backend/api/health.mpl`, `reference-backend/migrations/20260323010000_create_jobs.mpl`, `reference-backend/deploy/reference-backend.up.sql`, and `compiler/meshc/tests/e2e_reference_backend.rs`.
- New wiring introduced in this slice: real supervisor-owned recovery, stale/lease-style abandoned-job reclaim, health liveness derived from tick-age-aware lifecycle rules, and a deterministic in-flight seam for whole-process restart proof.
- What remains before the milestone is truly usable end-to-end: S08 still needs to reconcile README/website/UAT surfaces so they point only at these now-green recovery-aware proof paths.

## Tasks

- [x] **T01: Move recovery onto the real supervisor restart boundary** `est:3h`
  - Why: The current recovery contract is untrustworthy because `crash_after_claim(...)` simulates reboot/recovery inside the crashing worker and `reclaim_processing_jobs(...)` still requeues every `processing` row with no concurrency guard.
  - Files: `reference-backend/jobs/worker.mpl`, `reference-backend/storage/jobs.mpl`, `reference-backend/migrations/20260323010000_create_jobs.mpl`, `reference-backend/deploy/reference-backend.up.sql`, `compiler/meshc/tests/e2e_reference_backend.rs`
  - Do: Remove fake in-actor boot bookkeeping from the crash path, make the worker record exit intent then actually exit, move all boot/reclaim work behind the restarted child path, and replace blanket `processing` reclaim with a stale/lease-style contract derived from real timing so shared-DB recovery cannot steal live work; keep canonical migration and staged deploy SQL aligned with any recovery query/index changes.
  - Verify: `cargo run -p meshc -- build reference-backend && cargo run -p meshc -- fmt --check reference-backend && set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_migration_status_and_apply -- --ignored --nocapture && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_deploy_artifact_smoke -- --ignored --nocapture`
  - Done when: crash recovery is owned by the restarted worker, abandoned-job reclaim is stale/lease-guarded instead of blanket, and the worker-crash proof plus migration/deploy regressions pass on the updated contract.
- [x] **T02: Stabilize recovery health and restart visibility** `est:2h`
  - Why: After the restart boundary is real, S07 still needs `/health` to expose a deterministic degraded/recovering window and coherent metadata instead of stale-green or corrupted restart fields.
  - Files: `reference-backend/api/health.mpl`, `reference-backend/jobs/worker.mpl`, `compiler/meshc/tests/e2e_reference_backend.rs`
  - Do: Tie worker liveness to explicit lifecycle evidence such as `tick_age_ms`, preserve coherent `boot_id`/`started_at`/`last_recovery_*` fields across restarts, and tighten the slower restart-visibility proof so it asserts degraded recovery, pending-row visibility during reclaim, and healthy settlement after the restarted worker resumes processing.
  - Verify: `set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_restart_is_visible_in_health -- --ignored --nocapture`
  - Done when: the 500ms recovery proof reliably observes degraded/recovering health with stable metadata, then returns to healthy without null/corrupted fields or runtime panic.
- [x] **T03: Add deterministic whole-process restart recovery proof** `est:3h`
  - Why: S07 is not closed until the canonical harness can kill the entire backend process during an in-flight job, restart it, and prove the recovered job completes on the real reference backend path.
  - Files: `reference-backend/jobs/worker.mpl`, `reference-backend/storage/jobs.mpl`, `compiler/meshc/tests/e2e_reference_backend.rs`
  - Do: Add a deterministic in-flight hold-after-claim seam that the Rust harness can trigger without creating a second test surface, implement `e2e_reference_backend_process_restart_recovers_inflight_job` in the canonical harness, and rerun the full slice gate set so worker-crash, restart-visibility, process-restart, migration, deploy, build, fmt, and project tests all agree on the final recovery contract.
  - Verify: `cargo run -p meshc -- build reference-backend && cargo run -p meshc -- fmt --check reference-backend && cargo run -p meshc -- test reference-backend && set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_restart_is_visible_in_health -- --ignored --nocapture && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_process_restart_recovers_inflight_job -- --ignored --nocapture && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_migration_status_and_apply -- --ignored --nocapture && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_deploy_artifact_smoke -- --ignored --nocapture`
  - Done when: the missing whole-process restart proof exists in `compiler/meshc/tests/e2e_reference_backend.rs` and the full slice verification list passes against the same backend path S08 will later document.

## Files Likely Touched

- `reference-backend/jobs/worker.mpl`
- `reference-backend/storage/jobs.mpl`
- `reference-backend/api/health.mpl`
- `reference-backend/migrations/20260323010000_create_jobs.mpl`
- `reference-backend/deploy/reference-backend.up.sql`
- `compiler/meshc/tests/e2e_reference_backend.rs`
