---
id: S05
parent: M033
milestone: M033
provides:
  - Canonical `bash scripts/verify-m033-s05.sh` final-assembly replay for M033.
  - Public database docs that truthfully document the portable core, explicit `Pg.*` extras, raw escape hatches, and the SQLite-later seam.
  - Named per-phase verification logs under `.tmp/m033-s05/verify/` for diagnosing docs or proof-surface drift.
requires:
  - slice: S02
    provides: Explicit PostgreSQL JSONB/search/crypto helper surfaces and the verifier-owned boundary that S05 now replays and documents.
  - slice: S03
    provides: The honest raw-read keep-list and read-side proof surface that S05 names publicly and reruns end to end.
  - slice: S04
    provides: Explicit PostgreSQL schema/partition helpers plus live partition acceptance verifiers that S05 composes into the final assembled replay.
affects:
  []
key_files:
  - website/docs/docs/databases/index.md
  - scripts/verify-m033-s05.sh
  - scripts/verify-m033-s02.sh
  - scripts/verify-m033-s03.sh
  - scripts/verify-m033-s04.sh
  - .gsd/REQUIREMENTS.md
  - .gsd/KNOWLEDGE.md
  - .gsd/DECISIONS.md
  - .gsd/PROJECT.md
key_decisions:
  - Use a serial wrapper over the existing S02/S03/S04 live-Postgres verifiers plus a docs build/truth gate as the canonical S05 acceptance surface instead of inventing a new runtime harness.
  - Keep the public database docs explicit about the portable `Expr`/`Query`/`Repo`/`Migration.create_index(...)` core, PostgreSQL-only `Pg.*` extras, and the remaining raw escape hatches instead of implying zero raw SQL/DDL.
  - Treat `get_event_alert_rules(...)` and `get_threshold_rules(...)` as S03 honest raw-read keep-sites, not S02 PG-helper-owned builder coverage, so the docs and verifier ownership stay truthful.
patterns_established:
  - Close milestone-assembly slices by composing existing slice verifiers serially and preserving per-phase logs, instead of layering another end-to-end harness on top.
  - Pair public docs pages with exact-string truth sweeps so contract drift fails mechanically during acceptance rather than surfacing later as stale marketing.
  - Keep database capabilities split across three explicit buckets everywhere: portable core (`Expr` / `Query` / `Repo` / `Migration.create_index(...)`), PostgreSQL-only `Pg.*` extras, and a short named raw escape-hatch list.
observability_surfaces:
  - `.tmp/m033-s05/verify/01-docs-build.log` through `.tmp/m033-s05/verify/05-verify-m033-s04.log`
  - `scripts/verify-m033-s05.sh` phase labels (`docs-build`, `docs-truth`, `s02`, `s03`, `s04`) that localize the first failing step
drill_down_paths:
  - .gsd/milestones/M033/slices/S05/tasks/T01-SUMMARY.md
  - .gsd/milestones/M033/slices/S05/tasks/T02-SUMMARY.md
duration: ""
verification_result: passed
completed_at: 2026-03-26T06:11:17.093Z
blocker_discovered: false
---

# S05: Public docs and integrated Mesher acceptance

**Published a Mesher-backed public database guide and a canonical `bash scripts/verify-m033-s05.sh` replay that re-proves the assembled M033 Postgres path while locking the neutral-vs-PG-vs-raw contract in public docs.**

## What Happened

S05 closed M033 by turning the shipped data-layer boundary into both a public contract and a mechanically replayable acceptance surface. T01 rewrote `website/docs/docs/databases/index.md` from the older generic database brochure into a contract-first guide anchored in the real Mesh/Mesher APIs and files that ship today. The page now teaches the neutral core with the actual M033 entrypoints (`Expr.label`, `Expr.value`, `Expr.column`, `Expr.null`, `Expr.case_when`, `Expr.coalesce`, `Query.where_expr`, `Query.select_exprs`, `Repo.insert_expr`, `Repo.update_where_expr`, `Repo.insert_or_update_expr`, and honest `Migration.create_index(...)`), marks JSONB/search/crypto/partition/schema behavior as explicit PostgreSQL-only `Pg.*` work, names `Repo.query_raw`, `Repo.execute_raw`, and `Migration.execute` as the remaining escape hatches, and states clearly that SQLite-specific extras are later work rather than something runtime-proven here.

T02 then added `scripts/verify-m033-s05.sh` as the canonical assembled acceptance command. The wrapper runs the docs build first, performs an exact-string Python truth sweep over `website/docs/docs/databases/index.md`, and then replays `bash scripts/verify-m033-s02.sh`, `bash scripts/verify-m033-s03.sh`, and `bash scripts/verify-m033-s04.sh` strictly serially while preserving named phase logs under `.tmp/m033-s05/verify/`. Making that wrapper truthful also required a small verifier repair pass: the shared `fn_block(...)` logic in the S02/S03/S04 verifier scripts now stops at private `fn` declarations as well as `pub fn`, and the stale ownership assertions that treated `get_event_alert_rules(...)` / `get_threshold_rules(...)` as S02 builder coverage were realigned so those functions stay on the deliberate S03 raw-read keep-list. The result is one public docs page and one canonical command that both describe and re-prove the final M033 boundary instead of letting the docs and proof surface drift apart.

## Verification

Ran `npm --prefix website run build` from the repo root and it passed. Ran `bash scripts/verify-m033-s05.sh`; it passed and replayed the phases in order: docs build, docs-truth sweep, S02 verifier, S03 verifier, and S04 verifier. Confirmed the observability surface by verifying `.tmp/m033-s05/verify/01-docs-build.log` through `.tmp/m033-s05/verify/05-verify-m033-s04.log` exist after the successful run. Updated requirement tracking so R038 is now validated by the assembled replay, while R040 remains active but now explicitly notes the public docs truth gate that enforces the portable-core vs explicit-`Pg.*` vs SQLite-later seam.

## Requirements Advanced

- R040 — S05 made the portable-core vs explicit `Pg.*` vs SQLite-later seam public and mechanically enforced it through `website/docs/docs/databases/index.md` plus the docs-truth gate inside `scripts/verify-m033-s05.sh`.

## Requirements Validated

- R038 — `npm --prefix website run build` and `bash scripts/verify-m033-s05.sh` both passed, and the S05 wrapper serially replays the docs-truth sweep plus `bash scripts/verify-m033-s02.sh`, `bash scripts/verify-m033-s03.sh`, and `bash scripts/verify-m033-s04.sh` with named logs under `.tmp/m033-s05/verify/`.

## New Requirements Surfaced

None.

## Requirements Invalidated or Re-scoped

None.

## Deviations

Expanded slightly beyond the written plan to repair stale `fn_block(...)` parsing and raw-boundary ownership assertions inside `scripts/verify-m033-s02.sh`, `scripts/verify-m033-s03.sh`, and `scripts/verify-m033-s04.sh` so the new S05 wrapper matched the real current proof surface. No slice replan was needed.

## Known Limitations

SQLite-specific extras remain future work and are only documented as a later seam here; S05 does not runtime-prove them. Mesher still keeps a short named raw SQL/DDL keep-list by design. Also, `npm --prefix website run build` must not run concurrently with `bash scripts/verify-m033-s05.sh` because both VitePress builds share `.vitepress/.temp` and can produce false `ERR_MODULE_NOT_FOUND` failures.

## Follow-ups

Use `bash scripts/verify-m033-s05.sh` as the milestone-closeout and future drift-check replay for the M033 docs/proof surface. If the public database contract, raw keep-list, or S02/S03/S04 ownership boundaries change later, update the docs-truth sweep and the delegated verifier assertions in the same task instead of papering over drift with new exemptions.

## Files Created/Modified

- `website/docs/docs/databases/index.md` — Rewrote the public database guide around the real Mesh/Mesher boundary: portable core APIs, explicit PostgreSQL-only `Pg.*` helpers, named raw escape hatches, proof commands, and the SQLite-later seam.
- `scripts/verify-m033-s05.sh` — Added the canonical S05 acceptance wrapper that runs the docs build, an exact-string docs-truth sweep, and the S02/S03/S04 verifiers serially while preserving named phase logs under `.tmp/m033-s05/verify/`.
- `scripts/verify-m033-s02.sh` — Fixed the shared function-block extraction boundary to stop at private `fn` declarations and removed stale ownership assumptions that no longer matched the assembled raw-boundary ledger.
- `scripts/verify-m033-s03.sh` — Fixed the shared function-block extraction boundary so S03 raw-boundary checks only inspect the intended public function bodies.
- `scripts/verify-m033-s04.sh` — Fixed the shared function-block extraction boundary so S04 schema/runtime boundary checks do not swallow following private helpers into earlier public function bodies.
- `.gsd/REQUIREMENTS.md` — Marked R038 validated by the integrated S05 replay and noted that R040 is further advanced by the public docs truth gate while still awaiting later vendor-extra runtime proof.
- `.gsd/KNOWLEDGE.md` — Recorded the VitePress concurrency race, the verifier function-block parsing gotcha, and the S02-vs-S03 raw-boundary ownership note for future agents.
- `.gsd/DECISIONS.md` — Added D069 so the ownership of `get_event_alert_rules(...)` and `get_threshold_rules(...)` stays aligned with the S03 honest raw-read keep-list.
- `.gsd/PROJECT.md` — Refreshed project state to show S05 complete and M033 slice-complete pending milestone closeout / roadmap reassessment.
