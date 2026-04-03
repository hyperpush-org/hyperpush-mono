---
title: Distributed Proof
description: Public proof map for the scaffold/examples-first clustered story, the serious PostgreSQL Todo starter, the honest SQLite local starter, and the lower-level retained fixture rails behind them.
prev: false
next: false
---

# Distributed Proof

Mesh exposes one clustered-work story through two public clustered layers, plus one intentionally local starter:

- [Clustered Example](/docs/getting-started/clustered-example/) — the public scaffold-first walkthrough for `meshc init --clustered`
- [`examples/todo-postgres/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README.md) — the serious shared/deployable PostgreSQL starter
- [`examples/todo-sqlite/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README.md) — the honest local single-node SQLite starter
- [`reference-backend/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/reference-backend/README.md) — the deeper backend proof surface once you need the heavier runbook

The clustered public surfaces keep the same source-first runtime-owned contract, and the PostgreSQL Todo starter adds one bounded read-route adoption on top of it:

- `mesh.toml` stays package-only
- `work.mpl` owns an ordinary `@cluster pub fn add()` / `@cluster pub fn sync_todos()`-style declaration instead of a helper-shaped public entrypoint
- `main.mpl` boots only through `Node.start_from_env()` on the scaffold and PostgreSQL starter
- the runtime automatically starts declared work at startup
- operators inspect truth through `meshc cluster status`, continuity list, continuity record, and diagnostics
- the PostgreSQL Todo starter keeps `work.mpl` route-free, dogfoods explicit-count `HTTP.clustered(1, ...)` on `GET /todos` and `GET /todos/:id`, and keeps `GET /health` plus mutating routes local instead of inventing package-owned cluster control planes
- the SQLite starter stays outside this clustered proof surface on purpose: The SQLite Todo starter is the honest local path: single-node SQLite, generated package tests, and no `work.mpl`, `HTTP.clustered(...)`, or `meshc cluster` story

If you are migrating older clustered code, move `clustered(work)` into source-first `@cluster`, delete any `[cluster]` manifest stanza, and rename helper-shaped entries such as `execute_declared_work(...)` / `Work.execute_declared_work` to ordinary verbs like `add()` or `sync_todos()`. Keep the route-free `@cluster` surfaces canonical: the PostgreSQL Todo starter only dogfoods explicit-count `HTTP.clustered(1, ...)` on `GET /todos` and `GET /todos/:id`, while `GET /health` and mutating routes stay local. Default-count and two-node clustered-route behavior stay on the repo S07 rail (`cargo test -p meshc --test e2e_m047_s07 -- --nocapture`).

## Public surfaces and verifier rails

- [Clustered Example](/docs/getting-started/clustered-example/) — first stop for the public scaffold surface
- [`examples/todo-postgres/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README.md) — the fuller shared/deployable starter that keeps the same source-first contract while adding PostgreSQL, selected read-route `HTTP.clustered(1, ...)`, and Docker packaging
- [`examples/todo-sqlite/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README.md) — the honest local single-node SQLite starter, not a clustered/operator proof surface
- [`reference-backend/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/reference-backend/README.md) — the deeper backend proof surface after the starter examples
- `bash scripts/verify-m047-s04.sh` — the authoritative cutover rail for the source-first route-free clustered contract
- `bash scripts/verify-m047-s05.sh` — the retained historical clustered Todo subrail kept behind fixture-backed rails instead of the public starter contract
- `cargo test -p meshc --test e2e_m047_s07 -- --nocapture` — the repo S07 rail for default-count and two-node `HTTP.clustered(...)` behavior beyond the PostgreSQL Todo starter's explicit-count read routes
- `bash scripts/verify-m047-s06.sh` — the docs and retained-proof closeout rail that wraps S05, rebuilds docs truth, and owns the assembled `.tmp/m047-s06/verify` bundle
- `bash scripts/verify-m046-s06.sh` — the historical M046 closeout wrapper retained as a compatibility alias into the M047 cutover rail
- `bash scripts/verify-m046-s05.sh` — the historical M046 equal-surface wrapper retained as a compatibility alias into the M047 cutover rail
- `bash scripts/verify-m046-s04.sh` — the historical M046 package/startup wrapper retained as a compatibility alias into the M047 cutover rail
- `bash scripts/verify-m045-s05.sh` — the historical M045 closeout wrapper retained as a compatibility alias into the M047 cutover rail
- `bash scripts/verify-m045-s04.sh` — the historical M045 assembled wrapper retained as a compatibility alias into the M047 cutover rail
- `bash scripts/verify-m045-s03.sh` — historical failover-specific subrail
- `bash scripts/verify-m043-s04-fly.sh` — read-only Fly sanity/config/log/probe verifier

The lower-level retained fixture rails still exist for repo-owned proof, but they now live under `scripts/fixtures/clustered/tiny-cluster/` and `scripts/fixtures/clustered/cluster-proof/` instead of public README onboarding runbooks.

## What the public clustered contract proves

The public clustered story is intentionally smaller than a generic clustering marketing page:

- start with `meshc init --clustered`, then branch to the generated Postgres or SQLite example that matches the contract you actually want
- keep `meshc init --template todo-api --db postgres` as the fuller shared/deployable starter without changing the source-first `@cluster` contract
- keep `meshc init --template todo-api --db sqlite` on its honest local single-node contract instead of projecting clustered/operator claims onto it
- keep clustered declaration state in source instead of the manifest
- rename legacy helper-shaped names to ordinary verbs instead of preserving runtime-plumbing-shaped public APIs
- let the runtime own startup, placement, continuity, promotion, recovery, and diagnostics
- use the same operator flow everywhere: status, continuity list, continuity record, diagnostics
- keep the PostgreSQL Todo starter's clustered-route adoption narrow: `work.mpl` stays route-free, `GET /todos` and `GET /todos/:id` use explicit-count `HTTP.clustered(1, ...)`, and `GET /health` plus mutating routes stay local application routes
- defer default-count and two-node `HTTP.clustered(...)` behavior to `cargo test -p meshc --test e2e_m047_s07 -- --nocapture` instead of implying the public starter surfaces already prove it
- keep Fly as read-only evidence instead of destructive failover proof
- keep `reference-backend` as the deeper backend proof surface rather than a coequal first-contact clustered starter

## Named proof commands

These are the repo-level commands behind the current distributed proof story:

```bash
bash scripts/verify-m047-s04.sh
bash scripts/verify-m047-s05.sh
cargo test -p meshc --test e2e_m047_s07 -- --nocapture
bash scripts/verify-m047-s06.sh
bash scripts/verify-m046-s06.sh
bash scripts/verify-m046-s05.sh
bash scripts/verify-m046-s04.sh
bash scripts/verify-m045-s05.sh
bash scripts/verify-m045-s04.sh
bash scripts/verify-m045-s03.sh
cargo run -q -p meshc -- build scripts/fixtures/clustered/tiny-cluster
cargo run -q -p meshc -- test scripts/fixtures/clustered/tiny-cluster/tests
cargo run -q -p meshc -- build scripts/fixtures/clustered/cluster-proof
cargo run -q -p meshc -- test scripts/fixtures/clustered/cluster-proof/tests
npm --prefix website run build
bash scripts/verify-m043-s04-fly.sh --help
CLUSTER_PROOF_FLY_APP=mesh-cluster-proof \
CLUSTER_PROOF_BASE_URL=https://mesh-cluster-proof.fly.dev \
  bash scripts/verify-m043-s04-fly.sh
```

> **Note:** The Fly verifier is intentionally read-only. Use `bash scripts/verify-m043-s04-fly.sh --help` when you only want the non-live syntax/help path. Live mode inspects an already-deployed app and optionally reads an existing continuity key with `CLUSTER_PROOF_REQUEST_KEY`; it does not create new work or mutate authority.

## Operator workflow across the public clustered surfaces

Whichever public surface you start from, the operator flow stays the same:

1. `meshc cluster status <node-name@host:port> --json`
2. `meshc cluster continuity <node-name@host:port> --json`
3. `meshc cluster continuity <node-name@host:port> <request-key> --json`
4. `meshc cluster diagnostics <node-name@host:port> --json`

Use the list form first to discover startup or request keys. Only then drill into a single continuity record.

## Supported topology and non-goals

Supported topology and operator seam:

- one primary plus one standby using the same image and the same repo packaging path
- small env surface: cookie, discovery seed, explicit identity injection, continuity role, and promotion epoch
- same-image local proof for destructive failover and rejoin truth
- read-only Fly inspection for already-deployed apps

Non-goals for this public rail:

- active-active writes or active-active intake
- multi-standby quorum or consensus claims
- package-owned operator surfaces that compete with the runtime CLI
- presenting retained internal fixtures as the public onboarding story
- projecting clustered/operator claims onto the SQLite starter
- claiming the PostgreSQL starter already proves default-count or two-node `HTTP.clustered(...)` behavior that is actually owned by the repo S07 rail
- destructive failover on Fly as a required proof surface

## When to use this page vs the generic distributed guide

Use the generic [Distributed Actors](/docs/distributed/) guide when you want the language/runtime primitives.

Use this page when you want the named proof surfaces behind the scaffold/examples-first clustered story, the PostgreSQL starter, the SQLite-local boundary, the deeper `reference-backend` proof, and the repo-owned S07 clustered-route rail.

## Failure inspection map

If a proof fails, rerun the named command for the exact surface you care about:

- **Docs + retained-proof closeout rail:** `bash scripts/verify-m047-s06.sh`
- **Historical clustered Todo subrail:** `bash scripts/verify-m047-s05.sh`
- **Repo S07 clustered-route rail:** `cargo test -p meshc --test e2e_m047_s07 -- --nocapture`
- **Authoritative cutover rail:** `bash scripts/verify-m047-s04.sh`
- **Historical M046 closeout alias:** `bash scripts/verify-m046-s06.sh`
- **Historical M046 equal-surface alias:** `bash scripts/verify-m046-s05.sh`
- **Historical M046 package/startup alias:** `bash scripts/verify-m046-s04.sh`
- **Historical M045 closeout alias:** `bash scripts/verify-m045-s05.sh`
- **Historical M045 assembled alias:** `bash scripts/verify-m045-s04.sh`
- **Historical failover-only subrail:** `bash scripts/verify-m045-s03.sh`
- **Lower-level retained tiny-cluster fixture contract:** `cargo run -q -p meshc -- build scripts/fixtures/clustered/tiny-cluster && cargo run -q -p meshc -- test scripts/fixtures/clustered/tiny-cluster/tests`
- **Lower-level retained cluster-proof fixture contract:** `cargo run -q -p meshc -- build scripts/fixtures/clustered/cluster-proof && cargo run -q -p meshc -- test scripts/fixtures/clustered/cluster-proof/tests`
- **Public docs build:** `npm --prefix website run build`
- **Read-only Fly sanity:** `bash scripts/verify-m043-s04-fly.sh --help` for syntax, or `CLUSTER_PROOF_FLY_APP=mesh-cluster-proof CLUSTER_PROOF_BASE_URL=https://mesh-cluster-proof.fly.dev bash scripts/verify-m043-s04-fly.sh` for live inspection
