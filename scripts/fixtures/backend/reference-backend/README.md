# retained reference-backend fixture

This README is the canonical maintainer runbook for the retained backend-only proof surface in M051/S02. `scripts/fixtures/backend/reference-backend/` is a maintainer-only/internal fixture that preserves the same package identity (`reference-backend`) and the same backend-only Postgres contract (`GET /health`, `POST /jobs`, `GET /jobs/:id`, staged deploy SQL, worker crash recovery, restart visibility, and whole-process restart recovery) while later slices retarget or delete the repo-root compatibility copy.

## Startup contract

The retained runtime and the staged native bundle still require the same three startup variables:

- `DATABASE_URL` — required Postgres connection string
- `PORT` — required positive integer HTTP port
- `JOB_POLL_MS` — required positive integer worker poll interval in milliseconds

Package-local scripts intentionally fail closed if these inputs or their required artifacts are missing.

## Repo-root maintainer loop

From the repo root, the retained fixture’s fast proof loop is:

```bash
cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo run -q -p meshc -- migrate scripts/fixtures/backend/reference-backend status
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo run -q -p meshc -- migrate scripts/fixtures/backend/reference-backend up
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} PORT=18080 JOB_POLL_MS=500 bash scripts/fixtures/backend/reference-backend/scripts/smoke.sh
```

- `meshc test` keeps the retained fixture and its package-local scripts/source-only contract aligned.
- `meshc migrate ... status|up` is the direct maintainer migration surface for the retained fixture path.
- `scripts/smoke.sh` builds the internal fixture into `.tmp/m051-s02/fixture-smoke/`, starts the runtime from that artifact-local binary, waits for `GET /health`, and proves one job can move to `processed` without writing a binary back into this source tree.

## Staged deploy bundle

For the boring staged deploy path, keep the staged bundle outside the repo root and publish only bundle pointers/manifests back under `.tmp/m051-s02/verify/`.

Build the bundle from the repo root:

```bash
tmp_dir="$(mktemp -d)" && bash scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh "$tmp_dir"
```

The staged layout is:

```text
$tmp_dir/reference-backend
$tmp_dir/reference-backend.up.sql
$tmp_dir/apply-deploy-migrations.sh
$tmp_dir/deploy-smoke.sh
```

`stage-deploy.sh` must leave this fixture source-only while staging the executable and deploy SQL into the requested bundle directory.

On the runtime host, apply the staged SQL artifact without invoking `meshc` again:

```bash
bundle_dir=/opt/reference-backend
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} \
  bash "$bundle_dir/apply-deploy-migrations.sh" "$bundle_dir/reference-backend.up.sql"
```

Then start the staged binary from the staged directory:

```bash
bundle_dir=/opt/reference-backend
(
  cd "$bundle_dir"
  DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} \
  PORT=18080 \
  JOB_POLL_MS=500 \
  ./reference-backend
)
```

Finally, probe the running staged artifact:

```bash
bundle_dir=/opt/reference-backend
BASE_URL=http://127.0.0.1:18080 \
  bash "$bundle_dir/deploy-smoke.sh"
```

`deploy-smoke.sh` must fail closed on missing commands, malformed `BASE_URL`/`PORT`, or a job that never reaches `processed`.

## Live runtime smoke

Use the package-local smoke script when you want the retained fixture’s own build/run/probe flow without materializing a staged deploy bundle:

```bash
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} \
PORT=18080 \
JOB_POLL_MS=500 \
bash scripts/fixtures/backend/reference-backend/scripts/smoke.sh
```

That script is the maintainer-facing proof that:

- the retained fixture path can still be built directly with `meshc`
- the runtime binary lands under `.tmp/m051-s02/fixture-smoke/build/`
- `GET /health` becomes ready from the artifact-local binary
- one job can be created and observed at `processed`
- the retained fixture source tree stays source-only afterward

## `/health` recovery interpretation

`GET /health` remains the backend-only recovery surface to inspect when the recovery rails fail.

The important fields are:

- `restart_count` — worker-supervisor restart count
- `last_exit_reason` — last worker-supervisor exit reason
- `recovered_jobs` — count of jobs requeued from interrupted processing
- `last_recovery_at` — timestamp for the most recent recovery event
- `last_recovery_job_id` — the job requeued during the most recent recovery event
- `last_recovery_count` — number of jobs requeued during the most recent recovery event
- `recovery_active` — `true` during the degraded recovery window and `false` after healthy recovery finishes

Interpret the retained backend rails this way:

- **Worker crash recovery** — `e2e_reference_backend_worker_crash_recovers_job` should surface `restart_count=1`, `last_exit_reason="worker_crash_after_claim"`, `recovered_jobs=1`, and a degraded `/health` window before the job returns to `processed`.
- **Restart visibility** — `e2e_reference_backend_worker_restart_is_visible_in_health` should expose the degraded window itself, including changed worker `boot_id` / `started_at` plus populated `last_recovery_*` fields before the backend settles back to `status: "ok"` and worker `liveness: "healthy"`.
- **Whole-process restart recovery** — `e2e_reference_backend_process_restart_recovers_inflight_job` should requeue a `processing` row after a full process kill/restart, preserve `last_recovery_*`, set `recovered_jobs=1`, and keep `last_exit_reason=null` because the recovery came from boot recovery rather than a worker-supervisor crash.

If migration or staged deploy proof is green but one of these recovery interpretations drifts, treat that as a real runtime regression rather than as documentation-only drift.

## Authoritative proof rail

The slice-owned acceptance rail is:

```bash
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} bash scripts/verify-m051-s02.sh
```

That verifier is the authoritative retained replay for M051/S02. It runs, in order:

- cheap contract checks for this README, the package-local scripts, the slice contract target, and the preserved repo-root compatibility files
- `cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests`
- `cargo test -p meshc --test e2e_m051_s02 -- --nocapture`
- `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_migration_status_and_apply -- --ignored --nocapture`
- `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} PORT=18080 JOB_POLL_MS=500 bash scripts/fixtures/backend/reference-backend/scripts/smoke.sh`
- `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_deploy_artifact_smoke -- --ignored --nocapture`
- `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_crash_recovers_job -- --ignored --nocapture`
- `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_worker_restart_is_visible_in_health -- --ignored --nocapture`
- `cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_process_restart_recovers_inflight_job -- --ignored --nocapture`

On success the verifier leaves:

- `.tmp/m051-s02/verify/status.txt`
- `.tmp/m051-s02/verify/current-phase.txt`
- `.tmp/m051-s02/verify/phase-report.txt`
- `.tmp/m051-s02/verify/full-contract.log`
- `.tmp/m051-s02/verify/latest-proof-bundle.txt`

Follow `.tmp/m051-s02/verify/latest-proof-bundle.txt` into the copied runtime artifacts when you need the retained bundle instead of the phase logs alone.

## Compatibility boundary

Do not delete or retarget the repo-root compatibility path in this slice. `reference-backend/README.md` and `reference-backend/scripts/verify-production-proof-surface.sh` remain available for later M051 slices that still need the public compatibility surface while this retained fixture becomes the maintainer authority for backend-only proof.
