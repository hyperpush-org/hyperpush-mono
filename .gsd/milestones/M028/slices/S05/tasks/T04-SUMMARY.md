---
id: T04
parent: S05
milestone: M028
provides:
  - Added durable recovery-result scaffolding in storage and left an exact resume path for the remaining worker-state/compiler mismatch that still blocks the whole-process restart proof.
key_files:
  - reference-backend/storage/jobs.mpl
  - reference-backend/jobs/worker.mpl
  - .gsd/milestones/M028/slices/S05/S05-PLAN.md
  - .gsd/milestones/M028/slices/S05/tasks/T04-SUMMARY.md
key_decisions:
  - Fix the durable storage/recovery export gap first instead of inventing the T04 whole-process proof on top of a still-failing T03 build.
patterns_established:
  - The current Mesh compiler path in this area is sensitive to how worker-state service updates are expressed; the latest unverified attempt converts mutating state APIs from `cast` to synchronous `call ... :: Int` methods because repeated helper-level update calls kept failing with `E0012 non-exhaustive match on Int`.
observability_surfaces:
  - compiler/meshc/tests/e2e_reference_backend.rs
  - reference-backend/storage/jobs.mpl
  - reference-backend/jobs/worker.mpl
  - .gsd/milestones/M028/slices/S05/S05-PLAN.md
duration: 1h
verification_result: partial
completed_at: 2026-03-23 19:59:12 EDT
blocker_discovered: false
---

# T04: Prove whole-process recovery and document the supervision contract

**Added durable job-recovery storage scaffolding and isolated a remaining worker-state compiler mismatch that blocked the whole-process restart proof.**

## What Happened

I read the T04 contract, the slice plan, the prior T01–T03 summaries, and the current backend surfaces before changing anything. Local reality was still behind the slice goal: `compiler/meshc/tests/e2e_reference_backend.rs` already contained the T03 worker-crash/health proofs, but `reference-backend/storage/jobs.mpl` on disk still did **not** export `reclaim_processing_jobs(...)`, `reference-backend/jobs/worker.mpl` still imported that missing symbol, and the T04 process-restart proof and README section were still absent.

I reproduced the focused failing backend proof first with the existing ignored T03 command. That confirmed the immediate blockers were still compile-time, not runtime or database setup: the worker imported a missing `reclaim_processing_jobs` export, the worker helper path still produced `expected Int, found ()`, and the crash injector path was still brittle.

I fixed the durable storage gap in `reference-backend/storage/jobs.mpl` by adding:

- a public `RecoveryResult` struct,
- a raw SQL `reclaim_processing_jobs_sql()` path that requeues `processing` rows back to `pending`, sets `last_error` to the recovery hint, clears `processed_at`, and updates `updated_at`, and
- a public `reclaim_processing_jobs(pool, error_message)` export that returns the recovery count plus the last recovered job id.

I then iterated repeatedly on `reference-backend/jobs/worker.mpl` to align the worker-state bookkeeping with that new storage contract. The import/export mismatch was removed, but the worker still failed at build time on the state-update path. The failure evolved from the original missing-export / type-mismatch pair into a more specific `E0012 non-exhaustive match on Int` around worker-state update calls plus an LLVM verifier error involving `job_worker_loop`.

Before the context-budget warning forced wrap-up, I made one last unverified change: I converted the mutating `JobWorkerState` APIs (`NoteBoot`, `NoteRecovery`, `NoteTick`, `NoteIdle`, `NoteClaimed`, `NoteProcessed`, `NoteFailed`, `NoteCrashSoon`) from `cast` methods to synchronous `call ... :: Int` methods that return `(next_state, 0)`. I did **not** get to rerun the build after that last step.

Because the backend still did not build cleanly, I did **not** start the actual T04 work items that depend on a healthy T03 base:

- no `e2e_reference_backend_process_restart_recovers_inflight_job` proof was added,
- `reference-backend/README.md` was not updated with the supervision/recovery section, and
- the T04 checkbox remains unchecked in the slice plan.

## Verification

I repeatedly reran the smallest real backend proof instead of speculating:

- `set -a && source ./.env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture`

That command consistently failed before runtime execution because `meshc build reference-backend` still failed. The failures progressed as follows:

1. missing `Storage.Jobs.reclaim_processing_jobs` export,
2. worker helper mismatch around `JobWorkerState.note_processed(...)` (`expected Int, found ()`),
3. repeated `E0012 non-exhaustive match on Int` around worker-state updates, plus an LLVM verifier error on `job_worker_loop`.

The last on-disk change — converting the mutating worker-state APIs from `cast` to `call ... :: Int` — was not rerun after the context-budget warning, so there is no passing or failing proof yet for that final variant.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `set -a && source ./.env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture` | 101 | ❌ fail | 5.05s |
| 2 | `set -a && source ./.env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture` | 101 | ❌ fail | 5.13s |
| 3 | `set -a && source ./.env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture` | 101 | ❌ fail | 5.26s |
| 4 | `set -a && source ./.env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture` | 101 | ❌ fail | 4.86s |
| 5 | `set -a && source ./.env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture` | 101 | ❌ fail | 5.12s |
| 6 | `set -a && source ./.env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture` | 101 | ❌ fail | 5.34s |

## Diagnostics

Resume from the same focused proof command first:

- `set -a && source ./.env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture`

Resume order:

1. Re-run that exact command against the current on-disk state.
2. If the new `call ... :: Int` worker-state change builds, continue immediately to:
   - `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_restart_is_visible_in_health -- --ignored --nocapture`
3. Only after both T03 proofs pass, implement the actual T04 contract:
   - add `e2e_reference_backend_process_restart_recovers_inflight_job` to `compiler/meshc/tests/e2e_reference_backend.rs`, and
   - add the `Supervision and recovery` section plus exact ignored commands / `/health` field guidance to `reference-backend/README.md`.

The key remaining code path is `reference-backend/jobs/worker.mpl`. If the build still fails, focus on the worker-state update calls first; the storage export gap in `reference-backend/storage/jobs.mpl` is already fixed on disk.

### 2026-03-24 closer wrap-up

I used this slice-close attempt to verify the assembled backend proof instead of just trusting the task summaries. The supervisor donor tests and compiled supervisor e2e tests still pass, but the ignored `reference-backend` crash/recovery proof does **not** pass yet, so S05 is still incomplete.

What changed during this closer pass:

- `reference-backend/runtime/registry.mpl` now also carries `database_url` alongside the shared pool and `poll_ms`.
- `reference-backend/main.mpl` now passes `database_url` into `start_registry(...)`.
- `reference-backend/jobs/worker.mpl` was iterated again:
  - worker boot now opens its own pool from `Env.get("DATABASE_URL", "")` instead of reusing the registry pool handle (the earlier registry-pool path was crashing at `mesh_pool_checkout` with null/misaligned handle failures during startup recovery);
  - helper functions were rewritten around `call ... :: Int` state updates plus explicit `0` returns to get back past the previous `expected Int, found ()` compile blockers;
  - the worker crash path was changed from "panic the runtime" experiments to a cooperative `false` return from `process_claimed_job(...)`, with the intent that the supervised worker should exit and be restarted by the permanent supervisor rather than abort the whole backend process.

Important failed experiments / new evidence:

- A registry-provided `PoolHandle` inside the supervised worker path crashed during `reclaim_processing_jobs(...)` with runtime null/misaligned pointer failures in `mesh_pool_checkout` / `mesh_pool_open`. That is why the worker now opens its own local pool at boot.
- Simulating a crash via `List.head(List.tail([0]))` or via a non-exhaustive-match path that reaches `mesh_panic` **does not** give a restartable actor failure here. Both paths aborted the entire backend with `panic in a function that cannot unwind`, so they are not valid supervision proof techniques for this slice.
- The current on-disk worker code still does not satisfy the T03/T04 proof. The latest failing run reached runtime, injected the one-shot crash-after-claim event, and showed `/health` entering `{"status":"degraded","worker":{"status":"crashing", ...}}`, but the backend then died before the supervisor restart could become visible. The focused harness failure was:
  - `worker health never exposed degraded recovery state; last_health={..."liveness":"recovering"..."restart_count":0..."status":"crashing"...}; last_issue=GET /health failed on <port>: Connection refused`

Fastest safe resume path from the current disk state:

1. Start with `reference-backend/jobs/worker.mpl`; do **not** spend more time on parser/typechecker research first.
2. Finish the cooperative supervised-exit path instead of trying more panic-based crash tricks:
   - make `job_worker_loop(...)` stop recursing and let the worker actor return when `process_next_job(...)` reports the crash-after-claim branch,
   - make `handle_worker_pool_open_error(...)` also exit cooperatively so supervisor restart semantics are consistent.
3. Re-run the focused proof command above until `e2e_reference_backend_worker_crash_recovers_job` passes.
4. Then run `e2e_reference_backend_worker_restart_is_visible_in_health`.
5. Only after both pass should the next unit add the T04 whole-process restart proof and README section.

## Deviations

I did not reach the planned T04 harness/README work because the checked-out backend still failed at the prerequisite T03 build/proof layer. I made no README changes and no T04 process-restart test changes in this unit.

## Known Issues

- `reference-backend` still does not build cleanly under the focused ignored backend proof as of the last rerun.
- The final worker-state service conversion from `cast` to `call ... :: Int` is on disk but unverified.
- `compiler/meshc/tests/e2e_reference_backend.rs` still lacks the planned T04 whole-process restart proof.
- `reference-backend/README.md` still lacks the planned supervision/recovery documentation section.

## Files Created/Modified

- `reference-backend/storage/jobs.mpl` — added `RecoveryResult` and the exported `reclaim_processing_jobs(...)` durable requeue helper.
- `reference-backend/jobs/worker.mpl` — iterated on worker recovery bookkeeping and, in the final unverified state, converted the mutating worker-state APIs from `cast` to synchronous `call ... :: Int` methods.
- `.gsd/milestones/M028/slices/S05/S05-PLAN.md` — left a T04 resume note without falsely marking the task complete.
- `.gsd/milestones/M028/slices/S05/tasks/T04-SUMMARY.md` — durable partial handoff for the next unit.
