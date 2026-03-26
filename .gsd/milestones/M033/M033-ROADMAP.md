# M033: ORM Expressiveness & Schema Extras

**Vision:** Expand Mesh’s data-layer surface into a broader honest expression DSL plus explicit PostgreSQL extras, retire the recurring Mesher raw SQL/raw DDL families those surfaces can truthfully cover, preserve a clean SQLite extension seam, and prove the result through live Postgres-backed Mesher flows and public Mesh docs.

## Success Criteria

- Mesher’s recurring computed write, JSONB-heavy, search-heavy, alert, and partition-management paths use stronger Mesh ORM or migration surfaces wherever those surfaces are honest, with only a short justified raw keep-list left behind.
- The neutral data-layer contract now includes a real expression DSL for reusable select/update/insert/upsert work instead of a literal-map-only write surface and ad hoc raw fragments.
- PostgreSQL-only behavior is exposed explicitly for the real PG-first families (`JSONB`, full-text search, pgcrypto, partition lifecycle, and related schema extras) rather than being hidden inside a misleading neutral API.
- The shipped design leaves a credible later seam for SQLite-specific extras without forcing M033 to implement or prove SQLite runtime behavior now.
- Public Mesh database docs explain the shipped neutral DSL and explicit PG extras through a Mesher-backed example path, and the live Postgres-backed Mesher flows still behave the same from the product point of view.

## Slices

- [x] **S01: Neutral expression core on real write paths** `risk:high` `depends:[]`
  > After this: After this: live Postgres-backed Mesher write paths for issue upserts, alert state transitions, settings updates, and `NULL`/`now()`-driven mutations run through structured Mesh expressions instead of recurring raw SQL.

- [x] **S02: Explicit PG extras for JSONB, search, and crypto** `risk:high` `depends:[S01]`
  > After this: After this: Mesher event ingest, JSONB extraction, full-text search, and pgcrypto-backed auth flows work through explicit PostgreSQL helpers on the real runtime path.

- [x] **S03: Hard read-side coverage and honest raw-tail collapse** `risk:medium` `depends:[S01,S02]`
  > After this: After this: Mesher’s recurring scalar-subquery, derived-table, parameterized select, and expression-heavy read paths use the new builders wherever honest, and the remaining raw query keep-list is short and named.

- [x] **S04: Schema extras and live partition lifecycle proof** `risk:medium` `depends:[S01,S02]`
  > After this: After this: Mesher migrations and runtime retention/schema flows create, list, and drop partitions plus related PG schema extras through first-class helpers on a live Postgres database.

- [x] **S05: Public docs and integrated Mesher acceptance** `risk:low` `depends:[S02,S03,S04]`
  > After this: After this: the public Mesh database docs explain the shipped neutral DSL and PG extras through a Mesher-backed path, and the assembled Mesher data-layer behavior is re-proven end-to-end on live Postgres.

## Boundary Map

### S01 → S02
Produces:
- a neutral expression contract in `Query`/`Repo` for column refs, parameters, literals, `NULL`, function calls, `CASE`, `COALESCE`, and expression-valued `SELECT`/`SET`/`ON CONFLICT` work
- stable SQL serialization and placeholder-handling rules that later PG extras can compose with instead of bypassing
- rewritten Mesher mutation paths (`upsert_issue`, alert acknowledge/resolve, settings updates, unassign/null cases) proving the neutral write surface on real storage code

Consumes:
- nothing (first slice)

### S02 → S03
Produces:
- explicit PostgreSQL helper surfaces for JSONB read/write expressions, full-text search/ranking/query construction, and pgcrypto-backed auth operations
- rewritten Mesher event ingest/search/auth/alert-rule storage paths using those PG helpers with unchanged record shapes
- a concrete boundary rule for what stays neutral versus what must stay PG-namespaced

Consumes from S01:
- the neutral expression builder and serializer contract
- the proven write/update/upsert expression surface

### S02 → S04
Produces:
- PG-specific migration/schema seam for extension/index/partition-related helpers rather than scattered raw DDL strings
- naming and rendering conventions shared between migration-time helpers and runtime partition-management helpers
- proof that PG-only schema behavior stays explicit and does not leak back into the neutral DSL

Consumes from S01:
- expression/rendering contract used by helper serialization where relevant

### S03 → S05
Produces:
- rewritten Mesher read-side query modules for the recurring hard families (`list_issues_filtered`, `project_health_summary`, `get_event_neighbors`, `evaluate_threshold_rule`, `check_volume_spikes`, and similar recurring shapes where honest coverage exists)
- reconciled justified raw-query keep-list containing only dishonest leftovers
- regression surface proving rewritten queries preserve expected result shapes and behavior

Consumes from S01:
- neutral expression/query-building contract

Consumes from S02:
- PG JSONB/search helper surfaces and boundary rules

### S04 → S05
Produces:
- migration-time and runtime partition create/list/drop helpers proven against live catalogs
- rewritten Mesher schema and retention paths using first-class partition helpers where honest
- operational acceptance commands for migration apply, partition lifecycle, and retention cleanup replay

Consumes from S01:
- neutral rendering/building rules where shared

Consumes from S02:
- explicit PG schema/helper namespacing and related extension/index surfaces
