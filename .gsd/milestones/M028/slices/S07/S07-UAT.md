# S07 UAT — Recovery Proof Closure

**Milestone:** M028  
**Slice:** S07  
**Scope:** verify that `reference-backend/` exposes a real degraded/recovering window, reclaims abandoned work safely, and finishes the same durable job exactly once after both worker crash and whole-process restart.

## Preconditions
1. Run from repo root: `/Users/sn0w/Documents/dev/mesh-lang/.gsd/worktrees/M028`
2. `.env` contains a working `DATABASE_URL`
3. Postgres is reachable from this worktree
4. No other long-lived `reference-backend` process is bound to the test ports the harness selects
5. Use the repo-root `.env` for all ignored `e2e_reference_backend` commands:
   - `set -a && source .env && set +a`

## Test Case 1 — Worker crash recovery is visible and exact-once

### Goal
Prove that a worker can crash after claiming a job, expose a degraded/recovering window, requeue the job back to `pending`, then finish it exactly once after restart.

### Steps
1. Run:
   - `cargo run -p meshc -- build reference-backend`
   - `cargo run -p meshc -- fmt --check reference-backend`
2. Run:
   - `set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture`

### Expected outcomes
1. The test exits `0`.
2. Test logs show the sequence below in substance:
   - initial `Job worker boot ... restart_count=0`
   - `Job worker claimed ... attempts=1`
   - `Job worker crash injected ...`
   - restarted `Job worker boot ... restart_count=1`
   - `Job worker recovered jobs=1 ...`
   - `Job worker claimed ... attempts=2`
   - `Job worker processed ... attempts=2`
3. During the test, the authoritative health/job surfaces observe a degraded/recovering window before final healthy settlement:
   - `/health.status == "degraded"`
   - `/health.worker.liveness == "recovering"`
   - `/health.worker.restart_count == 1`
   - `/health.worker.recovery_active == true`
   - `/jobs/:id.status == "pending"`
   - durable `jobs.status == pending`
   - durable `jobs.attempts == 1`
4. Final settlement shows exact-once completion after restart:
   - `/jobs/:id.status == "processed"`
   - `/jobs/:id.attempts == 2`
   - final `/health.status == "ok"`
   - final `/health.worker.recovery_active == false`
   - durable job row is `processed`, not duplicated or left `processing`

## Test Case 2 — Restart metadata stays coherent in `/health`

### Goal
Prove that the slower recovery-visibility proof sees stable boot/restart metadata rather than null/corrupted fields.

### Steps
1. Run:
   - `set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_restart_is_visible_in_health -- --ignored --nocapture`

### Expected outcomes
1. The test exits `0`.
2. The proof observes an initial healthy worker, then a degraded/recovering worker, then healthy settlement again.
3. Across the restart boundary:
   - `boot_id` is non-empty before and after restart
   - `started_at` is non-empty before and after restart
   - restarted `boot_id` differs from the initial `boot_id`
   - restarted `started_at` differs from the initial `started_at`
4. Recovery metadata remains coherent after settlement:
   - `restart_count == 1`
   - `last_exit_reason == "worker_crash_after_claim"`
   - `recovered_jobs == 1`
   - `last_recovery_at` is non-null / non-empty
   - `last_recovery_job_id` matches the crashed job id
   - `last_recovery_count == 1`

## Test Case 3 — Whole-process restart recovers an in-flight job

### Goal
Prove that killing the entire backend process during an in-flight claimed job still leads to exact-once completion after restart.

### Steps
1. Run:
   - `set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_process_restart_recovers_inflight_job -- --ignored --nocapture`

### Expected outcomes
1. The test exits `0`.
2. The harness creates a job with the deterministic in-flight seam (`hold_after_claim_once`) and confirms the job is in `processing` before kill.
3. The harness kills the backend process, restarts it, and then observes boot recovery.
4. The durable row transitions through the intended process-restart contract:
   - before kill: `processing`, `attempts == 1`
   - after boot recovery: `pending`, still `attempts == 1`
   - final settlement: `processed`, `attempts == 2`
5. Final health is healthy, not stale or degraded.
6. No second duplicate job row is created; the original job id is the one that finishes.

## Test Case 4 — Migration and staged deploy truth still match the recovery contract

### Goal
Prove that the recovery/storage changes did not break the canonical Mesh migration path or the staged deploy artifact path.

### Steps
1. Run:
   - `set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_migration_status_and_apply -- --ignored --nocapture`
2. Run:
   - `set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_deploy_artifact_smoke -- --ignored --nocapture`

### Expected outcomes
1. Both tests exit `0`.
2. Mesh migration status/apply still works against the canonical migration source.
3. The staged deploy artifact still boots and processes jobs successfully.
4. Both paths expose the recovery-aligned schema, including the processing-reclaim index added for stale-cutoff recovery.
5. `_mesh_migrations` remains coherent after apply; staged deploy smoke still reaches healthy `/health` and successful job processing.

## Edge Cases to watch while running the script

### Edge Case A — Recovery happens too fast to observe
If worker-crash proof regresses by skipping straight to healthy, that is a failure. The slice contract requires a real observable degraded/recovering window, not only eventual success.

### Edge Case B — Worker restarts but abandoned job is never requeued
If `/health.worker.restart_count` increments but the durable row stays `processing`, recovery ownership is broken. That is a slice regression.

### Edge Case C — Whole-process restart finishes the wrong job or duplicates work
If the final proof shows multiple processed rows, a changed job id, or attempts not equal to `2`, exact-once recovery regressed.

### Edge Case D — Migration/deploy drift
If migration status/apply passes but staged deploy smoke fails, or vice versa, treat that as schema artifact drift between:
- `reference-backend/migrations/20260323010000_create_jobs.mpl`
- `reference-backend/deploy/reference-backend.up.sql`

## Minimal acceptance checklist
- [ ] `meshc build` passes for `reference-backend`
- [ ] `meshc fmt --check` passes for `reference-backend`
- [ ] `meshc test` passes for `reference-backend`
- [ ] worker-crash recovery proof passes
- [ ] restart-visibility proof passes
- [ ] whole-process restart proof passes
- [ ] migration status/apply proof passes
- [ ] staged deploy artifact smoke passes
- [ ] degraded/recovering health is observed before final healthy settlement
- [ ] recovered durable jobs finish exactly once after restart

## Failure signals
- `restart_count` never increments
- degraded/recovering health is never observed
- job remains stuck in `processing`
- final job attempts are not `2`
- final health remains `degraded` or `stale`
- deploy smoke or migration apply no longer matches the recovery-aware schema
