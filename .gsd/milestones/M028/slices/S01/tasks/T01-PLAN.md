---
estimated_steps: 4
estimated_files: 5
skills_used:
  - debug-like-expert
  - test
  - lint
---

# T01: Scaffold the `reference-backend/` package and startup contract

**Slice:** S01 — Canonical Backend Golden Path
**Milestone:** M028

## Description

Create a new top-level `reference-backend/` Mesh project instead of extending `mesher/`. This task establishes the stable package boundary and startup contract that the rest of S01 builds on. The executor should follow the real Mesh project contract: `meshc build` takes a project directory containing `main.mpl`, and `mesh.toml` should exist even if the package has no external dependencies yet. Reuse Mesher’s proven startup order (open pool, start long-lived pieces, then `HTTP.serve`) but keep the scope intentionally narrow: env parsing, pool wiring, modular router setup, and a real `GET /health` endpoint.

## Steps

1. Create `reference-backend/` as a top-level Mesh package with `mesh.toml`, `main.mpl`, and package-local modules rather than a single-file demo.
2. Add `reference-backend/config.mpl` to parse `DATABASE_URL`, `PORT`, and `JOB_POLL_MS` from the environment, and fail clearly when required values are missing.
3. Add `reference-backend/api/router.mpl` and `reference-backend/api/health.mpl`, and wire `GET /health` through those modules instead of defining routes inline in `main.mpl`.
4. Keep startup sequencing honest: open the Postgres pool from config, emit safe startup logs, and have `main.mpl` compose config, router, and HTTP serve in the order later tasks can extend.

## Must-Haves

- [ ] `reference-backend/` exists as a directory-based Mesh project with `main.mpl` and `mesh.toml`.
- [ ] Startup config is env-driven and does not hard-code a Postgres URL or port.
- [ ] `GET /health` is implemented through package-local modules and ready for later DB/worker checks.
- [ ] The package builds with `meshc build reference-backend` after `cargo build -p mesh-rt`.

## Verification

- `cargo build -p mesh-rt && cargo run -p meshc -- build reference-backend`
- Start the built binary with `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} PORT=18080 JOB_POLL_MS=1000 ./reference-backend/reference-backend` and confirm `curl -sf http://127.0.0.1:18080/health` returns success.

## Observability Impact

- Signals added/changed: startup log lines for config load, DB connect attempt, and HTTP bind
- How a future agent inspects this: process stdout plus `GET /health`
- Failure state exposed: missing env/config and pool-open failures become explicit instead of silent

## Inputs

- `mesher/main.mpl` — donor for the production startup order to copy in a narrower form
- `mesher/mesh.toml` — package metadata pattern for a top-level Mesh project
- `compiler/meshc/src/main.rs` — confirms `meshc build` expects a project directory containing `main.mpl`
- `compiler/meshc/tests/e2e_stdlib.rs` — existing HTTP runtime test shape to mirror when choosing port/health behavior
- `README.md` — stale single-file build example that this package must not repeat

## Expected Output

- `reference-backend/mesh.toml` — package metadata for the new canonical backend project
- `reference-backend/main.mpl` — env-driven startup entrypoint that opens the pool and serves HTTP
- `reference-backend/config.mpl` — startup contract parsing for `DATABASE_URL`, `PORT`, and `JOB_POLL_MS`
- `reference-backend/api/router.mpl` — router assembly module for the reference backend
- `reference-backend/api/health.mpl` — `GET /health` handler for the canonical backend surface
