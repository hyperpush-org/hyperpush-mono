---
title: Production Backend Proof
description: Canonical proof surface for Mesh's real backend package, staged deploy path, supervision signals, and documentation-truth checks
prev: false
next: false
---

# Production Backend Proof

This is the public entrypoint for Mesh's real backend proof surface.

It is intentionally small: the deepest operator/developer runbook lives in [`reference-backend/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/reference-backend/README.md), and this page tells you which concrete commands back the public claims.

## Canonical surfaces

- [`reference-backend/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/reference-backend/README.md) — deepest runbook for build, runtime, staged deploy, supervision, and recovery interpretation
- `compiler/meshc/tests/e2e_reference_backend.rs` — authoritative Rust harness for the backend e2e proofs
- `reference-backend/scripts/deploy-smoke.sh` — probe-only staged deploy smoke contract
- `reference-backend/scripts/verify-production-proof-surface.sh` — documentation-truth verifier for this public proof surface

## What the backend proof covers

The `reference-backend/` package is the canonical proof target for backend claims in this repo. It proves one real Mesh backend can compose:

- env-driven startup validation
- Postgres migrations
- `GET /health`
- `POST /jobs`
- `GET /jobs/:id`
- a supervised worker that processes persisted jobs
- a staged deploy bundle with a smoke-check script
- worker-crash recovery and whole-process restart recovery that stay visible in `/health`
- a repeatable documentation verifier that keeps the public proof links honest

## Named proof commands

These are the repo-level commands behind the public proof story:

```bash
cargo run -p meshc -- build reference-backend
cargo run -p meshc -- fmt --check reference-backend
cargo run -p meshc -- test reference-backend
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_migration_status_and_apply -- --ignored --nocapture
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_deploy_artifact_smoke -- --ignored --nocapture
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_restart_is_visible_in_health -- --ignored --nocapture
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_process_restart_recovers_inflight_job -- --ignored --nocapture
bash reference-backend/scripts/verify-production-proof-surface.sh
```

> **Note:** Run the ignored `e2e_reference_backend` database-backed proofs serially against a single `DATABASE_URL`. They share reset/migrate state and are not safe to run in parallel against the same database.

## Recovery signals to inspect

The runbook in `reference-backend/README.md` explains the full interpretation, but these are the public recovery fields that matter:

- `restart_count`
- `last_exit_reason`
- `recovered_jobs`
- `last_recovery_at`
- `last_recovery_job_id`
- `last_recovery_count`
- `recovery_active`

Interpret them with the named proofs above:

- **Worker crash recovery:** expect `restart_count=1`, `last_exit_reason="worker_crash_after_claim"`, and `recovered_jobs=1`.
- **Whole-process restart recovery:** expect a new worker `boot_id` / `started_at`, `recovered_jobs=1`, and `last_exit_reason=null` because the worker did not exit under in-process supervision; the backend process was replaced.
- **Recovery window visibility:** while recovery is in progress, `/health` should be `status: "degraded"`, worker `liveness: "recovering"`, and `recovery_active=true`. Once the recovered job finishes, the backend should return to `status: "ok"`, worker `liveness: "healthy"`, and `recovery_active=false`.

## When to use this page vs the generic guides

Use the generic guides when you want to learn a subsystem API such as HTTP routing, database access, supervision, tooling, or testing.

Use this page and `reference-backend/README.md` when you want to evaluate whether Mesh currently proves a real backend workflow end to end instead of inferring readiness from feature tutorials.

## Failure inspection map

If a proof fails, rerun the named command for the exact surface you care about:

- **Backend assembly or formatting:** `meshc build`, `meshc fmt --check`, `meshc test`
- **Migration truth:** `e2e_reference_backend_migration_status_and_apply`
- **Staged deploy path:** `e2e_reference_backend_deploy_artifact_smoke` and `reference-backend/scripts/deploy-smoke.sh`
- **Worker crash recovery:** `e2e_reference_backend_worker_crash_recovers_job`
- **Restart visibility in `/health`:** `e2e_reference_backend_worker_restart_is_visible_in_health`
- **Whole-process restart recovery:** `e2e_reference_backend_process_restart_recovers_inflight_job`
- **Documentation drift:** `bash reference-backend/scripts/verify-production-proof-surface.sh`

For the full environment contract and the full supervision/recovery runbook, continue to [`reference-backend/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/reference-backend/README.md).
