# S03 — Research

**Date:** 2026-03-26

## Requirements Targeted

- **R038 (primary)** — this slice owns the raw-tail collapse bar. The practical target is not zero raw SQL; it is a **short, justified, named** keep-list after the honest read-side rewrites land.
- **R037 (supporting)** — S02 already proved PG JSONB/search/auth helpers. S03 should build on that explicit `Pg.*` seam instead of widening the neutral core for PG-only read behavior.
- **R040 / R036 (constraint/support)** — keep the neutral baseline honest. If S03 adds runtime surface, it should be the smallest reusable read-side primitive that does not turn into a PG-only trap.

## Skills Discovered

Existing installed skills already cover the core technologies for this slice; no new skills were installed.

- **`rust-best-practices`** — relevant rule: extend existing serializer/query-builder paths instead of duplicating logic, and prove new API with focused, descriptive tests instead of broad incidental coverage.
- **`postgresql-database-engineering`** — relevant rule: do not replace a set-based Postgres statement with an N+1 application loop unless the candidate set is clearly bounded and the tradeoff is intentional; keep raw SQL when a dedicated abstraction would lie.

## Summary

S03 is **targeted, not greenfield**. The important discovery is that the repo already has enough read-side structure to retire a large amount of Mesher raw SQL **without** building a universal SQL AST.

The current hard raw whole-query list in `mesher/storage/queries.mpl` is concentrated and small:

- `check_volume_spikes` — `mesher/storage/queries.mpl:471`
- `extract_event_fields` — `mesher/storage/queries.mpl:487`
- `list_issues_filtered` — `mesher/storage/queries.mpl:509`
- `project_health_summary` — `mesher/storage/queries.mpl:682`
- `get_event_neighbors` — `mesher/storage/queries.mpl:708`
- `evaluate_threshold_rule` — `mesher/storage/queries.mpl:814`
- `check_sample_rate` — `mesher/storage/queries.mpl:1048`

S04-owned raw sites are still separate and should stay out of the S03 keep-list accounting:

- `get_expired_partitions` — `mesher/storage/queries.mpl:959`
- `drop_partition` — `mesher/storage/queries.mpl:971`

Two critical constraints showed up in the runtime code:

1. **`Query.from(...)` only supports a quoted table name**, not a derived table / raw source (`compiler/mesh-rt/src/db/query.rs:161-166`, `compiler/mesh-rt/src/db/repo.rs:291-293`).
2. **`Query.where_sub(...)` is not a general subquery serializer**. Its implementation in `compiler/mesh-rt/src/db/query.rs:787-853` manually serializes only `source + select + where`, and ignores joins, group/having, order, limit/offset, and `select_params`.

That means S03 should **not** assume the existing subquery surface can power complex scalar/derived-table rewrites.

The good news: many remaining raw read fragments are already cheap to collapse with the S01/S02 surface:

- simple UUID / boolean / text predicates can move from `where_raw(...)` to `where_expr(...)`
- `id::text`, `project_id::text`, timestamp casts, and `COALESCE(...::text, '')` can move from `select_raw(...)` to `select_expr(...)` / `select_exprs(...)`
- alias-based sorting like `rank DESC` or `count DESC` can usually move to repeated `Query.order_by(...)`

So the slice naturally splits into:

- **mechanical Mesher read cleanup** in `mesher/storage/queries.mpl`
- **hard raw-family decisions** for the 6–7 named whole-query sites above
- **optional runtime/compiler work only if the hard-family rewrites prove a real reusable gap**

## Recommendation

Start from the **lightest honest plan**:

1. **Do the Mesher-only read cleanup first.**
   There is a large mechanical win in `mesher/storage/queries.mpl` that does not require runtime work. Collapse obvious `select_raw(...)` / simple `where_raw(...)` usage first so the final hard keep-list is easier to see.

2. **Prefer decomposition over AST growth for the hard raw families.**
   Several “hard” whole-query sites can be retired by doing 2–3 smaller ORM-backed reads plus Mesh-side composition, while keeping behavior stable:
   - `project_health_summary(...)` -> three counts
   - `evaluate_threshold_rule(...)` -> event-count query + cooldown-eligibility query + Mesh-side boolean
   - `get_event_neighbors(...)` -> next query + previous query instead of two scalar subqueries in one `SELECT`
   - `list_issues_filtered(...)` / `list_alerts(...)` -> conditionally append filters in Mesh instead of SQL-side `($N = '' OR ...)`

3. **Only add runtime/compiler surface if a repeated shape survives decomposition.**
   The most reusable missing primitive is **boolean expression composition** (`Expr.and` / `Expr.or`). That would reduce the need for cursor-related raw fragments. By contrast, adding full derived-table or general scalar-subquery support is much riskier and pushes toward the fake-universal-AST problem D052 warned about.

4. **Treat `extract_event_fields`, `check_sample_rate`, and possibly `check_volume_spikes` as candidate final leftovers.**
   The current codebase gives no evidence that these need a new general abstraction. A short, named leftover list is acceptable under R038; a dishonest helper is not.

## Implementation Landscape

### 1. Runtime/query-builder state today

#### `compiler/mesh-rt/src/db/expr.rs`

Important existing capabilities:

- neutral expression nodes: column, value, null, call, binary op, case, coalesce, alias, cast
- PG helpers already used in Mesher: `Pg.uuid`, `Pg.text`, `Pg.jsonb`, `Pg.int`, `Pg.timestamptz`, `Pg.to_tsvector`, `Pg.plainto_tsquery`, `Pg.ts_rank`, `Pg.tsvector_matches`, `Pg.jsonb_contains`
- expression serialization already preserves ordered params cleanly

Important missing pieces:

- no subquery expression node in `SqlExpr`
- no boolean `AND` / `OR` builder surface
- no explicit row-value / tuple comparison surface

That makes current cursor predicates and multi-branch boolean predicates awkward unless they stay raw.

#### `compiler/mesh-rt/src/db/query.rs`

Important existing capabilities:

- `Query.where_expr(...)` for structured boolean predicates (`compiler/mesh-rt/src/db/query.rs:383-405`)
- `Query.select_expr(...)` / `Query.select_exprs(...)` with select-param plumbing (`compiler/mesh-rt/src/db/query.rs:422-462`)
- joins, group by, having, raw fragment append, and the narrow `where_sub(...)` entrypoint

Important limitations:

- `Query.from(...)` is only a table source (`compiler/mesh-rt/src/db/query.rs:161-166`)
- `where_sub(...)` is handwritten and **drops complexity** (`compiler/mesh-rt/src/db/query.rs:787-853`)
- there is no `group_by_expr`, `order_by_expr`, or derived-table source surface

#### `compiler/mesh-rt/src/db/repo.rs`

This is the best existing seam if runtime work becomes necessary:

- `build_select_sql_from_parts_with_select_params(...)` already handles **mixed RAW + EXPR select items** and correctly puts select params before where params (`compiler/mesh-rt/src/db/repo.rs:243-366`, tests at `3462-3607`)
- `build_where_from_query_parts(...)` already understands `RAW:` and `EXPR:` predicates (`compiler/mesh-rt/src/db/repo.rs:1808-1917`)

Implication: if S03 adds a new reusable read-side node, the repo SQL builder already has the right placeholder-renumbering shape. Do not build a second serializer path.

### 2. Hard whole-query raw family map

#### `mesher/storage/queries.mpl:471` — `check_volume_spikes(...)`

Current raw statement is one set-based `UPDATE ... WHERE id IN (SELECT ... JOIN ... GROUP BY ... HAVING count(*) > GREATEST(10, (subquery / 168 * 10)))`.

Planner guidance:

- This is the strongest case **against** “just decompose everything.”
- A multi-query rewrite is possible, but only if the candidate set is explicitly bounded and the N+1 cost is acceptable.
- If not, keep it as a named raw leftover instead of inventing a fake generic derived-table abstraction.

#### `mesher/storage/queries.mpl:487` — `extract_event_fields(...)`

This is still the most credible S03 leftover.

It depends on:

- `CASE`
- derived-table source `FROM (SELECT $1::jsonb AS j)`
- scalar subquery in the `SELECT`
- `jsonb_array_elements(...) WITH ORDINALITY`
- ordered `string_agg(...)`

Current runtime does not have an honest general surface for that combination. If this gets rewritten, it should be because there is a **narrow, obviously truthful** abstraction — not because the slice is chasing a raw-count metric.

#### `mesher/storage/queries.mpl:509` — `list_issues_filtered(...)`

The current raw form is solving two separate problems at once:

- optional filters through SQL-side `($N = '' OR column = $N)` clauses
- keyset cursor `(last_seen, id) < (...)`

The optional-filter part does **not** need runtime work. Mesh can append filters conditionally.

The cursor part is the real seam:

- either leave a narrow raw cursor predicate
- or add boolean composition so lexicographic ordering can be expanded into `last_seen < cursor OR (last_seen = cursor AND id < cursor_id)`

This is a good litmus test for whether `Expr.and/or` is worth landing.

#### `mesher/storage/queries.mpl:682` — `project_health_summary(...)`

This does **not** need scalar-subquery runtime support if the slice stays honest.

The API only returns three counts:

- unresolved issues
- 24h event count
- new issues today

Three separate count queries are a credible rewrite and avoid inventing a scalar-subquery SELECT surface just for one row shape.

#### `mesher/storage/queries.mpl:708` — `get_event_neighbors(...)`

This is also easier to retire by decomposition:

- one query for next
- one query for previous

The only real missing seam is the same cursor predicate issue as `list_issues_filtered(...)`.

#### `mesher/storage/queries.mpl:814` — `evaluate_threshold_rule(...)`

This raw query cross-joins two derived tables and computes a `CASE`.

But the business logic is actually simple:

- count events in the window
- verify cooldown eligibility
- combine the two booleans in Mesh

This is a good candidate for retirement **without** new runtime surface.

#### `mesher/storage/queries.mpl:1048` — `check_sample_rate(...)`

This is a candidate leftover unless S03 wants to ship a dedicated PG `random()` helper or there is an existing Mesh random primitive. Repo search found no Mesh-side `Random` / `random()` usage outside this raw keep-site.

### 3. Mesher-only cleanup surface already available today

The following functions still use raw read fragments but look mechanically collapsible with the current S01/S02 surface, mostly inside `mesher/storage/queries.mpl`:

- `count_unresolved_issues`
- `get_issue_project_id`
- `get_project_by_api_key`
- `validate_session`
- `list_issues_by_status`
- `list_events_for_issue`
- `event_volume_hourly`
- `error_breakdown_by_level`
- `top_issues_by_frequency`
- `issue_event_timeline`
- `get_event_detail`
- `get_members_with_users`
- `list_api_keys`
- `list_alert_rules`
- `check_new_issue`
- `should_fire_by_cooldown`
- `list_alerts`
- `get_all_project_retention`
- `get_project_storage`
- `get_project_settings`

Repeatable rewrite patterns already available:

- `?::uuid` -> `where_expr(Expr.eq(Expr.column(...), Pg.uuid(Expr.value(...))))`
- `id::text AS id` -> `Expr.label(Pg.text(Expr.column("id")), "id")`
- `COALESCE(col::text, '')` -> `Expr.label(Expr.coalesce([Pg.text(Expr.column(...)), Expr.value("")]), alias)`
- `count(*)::text AS count` -> `Expr.label(Pg.text(Expr.fn_call("count", [Expr.column("*")])), "count")`
- `order_by_raw("count DESC")` -> `order_by(:count, :desc)` once the alias exists explicitly

This is the cheapest S03 progress and should be separated from the harder whole-query decisions.

### 4. Caller surfaces that constrain rewrites

These files do not need heavy changes, but they define the return-shape contract:

- `mesher/api/search.mpl` — expects the same field keys from `list_issues_filtered(...)`
- `mesher/api/dashboard.mpl` — expects string/int field names from `project_health_summary(...)`, event volume, tag breakdown, etc.
- `mesher/api/detail.mpl` — expects `next_id` / `prev_id` navigation keys
- `mesher/ingestion/pipeline.mpl` — consumes `check_volume_spikes(...)` and `evaluate_threshold_rule(...)`
- `mesher/ingestion/routes.mpl` — consumes `check_sample_rate(...)`

As long as returned map keys and semantics stay stable, these callers should not need large rewrites.

## Natural Seams

1. **Mesher mechanical read cleanup**
   - file: `mesher/storage/queries.mpl`
   - goal: slash simple `select_raw` / `where_raw` / `order_by_raw` usage with existing `Expr` / `Query` / `Pg` surfaces
   - risk: low

2. **Hard raw-family rewrites**
   - file: `mesher/storage/queries.mpl`
   - goal: retire `list_issues_filtered`, `project_health_summary`, `get_event_neighbors`, `evaluate_threshold_rule`, and maybe `check_volume_spikes` / `check_sample_rate`
   - risk: medium; this is where the honest-keep-list decisions live

3. **Optional runtime/compiler extension**
   - files if needed: `compiler/mesh-rt/src/db/expr.rs`, `query.rs`, `repo.rs`, `lib.rs`, `compiler/mesh-typeck/src/infer.rs`, `compiler/mesh-codegen/src/mir/lower.rs`, `compiler/mesh-codegen/src/codegen/intrinsics.rs`, `compiler/meshc/tests/e2e.rs`
   - smallest honest candidate: boolean composition in `Expr`
   - high-risk candidate: general derived-table / scalar-subquery support

4. **Proof + guardrails**
   - add `compiler/meshc/tests/e2e_m033_s03.rs`
   - add `scripts/verify-m033-s03.sh`
   - add a mechanical keep-list sweep that names allowed leftovers and excludes S04-owned partition raw sites

## Risks and Unknowns

- **Do not build on `where_sub(...)` as if it were general.** It is not.
- **Do not turn `check_volume_spikes(...)` into an unbounded N+1 loop** just to reduce raw count.
- **Grouping by parameterized expressions is still awkward.** `event_breakdown_by_tag(...)` and `event_volume_hourly(...)` may still need ordinal/raw `GROUP BY` unless S03 adds a real `group_by_expr` surface or proves alias grouping explicitly.
- **`Expr.label(...)` remains the Mesh-callable alias entrypoint.** `Expr.alias(...)` is still not the stable source-level call path.
- **PG-only behavior must stay explicit.** If S03 adds runtime surface, keep PG-shaped pieces under `Pg` / explicit function calls instead of widening the neutral core with PG semantics.

## Verification

If S03 stays Mesher-only, the verification bundle can mirror S02 exactly:

- `cargo test -p meshc --test e2e_m033_s03 -- --nocapture`
- `cargo run -q -p meshc -- fmt --check mesher`
- `cargo run -q -p meshc -- build mesher`
- `bash scripts/verify-m033-s03.sh`

The S03 live Postgres harness should reuse the S02 pattern from `compiler/meshc/tests/e2e_m033_s02.rs`:

- dockerized temporary Postgres
- `meshc migrate mesher up`
- temp Mesh project copying `mesher/storage/` and `mesher/types/`
- direct DB assertions instead of HTTP-level proof

High-value slice assertions:

- filtered issue listing respects optional filters and keyset order
- project health counts match DB truth
- event neighbor next/prev IDs are correct around a pivot event
- threshold rule returns `true` only when both count and cooldown conditions are satisfied
- any retained raw leftovers are explicitly named in the keep-list sweep

If runtime/compiler surface is added, also add:

- focused Rust unit tests in `compiler/mesh-rt/src/db/repo.rs` / `expr.rs`
- compile/lowering smoke tests in `compiler/meshc/tests/e2e.rs`

The verifier should fail if whole-query raw usage creeps back into rewritten S03-owned functions, while allowing only the final named leftovers plus the S04-owned partition/catalog sites.
