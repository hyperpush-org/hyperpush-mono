---
id: S01
parent: M051
milestone: M051
provides:
  - Mesher on the scaffold-style maintainer contract: validated `DATABASE_URL`/port/rate-limit env, local Postgres pool open before runtime bootstrap, and `Node.start_from_env()` as the only clustered bootstrap path.
  - A dedicated Mesher maintainer proof surface through `cargo test -p meshc --test e2e_m051_s01 -- --nocapture`, with Docker Postgres, seeded default data, real app-surface ingest/readback, and redacted retained artifacts under `.tmp/m051-s01/`.
  - A canonical package-local maintainer runbook at `mesher/README.md` plus `bash scripts/verify-m051-s01.sh` as the fail-closed replay for migrate/build/run/live-smoke drift.
requires:
  []
affects:
  - S02
  - S03
  - S04
  - M051 milestone validation and closeout
key_files:
  - mesher/config.mpl
  - mesher/tests/config.test.mpl
  - mesher/main.mpl
  - mesher/ingestion/pipeline.mpl
  - mesher/.env.example
  - compiler/meshc/tests/support/m051_mesher.rs
  - compiler/meshc/tests/e2e_m051_s01.rs
  - compiler/meshc/tests/support/mod.rs
  - mesher/README.md
  - scripts/verify-m051-s01.sh
  - .gsd/PROJECT.md
key_decisions:
  - Validate Mesher env config and open the PostgreSQL pool before calling `Node.start_from_env()`, and use `Node.start_from_env()` as the only clustered bootstrap path.
  - Use a dedicated Mesher Rust support harness with Docker Postgres and redacted `.tmp/m051-s01` artifacts instead of reusing the older M033 hardcoded Mesher helpers.
  - Build Mesher to an artifact-local binary with `meshc build mesher --output ...` so the proof rail does not churn the tracked `mesher/mesher` package output.
  - Use `mesher/README.md` plus `scripts/verify-m051-s01.sh` as the canonical maintainer-facing Mesher run surface, and keep public README/VitePress docs untouched until the later docs slice.
  - Verifier self-audits should check the concrete `run_expect_success` phase-to-command mapping and retained bundle shape instead of raw denylist substring absence.
patterns_established:
  - For deeper clustered/backend Mesh apps, keep startup ordered as `validate app config -> open dependencies -> Node.start_from_env() -> listener/service startup` so bad app config and dependency failures stay fail-closed before runtime readiness.
  - Dedicated proof rails should build binaries under the artifact root with `meshc build ... --output ...` instead of reusing tracked package outputs; that keeps retained bundles self-contained and avoids churning repo-owned binaries during verification.
  - Maintainer-facing runbooks should be paired with one fail-closed verifier that checks commands, env keys, route/header names, and retained bundle markers against the same live e2e target instead of trusting prose alone.
  - Repo-owned verifier scripts that self-audit their own source should assert concrete replay phase -> command mappings rather than naive substring absence, or the verifier can fail on its own guard list before any real contract drift exists.
observability_surfaces:
  - .tmp/m051-s01/verify/status.txt
  - .tmp/m051-s01/verify/current-phase.txt
  - .tmp/m051-s01/verify/phase-report.txt
  - .tmp/m051-s01/verify/full-contract.log
  - .tmp/m051-s01/verify/latest-proof-bundle.txt
  - .tmp/m051-s01/verify/retained-proof-bundle/
  - .tmp/m051-s01/verify/retained-m051-s01-artifacts/
  - .tmp/m051-s01/verify/retained-proof-bundle/retained-m051-s01-artifacts/
  -  .tmp/m051-s01/verify/retained-proof-bundle/retained-m051-s01-artifacts/mesher-postgres-runtime-truth-*/runtime.stdout.log
  - .tmp/m051-s01/verify/retained-proof-bundle/retained-m051-s01-artifacts/mesher-postgres-runtime-truth-*/runtime.stderr.log
  - .tmp/m051-s01/verify/retained-proof-bundle/retained-m051-s01-artifacts/mesher-postgres-runtime-truth-*/project-settings-ready.json
  - .tmp/m051-s01/verify/retained-proof-bundle/retained-m051-s01-artifacts/mesher-postgres-runtime-truth-*/events-ingest-accepted.json
  - .tmp/m051-s01/verify/retained-proof-bundle/retained-m051-s01-artifacts/mesher-postgres-runtime-truth-*/project-issues-readback.json
  - .tmp/m051-s01/verify/retained-proof-bundle/retained-m051-s01-artifacts/mesher-postgres-runtime-truth-*/issue-events-readback.json
drill_down_paths:
  - .gsd/milestones/M051/slices/S01/tasks/T01-SUMMARY.md
  - .gsd/milestones/M051/slices/S01/tasks/T02-SUMMARY.md
  - .gsd/milestones/M051/slices/S01/tasks/T03-SUMMARY.md
duration: ""
verification_result: passed
completed_at: 2026-04-04T08:01:50.957Z
blocker_discovered: false
---

# S01: Modernize Mesher bootstrap and maintainer run path

**Mesher now boots on the scaffold-style runtime contract, has a dedicated Postgres-backed maintainer proof rail, and ships a canonical package-local runbook plus fail-closed verifier.**

## What Happened

S01 turned `mesher/` into the current deeper maintainer app surface instead of a stale donor from older backend work. T01 added `mesher/config.mpl`, package tests for the env contract, and a maintainer `.env.example`, then rewrote `mesher/main.mpl` so startup is `validate config -> open PostgreSQL pool -> Node.start_from_env() -> foundation/listeners`. The old hardcoded DSN and app-owned peer/bootstrap path are gone, `mesher/ingestion/pipeline.mpl` now consumes validated rate-limit values, and bad app config fails before any runtime or listener readiness claim.

T02 gave Mesher its own runtime proof surface instead of leaving confidence buried in older M033 rails. `compiler/meshc/tests/support/m051_mesher.rs` now starts a fresh Docker `postgres:16` container on a random host port, runs `meshc migrate mesher up`, builds Mesher to an artifact-local binary with `meshc build mesher --output ...`, spawns the runtime with the new env contract, and retains redacted build/migrate/stdout/stderr/HTTP/DB artifacts under `.tmp/m051-s01/`. The dedicated `e2e_m051_s01` target proves both the fail-closed missing-`DATABASE_URL` path and the real Postgres-backed app path: seeded default org/project/API key, 401s for missing/invalid auth, 400 for malformed JSON, 202 accepted event ingest, persisted `issues`/`events` rows, and readback through Mesher’s real settings, storage, issues, and issue-events surfaces.

T03 converted that runtime truth into a maintainer surface instead of tribal knowledge. `mesher/README.md` is now the canonical repo-root Mesher runbook for migrate/build/run/live smoke, `scripts/verify-m051-s01.sh` replays package tests, build, runbook/verifier contract checks, the dedicated e2e target, and retained-bundle shape checks, and the Rust e2e target pins the README/verifier contract in the same test target as the live runtime proof. Public README/VitePress docs were intentionally left alone in this slice: Mesher is now the deeper maintainer path for M051, not a reintroduced evaluator-facing first-contact path.

This slice also established the current M051 boundary for downstream work: Mesher can now carry the maintained deeper-app contract on current Mesh bootstrap/runtime patterns, while S02-S04 still need to extract the old `reference-backend/` proof surfaces, move bounded tooling rails to a smaller retained fixture, and only then retarget public docs around the new deeper-reference story.

## Verification

All slice-plan rails passed from the current tree:
- `cargo run -q -p meshc -- test mesher/tests`
- `cargo run -q -p meshc -- build mesher`
- `cargo test -p meshc --test e2e_m051_s01 -- --nocapture`
- `bash scripts/verify-m051-s01.sh`

The dedicated `e2e_m051_s01` target completed 4 passing tests: fail-closed missing `DATABASE_URL`, live Docker/Postgres migrate/build/run/ingest/readback, README contract, and verifier contract. The assembled shell verifier finished with `.tmp/m051-s01/verify/status.txt = ok`, `.tmp/m051-s01/verify/current-phase.txt = complete`, `.tmp/m051-s01/verify/latest-proof-bundle.txt -> .tmp/m051-s01/verify/retained-proof-bundle`, and a `phase-report.txt` that shows passed phases for `init`, `m051-s01-package-tests`, `m051-s01-build`, `m051-s01-contract`, `m051-s01-e2e`, `retain-m051-s01-artifacts`, and `m051-s01-bundle-shape`.

## Operational Readiness

- **Health signal:** a healthy Mesher boot logs `Config loaded`, `Connecting to PostgreSQL pool...`, `PostgreSQL pool ready`, `runtime bootstrap mode=...`, `Foundation ready`, `Runtime ready http_port=... ws_port=... db_backend=postgres`, and `HTTP server starting on :...`; the maintainer readiness probe is `GET /api/v1/projects/default/settings`, and the verifier health markers are `.tmp/m051-s01/verify/status.txt = ok` plus `.tmp/m051-s01/verify/current-phase.txt = complete`.
- **Failure signal:** missing `DATABASE_URL` now fails closed with `[Mesher] Config error: Missing required environment variable DATABASE_URL` before any pool-open or HTTP-start log; live runtime/auth/readback drift is localized by redacted `runtime.stdout.log`, `runtime.stderr.log`, HTTP snapshots, DB snapshots, and per-phase verifier logs under `.tmp/m051-s01/`.
- **Recovery procedure:** load `mesher/.env.example` into a local env file, set a real `DATABASE_URL`, rerun `cargo run -q -p meshc -- migrate mesher status`, `cargo run -q -p meshc -- migrate mesher up`, `cargo run -q -p meshc -- build mesher`, start `./mesher/mesher`, then replay `bash scripts/verify-m051-s01.sh`; if the verifier fails, start from `phase-report.txt` and `latest-proof-bundle.txt` before opening the copied scenario bundle.
- **Monitoring gaps:** the verifier and the e2e rail both write `.tmp/m051-s01/`, so concurrent runs can contend for the same retained-artifact root; runtime inspection commands are documented in `mesher/README.md`, but the maintainer verifier currently checks that contract text rather than executing the operator CLI calls live.

## Requirements Advanced

- R119 — Mesher now runs on the current scaffold-style runtime/bootstrap contract, has a package-local maintainer runbook, and has its own dedicated proof rail, which is the first concrete step toward replacing `reference-backend/` as the maintained deeper reference app.

## Requirements Validated

None.

## New Requirements Surfaced

None.

## Requirements Invalidated or Re-scoped

None.

## Deviations

None.

## Known Limitations

This slice modernizes Mesher’s maintainer/runtime path only. `reference-backend/` still exists, bounded tooling/editor/formatter rails still point at older backend-shaped surfaces until S02/S03, public README/VitePress docs intentionally still avoid promoting Mesher until S04, and `.tmp/m051-s01/` is a shared artifact root that should be used serially.

## Follow-ups

S02 should extract the backend-only retained proof that still matters out of `reference-backend/` while keeping the new Mesher runbook green. S03 should move bounded tooling/editor/formatter rails to a smaller retained backend-shaped fixture instead of full Mesher. S04 should retarget public docs, scaffold copy, and skills to point at scaffold/examples first and Mesher second once the deeper-reference story is complete.

## Files Created/Modified

- `mesher/config.mpl` — Added the canonical Mesher startup env helpers, defaults, and fail-closed error-text helpers.
- `mesher/tests/config.test.mpl` — Pinned the Mesher env-key names, local defaults, and missing/invalid config messages.
- `mesher/main.mpl` — Reworked startup around validated config, Postgres pool open, runtime-owned bootstrap via `Node.start_from_env()`, and maintainer-facing startup guidance.
- `mesher/ingestion/pipeline.mpl` — Switched pipeline startup to consume validated rate-limit config instead of hidden defaults.
- `mesher/.env.example` — Published the maintainer env contract for local Postgres plus the runtime-owned `MESH_*` bootstrap variables.
- `compiler/meshc/tests/support/m051_mesher.rs` — Added the dedicated Docker/Postgres Mesher harness that builds artifact-local binaries, starts Mesher, and retains redacted runtime/HTTP/DB artifacts.
- `compiler/meshc/tests/e2e_m051_s01.rs` — Added the fail-closed missing-`DATABASE_URL`, live Postgres-backed ingest/readback, README contract, and verifier contract rails for Mesher.
- `compiler/meshc/tests/support/mod.rs` — Exported the new `m051_mesher` test-support harness for the dedicated Mesher runtime target.
- `mesher/README.md` — Published the canonical maintainer runbook for Mesher’s migrate/build/run/live-smoke loop.
- `scripts/verify-m051-s01.sh` — Added the fail-closed Mesher maintainer verifier with package/build/e2e replays, contract checks, phase markers, and retained-bundle assertions.
- `.gsd/PROJECT.md` — Updated current project state to record that M051/S01 is complete and that Mesher now owns the deeper maintainer runtime contract.
