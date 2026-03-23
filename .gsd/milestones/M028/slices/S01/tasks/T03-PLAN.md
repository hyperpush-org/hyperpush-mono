---
estimated_steps: 5
estimated_files: 6
skills_used:
  - debug-like-expert
  - test
  - review
---

# T03: Wire the timer-driven worker and a package-local smoke path

**Slice:** S01 — Canonical Backend Golden Path
**Milestone:** M028

## Description

Add the missing background-job part of the golden path. Do not use `Job.async` as the canonical pattern here; the repo’s stronger donor is Mesher’s timer-recursive actors (`Timer.sleep` + recursive call). The worker should periodically find pending jobs, mark them processed, and surface enough diagnostics that a future agent can tell whether the worker is healthy or stalled. This task also adds the package-local smoke path that exercises startup, API creation, and background processing together.

## Steps

1. Add a timer-recursive worker module in `reference-backend/jobs/worker.mpl` that wakes on `JOB_POLL_MS`, queries pending jobs, and processes them in a repeatable loop.
2. Extend `reference-backend/storage/jobs.mpl` with the minimal claim/update operations the worker needs, including `attempts`, `processed_at`, and `last_error` updates.
3. Update `reference-backend/api/jobs.mpl` and `reference-backend/api/health.mpl` so the API exposes worker-relevant state instead of hiding it behind logs only.
4. Wire worker startup into `reference-backend/main.mpl` after pool creation and before `HTTP.serve`, following Mesher’s startup order.
5. Add `reference-backend/scripts/smoke.sh` that starts the binary, waits for `/health`, posts a job, polls `GET /jobs/:id`, and exits nonzero if processing never completes.

## Must-Haves

- [ ] The reference backend starts a long-running timer-driven worker as part of normal startup.
- [ ] Pending jobs transition to `processed` without manual DB intervention.
- [ ] Job responses expose enough state to debug a stuck or failed background run.
- [ ] The package-local smoke script fails loudly on startup, API, or worker regressions.

## Verification

- `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} PORT=18080 JOB_POLL_MS=500 bash reference-backend/scripts/smoke.sh`
- After the script runs, `curl -sf http://127.0.0.1:18080/jobs/<id>` or an equivalent helper check should show `processed` state with updated timestamps.

## Observability Impact

- Signals added/changed: worker tick/process logs plus per-job `status`, `attempts`, `last_error`, and `processed_at`
- How a future agent inspects this: `GET /health`, `GET /jobs/:id`, script output from `reference-backend/scripts/smoke.sh`, and the `jobs` table
- Failure state exposed: stalled worker loops, repeated attempts, and last processing error become observable

## Inputs

- `reference-backend/main.mpl` — startup composition that must gain the long-running worker
- `reference-backend/api/health.mpl` — health surface to extend with worker readiness/state
- `reference-backend/api/jobs.mpl` — job response layer that should expose processed state
- `reference-backend/storage/jobs.mpl` — persistence layer to extend for claim/update operations
- `mesher/ingestion/pipeline.mpl` — donor for timer-recursive actors and startup ordering
- `mesher/services/writer.mpl` — donor for the `flush_ticker` timer pattern that avoids `Timer.send_after`

## Expected Output

- `reference-backend/jobs/worker.mpl` — timer-recursive background worker for pending jobs
- `reference-backend/storage/jobs.mpl` — claim/update storage helpers for worker-driven transitions
- `reference-backend/api/health.mpl` — health output extended with worker-aware readiness/state
- `reference-backend/api/jobs.mpl` — job responses extended with processing diagnostics
- `reference-backend/main.mpl` — startup wiring that launches the worker before serving HTTP
- `reference-backend/scripts/smoke.sh` — package-local end-to-end smoke verifier for the golden path
