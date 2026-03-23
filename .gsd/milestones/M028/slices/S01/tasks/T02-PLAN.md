---
estimated_steps: 5
estimated_files: 6
skills_used:
  - debug-like-expert
  - test
  - lint
---

# T02: Add migration-managed jobs persistence and DB-backed API endpoints

**Slice:** S01 — Canonical Backend Golden Path
**Milestone:** M028

## Description

Turn the package skeleton into a real API + DB + migrations proof path. The reference backend should stay intentionally small: one durable `jobs` record shape shared by migrations, storage helpers, and HTTP handlers. Use Postgres, not SQLite, because `meshc migrate` is Postgres-only. Keep the schema and responses inspectable for later slices: a job should expose `id`, `status`, `attempts`, `last_error`, timestamps, and payload. Use raw SQL only where the migration DSL cannot express what is needed.

## Steps

1. Add a real migration file under `reference-backend/migrations/` that creates the `jobs` table and any minimal indexes needed for the pending-work scan.
2. Define a shared job shape in `reference-backend/types/job.mpl` so storage and API modules agree on field names and response semantics.
3. Implement create/read storage helpers in `reference-backend/storage/jobs.mpl` for inserting a pending job and loading a job by id.
4. Implement `POST /jobs` and `GET /jobs/:id` in `reference-backend/api/jobs.mpl`, returning stable JSON that reflects the durable row state.
5. Wire the new handlers into the router/startup path from T01 without reintroducing inline connection strings or route duplication.

## Must-Haves

- [ ] `meshc migrate status reference-backend` discovers the new migration file.
- [ ] `meshc migrate up reference-backend` can create the `jobs` table on Postgres.
- [ ] `POST /jobs` persists a new row with `pending` status and returns a stable id.
- [ ] `GET /jobs/:id` reads the durable row back through the same package-local storage layer.

## Verification

- `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo run -p meshc -- migrate status reference-backend && DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo run -p meshc -- migrate up reference-backend`
- `cargo build -p mesh-rt && cargo run -p meshc -- build reference-backend`

## Observability Impact

- Signals added/changed: migration status output, API-visible job state, and storage-layer error propagation
- How a future agent inspects this: `meshc migrate status reference-backend`, `GET /jobs/:id`, and direct inspection of the `jobs` table
- Failure state exposed: missing/pending migration state and insert/read failures stop being invisible

## Inputs

- `reference-backend/main.mpl` — startup composition from T01 that will receive DB/API wiring
- `reference-backend/config.mpl` — env contract supplying the Postgres connection string
- `reference-backend/api/router.mpl` — route assembly to extend with job endpoints
- `mesher/migrations/20260216120000_create_initial_schema.mpl` — real migration donor pattern for Mesh + Postgres
- `compiler/meshc/src/migrate.rs` — authoritative migration discovery and execution contract

## Expected Output

- `reference-backend/migrations/20260323010000_create_jobs.mpl` — migration-managed Postgres schema for the canonical job lifecycle
- `reference-backend/types/job.mpl` — shared job record shape used by storage and HTTP
- `reference-backend/storage/jobs.mpl` — create/read persistence helpers for the `jobs` table
- `reference-backend/api/jobs.mpl` — `POST /jobs` and `GET /jobs/:id` handlers
- `reference-backend/api/router.mpl` — router updated to expose the new endpoints
- `reference-backend/main.mpl` — startup composition updated for migration-era storage/API wiring
