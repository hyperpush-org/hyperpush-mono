# S07: Recovery Proof Closure — Research

## Summary

S07 primarily closes **R004** and directly unblocks the still-open **R008** and **R009** proof story.

Current repo reality is narrower than the stale slice summaries imply:

- `reference-backend` is **not broadly broken** anymore.
- `cargo run -p meshc -- build reference-backend`
- `cargo run -p meshc -- fmt --check reference-backend`
- `cargo run -p meshc -- test reference-backend`

all pass on the current worktree.

What is still red is the actual recovery contract.

The authoritative failure is no longer “health never shows degraded.” The current code now has **two different bad behaviors** across the two ignored recovery tests:

1. `e2e_reference_backend_worker_crash_recovers_job` now **does** expose a degraded/recovering window, but the recovered job stays stuck at `pending` and never gets processed again.
2. `e2e_reference_backend_worker_restart_is_visible_in_health` is worse in a different way: the worker can reprocess the job, but the degraded/recovering transition is not observed reliably, some health metadata becomes obviously corrupted (`boot_id` / `started_at` null while timestamp strings appear in unrelated fields), and the runtime later aborts with an `invalid value for char` panic.

The root problem is in `reference-backend/jobs/worker.mpl`:

- the worker is under a source-level supervisor,
- **but the crash path is still simulating a restart inside the worker itself**.

`crash_after_claim(...)` at `reference-backend/jobs/worker.mpl:324` currently calls `JobWorkerState.note_boot(...)` and `reclaim_processing_jobs(...)` directly before any real child exit/restart has happened. That means restart bookkeeping and recovery are being mutated by the crashing actor itself instead of being driven by an actual supervisor restart. The result is timing-sensitive fake recovery, not trustworthy recovery proof.

There is a second, deeper correctness hole in `reference-backend/storage/jobs.mpl:79`: `reclaim_processing_jobs(...)` requeues **all** `status='processing'` rows with no staleness or ownership guard. That is unsafe for crash recovery on a shared DB. A restarted worker could reclaim a legitimately in-flight job that belongs to another still-healthy instance.

That makes S07 a real closure slice, not cleanup:

- fix the recovery semantics,
- make the degraded/recovering window explicit and trustworthy,
- add the missing whole-process restart proof,
- and only then let S08 reconcile public docs.

Per the `debug-like-expert` skill, this slice needs **verify, don’t assume** discipline: the existing summaries and `.gsd/KNOWLEDGE.md` are already stale relative to current runtime behavior. Per the `test` skill, the right proof surface remains the existing `compiler/meshc/tests/e2e_reference_backend.rs` harness rather than a new ad hoc script.

## Recommendation

### 1. Stop simulating restarts inside the crashing worker

S07 should re-center on **real supervisor semantics**:

- the crashing worker records why it is about to exit,
- the worker actually exits,
- the **newly restarted child** performs boot bookkeeping and abandoned-job recovery,
- the health state transitions come from that real restart boundary.

Do **not** keep the current pattern where `crash_after_claim(...)` calls `note_boot(...)` itself. That is the main source of contradictory behavior between the two current ignored tests.

### 2. Make recovery stale/lease-based instead of blanket

`reclaim_processing_jobs(...)` cannot stay “requeue every processing row immediately.” That is not concurrency-safe.

The smallest plausible recovery contract is:

- only reclaim `status='processing'` rows that are older than a staleness threshold, using `updated_at` as the claim timestamp, or
- add explicit recovery/lease metadata if `updated_at` is not precise enough.

A practical reference-backend version could derive the threshold from `JOB_POLL_MS` instead of adding another env var immediately.

If the planner chooses a schema/index change, it must also update:

- `reference-backend/migrations/20260323010000_create_jobs.mpl`
- `reference-backend/deploy/reference-backend.up.sql`

because S04’s artifact-first deployment path is already established and must stay honest.

### 3. Add a deterministic in-flight window for whole-process restart proof

The current worker marks a claimed job processed immediately. That makes a full-process restart proof hard to run deterministically.

S07 likely needs one small test-oriented seam in `reference-backend/jobs/worker.mpl`, for example a payload-triggered pause/hold-after-claim branch, so the harness can:

- create a job,
- wait until the DB row is definitely `processing`,
- kill the whole backend process,
- restart it,
- prove the row is reclaimed and eventually processed.

Without that deterministic window, a process-restart test will be race-prone.

### 4. Stabilize health around real lifecycle states, not just raw status strings

`reference-backend/api/health.mpl:28` currently computes liveness only from `last_status`. It does **not** use `tick_age_ms` to classify stale/dead worker state.

That means health can stay “recovering” or “healthy” forever based on old state even if no worker is actually ticking anymore.

S07 should make `/health` trustworthy enough for the recovery tests by tying liveness to real lifecycle evidence:

- active degraded/recovering window while abandoned work is being reclaimed,
- healthy only after a post-restart tick / processed-or-idle transition,
- stale/dead classification if ticks stop beyond a threshold.

### 5. Keep docs changes out of the critical path

Public proof docs are already ahead of reality:

- `website/docs/docs/production-backend-proof/index.md` already cites `e2e_reference_backend_process_restart_recovers_inflight_job`
- that test does **not** exist yet in `compiler/meshc/tests/e2e_reference_backend.rs`
- `reference-backend/README.md` still has no supervision/recovery section
- `reference-backend/scripts/verify-production-proof-surface.sh` does not catch this specific drift

That is real doc drift, but it belongs after the runtime proof is green. Use S07 to make the recovery commands true; use S08 to reconcile the public surfaces.

## Current implementation landscape

### `reference-backend/jobs/worker.mpl`

This is the slice center.

What exists now:

- `JobWorkerState` service with recovery/restart fields (`boot_id`, `restart_count`, `last_exit_reason`, `recovered_jobs`, `last_recovery_*`) beginning at `reference-backend/jobs/worker.mpl:23`
- source-level supervisor `JobWorkerSupervisor` at `reference-backend/jobs/worker.mpl:172`
- real boot path in `supervised_job_worker()` at `reference-backend/jobs/worker.mpl:433`
- boot-time recovery in `handle_worker_pool_open(...)` at `reference-backend/jobs/worker.mpl:418`
- crash injection path in `crash_after_claim(...)` at `reference-backend/jobs/worker.mpl:324`

What is wrong now:

- `crash_after_claim(...)` manually calls `JobWorkerState.note_boot(...)` at `reference-backend/jobs/worker.mpl:330`
- it then immediately calls `reclaim_processing_jobs(...)` at `reference-backend/jobs/worker.mpl:333`
- so the worker is mutating “post-restart” state before a real restart boundary exists
- the same file is trying to be both the crashing child and the restarted child at once

Why this matters:

- in the 100ms proof, it can leave a job requeued but never processed again while `/health` stays degraded forever
- in the 500ms proof, it can process the recovered job but still present unstable or corrupted recovery metadata

Natural seam:

- keep crash injection local to `process_claimed_job(...)` / `crash_after_claim(...)`
- move all real restart bookkeeping back behind `supervised_job_worker()` / `handle_worker_pool_open(...)`
- introduce a dedicated exit marker helper if needed, rather than using `NoteBoot` as both exit and boot bookkeeping

### `reference-backend/storage/jobs.mpl`

What exists now:

- `claim_next_pending_job(...)` is still the good S02 atomic claim path
- `reclaim_processing_jobs(...)` at `reference-backend/storage/jobs.mpl:79` requeues processing rows back to pending

What is wrong now:

- recovery SQL is unconditional for all `processing` rows
- there is no staleness / lease guard
- there is no ownership concept

Why this matters:

- S07 is explicitly about closing concurrency trust
- blanket recovery is not safe under shared-DB multi-instance operation
- a restarting instance could steal work from a healthy instance

Natural seam:

- keep the recovery contract here
- if recovery remains `updated_at`-based, add the threshold parameter here
- if schema/index changes are needed, this file plus the migration/deploy artifact are the boundary

### `reference-backend/api/health.mpl`

What exists now:

- JSON output includes most of the fields S05/S06 wanted
- `worker_tick_age_ms(...)` exists at `reference-backend/api/health.mpl:16`
- `worker_liveness(...)` exists at `reference-backend/api/health.mpl:28`

What is wrong now:

- `worker_liveness(...)` ignores `tick_age_ms`
- liveness is derived only from `last_status`
- stale worker state can therefore look alive indefinitely

Why this matters:

- the first failing proof already showed `tick_age_ms` growing large while health stayed in a nominal state derived from old status
- process-restart closure needs health to say whether recovery is actively happening, completed, or stale

Natural seam:

- keep the health payload shape, but change the liveness classification rules
- do not bolt on more fields before the state transitions themselves are trustworthy

### `compiler/meshc/tests/e2e_reference_backend.rs`

This remains the canonical proof harness.

What exists now:

- `wait_for_worker_recovery_health(...)` at `compiler/meshc/tests/e2e_reference_backend.rs:750`
- `e2e_reference_backend_worker_crash_recovers_job()` at `:1639`
- `e2e_reference_backend_worker_restart_is_visible_in_health()` at `:1745`
- all the prior S02/S04 process, HTTP, DB, staging, and migration helpers

What is missing now:

- no `e2e_reference_backend_worker_supervision_starts`
- no `e2e_reference_backend_process_restart_recovers_inflight_job`

Planner implication:

- reuse this file for S07 closure
- do not introduce a second recovery harness
- add the whole-process restart proof here after the two existing recovery tests are green

### `compiler/meshc/tests/e2e_supervisors.rs`

This is now a donor regression, not the primary proof.

What exists now:

- `supervisor_basic()` at `compiler/meshc/tests/e2e_supervisors.rs:149`
- `supervisor_one_for_all()` at `:214`
- `supervisor_restart_limit()` at `:231`

What is still weak:

- these tests still assert banner strings like `"supervisor started"`, `"one_for_all supervisor started"`, and `"restart limit test started"`
- they do not prove child lifecycle in a way that is useful for S07 closure

Planner implication:

- keep them green as a donor guard
- do not let S07 expand into another large compiler-side supervisor audit unless a fresh repro proves the backend is failing because source-level supervisor restart is actually broken again

### `reference-backend/README.md` and `website/docs/docs/production-backend-proof/index.md`

Current state:

- `reference-backend/README.md` documents build/runtime/deploy/smoke but still has no supervision/recovery section
- `website/docs/docs/production-backend-proof/index.md` already advertises the missing whole-process restart proof command

Planner implication:

- treat these as downstream consumers
- do not spend S07 time polishing docs before the runtime proofs exist and pass

## Research evidence gathered

### 1. Baseline backend workflow is green

Commands run:

```bash
cargo run -p meshc -- build reference-backend
cargo run -p meshc -- fmt --check reference-backend
cargo run -p meshc -- test reference-backend
```

Observed:

- all passed on the current worktree
- the slice is not blocked on general buildability, formatting, or local package tests

### 2. Worker-crash recovery proof now fails later than the stale summaries say

Command run:

```bash
set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture
```

Observed:

- the test no longer fails at `wait_for_worker_recovery_health(...)`
- it fails later at `wait_for_processed_job_and_health(...)`
- final job row is still `pending`, `attempts=1`, `last_error="requeued after worker restart"`
- final `/health` is still `status="degraded"`, `worker.liveness="recovering"`, `restart_count=1`, `recovered_jobs=1`, `processed_jobs=0`
- worker logs show:
  - claim
  - injected crash
  - fake/early boot bookkeeping
  - recovery log
  - **no subsequent second claim / processed log**

Interpretation:

- the degraded/recovering window now exists
- but the current path can strand a requeued job without ever resuming real processing
- this is different from the stale `.gsd/KNOWLEDGE.md` note and should supersede it

### 3. Restart-visibility proof shows a second unstable mode

Command run:

```bash
set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_restart_is_visible_in_health -- --ignored --nocapture
```

Observed:

- the worker sometimes does reclaim and reprocess the job (`attempts=2` in logs)
- the harness still fails at `wait_for_worker_recovery_health(...)`
- the last observed health payload before failure shows inconsistent metadata:
  - `boot_id: null`
  - `started_at: null`
  - timestamp strings appearing in `last_exit_reason` and `last_job_id`
- after the harness failure, the runtime aborts with a Rust panic about `invalid value for char`

Interpretation:

- the current manual-restart path is not just timing-sensitive; it is unstable enough to corrupt or misreport recovery metadata under at least one timing profile
- S07 should treat the current worker state contract as untrustworthy until the restart boundary is made real

### 4. Whole-process restart proof is still absent while docs already cite it

Search evidence:

- `compiler/meshc/tests/e2e_reference_backend.rs` contains no `e2e_reference_backend_process_restart_recovers_inflight_job`
- `website/docs/docs/production-backend-proof/index.md` already advertises that command

Interpretation:

- runtime closure and public proof-surface closure are currently out of sync
- S07 should make the command real; S08 should reconcile the docs around it

### 5. The current recovery SQL is concurrency-unsafe for restart scenarios

Code evidence:

- `reference-backend/storage/jobs.mpl:79` requeues every `status='processing'` row
- `reference-backend/migrations/20260323010000_create_jobs.mpl` only indexes pending-job scans

Interpretation:

- closing S07 honestly likely requires either a staleness threshold based on `updated_at` or additional lease metadata
- if recovery queries become `status='processing' AND updated_at < cutoff`, a supporting partial index may be warranted

## Natural seams for planning

### Seam 1: Real worker exit/restart semantics

Files:

- `reference-backend/jobs/worker.mpl`
- possibly `reference-backend/api/health.mpl`

Goal:

- remove fake in-actor boot/recovery
- make the worker actually exit on the crash trigger
- let only the restarted child perform boot bookkeeping and recovery

Why separate:

- this is the highest-leverage root-cause fix
- it should be completed before any harness or docs adjustments

### Seam 2: Safe abandoned-job recovery semantics

Files:

- `reference-backend/storage/jobs.mpl`
- maybe `reference-backend/migrations/20260323010000_create_jobs.mpl`
- maybe `reference-backend/deploy/reference-backend.up.sql`
- `reference-backend/jobs/worker.mpl`

Goal:

- recover only stale or provably abandoned processing rows
- keep shared-DB restart behavior compatible with S02’s exact-once story

Why separate:

- this is the actual concurrency-trust closure
- if it changes schema or indexes, S04 artifacts must be updated too

### Seam 3: Health/liveness stabilization

Files:

- `reference-backend/api/health.mpl`
- `reference-backend/jobs/worker.mpl`
- `compiler/meshc/tests/e2e_reference_backend.rs`

Goal:

- make the degraded/recovering window explicit and consistently observable
- avoid stale-green or stale-degraded health states
- align metadata fields with real lifecycle transitions

Why separate:

- once restart/recovery semantics are real, the health contract can become simple and consistent again

### Seam 4: Missing whole-process restart proof

Files:

- `compiler/meshc/tests/e2e_reference_backend.rs`
- maybe `reference-backend/jobs/worker.mpl`
- maybe `reference-backend/storage/jobs.mpl`

Goal:

- add a deterministic process-restart proof for an inflight job

Why separate:

- this should be added after the two current worker-crash tests are green
- it likely needs a small deterministic hold-after-claim seam in the worker

### Seam 5: Downstream doc/knowledge reconciliation

Files:

- `.gsd/KNOWLEDGE.md`
- `reference-backend/README.md`
- `website/docs/docs/production-backend-proof/index.md`
- `reference-backend/scripts/verify-production-proof-surface.sh`

Goal:

- update stale failure descriptions and public proof commands only after the runtime truth is green

Why separate:

- not on the critical path for S07 runtime closure
- likely belongs with S08 final reconciliation

## What to build or prove first

1. **Fix the worker to use a real restart boundary.**
   - Remove the current `crash_after_claim(...)` fake boot bookkeeping.
   - Make the crashing worker record exit intent and actually stop.
   - Let `supervised_job_worker()` be the only place that increments restart state and runs recovery.

2. **Choose and implement safe recovery semantics.**
   - Prefer stale/lease-based reclaim over blanket reclaim.
   - If using `updated_at`, make the threshold explicit and testable.
   - If schema/index changes are required, update both canonical migration and deploy SQL artifact.

3. **Re-green `e2e_reference_backend_worker_crash_recovers_job`.**
   - This is now the sharpest single-signal regression.
   - It should prove: degraded/recovering visible, row reclaimed safely, row eventually processed, final health healthy.

4. **Re-green `e2e_reference_backend_worker_restart_is_visible_in_health`.**
   - Use it to validate that metadata is no longer corrupted and recovery visibility is stable across a slower poll cadence.

5. **Add and pass `e2e_reference_backend_process_restart_recovers_inflight_job`.**
   - Only after worker-crash semantics are trustworthy.

6. **Only then update downstream docs/knowledge.**
   - S07 should not end with public surfaces still referencing nonexistent or red proofs.

## Verification plan

### Baseline keep-green checks

```bash
cargo run -p meshc -- build reference-backend
cargo run -p meshc -- fmt --check reference-backend
cargo run -p meshc -- test reference-backend
cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_builds -- --nocapture
```

### Recovery closure gates

Run serially with env loaded:

```bash
set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture
set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_restart_is_visible_in_health -- --ignored --nocapture
```

### Missing whole-process recovery gate to add

```bash
set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_process_restart_recovers_inflight_job -- --ignored --nocapture
```

### If recovery SQL / schema changes

Re-run these too:

```bash
set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_migration_status_and_apply -- --ignored --nocapture
set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_deploy_artifact_smoke -- --ignored --nocapture
```

### Donor regression only

```bash
cargo test -p meshc --test e2e_supervisors -- --nocapture
```

Keep this green, but do not let it substitute for the backend recovery proofs.

## Risks / gotchas

- **The current `.gsd/KNOWLEDGE.md` recovery note is stale.** It still says the main blocker is “degraded state not visible.” That is no longer true for the first ignored recovery test.
- **Database-backed ignored proofs must run serially** against one `DATABASE_URL`.
- **Non-interactive shell commands here do not inherit `.env`.** Use `set -a && source .env && set +a`.
- **Blanket recovery is not concurrency-safe.** Do not call S07 closed if restart recovery can still reclaim active jobs from other instances.
- **Do not reintroduce panic-based crash injection.** The prior S05/S06 work already showed that panic-style paths can abort the whole backend instead of producing restartable actor failure.
- **`NoteBoot` is currently overloaded.** It infers exit reason from previous state. That is fragile and likely part of the unstable metadata behavior.
- **Process-restart proof probably needs a deterministic hold-after-claim seam.** Otherwise the harness will race a very fast happy-path worker.
- **If schema/index changes land, S04 assets must stay in sync.** Do not forget the staged deploy SQL artifact.
- **Public docs already overclaim recovery closure.** Keep that as a downstream cleanup, not a reason to skip the runtime work.

## Skill discovery

Relevant installed skills that should shape execution:

- `debug-like-expert` — use current failing behavior, not stale summaries, as the starting point; verify each hypothesis against the real harness.
- `test` — extend the existing `e2e_reference_backend.rs` harness instead of inventing a new recovery verifier.
- `review` — read the full worker/storage/health files in context before changing the recovery contract.

Relevant missing skills I checked but did **not** install:

- Rust: `npx skills add apollographql/skills@rust-best-practices`
- PostgreSQL recovery/query tuning: `npx skills add github/awesome-copilot@postgresql-optimization`
- PostgreSQL schema/index design: `npx skills add wshobson/agents@postgresql-table-design`

## Sources

Files inspected:

- `reference-backend/main.mpl`
- `reference-backend/runtime/registry.mpl`
- `reference-backend/api/health.mpl`
- `reference-backend/api/jobs.mpl`
- `reference-backend/storage/jobs.mpl`
- `reference-backend/jobs/worker.mpl`
- `reference-backend/migrations/20260323010000_create_jobs.mpl`
- `reference-backend/README.md`
- `reference-backend/scripts/verify-production-proof-surface.sh`
- `compiler/meshc/tests/e2e_reference_backend.rs`
- `compiler/meshc/tests/e2e_supervisors.rs`
- `tests/e2e/supervisor_basic.mpl`
- `tests/e2e/supervisor_one_for_all.mpl`
- `tests/e2e/supervisor_restart_limit.mpl`
- `website/docs/docs/production-backend-proof/index.md`
- `.gsd/REQUIREMENTS.md`
- `.gsd/KNOWLEDGE.md`
- `.gsd/milestones/M028/slices/S05/S05-PLAN.md`
- `.gsd/milestones/M028/slices/S05/tasks/T01-SUMMARY.md`
- `.gsd/milestones/M028/slices/S05/tasks/T02-SUMMARY.md`
- `.gsd/milestones/M028/slices/S05/tasks/T03-SUMMARY.md`
- `.gsd/milestones/M028/slices/S05/tasks/T04-SUMMARY.md`
- `.gsd/milestones/M028/slices/S06/S06-SUMMARY.md`
- `.gsd/milestones/M028/slices/S06/S06-UAT.md`
- `.gsd/milestones/M028/slices/S06/tasks/T01-SUMMARY.md`
- `.gsd/milestones/M028/slices/S06/tasks/T02-SUMMARY.md`

Commands run during research:

```bash
cargo run -p meshc -- build reference-backend
cargo run -p meshc -- fmt --check reference-backend
cargo run -p meshc -- test reference-backend
set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture
set -a && source .env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_restart_is_visible_in_health -- --ignored --nocapture
rg -n "worker_supervision_starts|process_restart_recovers_inflight_job|Supervision and recovery|restart_count|last_exit_reason|recovered_jobs" compiler/meshc/tests/e2e_reference_backend.rs reference-backend/README.md website/docs/docs/production-backend-proof/index.md .gsd/milestones/M028/slices/S07
rg -n "fn worker_tick_age_ms|fn worker_liveness|fn health_json" reference-backend/api/health.mpl
rg -n "fn crash_after_claim|JobWorkerState.note_boot|reclaim_processing_jobs\(|fn job_worker_loop|fn handle_worker_pool_open|actor supervised_job_worker|supervisor JobWorkerSupervisor|pub fn start_worker" reference-backend/jobs/worker.mpl compiler/meshc/tests/e2e_reference_backend.rs reference-backend/storage/jobs.mpl
rg -n "fn supervisor_basic|fn supervisor_one_for_all|fn supervisor_restart_limit|contains\(\"supervisor started|contains\(\"one_for_all supervisor started|contains\(\"restart limit test started" compiler/meshc/tests/e2e_supervisors.rs tests/e2e/supervisor_basic.mpl tests/e2e/supervisor_one_for_all.mpl tests/e2e/supervisor_restart_limit.mpl
npx skills find "Rust"
npx skills find "PostgreSQL"
```