---
id: T04
parent: S01
milestone: M028
provides:
  - worker scaffolding, worker-aware health output, a package-local smoke harness, and durable blocker evidence for the remaining runtime DB-query abort
key_files:
  - reference-backend/jobs/worker.mpl
  - reference-backend/storage/jobs.mpl
  - reference-backend/api/health.mpl
  - reference-backend/scripts/smoke.sh
  - compiler/meshc/src/migrate.rs
key_decisions:
  - Use `DateTime.utc_now()` ISO-8601 timestamps for worker diagnostics and job status updates instead of DB round-trips, because the timestamp-query path itself participates in the runtime crash surface.
  - Keep the smoke script focused on build/start/API/worker behavior and preflight for the `jobs` table instead of invoking `meshc migrate up`, because migration execution remains blocked by the lower-level runtime DB-query abort.
patterns_established:
  - When Mesh runtime DB calls are the unstable boundary, keep observability state in-process and use application timestamps rather than adding more DB probes.
observability_surfaces:
  - GET /health worker snapshot
  - GET /jobs/:id lifecycle fields
  - reference-backend/scripts/smoke.sh
  - startup and worker log lines
  - direct migration-child crash evidence in compiler/meshc/src/migrate.rs
duration: 2h10m
verification_result: blocked
completed_at: 2026-03-23T03:35:42-04:00
blocker_discovered: true
---

# T04: Wire the timer-driven worker and a package-local smoke path

**Added worker wiring, health/smoke scaffolding, and then confirmed the remaining slice is blocked by the lower-level runtime abort on the first real DB query/execute path.**

## What Happened

I implemented the T04 package surfaces that were still missing: `reference-backend/jobs/worker.mpl` now starts a timer-recursive worker using the Mesher `Timer.sleep` + recursive-call pattern, `reference-backend/api/health.mpl` now exposes worker state instead of only returning a bare `{status:"ok"}`, `reference-backend/main.mpl` now starts the worker before `HTTP.serve`, and `reference-backend/scripts/smoke.sh` now exercises the package-local build/start/create/poll flow while failing loudly if the `jobs` table is missing.

I also rewrote `reference-backend/storage/jobs.mpl` away from the original `Repo.query_raw(... RETURNING ...)` path and onto the safer ORM/query-builder primitives already used in `mesher/`, and I replaced the new worker timestamp probes with `DateTime.utc_now()` so the worker no longer crashed during startup on a `SELECT now()` round-trip.

That let the runtime get materially farther: the server now reaches pool ready, registry ready, worker started, worker ready, HTTP bind, and the first worker tick. The remaining blocker is the same one T03 isolated, but now with tighter proof: the very first real DB interaction on the worker tick still aborts the compiled process with the same `vec_deque` / `hint::assert_unchecked` panic path. I also directly probed the compiled temporary migration binary and confirmed the migration child still aborts the same way on DDL. At that point T04’s golden-path verification was no longer a task-local bug; it is a lower-level runtime/compiler/runtime-DB boundary issue that invalidates the rest of S01 until it is fixed.

## Verification

I verified that the package still compiles via `cargo run -p meshc -- build reference-backend`, that the missing-env startup failure remains explicit, and that `meshc migrate reference-backend status` still discovers the migration. I then reran the real DB-backed paths that T04 depends on.

The worker wiring itself is live enough to reach startup and tick once, but the process aborts on that first DB call. The direct migration runner is similarly blocked: `meshc migrate reference-backend up` still fails to complete, and running the compiled temp migration child directly reproduces the same abort. Because the runtime cannot survive its first DB query/execute inside a compiled Mesh process, I could not get the smoke script or the worker lifecycle transition to pass truthfully.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo run -p meshc -- build reference-backend` | 0 | ✅ pass | n/a |
| 2 | `env -u DATABASE_URL PORT=18080 JOB_POLL_MS=500 ./reference-backend/reference-backend 2>&1 | rg "DATABASE_URL"` | 0 | ✅ pass | n/a |
| 3 | `set -a && source .env && set +a && cargo run -p meshc -- migrate reference-backend status` | 0 | ✅ pass | n/a |
| 4 | `set -a && source .env && set +a && cargo run -p meshc -- migrate reference-backend up` | 124 | ❌ fail | 240s timeout |
| 5 | `set -a && source .env && set +a && PORT=18080 JOB_POLL_MS=500 ./reference-backend/reference-backend` (observed via `bg_shell`) | 134 | ❌ fail | n/a |
| 6 | `set -a && source .env && set +a && python3 - <<'PY' ... /var/.../_migrate ... PY` | -6 | ❌ fail | n/a |
| 7 | `set -a && source .env && set +a && PORT=18080 JOB_POLL_MS=500 bash reference-backend/scripts/smoke.sh` | 124 | ❌ fail | 120s timeout |

## Diagnostics

- Startup surface: `reference-backend/main.mpl` now logs config load, DB connect, pool ready, registry ready, worker start, worker ready, and HTTP bind.
- Worker surface: `reference-backend/jobs/worker.mpl` logs worker ticks, idle scans, claimed jobs, processed jobs, and worker failures.
- Health surface: `GET /health` now returns worker status, poll interval, started-at, last-tick-at, last job id, last error, and processed/failed counters.
- Job surface: `GET /jobs/:id` still exposes `status`, `attempts`, `last_error`, `updated_at`, and `processed_at` for debugging a stuck or failed job.
- Smoke surface: `reference-backend/scripts/smoke.sh` preflights the `jobs` table, builds the package, starts the binary, posts a job, polls `GET /jobs/:id`, and tails the captured server log on failure.
- Migration evidence: running the compiled temp migration child directly now reproduces the lower-level abort outside the `meshc migrate` wrapper, which is the cleanest current repro for the blocker.

## Deviations

- I updated `compiler/meshc/src/migrate.rs` so the synthetic migration success path no longer tries to `Pool.close(pool)` before signaling success, because that cleanup ordering was masking the more important child-process failure surface during investigation.
- I changed the smoke script to preflight for an existing `jobs` table instead of invoking `meshc migrate up` itself, because the task-owned smoke contract is build/start/API/worker and the migration runner remains blocked by the deeper runtime bug.

## Known Issues

- `reference-backend` still aborts on the first real DB query/execute after startup. The worker proves this reliably: the process reaches HTTP listen, logs the first worker tick, then aborts immediately.
- The compiled synthetic migration child still aborts on DB DDL, so `meshc migrate reference-backend up` cannot complete and therefore cannot serve as a trustworthy prerequisite for the smoke path yet.
- Because that lower-level runtime boundary is still broken, the intended `pending -> processed` lifecycle could not be verified truthfully in this task.

## Files Created/Modified

- `reference-backend/jobs/worker.mpl` — added the timer-recursive worker actor plus in-process worker state service and diagnostics.
- `reference-backend/storage/jobs.mpl` — rewrote job storage around ORM/query-builder helpers and added worker-facing claim/process/fail helpers.
- `reference-backend/api/health.mpl` — expanded `/health` to report worker state and counters.
- `reference-backend/api/jobs.mpl` — extended job fetch logging to surface `processed_at` with lifecycle reads.
- `reference-backend/main.mpl` — starts the worker before serving HTTP and logs worker readiness.
- `reference-backend/scripts/smoke.sh` — added the package-local smoke harness for build/start/create/poll with failure log tailing.
- `compiler/meshc/src/migrate.rs` — adjusted the synthetic migration success path while investigating the remaining migration-runner blocker.
