# Slice Summary — S07: Recovery Proof Closure

## Status
- **State:** done
- **Roadmap checkbox:** checked
- **Why:** the recovery contract is now backed by green build/fmt/test plus the full ignored recovery gate set on `reference-backend/`.

## What this slice actually delivered

### 1. Worker crash recovery now crosses a real restart boundary
The slice closed the fake-restart behavior that had been undermining the earlier supervision story.

What changed on the backend path:
- `reference-backend/jobs/worker.mpl` now treats a crash-after-claim as a real stalled worker boundary instead of simulating reboot bookkeeping inside the crashing actor.
- Recovery ownership moved to a restarted worker path. The restarted worker performs boot bookkeeping, abandoned-job reclaim, and resumed processing.
- A small watchdog actor now respawns `supervised_job_worker` when the worker state shows a stale `processing`/`crashing` actor by `tick_age_ms`, which gives the reference backend an actual observable restart boundary even though the Mesh source-level supervisor path was not delivering it reliably in this package context.

The result is that `e2e_reference_backend_worker_crash_recovers_job` now proves:
- worker enters a degraded/recovering window
- job is requeued from `processing` back to `pending`
- the same durable job is processed exactly once after restart
- `/health` shows restart evidence instead of a fake in-worker reboot narrative

### 2. Abandoned-job reclaim is stale-cutoff based, not blanket `processing` recovery
`reference-backend/storage/jobs.mpl` no longer requeues every `processing` row on boot. Recovery is now tied to a stale cutoff derived from real time.

That contract is kept aligned across both schema paths:
- `reference-backend/migrations/20260323010000_create_jobs.mpl`
- `reference-backend/deploy/reference-backend.up.sql`

The slice also keeps the processing-reclaim index aligned across those two paths so migration truth and staged deploy truth do not drift.

### 3. `/health` is now a trustworthy recovery surface
The slice hardened recovery observability rather than leaving it as log archaeology.

`reference-backend/api/health.mpl` now reports worker state with tick-age-aware liveness and coherent restart metadata, including:
- `status`
- `liveness`
- `tick_age_ms`
- `boot_id`
- `started_at`
- `restart_count`
- `last_exit_reason`
- `recovered_jobs`
- `last_recovery_at`
- `last_recovery_job_id`
- `last_recovery_count`
- `recovery_active`

Two timing rules make the proof visible and stable:
- the watchdog only respawns when a `processing` or `crashing` worker goes stale
- the restarted worker pauses for a reclaim grace window after recovery so the degraded/recovering state can actually be observed before the requeued job is processed again

That is the key difference between “recovery eventually happens” and “recovery is trustworthy and diagnosable.”

### 4. The canonical Rust harness now proves both worker-level and whole-process recovery
`compiler/meshc/tests/e2e_reference_backend.rs` is now the authoritative recovery proof surface for the milestone.

This slice added or completed proof for:
- worker crash recovery with visible degraded/recovering health
- restart visibility with stable `boot_id` / `started_at` / `last_recovery_*` fields
- deterministic whole-process restart recovery using the payload-driven in-flight seam (`hold_after_claim_once`)
- migration and staged deploy regressions staying green on the updated recovery/storage contract

That keeps the recovery proof in the same canonical harness the rest of M028 already uses, which is important for S08 and later roadmap reassessment.

## Verification run by the closer

All slice-level verification passed in this closure run.

### Passing commands
- `cargo run -p meshc -- build reference-backend`
- `cargo run -p meshc -- fmt --check reference-backend`
- `cargo run -p meshc -- test reference-backend`
- `set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture`
- `set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_restart_is_visible_in_health -- --ignored --nocapture`
- `set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_process_restart_recovers_inflight_job -- --ignored --nocapture`
- `set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_migration_status_and_apply -- --ignored --nocapture`
- `set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_deploy_artifact_smoke -- --ignored --nocapture`

## Requirement impact
- **R004:** now validated. Crash, restart, and failure visibility are proven on the real reference backend rather than advertised abstractly.
- **R009:** now validated. Mesh is now being judged through one real backend that includes recovery behavior, not isolated subsystems only.
- **R008:** still active. The backend proof is green now, but S08 still needs to reconcile the public README/docs/UAT surfaces so they point only at the green recovery-aware proof paths.

## Patterns established
- For this backend, restart proof is authoritative only when the same harness checks `/health`, `/jobs/:id`, and direct `jobs` row state together.
- A believable degraded/recovering window needs timing designed into the runtime contract, not just a fast eventual recovery.
- Shared-DB abandoned-job reclaim must be stale-cutoff based and schema-aligned across both the Mesh migration source and the staged deploy SQL artifact.
- Keep recovery proof in `compiler/meshc/tests/e2e_reference_backend.rs`; do not create separate ad hoc scripts for the same contract.

## Files that matter downstream
- `reference-backend/jobs/worker.mpl`
- `reference-backend/storage/jobs.mpl`
- `reference-backend/api/health.mpl`
- `reference-backend/migrations/20260323010000_create_jobs.mpl`
- `reference-backend/deploy/reference-backend.up.sql`
- `compiler/meshc/tests/e2e_reference_backend.rs`
- `.gsd/REQUIREMENTS.md`
- `.gsd/DECISIONS.md`
- `.gsd/KNOWLEDGE.md`

## What the next slice / reassess-roadmap agent should know
S07 closed the technical recovery blocker. The remaining milestone work is now primarily truth-surface reconciliation, not backend recovery invention.

That means S08 should:
- treat the Rust harness and green recovery commands as the single source of truth
- update public docs/UAT/validation surfaces to cite these exact recovery-aware proof paths
- remove or rewrite any stale wording that still implies recovery proof is partial or placeholder

If roadmap reassessment happens now, the right reading is:
- the reference backend is technically green across crash/restart recovery
- the remaining risk is documentation and promotion honesty, not backend correctness on this slice’s scope
