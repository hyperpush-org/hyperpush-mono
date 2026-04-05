---
title: Clustered Example
description: Start the generated clustered scaffold, inspect runtime-owned startup work through the CLI, then choose the honest local SQLite starter or the serious shared/deployable PostgreSQL starter.
---

# Clustered Example

Use `meshc init --clustered` when you want the minimal public clustered-app contract first: package-only `mesh.toml`, source-declared `@cluster` work in `work.mpl`, and runtime-owned inspection through `meshc cluster ...`.

This page stays on that scaffold first. Once you have the route-free clustered contract in hand, keep the public follow-on ladder ordered: honest local SQLite starter, serious shared/deployable PostgreSQL starter, then Production Backend Proof only when you need the maintainer-facing deeper backend proof. The retained verifier map stays behind the proof pages instead of a direct repo-root runbook handoff.

## Generate the scaffold

```bash
meshc init --clustered hello_cluster
cd hello_cluster
```

The generated project is intentionally small:

```text
hello_cluster/
  mesh.toml
  main.mpl
  work.mpl
  README.md
```

## What the scaffold proves

- `mesh.toml` is package-only and intentionally omits `[cluster]`
- `main.mpl` has one clustered bootstrap path: `Node.start_from_env()`
- `work.mpl` declares `@cluster pub fn add()`
- the runtime-owned handler name is derived from the ordinary source function name as `Work.add`
- the visible work body stays `1 + 1`
- the project does not own HTTP routes, submit handlers, or work-delay seams

## Understand the generated files

### `mesh.toml`

The clustered scaffold keeps the manifest package-only:

```toml
[package]
name = "hello_cluster"
version = "0.1.0"

[dependencies]
```

Clustered work is declared in source, not in the manifest.

### `main.mpl`

The generated app does not hand-roll clustering logic. It only logs runtime bootstrap success or failure:

```mesh
fn main() do
  case Node.start_from_env() do
    Ok(status) -> log_bootstrap(status)
    Err(reason) -> log_bootstrap_failure(reason)
  end
end
```

That keeps startup, placement, continuity ownership, and diagnostics inside the runtime.

### `work.mpl`

The clustered work contract lives in source:

```mesh
@cluster pub fn add() -> Int do
  1 + 1
end
```

The runtime automatically starts the source-declared `@cluster` handler and closes the continuity record when the declared work returns.

## Build the example

```bash
meshc build .
```

That produces `./hello_cluster` in the project root.

## Run two local nodes

The generated `README.md` lists the full environment contract. For a local two-node demo, start one primary and one standby with the same cookie and discovery seed.

### Terminal 1 — primary

```bash
MESH_CLUSTER_COOKIE=dev-cookie \
MESH_NODE_NAME=primary@127.0.0.1:4370 \
MESH_DISCOVERY_SEED=localhost \
MESH_CLUSTER_PORT=4370 \
MESH_CONTINUITY_ROLE=primary \
MESH_CONTINUITY_PROMOTION_EPOCH=0 \
./hello_cluster
```

### Terminal 2 — standby

```bash
MESH_CLUSTER_COOKIE=dev-cookie \
MESH_NODE_NAME='standby@[::1]:4370' \
MESH_DISCOVERY_SEED=localhost \
MESH_CLUSTER_PORT=4370 \
MESH_CONTINUITY_ROLE=standby \
MESH_CONTINUITY_PROMOTION_EPOCH=0 \
./hello_cluster
```

Both terminals should log a runtime bootstrap line showing the resolved node name, cluster port, and discovery seed.

## Inspect cluster truth with the runtime CLI

Follow the same operator order used by the scaffold README and the PostgreSQL starter.

### 1. Status

```bash
meshc cluster status primary@127.0.0.1:4370 --json
meshc cluster status 'standby@[::1]:4370' --json
```

Look for both nodes in membership plus runtime-owned authority fields such as `cluster_role`, `promotion_epoch`, and `replication_health`.

### 2. Continuity list

```bash
meshc cluster continuity primary@127.0.0.1:4370 --json
meshc cluster continuity 'standby@[::1]:4370' --json
```

Use the list form first to discover request keys and runtime-owned startup records.

### 3. Continuity record

Once the list output shows a request key you care about, inspect that single record:

```bash
meshc cluster continuity primary@127.0.0.1:4370 <request-key> --json
meshc cluster continuity 'standby@[::1]:4370' <request-key> --json
```

This gives the per-record continuity detail for the same runtime-owned work item.

### 4. Diagnostics

```bash
meshc cluster diagnostics primary@127.0.0.1:4370 --json
```

Use diagnostics when you need the broader cluster view after checking membership and continuity.

## After the scaffold, pick the follow-on starter

Take the public follow-on ladder in order: honest local SQLite starter, serious shared/deployable PostgreSQL starter, then Production Backend Proof only when you need the maintainer-facing deeper backend proof.

- `meshc init --template todo-api --db sqlite my_local_todo` — the honest local single-node starter. It matches [SQLite Todo starter](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-sqlite/README.md): generated package tests, local `/health`, and no `work.mpl`, `HTTP.clustered(...)`, or `meshc cluster` story.
- `meshc init --template todo-api --db postgres my_shared_todo` — the serious shared/deployable starter. It matches [PostgreSQL Todo starter](https://github.com/snowdamiz/mesh-lang/blob/main/examples/todo-postgres/README.md): route-free `work.mpl`, PostgreSQL-backed state, explicit-count `HTTP.clustered(1, ...)` on `GET /todos` and `GET /todos/:id`, and local `/health` plus mutating routes.
- [Production Backend Proof](/docs/production-backend-proof/) — the maintainer-facing backend proof page after the starter/examples-first ladder.

## Need the retained verifier map?

Use [Distributed Proof](/docs/distributed-proof/) when you need the repo-owned verifier map, retained compatibility wrappers, or the lower-level fixture-backed rails behind this public starter story. This page stays focused on the scaffold plus the SQLite/PostgreSQL starter split.

## What to read next

- [Getting Started](/docs/getting-started/) — the single-binary introduction and hello-world path
- [Developer Tools](/docs/tooling/) — scaffold generation, inspection CLI commands, and editor support
- [Distributed Actors](/docs/distributed/) — the language/runtime primitives behind node communication
- [Distributed Proof](/docs/distributed-proof/) — the named repo verifier map behind the public clustered surfaces
