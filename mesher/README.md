# Mesher

This README is the canonical maintainer runbook for Mesher's current PostgreSQL + runtime contract. It is intentionally maintainer-facing: deeper Mesher work should start here, while public first-contact docs stay untouched until the later docs slice.

## Startup contract

Mesher validates configuration locally, opens the PostgreSQL pool, and only then boots the runtime through `Node.start_from_env()`.

### Required for every run

- `DATABASE_URL` — PostgreSQL connection string

### Local-development defaults

- `PORT` — HTTP port (`8080` by default)
- `MESHER_WS_PORT` — WebSocket port (`8081` by default)
- `MESHER_RATE_LIMIT_WINDOW_SECONDS` — rate-limit window size (`60` by default)
- `MESHER_RATE_LIMIT_MAX_EVENTS` — rate-limit budget per window (`1000` by default)

### Cluster/runtime env

These stay on the runtime-owned contract that `Node.start_from_env()` expects:

- `MESH_CLUSTER_COOKIE`
- `MESH_NODE_NAME`
- `MESH_DISCOVERY_SEED`
- `MESH_CLUSTER_PORT`
- `MESH_CONTINUITY_ROLE`
- `MESH_CONTINUITY_PROMOTION_EPOCH`

`mesher/.env.example` carries the current local-development values for that full set.

## Seeded development data

`mesher/migrations/20260226000000_seed_default_org.mpl` inserts the local smoke data that this runbook and the S01 e2e rail prove:

- organization slug: `default`
- project slug: `default`
- dev API key label: `dev-default`
- dev API key: `mshr_devdefaultapikey000000000000000000000000000`

That seed is idempotent, so a maintainer can rerun migrations and keep using the same default project + API key when working locally.

## Repo-root maintainer loop

### 1. Load local env

From the repo root:

```bash
cp mesher/.env.example .env.mesher
# Update DATABASE_URL for your local Postgres, then load it.
set -a && source .env.mesher && set +a
```

### 2. Run the package tests

```bash
cargo run -q -p meshc -- test mesher/tests
```

### 3. Inspect migration state

```bash
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo run -q -p meshc -- migrate mesher status
```

### 4. Apply migrations

```bash
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} cargo run -q -p meshc -- migrate mesher up
```

### 5. Build Mesher from the repo root

```bash
cargo run -q -p meshc -- build mesher
```

That build writes the runnable binary to `./mesher/mesher`.

### 6. Run Mesher

```bash
DATABASE_URL=${DATABASE_URL:?set DATABASE_URL} \
PORT=${PORT:-8080} \
MESHER_WS_PORT=${MESHER_WS_PORT:-8081} \
MESHER_RATE_LIMIT_WINDOW_SECONDS=${MESHER_RATE_LIMIT_WINDOW_SECONDS:-60} \
MESHER_RATE_LIMIT_MAX_EVENTS=${MESHER_RATE_LIMIT_MAX_EVENTS:-1000} \
MESH_CLUSTER_COOKIE=${MESH_CLUSTER_COOKIE:-dev-cookie} \
MESH_NODE_NAME=${MESH_NODE_NAME:-mesher@127.0.0.1:4370} \
MESH_DISCOVERY_SEED=${MESH_DISCOVERY_SEED:-localhost} \
MESH_CLUSTER_PORT=${MESH_CLUSTER_PORT:-4370} \
MESH_CONTINUITY_ROLE=${MESH_CONTINUITY_ROLE:-primary} \
MESH_CONTINUITY_PROMOTION_EPOCH=${MESH_CONTINUITY_PROMOTION_EPOCH:-0} \
./mesher/mesher
```

On a healthy boot, Mesher should log:

- `Config loaded ...`
- `Connecting to PostgreSQL pool...`
- `PostgreSQL pool ready`
- `runtime bootstrap mode=...`
- `Foundation ready`
- `Runtime ready http_port=... ws_port=... db_backend=postgres ...`
- `HTTP server starting on :8080`

## Live seed-event smoke

Use the seeded default project and dev API key against the real Mesher HTTP surface.

### Readiness check

```bash
curl -sSf http://127.0.0.1:8080/api/v1/projects/default/settings
```

Expected shape: JSON with `retention_days` and `sample_rate`.

### Event ingest smoke

```bash
curl -sSf \
  -X POST \
  http://127.0.0.1:8080/api/v1/events \
  -H 'Content-Type: application/json' \
  -H 'x-sentry-auth: mshr_devdefaultapikey000000000000000000000000000' \
  -d '{"message":"README smoke event","level":"error"}'
```

Expected shape: `{"status":"accepted"}`.

### Read back the seeded project issues

```bash
curl -sSf 'http://127.0.0.1:8080/api/v1/projects/default/issues?status=unresolved'
```

The returned `data` array should include the newly ingested `README smoke event` row.

### Optional storage readback

```bash
curl -sSf http://127.0.0.1:8080/api/v1/projects/default/storage
```

That surface exposes the current `event_count` and `estimated_bytes` for the seeded default project.

## Runtime inspection

When you boot Mesher with the clustered env above, inspect runtime-owned state through Mesh CLI surfaces instead of package-owned control routes:

```bash
meshc cluster status <node-name@host:port> --json
meshc cluster continuity <node-name@host:port> --json
meshc cluster continuity <node-name@host:port> <request-key> --json
meshc cluster diagnostics <node-name@host:port> --json
```

Use the continuity list form first if you need to discover runtime-owned records.

## Authoritative proof rail

The repo-owned verifier for this maintainer surface is:

```bash
bash scripts/verify-m051-s01.sh
```

That wrapper replays:

- `cargo run -q -p meshc -- test mesher/tests`
- `cargo run -q -p meshc -- build mesher`
- `cargo test -p meshc --test e2e_m051_s01 -- --nocapture`
- fail-closed README contract checks for commands, env keys, route names, header names, and seeded smoke values

If the runtime stays green but `bash scripts/verify-m051-s01.sh` fails, treat that as maintainer-runbook drift instead of silently updating commands from memory.
