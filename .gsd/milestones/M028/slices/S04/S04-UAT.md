# S04 UAT — Boring Native Deployment

## Scope

This UAT verifies the exact deployment workflow S04 added for `reference-backend/`: build on a build host, stage a thin native bundle, apply schema on the runtime side without `meshc`, start the staged binary outside the repo root, and smoke-check real HTTP + DB + job processing.

## Preconditions

1. Run from the repo root.
2. `.env` exists with a reachable Postgres `DATABASE_URL`.
3. Required local tools are available:
   - `cargo`
   - `bash`
   - `psql`
   - `curl`
   - `python3`
4. Load env before DB-backed checks:

```bash
set -a && source .env && set +a
```

---

## Test Case 1 — Stage a thin deploy bundle on the build host

### Steps

1. Create a temp bundle directory and stage the deploy bundle:

```bash
tmp_dir="$(mktemp -d)"
bash reference-backend/scripts/stage-deploy.sh "$tmp_dir"
```

2. Verify the staged runtime assets exist and are executable where expected:

```bash
test -x "$tmp_dir/reference-backend"
test -f "$tmp_dir/reference-backend.up.sql"
test -x "$tmp_dir/apply-deploy-migrations.sh"
test -x "$tmp_dir/deploy-smoke.sh"
```

3. Verify the bundle is thin and does not stage compiler/source dependencies:

```bash
test ! -e "$tmp_dir/meshc"
test ! -e "$tmp_dir/main.mpl"
```

### Expected outcome

- `stage-deploy.sh` prints `[stage-deploy]` build/staging/layout phases.
- The bundle contains exactly the runtime assets needed for deploy-time apply/start/smoke.
- The bundle does not rely on `meshc` or package source files being copied into the runtime directory.

---

## Test Case 2 — Apply schema through the staged SQL artifact, not through `meshc`

### Steps

1. Apply the staged SQL artifact through the staged apply script:

```bash
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} \
  bash "$tmp_dir/apply-deploy-migrations.sh" "$tmp_dir/reference-backend.up.sql"
```

2. Verify the migration record exists in Postgres:

```bash
psql "$DATABASE_URL" -Atqc "select version::text from _mesh_migrations where version = 20260323010000"
```

### Expected outcome

- Output includes `[deploy-apply] sql artifact=` and `[deploy-apply] migration recorded version=20260323010000`.
- `_mesh_migrations` contains version `20260323010000`.
- No `meshc migrate` call is needed on the runtime side.

---

## Test Case 3 — Start the staged binary from outside the repo root

### Steps

1. Pick a port and start the staged binary from the staged bundle directory:

```bash
PORT=18080
JOB_POLL_MS=500
log_file="$tmp_dir/runtime.log"
(
  cd "$tmp_dir"
  DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} \
  PORT="$PORT" \
  JOB_POLL_MS="$JOB_POLL_MS" \
  ./reference-backend >"$log_file" 2>&1
) &
server_pid=$!
```

2. Wait for health to come up:

```bash
for attempt in $(seq 1 40); do
  if curl -fsS "http://127.0.0.1:${PORT}/health"; then
    break
  fi
  sleep 0.25
done
```

3. Inspect the log for startup signals:

```bash
rg -n "Config loaded|PostgreSQL pool ready|Runtime registry ready|Job worker ready|HTTP server starting" "$log_file"
```

4. Verify the log does not echo the database URL:

```bash
if rg -n --fixed-strings "$DATABASE_URL" "$log_file"; then
  echo "unexpected DATABASE_URL leak in runtime log" >&2
  exit 1
fi
```

5. Stop the staged binary after this case if you are not continuing directly into Test Case 4:

```bash
kill "$server_pid"
wait "$server_pid" || true
```

### Expected outcome

- `/health` returns 200 JSON with `status: "ok"`.
- The staged binary starts successfully from the temp bundle directory.
- Startup logs show normal runtime phases and do not leak `DATABASE_URL`.

---

## Test Case 4 — Probe the running staged artifact end to end

### Steps

1. If the staged binary from Test Case 3 is not already running, restart it using the same command.
2. Run the probe-only smoke script against the running instance:

```bash
BASE_URL="http://127.0.0.1:${PORT}" \
  bash "$tmp_dir/deploy-smoke.sh"
```

3. Verify the returned JSON line shows a processed job.
4. Verify durable DB truth for the most recent job if desired:

```bash
psql "$DATABASE_URL" -Atqc "select status, attempts from jobs order by created_at desc limit 1"
```

5. Clean up the staged process:

```bash
kill "$server_pid"
wait "$server_pid" || true
```

### Expected outcome

- Output includes:
  - `[deploy-smoke] health ready body=`
  - `[deploy-smoke] created job body=`
  - `[deploy-smoke] polling job id=`
  - `[deploy-smoke] processed job id=`
- Final JSON shows the job reached `processed` with `attempts` = `1` and non-empty `processed_at`.
- The DB agrees that the newest job row is processed.

---

## Test Case 5 — Missing deploy SQL artifact fails with an actionable error

### Steps

1. Run the apply script against a missing SQL file:

```bash
missing_dir="$(mktemp -d)"
if bash reference-backend/scripts/apply-deploy-migrations.sh "$missing_dir/missing-reference-backend.up.sql" >"$missing_dir/apply.log" 2>&1; then
  echo "expected apply-deploy-migrations.sh to fail for a missing artifact" >&2
  exit 1
fi
cat "$missing_dir/apply.log"
```

### Expected outcome

- The command fails non-zero.
- The error names the missing artifact explicitly with `[deploy-apply] missing deploy SQL artifact:`.
- Failure happens before a generic `DATABASE_URL` complaint masks the real problem.

---

## Test Case 6 — Compiler-facing regression proof still covers the deploy path

### Steps

1. Run the build-only reference-backend proof:

```bash
cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_builds -- --nocapture
```

2. Run the self-contained binary proof:

```bash
cargo test -p meshc e2e_self_contained_binary -- --nocapture
```

3. Run the ignored deploy-artifact proof:

```bash
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo test -p meshc --test e2e_reference_backend e2e_reference_backend_deploy_artifact_smoke -- --ignored --nocapture
```

### Expected outcome

- The build proof passes.
- The self-contained binary proof passes.
- The ignored deploy-artifact proof passes and exercises staged bundle -> staged apply -> staged startup -> staged smoke -> DB/log/redaction assertions.

---

## Acceptance Bar

S04 is accepted only if all of the following are true:
- the deploy bundle stages successfully
- schema can be applied from the staged SQL artifact without `meshc`
- the staged binary starts from outside the repo root
- `/health` responds successfully
- a created job reaches `processed`
- `_mesh_migrations` records version `20260323010000`
- runtime/apply/smoke logs do not echo `DATABASE_URL`
- missing-artifact failure is specific and actionable
