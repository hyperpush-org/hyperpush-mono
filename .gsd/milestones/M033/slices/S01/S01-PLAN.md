# S01: Neutral expression core on real write paths

**Goal:** Ship a neutral expression builder and expression-aware Query/Repo write surface that can drive real Mesher mutations — upserts, computed updates, `NULL`, and `now()`-driven writes — without hiding PostgreSQL-only behavior inside the baseline API.
**Demo:** `compiler/meshc/tests/e2e_m033_s01.rs` and `scripts/verify-m033-s01.sh` prove that live Postgres-backed Mesher issue ingest/upsert, alert acknowledge/resolve, project settings updates, API-key revoke, and issue assign/unassign all behave the same through structured Query/Repo expressions, while the remaining raw write keep-list stays limited to PG-specific JSONB work that later slices own.

## Must-Haves

- The neutral Mesh data-layer contract gains a structured expression surface for column refs, literal/parameter values, `NULL`, function calls, arithmetic/comparison, `CASE`, `COALESCE`, and expression-valued `SELECT`/`SET`/`ON CONFLICT` work, wired through the compiler/runtime without `RAW:` escape hatches for portable cases.
- `mesher/storage/queries.mpl` stops using raw SQL for the S01-owned write families: `upsert_issue`, `assign_issue` unassign, `revoke_api_key`, `acknowledge_alert`, `resolve_fired_alert`, and `update_project_settings`.
- The slice keeps the R036/R040 boundary honest: PG-specific JSONB/crypto write helpers such as `create_alert_rule`, `fire_alert`, and `insert_event` stay explicit keep-sites for S02 instead of leaking into the neutral core, and the settings path moves to Mesh-side parsing rather than PG-only JSON extraction.

## Proof Level

- This slice proves: integration
- Real runtime required: yes
- Human/UAT required: no

## Verification

- `cargo test -p meshc --test e2e_m033_s01 -- --nocapture`
- `cargo test -p meshc --test e2e_m033_s01 expr_error_ -- --nocapture`
- `cargo run -q -p meshc -- fmt --check mesher`
- `cargo run -q -p meshc -- build mesher`
- `bash scripts/verify-m033-s01.sh`

## Observability / Diagnostics

- Runtime signals: named `e2e_m033_*` failures for expression-contract vs live-mesher write paths, captured Mesher stdout/stderr in the acceptance harness, and direct row snapshots from `issues`, `alerts`, `projects`, and `api_keys`
- Inspection surfaces: `compiler/meshc/tests/e2e_m033_s01.rs`, `scripts/verify-m033-s01.sh`, direct Postgres queries in the Rust harness, and the raw-write keep-list grep over `mesher/storage/queries.mpl` / `mesher/storage/writer.mpl`
- Failure visibility: each acceptance proof must surface the exact route or storage function that drifted plus the mismatched DB fields (`status`, `event_count`, `last_seen`, `assigned_to`, `acknowledged_at`, `resolved_at`, `retention_days`, `sample_rate`, `revoked_at`)
- Redaction constraints: do not print `DATABASE_URL`, cookies, or generated API keys in failing test output; only surface route names, IDs, and row-field assertions

## Integration Closure

- Upstream surfaces consumed: `compiler/mesh-rt/src/db/{query,repo,orm}.rs`, compiler Query/Repo module typing in `compiler/mesh-typeck/src/infer.rs`, intrinsic wiring in `compiler/mesh-codegen/src/{mir/lower.rs,codegen/intrinsics.rs}`, and Mesher write paths in `mesher/storage/queries.mpl`
- New wiring introduced in this slice: a dedicated neutral expression builder plus expression-aware Query/Repo entrypoints, and a live Mesher write-path acceptance harness that exercises HTTP routes then inspects database state
- What remains before the milestone is truly usable end-to-end: S02 still needs explicit PG JSONB/search/crypto helpers for `create_alert_rule`, `fire_alert`, `insert_event`, and other PG-shaped escape hatches; S03 still owns the harder read-side query families; S04 still owns partition/schema helpers

## Tasks

- [ ] **T01: Ship the neutral expression contract through compiler and runtime** `est:2.5h`
  - Why: R036 and R040 are blocked until Mesh has a real neutral expression surface that can represent portable computed writes without smuggling PostgreSQL syntax through raw strings.
  - Files: `compiler/mesh-rt/src/db/expr.rs`, `compiler/mesh-rt/src/db/query.rs`, `compiler/mesh-rt/src/db/repo.rs`, `compiler/mesh-rt/src/db/mod.rs`, `compiler/mesh-rt/src/lib.rs`, `compiler/mesh-typeck/src/infer.rs`, `compiler/mesh-codegen/src/mir/lower.rs`, `compiler/mesh-codegen/src/codegen/intrinsics.rs`, `compiler/meshc/tests/e2e_m033_s01.rs`
  - Do: Add a dedicated neutral expression builder and the expression-aware Query/Repo entrypoints Mesher needs for expression-valued `SELECT`, `SET`, and `ON CONFLICT` work. Cover column refs, literals/params, `NULL`, function calls, arithmetic/comparison, `CASE`, and `COALESCE`, plus whatever neutral conflict-update reference the upsert path needs. Wire the new surface through type inference, MIR lowering, runtime exports, and serializer logic, and add named `e2e_m033_expr_*` proofs for placeholder stability and portable expression rendering. Keep PG-only JSONB/search/crypto helpers out of this task.
  - Verify: `cargo test -p meshc --test e2e_m033_s01 expr_ -- --nocapture`
  - Done when: Mesh code can build and execute neutral expression trees for portable select/update/upsert cases without falling back to `RAW:` or `Repo.query_raw`.
- [ ] **T02: Move direct Mesher mutations onto the neutral write core** `est:2h`
  - Why: The simplest real write paths — `NULL`, `now()`, and partial settings updates — should prove the new core on live product routes before the harder conflict-upsert path lands.
  - Files: `mesher/storage/queries.mpl`, `mesher/api/alerts.mpl`, `mesher/api/settings.mpl`, `mesher/api/team.mpl`, `mesher/services/project.mpl`, `mesher/ingestion/routes.mpl`, `compiler/meshc/tests/e2e_m033_s01.rs`
  - Do: Rewrite `revoke_api_key`, `assign_issue`, `acknowledge_alert`, `resolve_fired_alert`, and `update_project_settings` to use the new expression-aware Query/Repo API. Parse settings JSON in Mesh-side helpers so the neutral write path does not depend on PG JSON extraction, keep route/service signatures stable, and extend the live Rust acceptance tests to hit the real HTTP routes and assert DB-side `NULL` / timestamp / field updates rather than only HTTP 200s.
  - Verify: `cargo test -p meshc --test e2e_m033_s01 mesher_mutations -- --nocapture`
  - Done when: those named Mesher mutation functions no longer use raw SQL and the live route tests prove identical behavior through the new neutral write core.
- [ ] **T03: Rewrite issue upsert and close the slice with live Mesher acceptance** `est:2.5h`
  - Why: `upsert_issue` is the hardest neutral-write family in the slice and the proof that the core is honest rather than just good enough for single-column updates.
  - Files: `mesher/storage/queries.mpl`, `mesher/services/event_processor.mpl`, `mesher/ingestion/routes.mpl`, `mesher/storage/writer.mpl`, `compiler/meshc/tests/e2e_m033_s01.rs`, `scripts/verify-m033-s01.sh`
  - Do: Replace the raw `INSERT ... ON CONFLICT` in `upsert_issue` with structured conflict-update expressions that increment `event_count`, refresh `last_seen` with `now()`, and flip resolved issues back to `unresolved` through `CASE`. Extend the live acceptance harness to ingest repeated events around a manual resolve so it proves new-issue creation, regression-to-unresolved, `event_count` growth, and `last_seen` movement on the real Mesher runtime path. Add `scripts/verify-m033-s01.sh` to run the new test target, keep Mesher fmt/build green, and assert that the remaining raw write keep-list is limited to the PG-specific JSONB helpers deferred to S02.
  - Verify: `cargo test -p meshc --test e2e_m033_s01 mesher_issue_upsert -- --nocapture`; `bash scripts/verify-m033-s01.sh`
  - Done when: the slice demo is true end-to-end and the S01-owned raw write families are gone from the Mesher write path.

## Files Likely Touched

- `compiler/mesh-rt/src/db/expr.rs`
- `compiler/mesh-rt/src/db/query.rs`
- `compiler/mesh-rt/src/db/repo.rs`
- `compiler/mesh-rt/src/db/mod.rs`
- `compiler/mesh-rt/src/lib.rs`
- `compiler/mesh-typeck/src/infer.rs`
- `compiler/mesh-codegen/src/mir/lower.rs`
- `compiler/mesh-codegen/src/codegen/intrinsics.rs`
- `compiler/meshc/tests/e2e_m033_s01.rs`
- `mesher/storage/queries.mpl`
- `mesher/api/alerts.mpl`
- `mesher/api/settings.mpl`
- `mesher/api/team.mpl`
- `mesher/services/project.mpl`
- `mesher/services/event_processor.mpl`
- `mesher/ingestion/routes.mpl`
- `mesher/storage/writer.mpl`
- `scripts/verify-m033-s01.sh`
