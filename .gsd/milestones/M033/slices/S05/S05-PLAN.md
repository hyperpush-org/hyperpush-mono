# S05: Public docs and integrated Mesher acceptance

**Goal:** Publish the real M033 database boundary in the public Mesh docs and close the milestone with one canonical acceptance command that replays the assembled live-Postgres proof stack plus a docs-truth sweep.
**Demo:** After this: the public Mesh database docs explain the shipped neutral DSL and PG extras through a Mesher-backed path, and the assembled Mesher data-layer behavior is re-proven end-to-end on live Postgres.

## Must-Haves

- R038: `scripts/verify-m033-s05.sh` is the canonical S05 command, runs `bash scripts/verify-m033-s02.sh`, `bash scripts/verify-m033-s03.sh`, `bash scripts/verify-m033-s04.sh`, and `npm --prefix website run build` serially, and fails with named logs when docs or proof drift.
- R038: `website/docs/docs/databases/index.md` explains the remaining raw boundary as a short honest leftover / escape-hatch story, explicitly naming `Repo.query_raw`, `Repo.execute_raw`, and `Migration.execute` instead of claiming zero raw SQL/DDL.
- R040: the public docs separate the portable core (`Expr`, `Query`, `Repo`, and honest `Migration.create_index(...)`) from PostgreSQL-only helpers under `Pg.*`, and explicitly say SQLite-specific extras are later work rather than a runtime-proven surface here.
- The docs are Mesher-backed and teach the real shipped APIs and anchors: `Expr.label`, `Query.where_expr`, `Query.select_exprs`, `Repo.insert_expr`, `Repo.update_where_expr`, `Repo.insert_or_update_expr`, `mesher/storage/queries.mpl`, `mesher/storage/writer.mpl`, `mesher/migrations/20260216120000_create_initial_schema.mpl`, and `mesher/storage/schema.mpl`.

## Proof Level

- This slice proves: final-assembly
- Real runtime required: yes
- Human/UAT required: no

## Integration Closure

- Upstream surfaces consumed: `scripts/verify-m033-s02.sh`, `scripts/verify-m033-s03.sh`, `scripts/verify-m033-s04.sh`, `website/docs/docs/databases/index.md`, `website/docs/docs/production-backend-proof/index.md`, `compiler/meshc/tests/e2e_m033_s01.rs`, `mesher/storage/queries.mpl`, `mesher/storage/writer.mpl`, `mesher/migrations/20260216120000_create_initial_schema.mpl`, `mesher/storage/schema.mpl`, and `compiler/meshc/src/migrate.rs`.
- New wiring introduced in this slice: the public database docs are rewritten around the real neutral-vs-PG-vs-escape-hatch contract, and `scripts/verify-m033-s05.sh` becomes the serial wrapper plus docs-truth gate for the assembled M033 proof stack.
- What remains before the milestone is truly usable end-to-end: nothing for M033 once the docs build and `bash scripts/verify-m033-s05.sh` both pass.

## Verification

- Acceptance commands: `npm --prefix website run build` and `bash scripts/verify-m033-s05.sh` must both pass.
- Runtime signals: `scripts/verify-m033-s05.sh` should preserve per-phase logs under `.tmp/m033-s05/verify/` and name whether docs build, docs truth, S02, S03, or S04 failed first.
- Inspection surfaces: `website/docs/docs/databases/index.md`, `.tmp/m033-s05/verify/*.log`, and the underlying slice verifier logs.
- Failure visibility: docs drift should fail on missing exact API names / file anchors / proof commands; runtime proof drift should surface the first failing delegated verifier without parallelizing the shared Postgres port 5432 flow; never log DSNs or other secrets.

## Tasks

- [x] **T01: Rewrite the public database guide around the honest Mesh/Mesher boundary** `est:2h`
  Why: The current database guide is still a generic `Sqlite` / `Pg` / `Pool` brochure, so S05 needs a public Mesher-backed rewrite before automation can lock the milestone’s truth surface.

Do:
1. Reframe `website/docs/docs/databases/index.md` around the shipped M033 contract instead of the generic database tour: neutral `Expr` / `Query` / `Repo` / honest `Migration.create_index(...)`, explicit `Pg.*` extras, remaining raw escape hatches, and the proof/failure map.
2. Build the neutral section from the real S01/S02/S04 sources and examples: use `Expr.label`, `Expr.value`, `Expr.column`, `Expr.null`, `Expr.case_when`, `Expr.coalesce`, `Query.where_expr`, `Query.select_exprs`, `Repo.insert_expr`, `Repo.update_where_expr`, and `Repo.insert_or_update_expr` as they appear in `compiler/meshc/tests/e2e_m033_s01.rs`, `mesher/storage/writer.mpl`, and `mesher/storage/queries.mpl`.
3. Build the PostgreSQL-only section from the real Mesher-backed helpers in `mesher/storage/queries.mpl`, `mesher/migrations/20260216120000_create_initial_schema.mpl`, `mesher/storage/schema.mpl`, and `compiler/meshc/src/migrate.rs`; keep JSONB/search/crypto/partition/schema helpers explicitly under `Pg.*`, and avoid inventing brittle pseudo-examples (for example, do not introduce a `jsonb_build_object(...)` example unless the required `Pg.text(...)` casts are shown).
4. Add the honest boundary/proof sections: name `Repo.query_raw`, `Repo.execute_raw`, and `Migration.execute` as escape hatches; explain that M033 leaves a short named raw leftover list instead of promising zero raw SQL/DDL; say SQLite extras are later work and not runtime-proven here; keep the page aligned with the repo’s proof-surface language/style.

Must-Haves:
- [ ] The page teaches the shipped neutral surface with real API names, including `Expr.label` rather than `Expr.alias`.
- [ ] The page marks JSONB/search/crypto/partition/schema helpers as PostgreSQL-only `Pg.*` behavior, not as portable APIs.
- [ ] The page tells an honest raw-leftover / SQLite-later story and anchors it in the real Mesher files that ship today.
  - Files: `website/docs/docs/databases/index.md`, `website/docs/docs/production-backend-proof/index.md`, `compiler/meshc/tests/e2e_m033_s01.rs`, `mesher/storage/queries.mpl`, `mesher/storage/writer.mpl`, `mesher/migrations/20260216120000_create_initial_schema.mpl`, `mesher/storage/schema.mpl`, `compiler/meshc/src/migrate.rs`
  - Verify: npm --prefix website run build
  - Done when: `website/docs/docs/databases/index.md` teaches the shipped neutral APIs, marks PG extras as explicit `Pg.*` behavior, tells an honest raw-leftover / SQLite-later story, and the VitePress build passes.

- [x] **T02: Add the canonical S05 verifier and final acceptance replay** `est:2h`
  Why: R038 only closes once one public command replays the assembled proof stack and mechanically fails when docs drift away from the real boundary.

Do:
1. Add `scripts/verify-m033-s05.sh` using the same failure-reporting pattern as the existing slice verifiers plus the docs-truth style from `reference-backend/scripts/verify-production-proof-surface.sh`, with a dedicated `.tmp/m033-s05/verify` artifact directory and named phase logs.
2. Make the wrapper run the cheap docs gate first (`npm --prefix website run build`), then an exact-string Python sweep over `website/docs/docs/databases/index.md`, then `bash scripts/verify-m033-s02.sh`, `bash scripts/verify-m033-s03.sh`, and `bash scripts/verify-m033-s04.sh` serially. Do not parallelize: the Postgres-backed proof surfaces share host port `5432`.
3. In the docs-truth sweep, require the exact neutral API names, PG-only API names, honest boundary wording, SQLite-later wording, Mesher-backed file references, and canonical proof commands that the public docs are supposed to stand behind.
4. Tighten `website/docs/docs/databases/index.md` as needed so the final public page includes the new canonical `bash scripts/verify-m033-s05.sh` command and the exact phrases the verifier enforces, without turning the page into zero-raw marketing.

Must-Haves:
- [ ] `scripts/verify-m033-s05.sh` becomes the canonical S05 acceptance command and preserves serial execution across the existing live-Postgres verifiers.
- [ ] The Python docs-truth sweep fails on missing API names, boundary wording, Mesher file anchors, or proof commands instead of silently allowing docs drift.
- [ ] The final docs page and the new script agree on the exact public contract, including the honest leftover / escape-hatch story and the SQLite-later seam.
  - Files: `scripts/verify-m033-s05.sh`, `website/docs/docs/databases/index.md`, `scripts/verify-m033-s02.sh`, `scripts/verify-m033-s03.sh`, `scripts/verify-m033-s04.sh`, `reference-backend/scripts/verify-production-proof-surface.sh`
  - Verify: bash scripts/verify-m033-s05.sh
  - Done when: `scripts/verify-m033-s05.sh` runs the docs build, exact-string docs sweep, and S02/S03/S04 proof stack serially with per-phase logs, and the docs page includes the canonical `bash scripts/verify-m033-s05.sh` contract it enforces.

## Files Likely Touched

- website/docs/docs/databases/index.md
- website/docs/docs/production-backend-proof/index.md
- compiler/meshc/tests/e2e_m033_s01.rs
- mesher/storage/queries.mpl
- mesher/storage/writer.mpl
- mesher/migrations/20260216120000_create_initial_schema.mpl
- mesher/storage/schema.mpl
- compiler/meshc/src/migrate.rs
- scripts/verify-m033-s05.sh
- scripts/verify-m033-s02.sh
- scripts/verify-m033-s03.sh
- scripts/verify-m033-s04.sh
- reference-backend/scripts/verify-production-proof-surface.sh
