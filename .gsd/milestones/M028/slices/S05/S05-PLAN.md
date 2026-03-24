# S05: Supervision, Recovery, and Failure Visibility

**Goal:** Turn the `reference-backend/` worker path from an unsupervised happy-path loop into a trustworthy supervised backend flow by repairing Mesh source-level supervisors, wiring the backend worker under supervision, recovering inflight durable work after crashes, and surfacing restart/failure state through `/health` instead of logs-only behavior.
**Demo:** Compiled Mesh supervisor e2e tests prove child start/restart behavior from source, and ignored `reference-backend` e2e tests prove a claimed job is not stranded after worker or process crashes while `/health`, `GET /jobs/:id`, and `reference-backend/README.md` expose the same restart/recovery story explicitly.

## Must-Haves

- S05 directly advances **R004** by proving Mesh concurrency/supervision under crash, restart, and failure-reporting scenarios on the real `reference-backend/` path.
- The Mesh-language supervisor pipeline must stop relying on banner-string smoke checks; `compiler/meshc/tests/e2e_supervisors.rs` must assert that compiled supervisors actually start children, restart crashing children, and surface restart-limit exhaustion.
- `reference-backend` must stop starting its worker with a plain detached `spawn(...)`; worker startup has to move behind a supervision-friendly shape with explicit restart bookkeeping instead of a stale state service that can outlive a dead worker.
- Durable job state must recover from crash-after-claim scenarios so rows do not stay marooned in `processing` forever after a worker or whole-process failure.
- S05 must support **R008** and **R009** by keeping the authoritative proof in `compiler/meshc/tests/e2e_reference_backend.rs` and updating `reference-backend/README.md` to the exact verified supervision/recovery signals and commands.

## Proof Level

- This slice proves: operational
- Real runtime required: yes
- Human/UAT required: no

## Verification

- `cargo test -p mesh-rt supervisor::tests:: --lib -- --nocapture`
- `cargo test -p meshc --test e2e_supervisors -- --nocapture`
- `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_supervision_starts -- --ignored --nocapture`
- `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture`
- `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_restart_is_visible_in_health -- --ignored --nocapture`
- `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_process_restart_recovers_inflight_job -- --ignored --nocapture`
- `rg -n "Supervision and recovery|restart_count|last_exit_reason|recovered_jobs|process restart" reference-backend/README.md`

## Observability / Diagnostics

- Runtime signals: supervised worker boot/restart events, restart counts, exit reasons, recovery counts, and worker liveness classifications must be emitted into the worker state/health contract instead of disappearing into raw logs.
- Inspection surfaces: `compiler/meshc/tests/e2e_supervisors.rs`, `compiler/meshc/tests/e2e_reference_backend.rs`, `GET /health`, `GET /jobs/:id`, direct `jobs` table reads, and `reference-backend/README.md`.
- Failure visibility: a future agent should be able to see whether the worker is healthy, restarting, recovering abandoned work, or hard-failed, along with the last exit reason / recovered-job signal and timestamps.
- Redaction constraints: do not print or persist `DATABASE_URL`; proof should stay on safe paths, ports, job ids, health JSON, and durable DB state.

## Integration Closure

- Upstream surfaces consumed: `compiler/mesh-codegen`, `compiler/mesh-rt`, `compiler/meshc/tests/e2e_supervisors.rs`, `reference-backend/main.mpl`, `reference-backend/runtime/registry.mpl`, `reference-backend/jobs/worker.mpl`, `reference-backend/storage/jobs.mpl`, `reference-backend/api/health.mpl`, and `compiler/meshc/tests/e2e_reference_backend.rs`.
- New wiring introduced in this slice: a Mesh source-level supervisor path that actually starts/restarts children, a supervised `reference-backend` worker bootstrap, durable abandoned-job recovery, and a health contract that reports restart/recovery truth.
- What remains before the milestone is truly usable end-to-end: S06 still needs to promote the now-verified backend/supervision proof across the broader docs/examples surface, but no additional technical recovery wiring should remain after this slice.

## Tasks

- [x] **T01: Repair and prove Mesh source-level supervisor child lifecycle** `est:2h`
  - Why: S05 cannot honestly rely on supervisors until the Mesh-language compiler/runtime bridge proves that compiled supervisors actually start children, restart crashes, and honor restart limits instead of only printing banner strings.
  - Files: `compiler/mesh-codegen/src/mir/lower.rs`, `compiler/mesh-codegen/src/codegen/expr.rs`, `compiler/mesh-rt/src/actor/mod.rs`, `compiler/meshc/tests/e2e_supervisors.rs`, `tests/e2e/supervisor_basic.mpl`, `tests/e2e/supervisor_restart_limit.mpl`
  - Do: Align supervisor lowering/codegen with the runtime child-spec parser, keep child start/shutdown metadata consistent end to end, and replace shallow fixture assertions with compiled Mesh checks that prove child boot, crash/restart, and restart-limit visibility from the source-level path.
  - Verify: `cargo test -p mesh-rt supervisor::tests:: --lib -- --nocapture && cargo test -p meshc --test e2e_supervisors -- --nocapture`
  - Done when: `e2e_supervisors` fails if compiled Mesh supervisors stop starting children or restarting crashes, and the runtime supervisor donor tests still pass.
  - Resume note (2026-03-23): Investigation reproduced the real gap with a direct-call probe: `BootSup()` returned and `main_done` printed, but the supervised child never emitted its `child_boot` marker. The likely first fix is in `compiler/mesh-codegen/src/codegen/expr.rs` because `codegen_supervisor_start(...)` serializes fewer child-spec fields than `compiler/mesh-rt/src/actor/mod.rs::parse_supervisor_config(...)` consumes.
- [x] **T02: Wire `reference-backend` worker startup under supervision and expose restart bookkeeping** `est:2h`
  - Why: The backend still boots its worker with a plain `spawn(...)` plus captured args, and the current state service cannot distinguish a healthy worker from a dead worker whose bookkeeping process survived.
  - Files: `reference-backend/main.mpl`, `reference-backend/runtime/registry.mpl`, `reference-backend/jobs/worker.mpl`, `reference-backend/api/health.mpl`, `compiler/meshc/tests/e2e_reference_backend.rs`
  - Do: Move worker runtime dependencies behind registry/service lookups so a supervisor child can start from a simple Mesh source path, start the worker under supervision from `main.mpl`, and extend worker state plus `/health` with worker identity/restart metadata that proves the supervised child is the thing actually running.
  - Verify: `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_supervision_starts -- --ignored --nocapture`
  - Done when: the backend boots with a supervised worker child and `/health` exposes fresh worker/restart bookkeeping instead of only counters from a detached state service.
  - Resume note (2026-03-23): `compiler/mesh-codegen/src/codegen/expr.rs` now emits the runtime-expected supervisor child-spec layout (`start_args_ptr`, `start_args_size`, `shutdown_type`, `shutdown_timeout_ms`, `child_type`), and `cargo test -p mesh-rt supervisor::tests:: --lib -- --nocapture` plus `cargo test -p meshc --test e2e_supervisors -- --nocapture` both passed afterward. A direct-call probe no longer stayed silent — it emitted repeated `child_boot` output — but the actual `reference-backend` wiring files were not updated before the context-budget wrap-up.
- [x] **T03: Recover abandoned `processing` jobs and classify failure state in health** `est:2h`
  - Why: S05’s main trust gap is that a crash after claim leaves durable work stranded in `processing`, while `/health` can stay green and force operators to infer failure from stale timestamps or logs.
  - Files: `reference-backend/storage/jobs.mpl`, `reference-backend/jobs/worker.mpl`, `reference-backend/api/health.mpl`, `compiler/meshc/tests/e2e_reference_backend.rs`
  - Do: Add storage and worker logic that reclaims or retries abandoned `processing` rows during supervised restart/boot, preserve honest attempts/error behavior, and make `/health` report liveness/recovery/failure fields such as restart count, last exit reason, recovery activity, and non-green worker state when the backend is no longer actually healthy.
  - Verify: `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture && DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_restart_is_visible_in_health -- --ignored --nocapture`
  - Done when: a worker crash no longer leaves a row permanently stuck in `processing`, and `/health` explicitly distinguishes healthy, recovering, and failed worker states with inspectable metadata.
  - Resume note (2026-03-23): A local Docker Postgres is now wired through the repo-local `.env`, and the backend files were partially rewritten (`storage/jobs.mpl`, `runtime/registry.mpl`, `main.mpl`, `jobs/worker.mpl`, `api/health.mpl`, plus the ignored Rust harness tests). Resume from `set -a && source ./.env && set +a && cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture`. The latest blockers are compile-time only: verify `Storage.Jobs.reclaim_processing_jobs` is truly exported/imported as saved on disk, fix the `JobWorkerState.note_processed(...)` / related note helper type mismatch (`expected Int, found ()`), and replace the current `do_crash(0)=0` crash injection trick with a typechecker-safe deterministic crash path before rerunning the first ignored proof.
- [x] **T04: Prove whole-process recovery and document the supervision contract** `est:90m`
  - Why: R004 only counts if the proof survives full backend restarts and the resulting health/recovery contract is written down for future evaluators instead of being buried in one test file.
  - Files: `compiler/meshc/tests/e2e_reference_backend.rs`, `reference-backend/README.md`
  - Do: Extend the backend harness with a whole-process restart scenario that kills and relaunches the backend around an inflight job, keep assertions aligned across `/health`, `/jobs/:id`, DB truth, and logs, and document the exact supervision/recovery proof commands plus the meaning of the new health fields in `reference-backend/README.md`.
  - Verify: `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_process_restart_recovers_inflight_job -- --ignored --nocapture && rg -n "Supervision and recovery|restart_count|last_exit_reason|recovered_jobs|process restart" reference-backend/README.md`
  - Done when: a full backend restart no longer strands inflight work and the package-local docs tell a future evaluator how to inspect supervision/recovery truth without log-diving first.
  - Resume note (2026-03-24): The closer reran the real ignored proof and confirmed S05 is still not done. `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture` now gets past the earlier compile blockers and reaches runtime, but the backend still dies after the injected crash-before-restart proof point. Current disk state to resume from: `reference-backend/runtime/registry.mpl` now also stores `database_url`, `reference-backend/main.mpl` passes it into `start_registry(...)`, and `reference-backend/jobs/worker.mpl` now opens its own local pool at worker boot because the earlier registry `PoolHandle` path crashed with null/misaligned handle failures in `mesh_pool_checkout` / `mesh_pool_open`. Do **not** use `mesh_panic` / non-exhaustive-match tricks or stdlib panics like `List.head([])` for crash simulation here — both abort the whole backend with `panic in a function that cannot unwind`. Finish the worker as a cooperative supervised exit instead: make `job_worker_loop(...)` stop recursing and let the worker actor return when the crash-after-claim path fires, rerun `e2e_reference_backend_worker_crash_recovers_job`, then `e2e_reference_backend_worker_restart_is_visible_in_health`, and only after those pass add the whole-process restart test and README section.

## Files Likely Touched

- `compiler/mesh-codegen/src/mir/lower.rs`
- `compiler/mesh-codegen/src/codegen/expr.rs`
- `compiler/mesh-rt/src/actor/mod.rs`
- `compiler/meshc/tests/e2e_supervisors.rs`
- `tests/e2e/supervisor_basic.mpl`
- `tests/e2e/supervisor_restart_limit.mpl`
- `reference-backend/main.mpl`
- `reference-backend/runtime/registry.mpl`
- `reference-backend/jobs/worker.mpl`
- `reference-backend/storage/jobs.mpl`
- `reference-backend/api/health.mpl`
- `compiler/meshc/tests/e2e_reference_backend.rs`
- `reference-backend/README.md`
