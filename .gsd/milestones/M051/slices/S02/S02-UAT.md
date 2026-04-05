# S02: Extract retained backend-only proof out of reference-backend — UAT

**Milestone:** M051
**Written:** 2026-04-04T15:08:26.648Z

# S02: Extract retained backend-only proof out of reference-backend — UAT

**Milestone:** M051
**Written:** 2026-04-04

## UAT Type

- UAT mode: mixed
- Why this mode is sufficient: This slice ships both artifact-shape changes (retained fixture, bundle layout, verifier markers) and a real DB-backed runtime replay (migration, staged deploy, crash/restart recovery). Both surfaces must work to prove the backend-only proof really moved off the repo-root app path.

## Preconditions

- Run from the repo root.
- `cargo`, `bash`, `python3`, `psql`, and Docker are available locally.
- A disposable Postgres database is available and exported as `DATABASE_URL`.
- No tracked in-place binary exists at `scripts/fixtures/backend/reference-backend/reference-backend`.

## Smoke Test

1. Run `cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests`.
2. **Expected:** the retained fixture package tests pass, proving the internal fixture is present and healthy before any DB-backed replay.

## Test Cases

### 1. Retained fixture stages a deploy bundle without polluting the source tree

1. `tmp_dir="$(mktemp -d)"`
2. `bash scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh "$tmp_dir"`
3. `test -x "$tmp_dir/reference-backend"`
4. `test -f "$tmp_dir/reference-backend.up.sql"`
5. `test -x "$tmp_dir/apply-deploy-migrations.sh"`
6. `test -x "$tmp_dir/deploy-smoke.sh"`
7. `test ! -e scripts/fixtures/backend/reference-backend/reference-backend`
8. **Expected:** the staged bundle exists outside the repo root with the runtime binary plus SQL and helper scripts, and the retained fixture tree stays source-only.

### 2. Slice-owned contract rail and shared harness stay bound to the internal fixture

1. `cargo test -p meshc --test e2e_m051_s02 -- --nocapture`
2. `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_stage_deploy_bundle -- --nocapture`
3. **Expected:** `e2e_m051_s02` runs real tests and passes, and the legacy stage-deploy rail still passes while using the retained fixture/support-module path instead of depending on `reference-backend/reference-backend`.

### 3. Authoritative retained backend replay publishes green verifier markers and a proof bundle

1. Export `DATABASE_URL` for a disposable Postgres database.
2. Run `bash scripts/verify-m051-s02.sh`.
3. `test "$(cat .tmp/m051-s02/verify/status.txt)" = "ok"`
4. `test "$(cat .tmp/m051-s02/verify/current-phase.txt)" = "complete"`
5. `rg -F 'm051-s02-process-restart-recovery	passed' .tmp/m051-s02/verify/phase-report.txt`
6. `test -s .tmp/m051-s02/verify/latest-proof-bundle.txt`
7. `bundle_dir="$(cat .tmp/m051-s02/verify/latest-proof-bundle.txt)" && test -d "$bundle_dir/retained-reference-backend-runtime" && test -d "$bundle_dir/retained-fixture-smoke" && test -d "$bundle_dir/retained-contract-artifacts"`
8. **Expected:** the serial replay passes migration status/apply, package-local smoke, staged deploy smoke, worker crash recovery, worker restart visibility, and process restart recovery, then publishes a retained proof bundle pointer plus copied retained artifacts.

### 4. Staged deploy path still exercises the real backend runtime contract

1. `tmp_dir="$(mktemp -d)"`
2. `bash scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh "$tmp_dir"`
3. `bash "$tmp_dir/apply-deploy-migrations.sh" "$tmp_dir/reference-backend.up.sql"`
4. In one terminal, run `PORT=18080 JOB_POLL_MS=100 DATABASE_URL="$DATABASE_URL" "$tmp_dir/reference-backend"`.
5. In another terminal, run `PORT=18080 BASE_URL=http://127.0.0.1:18080 bash "$tmp_dir/deploy-smoke.sh"`.
6. **Expected:** `/health` becomes ready, `POST /jobs` returns a created job, the poll reaches `status=processed`, and the staged binary behaves like the retained backend contract rather than depending on repo-root source files.

## Edge Cases

### Missing staged SQL fails closed instead of drifting to the repo-root copy

1. `tmp_dir="$(mktemp -d)" && bash scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh "$tmp_dir" && mv "$tmp_dir/reference-backend.up.sql" "$tmp_dir/reference-backend.up.sql.bak"`
2. `if bash "$tmp_dir/apply-deploy-migrations.sh" "$tmp_dir/reference-backend.up.sql"; then false; fi`
3. **Expected:** the apply script exits non-zero immediately and reports the missing SQL artifact instead of silently falling back to repo-root `reference-backend/` files.

### Interrupted fixture-smoke workers do not poison later shared-DB phases

1. Leave a `.tmp/m051-s02/fixture-smoke/build/reference-backend` process running, or interrupt a prior `bash scripts/fixtures/backend/reference-backend/scripts/smoke.sh` run after the app starts.
2. Re-run `bash scripts/verify-m051-s02.sh`.
3. `ps -axo pid,command | rg '.tmp/m051-s02/fixture-smoke/build/reference-backend' | rg -v 'rg '`
4. **Expected:** the verifier cleans stale fixture-smoke workers before reusing the shared DB, the later deploy/recovery phases still pass, and the final process check is empty for that path.

## Failure Signals

- `.tmp/m051-s02/verify/status.txt` is not `ok` or `current-phase.txt` is not `complete` after the verifier exits.
- `phase-report.txt` is missing a `passed` marker for any required phase, especially `m051-s02-deploy-artifact-smoke` or `m051-s02-process-restart-recovery`.
- `m051-s02-deploy-artifact-smoke` fails with `staged worker never recorded a processed job in /health`; this points first to stale `.tmp/m051-s02/fixture-smoke/build/reference-backend` workers.
- A staged bundle run requires repo-root source files or leaves `scripts/fixtures/backend/reference-backend/reference-backend` behind.

## Requirements Proved By This UAT

- R119 — Advances the retirement path by keeping backend-only proof internal and maintainer-owned while `mesher/` remains the maintained deeper reference app.

## Not Proven By This UAT

- Public docs, scaffold output, and skill guidance retargeting away from repo-root `reference-backend/` (S04).
- Tooling/editor/LSP/formatter migration to the retained backend fixture (S03).
- Final deletion of repo-root `reference-backend/` and the post-deletion assembled acceptance rail (S05).

## Notes for Tester

Use a disposable Postgres database; if this worktree has no repo-local `DATABASE_URL`, a local Docker Postgres instance is sufficient. The retained proof bundle pointer lives at `.tmp/m051-s02/verify/latest-proof-bundle.txt`. Repo-root `reference-backend/` still being present is expected in this slice — it is a compatibility copy until later M051 slices retarget or delete it.
