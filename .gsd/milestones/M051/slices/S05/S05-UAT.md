# S05: Delete reference-backend and close the assembled acceptance rail — UAT

**Milestone:** M051
**Written:** 2026-04-04T23:51:27.387Z

# S05 UAT — Post-deletion acceptance rail

**Milestone:** M051

## Preconditions
- Repository is checked out at the completed M051/S05 state.
- `cargo`, `bash`, `node`, `npm`, `docker`, and `python3` are available on PATH.
- The repo root does **not** contain `reference-backend/` anymore.
- A disposable local Postgres instance is available for the DB-backed retained rails. One concrete setup is:
  ```bash
  docker run --rm --name m051-s05-pg \
    -e POSTGRES_PASSWORD=postgres \
    -e POSTGRES_DB=mesh_m051_s05 \
    -p 127.0.0.1:51798:5432 \
    postgres:16
  ```
- Use a matching `DATABASE_URL`, for example:
  ```bash
  export DATABASE_URL=postgresql://postgres:postgres@127.0.0.1:51798/mesh_m051_s05
  ```

## Test Case 1 — The repo-root compatibility tree is gone and the proof-page verifier survives at the top level
1. Run `test ! -e reference-backend`.
   - Expected: exit code 0; the repo-root compatibility tree is absent.
2. Run `bash scripts/verify-production-proof-surface.sh`.
   - Expected: it reports the production proof surface verified.
3. Open `website/docs/docs/production-backend-proof/index.md`.
   - Expected: it names `bash scripts/verify-production-proof-surface.sh`, `mesher/README.md`, `bash scripts/verify-m051-s01.sh`, and `bash scripts/verify-m051-s02.sh`, and it does **not** point at `reference-backend/scripts/verify-production-proof-surface.sh`.

## Test Case 2 — Public docs and historical wrapper/source contracts stay green after deletion
1. Run `node --test scripts/tests/verify-m036-s03-contract.test.mjs`.
   - Expected: all tests pass.
2. Run `node --test scripts/tests/verify-m050-s02-first-contact-contract.test.mjs`.
   - Expected: all tests pass.
3. Run `node --test scripts/tests/verify-m050-s03-secondary-surfaces.test.mjs`.
   - Expected: all tests pass.
4. Run `cargo test -p meshc --test e2e_m050_s01 -- --nocapture`.
   - Expected: targeted docs/proof-page contract tests pass.
5. Run `cargo test -p meshc --test e2e_m050_s03 -- --nocapture`.
   - Expected: targeted secondary-surface contract tests pass.
6. Run `cargo test -p meshc --test e2e_m051_s04 -- --nocapture`.
   - Expected: the slice-owned S04 contract target passes on the post-deletion tree.

## Test Case 3 — The retained backend-only fixture still proves migrations, smoke, deploy artifact, and recovery after deletion
1. Run `cargo test -p meshc --test e2e_m051_s02 -- --nocapture`.
   - Expected: all 7 tests pass, including the deleted-path contract and retained bundle-shape guards.
2. Run `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} bash scripts/verify-m051-s02.sh`.
   - Expected: it finishes with `verify-m051-s02: ok`.
3. Inspect `.tmp/m051-s02/verify/status.txt`, `.tmp/m051-s02/verify/current-phase.txt`, and `.tmp/m051-s02/verify/phase-report.txt`.
   - Expected: `status.txt` is `ok`, `current-phase.txt` is `complete`, and every phase from `m051-s02-contract` through `m051-s02-bundle-shape` is marked `passed`.
4. Inspect `.tmp/m051-s02/verify/m051-s02-fixture-smoke.log`.
   - Expected: the deploy-smoke handoff prints health poll lines and only creates the smoke job after `status=ok`, `liveness=healthy`, and `recovery_active=false`.
5. Open `.tmp/m051-s02/verify/latest-proof-bundle.txt` and inspect the pointed bundle.
   - Expected: it contains `fixture.README.md`, `verify-m051-s02.sh`, `retained-reference-backend-runtime/`, `retained-fixture-smoke/`, and `retained-contract-artifacts/`.

## Test Case 4 — The slice-owned post-deletion source contract still guards the final closeout rail
1. Run `cargo test -p meshc --test e2e_m051_s05 -- --nocapture`.
   - Expected: both tests pass.
2. Inspect the retained artifacts under `.tmp/m051-s05/` that the test creates.
   - Expected: the archived source contracts reference the top-level proof-page verifier and the S01-S04 wrappers, and do not resurrect repo-root `reference-backend/` paths.

## Test Case 5 — The final assembled acceptance rail replays Mesher, retained backend, tooling, and docs together
1. Run `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} bash scripts/verify-m051-s05.sh`.
   - Expected: it finishes with `verify-m051-s05: ok`.
2. Inspect `.tmp/m051-s05/verify/status.txt`, `.tmp/m051-s05/verify/current-phase.txt`, and `.tmp/m051-s05/verify/phase-report.txt`.
   - Expected: `status.txt` is `ok`, `current-phase.txt` is `complete`, and the phase report marks `m051-s01-wrapper`, `m051-s02-wrapper`, `m051-s03-wrapper`, `m051-s04-wrapper`, and the retained bundle phases as `passed`.
3. Open `.tmp/m051-s05/verify/latest-proof-bundle.txt`.
   - Expected: it points at `.tmp/m051-s05/verify/retained-proof-bundle`.
4. Inspect `.tmp/m051-s05/verify/retained-proof-bundle/`.
   - Expected: it contains `retained-m051-s01-verify/` + `retained-m051-s01-proof-bundle/`, `retained-m051-s02-verify/` + `retained-m051-s02-proof-bundle/`, `retained-m051-s03-verify/` + `retained-m051-s03-proof-bundle/`, `retained-m051-s04-verify/` + `retained-m051-s04-proof-bundle/`, plus `e2e_m051_s05.rs`, `verify-m051-s05.sh`, `scripts.verify-production-proof-surface.sh`, and `retained-m051-s05-artifacts.manifest.txt`.
5. Inspect each copied child verify directory’s `latest-proof-bundle.txt`.
   - Expected: every copied child pointer resolves to the copied child bundle inside the S05 retained bundle, not to a live external `.tmp` tree.

## Edge Cases
- If the retained backend smoke phase stalls with jobs stuck at `pending`, inspect `.tmp/m051-s02/verify/m051-s02-fixture-smoke.log` first and confirm the deploy-smoke health poll reached the healthy `/health` contract before job creation.
- If retained backend recovery rails go red, trust the split contract: `e2e_reference_backend_worker_crash_recovers_job` proves crash/requeue/processed correctness, while `e2e_reference_backend_worker_restart_is_visible_in_health` proves preserved restart metadata on final `/health`; do not reopen the S05 wrapper first.
- If the final bundle-shape phase fails, inspect the copied child `latest-proof-bundle.txt` files inside `.tmp/m051-s05/verify/retained-proof-bundle/` rather than the retired repo-root backend path.
