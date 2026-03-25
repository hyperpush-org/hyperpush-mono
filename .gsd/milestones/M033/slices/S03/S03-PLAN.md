# S03: Hard read-side coverage and honest raw-tail collapse

**Goal:** Collapse Mesher’s read-side raw SQL honestly by rewriting the mechanically expressible read helpers and the recurring hard whole-query families onto the existing `Query` / `Expr` / `Pg` surfaces or small Mesh-side decompositions, while leaving only a short named raw keep-list instead of a fake universal SQL abstraction.
**Demo:** After this: Mesher’s recurring scalar-subquery, derived-table, parameterized select, and expression-heavy read paths use the new builders wherever honest, and the remaining raw query keep-list is short and named.

## Must-Haves

- `mesher/storage/queries.mpl` moves the mechanically expressible read helpers — including the simple count/cast/COALESCE, aggregate, join, and listing families that already fit the current ORM surface — off raw projection strings and off whole-query raw SQL wherever `Query.select_expr{s}`, `Query.where_expr`, `Expr.label`, `Expr.coalesce`, regular `group_by` / `order_by`, and explicit `Pg.*` casts can express them honestly, while preserving the row keys consumed by `mesher/api/{search,dashboard,detail,alerts}.mpl` and related callers.
- The S03-owned hard whole-query raw families `list_issues_filtered`, `project_health_summary`, `get_event_neighbors`, and `evaluate_threshold_rule` stop using `Repo.query_raw(...)` through honest conditional query building and Mesh-side composition, and the remaining read-side keep-list stays short, named, and justified instead of being hidden behind a misleading neutral AST; `extract_event_fields`, `check_volume_spikes`, and `check_sample_rate` remain explicit only if they are still dishonest after the rewrite pass.
- A new live Postgres-backed proof bundle in `compiler/meshc/tests/e2e_m033_s03.rs` plus `scripts/verify-m033-s03.sh` proves the rewritten read helpers, filtered issue listing, health summary counts, event neighbor navigation, threshold evaluation, and the owned raw keep-list boundary on the real Mesher storage path.

## Proof Level

- This slice proves: - This slice proves: integration
- Real runtime required: yes
- Human/UAT required: no

## Integration Closure

- Upstream surfaces consumed: S01’s neutral `Expr` / `Query` / `Repo` contract, S02’s explicit `Pg.*` helper seam, and the caller contracts in `mesher/api/{search,dashboard,detail,alerts}.mpl` plus `mesher/ingestion/{pipeline,routes}.mpl`.
- New wiring introduced in this slice: read-side query rewrites in `mesher/storage/queries.mpl`, targeted live Postgres assertions in `compiler/meshc/tests/e2e_m033_s03.rs`, and an S03-specific keep-list verifier in `scripts/verify-m033-s03.sh`.
- What remains before the milestone is truly usable end-to-end: S04 still owns schema/partition helpers, and S05 still owns public docs plus the final integrated replay.

## Verification

- Runtime signals: named `e2e_m033_s03_*` failures should isolate projection-shape drift, aggregate/count mismatches, cursor-order bugs, and threshold-evaluation regressions; `scripts/verify-m033-s03.sh` should name the offending function when the raw-boundary contract drifts.
- Inspection surfaces: `compiler/meshc/tests/e2e_m033_s03.rs`, `scripts/verify-m033-s03.sh`, and the direct Postgres assertions inside the Rust harness against `issues`, `events`, `alert_rules`, and `alerts`.
- Failure visibility: row-shape mismatches, ordering/cursor drift, and keep-list regressions should be explicit without printing passwords, tokens, or full connection strings.
- Redaction constraints: never log secret-bearing inputs or `DATABASE_URL`; assert on IDs, counts, booleans, timestamps, and map keys only.

## Tasks

- [x] **T01: Seeded the S03 harness and rewrote the basic read helpers, but the new Mesh probes still need quote cleanup** `est:2h`
  Start S03 with the lowest-risk raw-tail collapse and the permanent proof harness. This task should create the first real `compiler/meshc/tests/e2e_m033_s03.rs` file instead of deferring all proof work to the end, then use the current S01/S02 `Expr` / `Query` / `Pg` surface to eliminate the simplest projection/count/cast read helpers in `mesher/storage/queries.mpl`. The key constraint is caller stability: preserve every row key the existing API and ingestion callers read today.

## Steps

1. Copy the Docker/Postgres harness pattern from `compiler/meshc/tests/e2e_m033_s02.rs` into a new `compiler/meshc/tests/e2e_m033_s03.rs` target and add the first named `e2e_m033_s03_basic_reads_*` proofs for the easy helper families.
2. Rewrite the plain projection/count/cast helpers in `mesher/storage/queries.mpl` — `count_unresolved_issues`, `get_issue_project_id`, `validate_session`, `list_api_keys`, `list_alert_rules`, `get_all_project_retention`, `get_project_storage`, and `get_project_settings` — to use `Query.select_expr{s}`, `Query.where_expr`, `Expr.label`, `Expr.coalesce`, and explicit `Pg.*` casts where the current surface already expresses the query honestly.
3. Keep the caller-visible map keys stable for `mesher/ingestion/routes.mpl`, `mesher/api/dashboard.mpl`, `mesher/api/settings.mpl`, and `mesher/api/alerts.mpl`; only touch a caller if a field name would otherwise drift.
4. Leave the hard whole-query raw families and the named S03 leftovers for later tasks instead of sneaking in dishonest abstractions during the easy cleanup pass.

## Must-Haves

- [ ] `compiler/meshc/tests/e2e_m033_s03.rs` exists with named `e2e_m033_s03_basic_reads_*` coverage for the T01 helper families
- [ ] The T01 helper families no longer depend on raw projection strings or trivial raw whole-query SQL where the existing builder surface is already honest
- [ ] Caller-visible row keys such as `cnt`, `project_id`, `token`, `revoked_at`, `retention_days`, `sample_rate`, `event_count`, and `estimated_bytes` remain unchanged
  - Files: `compiler/meshc/tests/e2e_m033_s03.rs`, `mesher/storage/queries.mpl`, `mesher/ingestion/routes.mpl`, `mesher/api/dashboard.mpl`, `mesher/api/settings.mpl`, `mesher/api/alerts.mpl`
  - Verify: - `cargo test -p meshc --test e2e_m033_s03 basic_reads -- --nocapture`
- `cargo run -q -p meshc -- build mesher`

- [x] **T02: Attempted the T02 composed-read proof expansion, fixed the probe-compatible boolean helpers, and recorded a storage-probe blocker for the remaining read families.** `est:2.5h`
  Keep pushing the read-side cleanup on the families that already fit the current ORM surface but still lean on raw SELECT, ORDER BY, or GROUP BY fragments. This task is still Mesher-only work: use the current builder and explicit `Pg.*` seam rather than widening the neutral core. The important constraint is that the dashboard/detail/search/team/alerts callers must see the same row keys and ordering semantics they consume today.

## Steps

1. Extend `compiler/meshc/tests/e2e_m033_s03.rs` with named `e2e_m033_s03_composed_reads_*` coverage for the joined, aggregate, and list families this task owns.
2. Rewrite the joined and aggregate read helpers in `mesher/storage/queries.mpl` — `get_project_by_api_key`, `list_issues_by_status`, `event_volume_hourly`, `error_breakdown_by_level`, `top_issues_by_frequency`, `event_breakdown_by_tag`, `get_event_detail`, and `get_members_with_users` — onto `Query.select_expr{s}`, ordinary `group_by` / `order_by`, `Expr.label`, `Expr.coalesce`, and explicit `Pg.*` casts wherever those surfaces already tell the truth.
3. Rewrite the remaining current-surface list helpers that only need conditional query assembly or projection cleanup — `list_events_for_issue`, `list_alerts`, `check_new_issue`, and `should_fire_by_cooldown` — without promoting them back to `Repo.query_raw(...)` whole-query strings.
4. Keep the map keys, sort order, and null/default handling stable for `mesher/api/search.mpl`, `mesher/api/dashboard.mpl`, `mesher/api/detail.mpl`, `mesher/api/alerts.mpl`, and `mesher/api/team.mpl`.

## Must-Haves

- [ ] The T02 joined, aggregate, and list helpers use the current builder surface wherever it is already honest instead of recurring raw SELECT / ORDER / GROUP fragments
- [ ] `compiler/meshc/tests/e2e_m033_s03.rs` contains named `e2e_m033_s03_composed_reads_*` proofs for the T02 families
- [ ] Caller-visible row keys, ordering, and null/default semantics stay unchanged for the dashboard/detail/search/team/alerts surfaces
  - Files: `compiler/meshc/tests/e2e_m033_s03.rs`, `mesher/storage/queries.mpl`, `mesher/api/search.mpl`, `mesher/api/dashboard.mpl`, `mesher/api/detail.mpl`, `mesher/api/alerts.mpl`, `mesher/api/team.mpl`
  - Verify: - `cargo test -p meshc --test e2e_m033_s03 composed_reads -- --nocapture`
- `cargo run -q -p meshc -- build mesher`

- [x] **T03: Replace the failing storage-probe proof surface with a Mesher-backed composed-read harness** `est:3h`
  Why: T02 showed the copied storage-only probe cannot safely consume the remaining struct-list and aggregate read shapes, so S03 needs a higher-level proof boundary before more read-side work is credible.

Do: Keep the passing `basic_reads` family, then move the partial `composed_reads` coverage off the direct storage-probe staging path and onto a Mesher-backed surface that exercises the same `search` / `dashboard` / `detail` / `alerts` / `team` caller contracts. Prove the already-rewritten joined, list, aggregate, and boolean helper families there (`get_project_by_api_key`, `list_issues_by_status`, `event_volume_hourly`, `error_breakdown_by_level`, `top_issues_by_frequency`, `event_breakdown_by_tag`, `get_event_detail`, `get_members_with_users`, `list_events_for_issue`, `list_alerts`, `check_new_issue`, `should_fire_by_cooldown`). If the new proof surface still trips the same staging bug, limit any compiler/runtime-side changes to the smallest test-enabler needed for honest read assertions rather than widening Mesh product scope.
  - Files: `compiler/meshc/tests/e2e_m033_s03.rs`, `mesher/storage/queries.mpl`, `mesher/api/search.mpl`, `mesher/api/dashboard.mpl`, `mesher/api/detail.mpl`, `mesher/api/alerts.mpl`, `mesher/api/team.mpl`, `.gsd/KNOWLEDGE.md`
  - Verify: cargo test -p meshc --test e2e_m033_s03 composed_reads -- --nocapture
cargo run -q -p meshc -- build mesher

- [ ] **T04: Retire the hard whole-query raw read families on the new proof surface** `est:3h`
  Why: Once the proof surface is honest again, S03 still has to retire the slice-owned whole-query raw families rather than leaving the main raw tail untouched.

Do: Rewrite `list_issues_filtered`, `project_health_summary`, `get_event_neighbors`, and `evaluate_threshold_rule` to use conditional builder-backed reads plus small Mesh-side composition, then add named `hard_reads` proofs on the Mesher-backed harness. Re-evaluate `extract_event_fields`, `check_volume_spikes`, and `check_sample_rate` after the rewrite pass; retire any that become honest, and keep only the genuinely dishonest leftovers in an explicit named keep-list with justification instead of hiding them behind a fake universal query abstraction.
  - Files: `compiler/meshc/tests/e2e_m033_s03.rs`, `mesher/storage/queries.mpl`, `mesher/api/search.mpl`, `mesher/api/dashboard.mpl`, `mesher/api/detail.mpl`, `mesher/api/alerts.mpl`, `mesher/ingestion/pipeline.mpl`, `mesher/ingestion/routes.mpl`
  - Verify: cargo test -p meshc --test e2e_m033_s03 hard_reads -- --nocapture
cargo run -q -p meshc -- build mesher

- [ ] **T05: Close S03 with the live Postgres verifier and named keep-list gate** `est:2h`
  Why: After the proof-surface pivot and hard-family rewrites, the slice still needs one stable rerunnable acceptance path that proves both behavior and the raw-boundary contract.

Do: Finish the full live-Postgres `e2e_m033_s03.rs` suite on the new harness, then add or update `scripts/verify-m033-s03.sh` so it runs the full S03 test target, Mesher fmt/build checks, and a keep-list sweep naming the only allowed S03 leftovers while excluding the S04-owned partition/catalog raw sites. Make failures point at the drifting proof family or offending function block so future agents do not need to rediscover the boundary by hand.
  - Files: `compiler/meshc/tests/e2e_m033_s03.rs`, `scripts/verify-m033-s03.sh`, `mesher/storage/queries.mpl`, `compiler/meshc/tests/e2e_m033_s02.rs`, `scripts/verify-m033-s02.sh`
  - Verify: cargo test -p meshc --test e2e_m033_s03 -- --nocapture
cargo run -q -p meshc -- fmt --check mesher
cargo run -q -p meshc -- build mesher
bash scripts/verify-m033-s03.sh

## Files Likely Touched

- compiler/meshc/tests/e2e_m033_s03.rs
- mesher/storage/queries.mpl
- mesher/ingestion/routes.mpl
- mesher/api/dashboard.mpl
- mesher/api/settings.mpl
- mesher/api/alerts.mpl
- mesher/api/search.mpl
- mesher/api/detail.mpl
- mesher/api/team.mpl
- .gsd/KNOWLEDGE.md
- mesher/ingestion/pipeline.mpl
- scripts/verify-m033-s03.sh
- compiler/meshc/tests/e2e_m033_s02.rs
- scripts/verify-m033-s02.sh
