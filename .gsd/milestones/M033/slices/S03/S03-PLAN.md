# S03: Hard read-side coverage and honest raw-tail collapse

**Goal:** Collapse Mesher’s read-side raw SQL honestly by rewriting the mechanically expressible read helpers and the recurring hard whole-query families onto the existing `Query` / `Expr` / `Pg` surfaces or small Mesh-side decompositions, while leaving only a short named raw keep-list instead of a fake universal SQL abstraction.
**Demo:** After this: Mesher’s recurring scalar-subquery, derived-table, parameterized select, and expression-heavy read paths use the new builders wherever honest, and the remaining raw query keep-list is short and named.

## Must-Haves

- `mesher/storage/queries.mpl` moves the mechanically expressible read helpers — including the simple count/cast/COALESCE, aggregate, join, and listing families that already fit the current ORM surface — off raw projection strings and off whole-query raw SQL wherever `Query.select_expr{s}`, `Query.where_expr`, `Expr.label`, `Expr.coalesce`, regular `group_by` / `order_by`, and explicit `Pg.*` casts can express them honestly, while preserving the row keys consumed by `mesher/api/{search,dashboard,detail,alerts}.mpl` and related callers.
- The S03-owned hard whole-query raw families `list_issues_filtered`, `project_health_summary`, `get_event_neighbors`, and `evaluate_threshold_rule` stop using `Repo.query_raw(...)` through honest conditional query building and Mesh-side composition, and the remaining read-side keep-list stays short, named, and justified instead of being hidden behind a misleading neutral AST; `extract_event_fields`, `check_volume_spikes`, and `check_sample_rate` remain explicit only if they are still dishonest after the rewrite pass.
- A new live Postgres-backed proof bundle in `compiler/meshc/tests/e2e_m033_s03.rs` plus `scripts/verify-m033-s03.sh` proves the rewritten read helpers, filtered issue listing, health summary counts, event neighbor navigation, threshold evaluation, and the owned raw keep-list boundary on the real Mesher storage path.

## Proof Level

- This slice proves: integration
- Real runtime required: yes
- Human/UAT required: no

## Verification

- `cargo test -p meshc --test e2e_m033_s03 -- --nocapture`
- `cargo run -q -p meshc -- fmt --check mesher`
- `cargo run -q -p meshc -- build mesher`
- `bash scripts/verify-m033-s03.sh`

## Observability / Diagnostics

- Runtime signals: named `e2e_m033_s03_*` failures should isolate projection-shape drift, aggregate/count mismatches, cursor-order bugs, and threshold-evaluation regressions; `scripts/verify-m033-s03.sh` should name the offending function when the raw-boundary contract drifts.
- Inspection surfaces: `compiler/meshc/tests/e2e_m033_s03.rs`, `scripts/verify-m033-s03.sh`, and the direct Postgres assertions inside the Rust harness against `issues`, `events`, `alert_rules`, and `alerts`.
- Failure visibility: row-shape mismatches, ordering/cursor drift, and keep-list regressions should be explicit without printing passwords, tokens, or full connection strings.
- Redaction constraints: never log secret-bearing inputs or `DATABASE_URL`; assert on IDs, counts, booleans, timestamps, and map keys only.

## Integration Closure

- Upstream surfaces consumed: S01’s neutral `Expr` / `Query` / `Repo` contract, S02’s explicit `Pg.*` helper seam, and the caller contracts in `mesher/api/{search,dashboard,detail,alerts}.mpl` plus `mesher/ingestion/{pipeline,routes}.mpl`.
- New wiring introduced in this slice: read-side query rewrites in `mesher/storage/queries.mpl`, targeted live Postgres assertions in `compiler/meshc/tests/e2e_m033_s03.rs`, and an S03-specific keep-list verifier in `scripts/verify-m033-s03.sh`.
- What remains before the milestone is truly usable end-to-end: S04 still owns schema/partition helpers, and S05 still owns public docs plus the final integrated replay.

## Tasks

- [x] **T01: Seed the S03 harness and replace basic read projection helpers** `est:2h`
  - Why: This is the lowest-risk R038 progress and it also creates the permanent S03 proof target that later tasks will extend instead of inventing their own one-off checks.
  - Files: `compiler/meshc/tests/e2e_m033_s03.rs`, `mesher/storage/queries.mpl`, `mesher/ingestion/routes.mpl`, `mesher/api/dashboard.mpl`, `mesher/api/settings.mpl`, `mesher/api/alerts.mpl`
  - Do: Copy the S02 live-Postgres harness pattern into `e2e_m033_s03.rs`, add named `basic_reads` proofs, and rewrite the simple projection/count/cast helper families (`count_unresolved_issues`, `get_issue_project_id`, `validate_session`, `list_api_keys`, `list_alert_rules`, `get_all_project_retention`, `get_project_storage`, `get_project_settings`) onto `Query.select_expr{s}`, `Query.where_expr`, `Expr.label`, `Expr.coalesce`, and explicit `Pg.*` casts. Preserve every caller-visible row key.
  - Verify: `cargo test -p meshc --test e2e_m033_s03 basic_reads -- --nocapture`; `cargo run -q -p meshc -- build mesher`
  - Done when: the T01 helper families no longer depend on raw projection strings or trivial raw whole-query SQL where the current builder surface is already honest, and the first named `e2e_m033_s03_basic_reads_*` proofs exist.
- [ ] **T02: Rewrite joined, aggregate, and list read queries onto the current builder surface** `est:2.5h`
  - Why: The slice cannot honestly claim read-side collapse until the recurring joined, aggregate, and list helpers also stop leaning on raw SELECT / ORDER / GROUP fragments that the current ORM surface already covers.
  - Files: `compiler/meshc/tests/e2e_m033_s03.rs`, `mesher/storage/queries.mpl`, `mesher/api/search.mpl`, `mesher/api/dashboard.mpl`, `mesher/api/detail.mpl`, `mesher/api/alerts.mpl`, `mesher/api/team.mpl`
  - Do: Extend the S03 test target with named `composed_reads` proofs and rewrite the joined, aggregate, and list helpers (`get_project_by_api_key`, `list_issues_by_status`, `event_volume_hourly`, `error_breakdown_by_level`, `top_issues_by_frequency`, `event_breakdown_by_tag`, `get_event_detail`, `get_members_with_users`, `list_events_for_issue`, `list_alerts`, `check_new_issue`, `should_fire_by_cooldown`) onto `Query.select_expr{s}`, ordinary `group_by` / `order_by`, explicit `Pg.*` casts, and conditional query assembly wherever those surfaces already tell the truth.
  - Verify: `cargo test -p meshc --test e2e_m033_s03 composed_reads -- --nocapture`; `cargo run -q -p meshc -- build mesher`
  - Done when: the T02 families use the current builder surface wherever it is already honest, and the dashboard/detail/search/team/alerts callers still see the same row keys, ordering, and null/default semantics.
- [ ] **T03: Retire the hard whole-query raw read families with honest decomposition** `est:3h`
  - Why: R038’s primary bar is not met until the recurring whole-query raw families themselves are retired or narrowed to an explicit named leftover list rather than being hidden behind a misleading generic abstraction.
  - Files: `compiler/meshc/tests/e2e_m033_s03.rs`, `mesher/storage/queries.mpl`, `mesher/api/search.mpl`, `mesher/api/dashboard.mpl`, `mesher/api/detail.mpl`, `mesher/ingestion/pipeline.mpl`, `mesher/ingestion/routes.mpl`
  - Do: Add named `hard_reads` proofs, replace `list_issues_filtered`, `project_health_summary`, `get_event_neighbors`, and `evaluate_threshold_rule` with honest conditional query building plus small Mesh-side combinations of builder-backed reads, preserve their exact output keys, and keep only the short named leftovers that are still genuinely dishonest (`extract_event_fields`, `check_volume_spikes`, `check_sample_rate` unless execution proves otherwise).
  - Verify: `cargo test -p meshc --test e2e_m033_s03 hard_reads -- --nocapture`; `cargo run -q -p meshc -- build mesher`
  - Done when: the hard read families no longer depend on `Repo.query_raw(...)` whole-query strings and the remaining keep-list is short, explicit, and justified.
- [ ] **T04: Close S03 with the live Postgres verifier and named keep-list gate** `est:2h`
  - Why: The slice is not done until a future agent can rerun one stable proof bundle and one stable keep-list gate without re-reading the research or re-auditing raw SQL by hand.
  - Files: `compiler/meshc/tests/e2e_m033_s03.rs`, `scripts/verify-m033-s03.sh`, `mesher/storage/queries.mpl`, `compiler/meshc/tests/e2e_m033_s01.rs`, `compiler/meshc/tests/e2e_m033_s02.rs`, `scripts/verify-m033-s02.sh`
  - Do: Finish the full live-Postgres S03 suite in `e2e_m033_s03.rs`, then add `scripts/verify-m033-s03.sh` to run the full test target, Mesher fmt/build checks, and a Python keep-list sweep that names the only allowed S03 leftovers while excluding the S04-owned partition/catalog raw sites.
  - Verify: `cargo test -p meshc --test e2e_m033_s03 -- --nocapture`; `bash scripts/verify-m033-s03.sh`
  - Done when: the slice-level verification section passes unchanged and the verifier names the exact drifting proof family or function block whenever S03 regresses.

## Files Likely Touched

- `compiler/meshc/tests/e2e_m033_s03.rs`
- `mesher/storage/queries.mpl`
- `mesher/ingestion/routes.mpl`
- `mesher/api/dashboard.mpl`
- `mesher/api/settings.mpl`
- `mesher/api/alerts.mpl`
- `mesher/api/search.mpl`
- `mesher/api/detail.mpl`
- `mesher/api/team.mpl`
- `mesher/ingestion/pipeline.mpl`
- `scripts/verify-m033-s03.sh`
- `compiler/meshc/tests/e2e_m033_s01.rs`
- `compiler/meshc/tests/e2e_m033_s02.rs`
- `scripts/verify-m033-s02.sh`
