---
id: T03
parent: S05
milestone: M051
provides: []
requires: []
affects: []
key_files: ["scripts/fixtures/backend/reference-backend/README.md", "scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl", "scripts/fixtures/backend/reference-backend/deploy/reference-backend.up.sql", "compiler/meshc/tests/e2e_m051_s02.rs", "scripts/verify-m051-s02.sh", ".gitignore", ".gsd/KNOWLEDGE.md"]
key_decisions: ["Treat the repo-root delete surface as an explicit verifier phase instead of inferring deletion from retained bundle contents.", "Retain post-deletion bundle evidence with `.gitignore` plus the top-level proof-page verifier rather than copying the deleted compatibility README."]
patterns_established: []
drill_down_paths: []
observability_surfaces: []
duration: ""
verification_result: "`test ! -e reference-backend`, `cargo test -p meshc --test e2e_m051_s02 -- --nocapture`, `bash scripts/verify-production-proof-surface.sh`, `cargo test -p meshc --test e2e_m050_s01 -- --nocapture`, and `cargo test -p meshc --test e2e_m050_s03 -- --nocapture` all passed after the post-deletion contract updates. The first full `bash scripts/verify-m051-s02.sh` replay failed fast because `DATABASE_URL` was unset. After the user clarified the expected local-Docker path, a second replay against a disposable `postgres:16` container advanced through migrations and fixture smoke but failed in `m051-s02-deploy-artifact-smoke`; the retained log is `.tmp/m051-s02/verify/m051-s02-deploy-artifact-smoke.log`."
completed_at: 2026-04-04T22:36:50.934Z
blocker_discovered: false
---

# T03: Deleted the repo-root reference-backend tree and rewrote the retained S02 contracts to post-deletion truth.

> Deleted the repo-root reference-backend tree and rewrote the retained S02 contracts to post-deletion truth.

## What Happened
---
id: T03
parent: S05
milestone: M051
key_files:
  - scripts/fixtures/backend/reference-backend/README.md
  - scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl
  - scripts/fixtures/backend/reference-backend/deploy/reference-backend.up.sql
  - compiler/meshc/tests/e2e_m051_s02.rs
  - scripts/verify-m051-s02.sh
  - .gitignore
  - .gsd/KNOWLEDGE.md
key_decisions:
  - Treat the repo-root delete surface as an explicit verifier phase instead of inferring deletion from retained bundle contents.
  - Retain post-deletion bundle evidence with `.gitignore` plus the top-level proof-page verifier rather than copying the deleted compatibility README.
duration: ""
verification_result: mixed
completed_at: 2026-04-04T22:36:50.935Z
blocker_discovered: false
---

# T03: Deleted the repo-root reference-backend tree and rewrote the retained S02 contracts to post-deletion truth.

**Deleted the repo-root reference-backend tree and rewrote the retained S02 contracts to post-deletion truth.**

## What Happened

Rewrote the retained backend fixture README, package test, deploy SQL provenance comment, S02 Rust contract target, and `scripts/verify-m051-s02.sh` so they describe and assert the post-deletion repo shape instead of preserving the old compatibility copy. Removed the repo-root `reference-backend/` tree and the legacy `.gitignore` binary rule. The updated verifier now treats `test ! -e reference-backend` as an explicit phase and snapshots `.gitignore` plus the top-level proof-page verifier in the retained bundle instead of copying the deleted compatibility README. The remaining red rail is localized to the DB-backed deploy-artifact smoke path inside `scripts/verify-m051-s02.sh` when replayed against a disposable local Postgres container.

## Verification

`test ! -e reference-backend`, `cargo test -p meshc --test e2e_m051_s02 -- --nocapture`, `bash scripts/verify-production-proof-surface.sh`, `cargo test -p meshc --test e2e_m050_s01 -- --nocapture`, and `cargo test -p meshc --test e2e_m050_s03 -- --nocapture` all passed after the post-deletion contract updates. The first full `bash scripts/verify-m051-s02.sh` replay failed fast because `DATABASE_URL` was unset. After the user clarified the expected local-Docker path, a second replay against a disposable `postgres:16` container advanced through migrations and fixture smoke but failed in `m051-s02-deploy-artifact-smoke`; the retained log is `.tmp/m051-s02/verify/m051-s02-deploy-artifact-smoke.log`.

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `test ! -e reference-backend` | 0 | ✅ pass | 10ms |
| 2 | `/usr/bin/time -p cargo test -p meshc --test e2e_m051_s02 -- --nocapture` | 0 | ✅ pass | 44590ms |
| 3 | `/usr/bin/time -p bash scripts/verify-production-proof-surface.sh` | 0 | ✅ pass | 5640ms |
| 4 | `/usr/bin/time -p cargo test -p meshc --test e2e_m050_s01 -- --nocapture` | 0 | ✅ pass | 48070ms |
| 5 | `/usr/bin/time -p cargo test -p meshc --test e2e_m050_s03 -- --nocapture` | 0 | ✅ pass | 52320ms |
| 6 | `bash -c 'if [ -f .env ]; then set -a; source .env; set +a; fi; /usr/bin/time -p bash scripts/verify-m051-s02.sh'` | 1 | ❌ fail | 75630ms |
| 7 | `bash -c 'docker run postgres:16 ...; DATABASE_URL=postgresql://postgres:postgres@127.0.0.1:<port>/mesh_m051_s02 /usr/bin/time -p bash scripts/verify-m051-s02.sh'` | 1 | ❌ fail | 206870ms |


## Deviations

Used a disposable local Docker Postgres container for the retained verifier after the secure env flow was skipped and the user explicitly directed local-Docker testing instead of manual URL entry.

## Known Issues

`bash scripts/verify-m051-s02.sh` is still red in `m051-s02-deploy-artifact-smoke`. The staged deploy-artifact job reaches `processed`, but the health-alignment assertion still sees `worker.last_job_id` from an earlier job; inspect `.tmp/m051-s02/verify/m051-s02-deploy-artifact-smoke.log` before changing delete-surface work again.

## Files Created/Modified

- `scripts/fixtures/backend/reference-backend/README.md`
- `scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl`
- `scripts/fixtures/backend/reference-backend/deploy/reference-backend.up.sql`
- `compiler/meshc/tests/e2e_m051_s02.rs`
- `scripts/verify-m051-s02.sh`
- `.gitignore`
- `.gsd/KNOWLEDGE.md`


## Deviations
Used a disposable local Docker Postgres container for the retained verifier after the secure env flow was skipped and the user explicitly directed local-Docker testing instead of manual URL entry.

## Known Issues
`bash scripts/verify-m051-s02.sh` is still red in `m051-s02-deploy-artifact-smoke`. The staged deploy-artifact job reaches `processed`, but the health-alignment assertion still sees `worker.last_job_id` from an earlier job; inspect `.tmp/m051-s02/verify/m051-s02-deploy-artifact-smoke.log` before changing delete-surface work again.
