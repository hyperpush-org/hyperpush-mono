---
name: mesh-clustering
description: Mesh clustered runtime: source-first `@cluster` / `@cluster(N)`, `Node.start_from_env()`, `meshc init --clustered`, `meshc init --template todo-api --db postgres`, Mesh operator CLI surfaces, the SQLite local-only boundary, and bounded `HTTP.clustered(...)` guidance.
---

## Canonical Clustered Contract

Rules:
1. Declare clustered startup work in source with `@cluster` or `@cluster(N)` on a public function in `work.mpl`.
2. `main.mpl` boots through `Node.start_from_env()`; the runtime owns startup, placement, continuity, and diagnostics.
3. The runtime derives the clustered handler name from the ordinary function name (for example, `Work.add` or `Work.sync_todos`).
4. The public onboarding flow starts with `meshc init --clustered`, then branches to `examples/todo-postgres` for the shared/deployable starter and `examples/todo-sqlite` for the honest local starter.
5. The public deeper backend handoff goes through `/docs/production-backend-proof/`, which keeps the starter/examples-first ladder intact and names `mesher/README.md` plus `bash scripts/verify-m051-s01.sh` and `bash scripts/verify-m051-s02.sh` as maintainer-only surfaces once the generated starters stop being enough.
6. Keep the clustered story source-first and runtime-owned — do not invent package-owned control or inspection surfaces, and do not project clustered/operator claims onto the SQLite starter.

Code example (minimal clustered surface):
```mesh
@cluster pub fn add() -> Int do
  1 + 1
end

fn main() do
  case Node.start_from_env() do
    Ok(status) -> println(status)
    Err(reason) -> println(reason)
  end
end
```

## Scaffolds and Starters

Rules:
1. `meshc init --clustered <name>` is the primary public clustered-app scaffold.
2. It generates `main.mpl` with `Node.start_from_env()`, `work.mpl` with `@cluster pub fn add()`, and a README aligned with the generated repo examples `examples/todo-postgres` and `examples/todo-sqlite` instead of internal proof fixtures.
3. `meshc init --template todo-api --db postgres <name>` is the fuller shared or deployable starter layered on top of that same route-free clustered contract.
4. The PostgreSQL Todo starter keeps `work.mpl` on `@cluster pub fn sync_todos()`, starts with `Node.start_from_env()`, and dogfoods explicit-count `HTTP.clustered(1, ...)` only on `GET /todos` and `GET /todos/:id`; `GET /health` plus mutating routes stay local.
5. `meshc init --template todo-api --db sqlite <name>` is the honest local single-node starter: generated package tests, local `/health`, actor-backed write rate limiting, and Docker packaging around `meshc build .`.
6. The SQLite Todo starter does not claim `work.mpl`, `HTTP.clustered(...)`, `meshc cluster`, or clustered/operator proof surfaces.
7. Use the Postgres Todo template when you need the packaged clustered HTTP starter; use the clustered scaffold when you want the minimal route-free public clustered surface; use the SQLite Todo template only when you want the local-first path.
8. The generated repo examples live at `examples/todo-postgres` and `examples/todo-sqlite`; after those public starters, send repo maintainers through `/docs/production-backend-proof/` so the deeper maintained app stays on `mesher/README.md` and the retained backend-only proof stays on the named maintainer verifiers `bash scripts/verify-m051-s01.sh` and `bash scripts/verify-m051-s02.sh` instead of a first-contact repo-root runbook.

## Deeper Maintainer Surfaces

Rules:
1. Keep the public clustered story scaffold/examples-first: `meshc init --clustered`, then the PostgreSQL or SQLite Todo examples depending whether the reader needs shared/deployable or local-only behavior.
2. Use `/docs/production-backend-proof/` only when the question has moved beyond those public starters and into maintainer-facing backend proof.
3. `mesher/README.md` is the deeper maintained app runbook for repo maintainers.
4. `bash scripts/verify-m051-s01.sh` verifies the shipped Mesher app surface.
5. `bash scripts/verify-m051-s02.sh` verifies the retained backend-only proof replay.
6. Do not teach `reference-backend/README.md` or fixture/runbook paths as the public next step after the clustered scaffold or Todo starters.

## Runtime Inspection

Rules:
1. Once a clustered node is running, inspect it with Mesh-owned CLI commands instead of package-owned routes.
2. Use the continuity list form first to discover startup or request keys, then inspect a single continuity record.
3. These commands are the operator-facing runtime surface:

```bash
meshc cluster status <node-name@host:port> --json
meshc cluster continuity <node-name@host:port> --json
meshc cluster continuity <node-name@host:port> <request-key> --json
meshc cluster diagnostics <node-name@host:port> --json
```

## HTTP.clustered(...) Boundary

Rules:
1. Keep route-free `@cluster` work canonical. The clustered runtime story starts with `meshc init --clustered`, not with routed HTTP wrappers.
2. `HTTP.clustered(handler)` and `HTTP.clustered(1, handler)` are valid route wrappers.
3. The shipped PostgreSQL Todo starter only dogfoods explicit-count `HTTP.clustered(1, ...)` on `GET /todos` and `GET /todos/:id`; `GET /health` plus mutating routes stay local.
4. `meshc init --template todo-api --db sqlite` is intentionally local-only and does not make `HTTP.clustered(...)` part of its public contract.
5. Use `HTTP.clustered(...)` when you need routed clustered reads; use `@cluster` / `Node.start_from_env()` for runtime bootstrapping and background clustered work.
6. For route helper details load `skills/http`; for decorator syntax load `skills/syntax`.

Code example (shipped PostgreSQL Todo starter shape):
```mesh
let router = HTTP.router()
  |> HTTP.on_get("/health", handle_health)
  |> HTTP.on_get("/todos", HTTP.clustered(1, handle_list_todos))
  |> HTTP.on_get("/todos/:id", HTTP.clustered(1, handle_get_todo))
  |> HTTP.on_post("/todos", handle_create_todo)
  |> HTTP.on_put("/todos/:id", handle_toggle_todo)
  |> HTTP.on_delete("/todos/:id", handle_delete_todo)
```

Code example (accepted default-count wrapper form):
```mesh
let router = HTTP.router()
  |> HTTP.on_get("/todos", HTTP.clustered(handle_list_todos))
```
