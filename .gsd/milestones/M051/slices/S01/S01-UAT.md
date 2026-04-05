# S01: Modernize Mesher bootstrap and maintainer run path — UAT

**Milestone:** M051
**Written:** 2026-04-04T08:01:50.963Z

# S01: Modernize Mesher bootstrap and maintainer run path — UAT

**Milestone:** M051
**Mode:** Maintainer runbook and runtime proof

## Purpose

Validate the Mesher maintainer contract this slice shipped:

- Mesher startup follows `validate app config -> open PostgreSQL pool -> Node.start_from_env() -> foundation/listeners`.
- A maintainer can migrate, build, run, and smoke Mesher from the repo root without relying on `reference-backend/README.md`.
- The seeded default org/project/API key powers a real event ingest and readback through Mesher’s real app surface.
- The dedicated Mesher runtime rail and shell verifier retain redacted debug artifacts under `.tmp/m051-s01/`.

## Preconditions

- Run from the repo root.
- Rust/Cargo, Docker, Python 3, and `rg` are available.
- For the manual maintainer loop, a local PostgreSQL instance is reachable through `DATABASE_URL`.
- No concurrent `cargo test -p meshc --test e2e_m051_s01 -- --nocapture` or `bash scripts/verify-m051-s01.sh` runs are in flight; both rails write `.tmp/m051-s01/`.

## Test Cases

### 1. Automated assembled Mesher maintainer rail

1. Run `bash scripts/verify-m051-s01.sh`.
2. **Expected:** the command ends with `verify-m051-s01: ok`.
3. Inspect:
   - `.tmp/m051-s01/verify/status.txt`
   - `.tmp/m051-s01/verify/current-phase.txt`
   - `.tmp/m051-s01/verify/latest-proof-bundle.txt`
   - `.tmp/m051-s01/verify/phase-report.txt`
4. **Expected:**
   - `status.txt` is `ok`
   - `current-phase.txt` is `complete`
   - `latest-proof-bundle.txt` points at `.tmp/m051-s01/verify/retained-proof-bundle`
   - `phase-report.txt` contains passed markers for `init`, `m051-s01-package-tests`, `m051-s01-build`, `m051-s01-contract`, `m051-s01-e2e`, `retain-m051-s01-artifacts`, and `m051-s01-bundle-shape`
5. **Expected retained bundle shape:** `.tmp/m051-s01/verify/retained-proof-bundle/retained-m051-s01-artifacts/` contains exactly one copied `mesher-missing-database-url-*` directory and one copied `mesher-postgres-runtime-truth-*` directory.

### 2. Fail-closed startup without `DATABASE_URL`

1. Run `cargo run -q -p meshc -- build mesher`.
2. Run:

```bash
env -u DATABASE_URL \
  PORT=18080 \
  MESHER_WS_PORT=18081 \
  MESHER_RATE_LIMIT_WINDOW_SECONDS=60 \
  MESHER_RATE_LIMIT_MAX_EVENTS=1000 \
  MESH_CLUSTER_COOKIE=dev-cookie \
  MESH_NODE_NAME=mesher@127.0.0.1:4370 \
  MESH_DISCOVERY_SEED=localhost \
  MESH_CLUSTER_PORT=4370 \
  MESH_CONTINUITY_ROLE=primary \
  MESH_CONTINUITY_PROMOTION_EPOCH=0 \
  ./mesher/mesher
```

3. **Expected:** stdout/stderr contains `[Mesher] Config error: Missing required environment variable DATABASE_URL`.
4. **Expected:** output does **not** contain `Connecting to PostgreSQL pool...`, `Runtime ready`, or `HTTP server starting on :`.
5. **Expected:** the process exits instead of hanging.

### 3. Repo-root maintainer loop against PostgreSQL

1. Copy the sample env: `cp mesher/.env.example .env.mesher`.
2. Edit only `DATABASE_URL` to point at a real local PostgreSQL instance.
3. Load it: `set -a && source .env.mesher && set +a`.
4. Run `cargo run -q -p meshc -- test mesher/tests`.
5. Run `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo run -q -p meshc -- migrate mesher status`.
6. Run `DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo run -q -p meshc -- migrate mesher up`.
7. Run `cargo run -q -p meshc -- build mesher`.
8. Start Mesher with the command block from `mesher/README.md`.
9. **Expected logs, in order:**
   - `Config loaded`
   - `Connecting to PostgreSQL pool...`
   - `PostgreSQL pool ready`
   - `runtime bootstrap mode=...`
   - `Foundation ready`
   - `Runtime ready http_port=... ws_port=... db_backend=postgres`
   - `HTTP server starting on :<PORT>`

### 4. Live readiness and seeded event smoke

1. With Mesher from test case 3 still running, call `curl -sSf http://127.0.0.1:${PORT:-8080}/api/v1/projects/default/settings`.
2. **Expected:** JSON response includes `retention_days` and `sample_rate`, and does not include a `database_url` field.
3. Submit the README smoke event:

```bash
curl -sSf \
  -X POST \
  http://127.0.0.1:${PORT:-8080}/api/v1/events \
  -H 'Content-Type: application/json' \
  -H 'x-sentry-auth: mshr_devdefaultapikey000000000000000000000000000' \
  -d '{"message":"README smoke event","level":"error"}'
```

4. **Expected:** HTTP 202 with `{"status":"accepted"}`.
5. Fetch issues: `curl -sSf "http://127.0.0.1:${PORT:-8080}/api/v1/projects/default/issues?status=unresolved"`.
6. **Expected:** `data` contains an item with `title == "README smoke event"`, `level == "error"`, and `status == "unresolved"`. Capture its `id`.
7. Fetch storage: `curl -sSf http://127.0.0.1:${PORT:-8080}/api/v1/projects/default/storage`.
8. **Expected:** `event_count >= 1` and `estimated_bytes` is numeric.
9. Fetch issue events with the captured id: `curl -sSf http://127.0.0.1:${PORT:-8080}/api/v1/issues/<issue-id>/events`.
10. **Expected:** `data` contains an event whose `message == "README smoke event"` and whose `received_at` is non-empty.

### 5. Auth and malformed-payload rejection

1. POST `/api/v1/events` without `x-sentry-auth`.
2. **Expected:** HTTP 401 with `{"error":"unauthorized"}`.
3. POST `/api/v1/events` with `x-sentry-auth: mshr_invalid_key`.
4. **Expected:** HTTP 401 with `{"error":"unauthorized"}`.
5. POST `/api/v1/events` with valid auth but malformed JSON body `{"message":`.
6. **Expected:** HTTP 400 with a non-empty `error` string.
7. **Expected DB truth:** rerunning the issues/storage readback after only the rejected requests does not create extra persisted events.

### 6. Runtime-owned inspection commands

1. With Mesher still running, run `meshc cluster status "${MESH_NODE_NAME}" --json`.
2. Run `meshc cluster diagnostics "${MESH_NODE_NAME}" --json`.
3. Optionally run `meshc cluster continuity "${MESH_NODE_NAME}" --json`.
4. **Expected:** status and diagnostics return JSON for the running Mesher node on the runtime-owned CLI surface. The continuity list may be empty if no continuity-backed request exists yet; that is acceptable for this slice.

## Edge Cases

### Alternate positive ports

1. Set `PORT=18100` and `MESHER_WS_PORT=18101` in `.env.mesher`.
2. Repeat the startup and readiness smoke.
3. **Expected:** Mesher boots on the new ports, the settings and ingest smoke succeed on the new HTTP port, and no silent fallback to 8080/8081 occurs.

### Shared `.tmp/m051-s01` contention

1. Start `bash scripts/verify-m051-s01.sh`.
2. While it is still running, start a second `bash scripts/verify-m051-s01.sh` or `cargo test -p meshc --test e2e_m051_s01 -- --nocapture`.
3. **Expected:** this is not a supported operating mode. If phase files or retained bundles drift, kill the overlapping runs and replay once serially from a clean `.tmp/m051-s01/` tree.

## Failure Signals

- `verify-m051-s01: ok` is missing, `status.txt != ok`, `current-phase.txt != complete`, or `phase-report.txt` stops before `m051-s01-bundle-shape`.
- Missing `DATABASE_URL` still allows `Connecting to PostgreSQL pool...`, `Runtime ready`, or `HTTP server starting on :`.
- Settings/readiness response leaks `database_url` or does not expose `retention_days` / `sample_rate`.
- Authorized ingest does not return `{"status":"accepted"}`, or readback does not show the ingested issue/event on Mesher’s real endpoints.
- Unauthorized or malformed event submissions stop returning 401/400 with explicit error bodies.

## Requirements Advanced By This UAT

- **R119** — Mesher now replays on the current runtime/bootstrap contract and has a package-local maintainer runbook plus dedicated proof rail, which is the first concrete step toward replacing `reference-backend/` as the deeper maintained reference app.

## Notes for Tester

- Debug from `.tmp/m051-s01/verify/phase-report.txt` and `latest-proof-bundle.txt` first. The copied `mesher-missing-database-url-*` and `mesher-postgres-runtime-truth-*` bundles retain the exact build, migrate, runtime, HTTP, and DB artifacts from the authoritative e2e replay.
- If manual local PostgreSQL behavior diverges from the Docker-backed rail, trust `cargo test -p meshc --test e2e_m051_s01 -- --nocapture` as the slice’s authoritative proof surface first, then reconcile local env drift separately.
