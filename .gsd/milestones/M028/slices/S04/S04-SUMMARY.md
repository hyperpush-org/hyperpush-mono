# S04 Summary — Boring Native Deployment

## Status

Done. S04 turned `reference-backend/` into an artifact-first native deployment proof instead of a repo-only development story.

## Slice Goal

Prove that `reference-backend/` can be built once, staged as a native bundle, migrated on the runtime side without `meshc`, started from outside the repo root, and smoke-checked through the real HTTP + DB + background-job path.

## What This Slice Actually Delivered

### 1. A boring staged deployment bundle

S04 established a concrete staged bundle shape produced by `reference-backend/scripts/stage-deploy.sh`:

```text
<bundle-dir>/reference-backend
<bundle-dir>/reference-backend.up.sql
<bundle-dir>/apply-deploy-migrations.sh
<bundle-dir>/deploy-smoke.sh
```

That bundle is intentionally thin:
- native `reference-backend` binary
- checked-in deploy SQL artifact
- runtime-side migration apply script
- probe-only smoke script

It intentionally does **not** require `meshc`, `cargo`, `libmesh_rt.a`, or a source checkout on the runtime host after staging.

### 2. A runtime-side migration path that does not depend on Mesh tooling

S04 kept `reference-backend/migrations/20260323010000_create_jobs.mpl` as the canonical Mesh migration for dev/CI, but added a separate checked-in deploy artifact at:

- `reference-backend/deploy/reference-backend.up.sql`

`reference-backend/scripts/apply-deploy-migrations.sh` applies that artifact through `psql`, verifies `_mesh_migrations`, and stops with a named missing-artifact error before falling through to generic env failures.

This is the core deployment pattern S04 established:
- Mesh migration files remain the source of truth for compiler-driven workflows.
- Runtime deployment uses a staged SQL artifact and `psql`, not `meshc migrate`.

### 3. A probe-only smoke path for already-running staged artifacts

`reference-backend/scripts/deploy-smoke.sh` verifies a running instance without rebuilding anything. It:
- waits for `GET /health`
- creates a job through `POST /jobs`
- polls `GET /jobs/:id`
- exits only after the job becomes `processed`

That makes the operator/runtime-host workflow separate from the build-host workflow.

### 4. Compiler-facing proof that the staged artifact works outside the repo root

`compiler/meshc/tests/e2e_reference_backend.rs` now proves the deployment story mechanically:
- stages the bundle into a temp directory outside the repo root
- asserts the bundle contains only the intended runtime assets
- applies the staged SQL artifact through the staged apply script
- starts the staged binary from the staged bundle directory
- runs the staged `deploy-smoke.sh`
- cross-checks `/health`, `/jobs/:id`, `jobs`, and `_mesh_migrations`
- asserts deploy/apply/smoke output and runtime logs do **not** echo `DATABASE_URL`

This turns the deployment story into a real regression surface instead of a doc claim.

### 5. Operator-facing docs that match the verified path

`reference-backend/README.md` now documents:
- build-host vs runtime-host responsibilities
- exact staging command
- exact runtime-side schema apply command
- exact staged binary startup command
- exact staged smoke command
- the thin runtime contract: `DATABASE_URL`, `PORT`, `JOB_POLL_MS`

`reference-backend/.env.example` now matches that runtime contract directly.

## Files / Surfaces That Matter

- `reference-backend/deploy/reference-backend.up.sql`
- `reference-backend/scripts/stage-deploy.sh`
- `reference-backend/scripts/apply-deploy-migrations.sh`
- `reference-backend/scripts/deploy-smoke.sh`
- `reference-backend/scripts/smoke.sh`
- `compiler/meshc/tests/e2e_reference_backend.rs`
- `reference-backend/README.md`
- `reference-backend/.env.example`

## Verification Run for Slice Closure

All slice-level verification passed.

### Passed commands

```bash
cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_builds -- --nocapture
```

```bash
tmp_dir="$(mktemp -d)" && bash reference-backend/scripts/stage-deploy.sh "$tmp_dir" && test -x "$tmp_dir/reference-backend" && test -f "$tmp_dir/reference-backend.up.sql" && test -x "$tmp_dir/deploy-smoke.sh"
```

```bash
cargo test -p meshc e2e_self_contained_binary -- --nocapture
```

```bash
set -a && source .env && set +a && DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_deploy_artifact_smoke -- --ignored --nocapture
```

```bash
tmp_dir="$(mktemp -d)" && if bash reference-backend/scripts/apply-deploy-migrations.sh "$tmp_dir/missing-reference-backend.up.sql" >"$tmp_dir/apply-missing.log" 2>&1; then echo "expected apply-deploy-migrations.sh to fail for a missing artifact" >&2; exit 1; else rg -n "\[deploy-apply\] missing deploy SQL artifact" "$tmp_dir/apply-missing.log"; fi
```

```bash
rg -n "Boring native deployment|stage-deploy\.sh|apply-deploy-migrations\.sh|deploy-smoke\.sh|runtime host" reference-backend/README.md && rg -n "^DATABASE_URL=|^PORT=|^JOB_POLL_MS=" reference-backend/.env.example
```

## Observability / Diagnostics Confirmed

S04’s named operational surfaces are live and useful:
- `[stage-deploy]` phases identify build and bundle layout steps
- `[deploy-apply]` phases identify SQL artifact and migration-recording steps
- `[deploy-smoke]` phases identify health, create, poll, and processed-job transitions
- missing deploy SQL artifact errors stop with the specific artifact-path message
- the ignored deploy e2e confirms `DATABASE_URL` redaction across apply output, smoke output, and staged runtime logs

The durable truth surfaces for debugging this path are:
- `GET /health`
- `GET /jobs/:id`
- `jobs`
- `_mesh_migrations`
- staged stdout/stderr logs

## Requirement / Roadmap Impact

- **R005** is now validated: Mesh has one honest boring native deployment path for `reference-backend/`.
- **R008** is supported but not fully closed here: package-local docs are now honest and concrete, but broader proof-surface/documentation promotion still belongs to S06.
- **R010** is supported with concrete evidence for the “easier deployment” claim, but the broader differentiator story still needs later milestone work.

## Decisions / Patterns Established

### Established deployment pattern

Use an artifact-first deployment path:
1. build on the build host
2. stage a thin bundle
3. apply staged SQL through `psql`
4. start the staged binary from the bundle directory
5. verify with the probe-only smoke script

### Important architectural boundary

Keep compiler-driven migrations and runtime deployment migrations separate:
- `meshc migrate` remains for developer/build-host workflows
- staged SQL + `psql` is the runtime-host path

That boundary is what makes the deployment story boring instead of turning the runtime host into a Mesh workstation.

## What The Next Slices Should Know

### For S05

S05 should build on the exact staged runtime contract established here:
- binary: staged `reference-backend`
- runtime env: `DATABASE_URL`, `PORT`, `JOB_POLL_MS`
- schema truth: `_mesh_migrations`
- operational truth: `/health`, `/jobs/:id`, `jobs`, staged logs

If supervision/recovery work changes startup or worker behavior, it should preserve this staged-binary contract rather than reintroducing repo/build-tool dependencies on the runtime host.

### For S06

S06 should reuse the exact commands and bundle shape already proven here. It should promote this deployment story honestly, but it should not overclaim broader packaging/platform support that S04 did not prove.

## Remaining Limits

S04 does **not** prove:
- supervision/restart correctness under crash conditions
- broader deployment targets beyond this boring staged-bundle workflow
- broader docs/example promotion beyond the package-local operator path

Those belong to S05 and S06.
