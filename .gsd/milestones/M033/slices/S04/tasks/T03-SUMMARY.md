---
id: T03
parent: S04
milestone: M033
key_files:
  - mesher/storage/schema.mpl
  - mesher/storage/queries.mpl
  - mesher/services/retention.mpl
  - mesher/main.mpl
  - compiler/mesh-rt/src/db/pg_schema.rs
  - compiler/meshc/tests/e2e_m033_s04.rs
  - scripts/verify-m033-s04.sh
key_decisions:
  - Keep Mesher’s runtime partition lifecycle explicitly PostgreSQL-shaped inside `Storage.Schema` wrappers over `Pg.*` helpers instead of widening `Storage.Queries` or the generic query API.
  - Allow `Pg.create_range_partitioned_table` to accept raw table-constraint entries so helper-driven partitioned migrations can express composite primary keys without falling back to raw DDL.
  - Add a dedicated S04 live verifier (`e2e_m033_s04.rs` plus `verify-m033-s04.sh`) and have its migration/temp-binary paths rebuild `mesh-rt` first so verification exercises the current runtime ABI instead of a stale static archive.
duration: ""
verification_result: passed
completed_at: 2026-03-25T23:26:15.950Z
blocker_discovered: false
---

# T03: Move Mesher partition lifecycle into Storage.Schema and add live S04 verification coverage

**Move Mesher partition lifecycle into Storage.Schema and add live S04 verification coverage**

## What Happened

I moved Mesher’s runtime partition create/list/drop ownership into `mesher/storage/schema.mpl` by replacing the old raw SQL loop with explicit `Pg.create_daily_partitions_ahead(...)`, `Pg.list_daily_partitions_before(...)`, and `Pg.drop_partition(...)` wrappers. I removed the S04-owned partition/catalog helpers from `mesher/storage/queries.mpl`, rewired `mesher/services/retention.mpl` to import partition lifecycle functions from `Storage.Schema`, updated its cleanup flow to operate on `List<String>` partition names, and added stage-specific logging so project cleanup, partition listing, and partition drop failures localize cleanly. I also improved Mesher startup logging in `mesher/main.mpl` so the partition bootstrap path now reports success/failure explicitly without printing secrets.

During verification, the planned S04 acceptance assets were missing in this checkout, so I added `compiler/meshc/tests/e2e_m033_s04.rs` and `scripts/verify-m033-s04.sh` following the existing M033 slice pattern. The first live S04 gate then exposed a real regression outside the four Mesher files: `Pg.create_range_partitioned_table` still rejected the raw `PRIMARY KEY (id, received_at)` table constraint used by Mesher’s helper-driven initial migration. I fixed that root cause in `compiler/mesh-rt/src/db/pg_schema.rs` by teaching the helper to accept raw table constraints while still tracking real column entries for partition-column validation, and I added a focused mesh-rt unit test covering that case. The new S04 e2e target now proves the helper-driven migration against live Postgres catalogs, exercises `Storage.Schema` partition create/list/drop behavior on a real database, and verifies the real Mesher startup path creates seven daily partitions ahead and logs partition bootstrap success without leaking DSNs or env-var names.

## Verification

Verified the S04 runtime boundary and the full slice acceptance path. A focused mesh-rt unit test confirmed `Pg.create_range_partitioned_table` now accepts raw table constraints needed by the Mesher migration. The new `e2e_m033_s04` integration target passed against live Postgres, proving: pgcrypto extension installation, `events` remaining range-partitioned on `received_at`, GIN/jsonb_path_ops index presence, `Storage.Schema` partition creation/list/drop behavior, and real Mesher startup partition bootstrap plus log visibility. Final acceptance commands all passed: `cargo run -q -p meshc -- fmt --check mesher`, `cargo run -q -p meshc -- build mesher`, and `bash scripts/verify-m033-s04.sh` (which reruns the S04 e2e target plus ownership/logging checks).

## Verification Evidence

| # | Command | Exit Code | Verdict | Duration |
|---|---------|-----------|---------|----------|
| 1 | `cargo test -p mesh-rt migration_pg_schema_build_create_range_partitioned_table_sql_allows_table_constraints -- --nocapture` | 0 | ✅ pass | 18918ms |
| 2 | `cargo test -p meshc --test e2e_m033_s04 -- --nocapture` | 0 | ✅ pass | 172660ms |
| 3 | `cargo run -q -p meshc -- fmt --check mesher` | 0 | ✅ pass | 13233ms |
| 4 | `cargo run -q -p meshc -- build mesher` | 0 | ✅ pass | 24336ms |
| 5 | `bash scripts/verify-m033-s04.sh` | 0 | ✅ pass | 94665ms |


## Deviations

The task plan expected only the four Mesher files to change, but the checkout was missing the planned S04 acceptance artifacts and the live gate exposed a pre-existing helper bug in `compiler/mesh-rt/src/db/pg_schema.rs`. I added the missing S04 verifier assets and fixed that root-cause runtime helper regression so the slice acceptance contract could actually pass.

## Known Issues

None.

## Files Created/Modified

- `mesher/storage/schema.mpl`
- `mesher/storage/queries.mpl`
- `mesher/services/retention.mpl`
- `mesher/main.mpl`
- `compiler/mesh-rt/src/db/pg_schema.rs`
- `compiler/meshc/tests/e2e_m033_s04.rs`
- `scripts/verify-m033-s04.sh`
