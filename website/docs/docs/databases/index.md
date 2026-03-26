---
title: Databases
description: "Shipped Mesh/Mesher database boundary: neutral Expr/Query/Repo plus PostgreSQL-only Pg helpers"
---

# Databases

Mesh's database story is currently best understood through the real Mesher storage layer that ships in this repo.

This page does **not** promise a universal ORM that erases backend differences. It names the boundary M033 actually proves today:

- a neutral expression/query/write surface built from `Expr`, `Query`, `Repo`, and `Migration`
- explicit PostgreSQL-only helpers under `Pg.*`
- a named set of raw escape hatches that stays honest instead of pretending every SQL shape is portable

> **Production backend proof:** This page explains the database boundary itself. For the end-to-end backend proof surface that wires migrations, health checks, workers, and deploy artifacts together, start with [Production Backend Proof](/docs/production-backend-proof/) and `reference-backend/README.md`.

## What is runtime-proven today

The public database guide is anchored in the same repo paths the live proof stack exercises:

- `compiler/meshc/tests/e2e_m033_s01.rs` — neutral `Expr` / `Query` / `Repo` expression builder coverage
- `mesher/storage/writer.mpl` and `mesher/storage/queries.mpl` — the real Mesher write/read paths
- `mesher/migrations/20260216120000_create_initial_schema.mpl` — the shipped migration that uses the new helpers
- `mesher/storage/schema.mpl` — runtime partition lifecycle helpers
- `compiler/meshc/src/migrate.rs` — migration generator guidance that keeps neutral DDL under `Migration.*` and PostgreSQL extras under `Pg.*`

The proof commands behind those surfaces are:

```bash
bash scripts/verify-m033-s01.sh
bash scripts/verify-m033-s02.sh
bash scripts/verify-m033-s03.sh
bash scripts/verify-m033-s04.sh
npm --prefix website run build
bash scripts/verify-m033-s05.sh
```

For the assembled docs + live-Postgres acceptance replay, run `bash scripts/verify-m033-s05.sh`.

All of the runtime proofs above target live PostgreSQL. SQLite-specific extras are later work and are **not** the proof target for this page.

## Neutral surface: `Expr`, `Query`, `Repo`, `Migration`

The neutral part of the boundary is the shape that can be described without pretending JSONB, pgcrypto, partitions, or PostgreSQL catalogs are portable.

### `Expr` builds values, columns, NULLs, defaults, and derived projections

M033's shipped API uses `Expr.label`, not `Expr.alias`.

```mesh
let q = Query.from("m033_expr_selects")
  |> Query.select_exprs([
    Expr.label(Expr.coalesce([Expr.column("nickname"), Expr.value("fallback")]), "nick"),
    Expr.label(Expr.add(Expr.column("amount"), Expr.value("2")), "next_amount"),
    Expr.label(
      Expr.case_when(
        [Expr.eq(Expr.column("status"), Expr.value("resolved"))],
        [Expr.value("closed")],
        Expr.column("status")
      ),
      "display_status"
    )
  ])
  |> Query.where(:id, "row-1")
```

That exact shape is exercised in `compiler/meshc/tests/e2e_m033_s01.rs`. The important neutral pieces are:

- `Expr.value(...)` — bind a literal or parameter
- `Expr.column(...)` — refer to a column
- `Expr.null()` — write an actual `NULL`
- `Expr.case_when(...)` — keep SQL branching in the expression tree
- `Expr.coalesce([...])` — express fallback/default logic
- `Expr.label(expr, "name")` — name derived output columns

### `Query.where_expr` and `Query.select_exprs` keep predicates and row shapes explicit

Mesher's read helpers use the same builder surface for real storage paths:

```mesh
let q = Query.from(Issue.__table__())
  |> Query.where_expr(Expr.eq(Expr.column("project_id"), Pg.uuid(Expr.value(project_id))))
  |> Query.where_expr(Expr.eq(Expr.column("status"), Expr.value("unresolved")))
  |> Query.select_expr(Expr.label(Pg.text(Expr.fn_call("count", [Expr.column("*")])), "cnt"))
```

The builder calls here are the neutral part: `Query.where_expr(...)`, `Query.select_expr(...)`, and `Query.select_exprs([...])`.
The nested `Pg.uuid(...)` and `Pg.text(...)` wrappers are the honest PostgreSQL-only part.

### `Repo.insert_expr`, `Repo.update_where_expr`, and `Repo.insert_or_update_expr` accept expression-valued writes

Mesher uses expression-aware writes instead of falling back to handwritten SQL for common insert/update/upsert paths:

```mesh
let row = Repo.insert_expr(pool,
  User.__table__(),
  %{
    "email" => Expr.value(email),
    "password_hash" => Pg.crypt(Expr.value(password), Pg.gen_salt("bf", 12)),
    "display_name" => Expr.value(display_name)
  })?
```

```mesh
let update_result = Repo.update_where_expr(pool,
  "m033_expr_updates",
  %{
    "amount" => Expr.add(Expr.column("amount"), Expr.value("2")),
    "touched_at" => Expr.fn_call("now", []),
    "status" => Expr.case_when(
      [Expr.eq(Expr.column("status"), Expr.value("resolved"))],
      [Expr.value("unresolved")],
      Expr.column("status")
    )
  },
  q)
```

```mesh
let row = Repo.insert_or_update_expr(pool,
  Issue.__table__(),
  %{"project_id" => project_id, "fingerprint" => fingerprint, "title" => title, "level" => level, "event_count" => "1"},
  ["project_id", "fingerprint"],
  %{
    "event_count" => Expr.add(Expr.column("issues.event_count"), Expr.value("1")),
    "last_seen" => Expr.fn_call("now", []),
    "status" => Expr.case_when(
      [Expr.eq(Expr.column("issues.status"), Expr.value("resolved"))],
      [Expr.value("unresolved")],
      Expr.column("issues.status")
    )
  })?
```

And when a real null assignment is needed, Mesher uses `Expr.null()` explicitly:

```mesh
Repo.update_where_expr(pool, Issue.__table__(), %{"assigned_to" => Expr.null()}, q)?
```

### `Migration.create_index(...)` is the neutral DDL path

The migration helper guidance in `compiler/meshc/src/migrate.rs` and the shipped Mesher migration both keep plain index creation under `Migration.create_index(...)`:

```mesh
Migration.create_index(pool,
  "events",
  ["issue_id", "received_at:DESC"],
  "name:idx_events_issue_received")?
```

Use `Migration.*` for the common DDL surface. When the DDL depends on PostgreSQL-only behavior, the repo keeps that explicit under `Pg.*` instead of hiding it behind a fake portable abstraction.

## PostgreSQL-only `Pg.*` extras

When behavior depends on PostgreSQL types, operators, extensions, partitions, or catalogs, the repo names that dependency directly under `Pg.*`.

### Casts and typed expressions

Mesher uses typed PG wrappers inside otherwise neutral builders:

- `Pg.uuid(Expr.value(project_id))`
- `Pg.timestamptz(Expr.fn_call("now", []))`
- `Pg.text(Expr.column("created_at"))`
- `Pg.cast(Expr.value("1024"), "bigint")`

These are PostgreSQL-only. The surrounding `Expr` / `Query` / `Repo` call may be neutral, but the typed behavior is not.

### JSONB and search helpers

The shipped Mesher search/tag paths stay explicit about PostgreSQL JSONB and text search:

```mesh
let search_vector = Pg.to_tsvector("english", Expr.column("message"))
let search_terms = Pg.plainto_tsquery("english", Expr.value(search_query))

let q = Query.from(Event.__table__())
  |> Query.where_expr(Expr.eq(Expr.column("project_id"), Pg.uuid(Expr.value(project_id))))
  |> Query.where_expr(Pg.tsvector_matches(search_vector, search_terms))
  |> Query.select_expr(Expr.label(Pg.ts_rank(search_vector, search_terms), "rank"))
```

```mesh
let q = Query.from(Event.__table__())
  |> Query.where_expr(Expr.eq(Expr.column("project_id"), Pg.uuid(Expr.value(project_id))))
  |> Query.where_expr(Pg.jsonb_contains(Expr.column("tags"), Pg.jsonb(Expr.value(tag_json))))
```

The corresponding schema uses PostgreSQL-specific indexing too:

```mesh
Pg.create_gin_index(pool, "events", "idx_events_tags", "tags", "jsonb_path_ops")?
```

### Crypto helpers

Mesher's auth path does not hide pgcrypto behind a generic password API. It uses PostgreSQL-only helpers directly:

```mesh
let row = Repo.insert_expr(pool,
  User.__table__(),
  %{
    "email" => Expr.value(email),
    "password_hash" => Pg.crypt(Expr.value(password), Pg.gen_salt("bf", 12)),
    "display_name" => Expr.value(display_name)
  })?
```

That dependency is backed by the migration's explicit extension install:

```mesh
Pg.create_extension(pool, "pgcrypto")?
```

### Partition and schema helpers

The migration/runtime partition story is also explicit about PostgreSQL:

```mesh
Pg.create_range_partitioned_table(pool, "events", [...], "received_at")?
Pg.create_daily_partitions_ahead(pool, "events", days)?
Pg.list_daily_partitions_before(pool, "events", max_days)?
Pg.drop_partition(pool, partition_name)?
```

`mesher/migrations/20260216120000_create_initial_schema.mpl` handles the schema-time partitioned table creation, while `mesher/storage/schema.mpl` owns the runtime create/list/drop lifecycle for those partitions.

## Escape hatches and the honest leftover raw list

M033 does **not** claim that Mesh or Mesher have reached zero raw SQL/DDL.

The escape hatches are part of the public contract:

- `Repo.query_raw(pool, sql, params)` — raw read boundary
- `Repo.execute_raw(pool, sql, params)` — raw write/update boundary
- `Migration.execute(pool, sql)` — raw DDL boundary when a migration shape does not fit `Migration.*` or `Pg.*`

The shipped M033 Mesher migration does not currently need `Migration.execute(...)` because `Migration.create_index(...)` plus `Pg.create_*` cover the DDL it uses today.

The current Mesher storage layer still keeps a named raw list in `mesher/storage/queries.mpl` instead of pretending those shapes are portable:

- `check_volume_spikes` — correlated update with `JOIN`, `HAVING`, `GREATEST`, and interval arithmetic
- `extract_event_fields` — JSONB fingerprint fallback chain with `WITH ORDINALITY`
- `list_issues_filtered` — optional filters plus tuple-cursor pagination
- `event_volume_hourly` — `date_trunc(...)` bucket projection that must preserve labeled bucket rows on the live dashboard path
- `get_event_neighbors` — paired scalar subqueries with tuple comparison for next/previous navigation
- `evaluate_threshold_rule` — threshold + cooldown evaluation over derived subqueries
- `get_event_alert_rules` and `get_threshold_rules` — explicit alert-rule selectors that keep stable text rows on the live alert path
- `should_fire_by_cooldown` — boolean cooldown gate over interval arithmetic
- `list_alerts` — alert listing with optional status filter on the live alerts route
- `check_sample_rate` — server-side `random() < COALESCE((SELECT sample_rate ...), 1.0)` sampling predicate

That is the honest boundary today: most common expression/query/write/schema shapes are on the builder/helper surface, and the remaining raw sites are named on purpose.

## What this page is not claiming

- It is **not** claiming that PostgreSQL JSONB/search/crypto/partition features are portable to SQLite.
- It is **not** claiming that every SQL/DDL shape has a universal Mesh abstraction.
- It is **not** claiming runtime proof for SQLite extras in M033.
- It **is** claiming that the repo now names the portable core, names the PostgreSQL-only helpers, and names the deliberate raw keep-sites instead of hiding them.

## Proof and failure map

If this surface drifts, rerun the proof that matches the boundary you touched:

| Surface | Proof command | Primary files |
| --- | --- | --- |
| neutral `Expr` / `Query` / `Repo` builder | `bash scripts/verify-m033-s01.sh` | `compiler/meshc/tests/e2e_m033_s01.rs` |
| PostgreSQL JSONB/search/crypto helpers | `bash scripts/verify-m033-s02.sh` | `mesher/storage/writer.mpl`, `mesher/storage/queries.mpl` |
| composed reads and named raw keep-sites | `bash scripts/verify-m033-s03.sh` | `mesher/storage/queries.mpl` |
| migration + schema helpers | `bash scripts/verify-m033-s04.sh` | `mesher/migrations/20260216120000_create_initial_schema.mpl`, `mesher/storage/schema.mpl`, `compiler/meshc/src/migrate.rs` |
| docs page rendering | `npm --prefix website run build` | `website/docs/docs/databases/index.md` |
| assembled docs + live-Postgres proof replay | `bash scripts/verify-m033-s05.sh` | `website/docs/docs/databases/index.md`, `scripts/verify-m033-s02.sh`, `scripts/verify-m033-s03.sh`, `scripts/verify-m033-s04.sh` |

## What's next?

- [Production Backend Proof](/docs/production-backend-proof/) — repo-level proof surface for the assembled backend
- [Web](/docs/web/) — HTTP and WebSocket primitives that consume these storage helpers
- `reference-backend/README.md` — operator/developer runbook for the broader backend package
