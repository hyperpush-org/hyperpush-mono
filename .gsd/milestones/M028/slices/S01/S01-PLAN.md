# S01: Canonical Backend Golden Path

**Goal:** Establish a small, auditable Postgres-first Mesh backend package that proves the repo can compose HTTP, migrations, persistent state, and periodic background work in one real runtime.
**Demo:** From `reference-backend/`, a developer can run the canonical migrate/build/start flow, hit `GET /health`, `POST /jobs`, `GET /jobs/:id`, and observe the same persisted job move from `pending` to `processed` without manual intervention.

## Must-Haves

- A new top-level `reference-backend/` Mesh project exists with a stable startup contract built around `DATABASE_URL`, `PORT`, and `JOB_POLL_MS`, instead of promoting `mesher/` into the milestone proof path.
- The reference backend wires one real Postgres-backed lifecycle: migration-managed `jobs` schema, create/read HTTP endpoints, and a background worker that updates the same durable row shape.
- The golden path is inspectable: startup logs, `GET /health`, `GET /jobs/:id`, and the `jobs` table make it obvious whether the app is healthy, stuck, or failed.
- The repo contains executable proof artifacts for this package, including a package-local smoke script, a Rust e2e test file, and package-local commands/docs that downstream slices can reuse.

## Proof Level

- This slice proves: integration
- Real runtime required: yes
- Human/UAT required: no

## Verification

- `cargo build -p mesh-rt && cargo test -p meshc e2e_reference_backend_builds --test e2e_reference_backend -- --nocapture`
- `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc e2e_reference_backend_postgres_smoke --test e2e_reference_backend -- --ignored --nocapture`
- `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} PORT=18080 JOB_POLL_MS=500 bash reference-backend/scripts/smoke.sh`

## Observability / Diagnostics

- Runtime signals: explicit startup/worker log lines for config load, DB connect, HTTP bind, worker ticks, and job state transitions
- Inspection surfaces: `GET /health`, `GET /jobs/:id`, `meshc migrate status reference-backend`, `reference-backend/scripts/smoke.sh`, and the Postgres `jobs` table
- Failure visibility: missing env/config errors, migration pending/applied state, per-job `status`/`attempts`/`last_error`/timestamps, and smoke-script nonzero exits on startup or processing failure
- Redaction constraints: never log `DATABASE_URL`; only report safe config fields and job payload metadata

## Integration Closure

- Upstream surfaces consumed: `compiler/meshc/src/main.rs`, `compiler/meshc/src/migrate.rs`, `compiler/meshc/src/test_runner.rs`, `compiler/meshc/tests/e2e.rs`, `compiler/meshc/tests/e2e_stdlib.rs`, `mesher/main.mpl`, `mesher/ingestion/pipeline.mpl`, `mesher/services/writer.mpl`
- New wiring introduced in this slice: a top-level `reference-backend/` package with env-driven startup, migration-managed schema, HTTP router/handlers, job storage module, timer-driven worker, smoke script, and compiler-facing e2e proof
- What remains before the milestone is truly usable end-to-end: stronger runtime correctness/failure-path coverage (S02), tooling hardening (S03), native deployment proof (S04), supervision/recovery trust (S05), and broader docs promotion (S06)

## Tasks

- [ ] **T01: Scaffold the `reference-backend/` package and startup contract** `est:1h`
  - Why: S01 needs a new narrow proof target; without a stable package boundary and env contract, later work will keep leaning on `mesher/` or ad-hoc commands.
  - Files: `reference-backend/mesh.toml`, `reference-backend/main.mpl`, `reference-backend/config.mpl`, `reference-backend/api/router.mpl`, `reference-backend/api/health.mpl`
  - Do: Create the top-level Mesh project, encode the `DATABASE_URL`/`PORT`/`JOB_POLL_MS` startup contract in package-local code, follow Mesher’s startup order (pool first, services next, HTTP serve last), and keep the initial HTTP surface to a real `GET /health` path wired through modules rather than a monolithic `main.mpl`.
  - Verify: `cargo build -p mesh-rt && cargo run -p meshc -- build reference-backend`
  - Done when: `reference-backend/` builds as a directory-based Mesh project, startup config is env-driven, and `/health` is reachable through package-local routing code.
- [ ] **T02: Add migration-managed jobs persistence and DB-backed API endpoints** `est:1h`
  - Why: The slice only advances R002 if one durable row shape is shared by migrations, storage code, and HTTP create/read handlers.
  - Files: `reference-backend/migrations/20260323010000_create_jobs.mpl`, `reference-backend/types/job.mpl`, `reference-backend/storage/jobs.mpl`, `reference-backend/api/jobs.mpl`, `reference-backend/api/router.mpl`, `reference-backend/main.mpl`
  - Do: Add a real Postgres `jobs` schema via `migrations/`, keep the record shape small and inspectable (`status`, `attempts`, `last_error`, timestamps, payload), implement create/read storage helpers, wire `POST /jobs` and `GET /jobs/:id`, and thread pool access through the package without reintroducing hard-coded connection strings.
  - Verify: `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo run -p meshc -- migrate status reference-backend && DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo run -p meshc -- migrate up reference-backend && cargo build -p mesh-rt && cargo run -p meshc -- build reference-backend`
  - Done when: the migration applies cleanly, the `jobs` table exists with the expected lifecycle fields, and the API modules can create/read one persisted job row by stable id.
- [ ] **T03: Wire the timer-driven worker and a package-local smoke path** `est:1h`
  - Why: CRUD alone is not the canonical backend proof; S01 must show one periodic background service mutating the same durable state the API exposes.
  - Files: `reference-backend/jobs/worker.mpl`, `reference-backend/storage/jobs.mpl`, `reference-backend/api/health.mpl`, `reference-backend/api/jobs.mpl`, `reference-backend/main.mpl`, `reference-backend/scripts/smoke.sh`
  - Do: Implement a timer-recursive worker patterned after Mesher’s long-running actors instead of `Job.async`, have it claim pending jobs and mark them processed, surface per-job diagnostics through storage/API responses, update health output to reflect worker readiness, and add a package-local smoke script that starts the binary, hits the API, and waits for the state transition.
  - Verify: `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} PORT=18080 JOB_POLL_MS=500 bash reference-backend/scripts/smoke.sh`
  - Done when: posting a job causes the persisted row to transition from `pending` to `processed` without manual DB edits, and the smoke script fails loudly on startup, HTTP, migration, or worker-processing regressions.
- [ ] **T04: Add compiler-facing e2e proof and canonical package documentation** `est:1h`
  - Why: Downstream slices need one mechanical proof target and one authoritative command reference; otherwise the golden path remains tribal knowledge.
  - Files: `compiler/meshc/tests/e2e_reference_backend.rs`, `reference-backend/README.md`, `reference-backend/.env.example`
  - Do: Reuse existing compiler e2e helpers to add an on-disk reference-backend test file, include a build-only test plus an ignored Postgres smoke test, document the exact prerequisite/build/migrate/run/smoke commands in the package README, and publish an `.env.example` that matches the code and test expectations.
  - Verify: `cargo build -p mesh-rt && cargo test -p meshc e2e_reference_backend_builds --test e2e_reference_backend -- --nocapture && DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc e2e_reference_backend_postgres_smoke --test e2e_reference_backend -- --ignored --nocapture`
  - Done when: the codebase contains one Rust test file and one package-local README that both execute the same golden-path contract without relying on stale root-level examples.

## Files Likely Touched

- `reference-backend/mesh.toml`
- `reference-backend/main.mpl`
- `reference-backend/config.mpl`
- `reference-backend/api/router.mpl`
- `reference-backend/api/health.mpl`
- `reference-backend/api/jobs.mpl`
- `reference-backend/types/job.mpl`
- `reference-backend/storage/jobs.mpl`
- `reference-backend/jobs/worker.mpl`
- `reference-backend/migrations/20260323010000_create_jobs.mpl`
- `reference-backend/scripts/smoke.sh`
- `reference-backend/README.md`
- `reference-backend/.env.example`
- `compiler/meshc/tests/e2e_reference_backend.rs`
