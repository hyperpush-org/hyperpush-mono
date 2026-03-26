# S05: Public docs and integrated Mesher acceptance — UAT

**Milestone:** M033
**Written:** 2026-03-26T06:11:17.125Z

## UAT Type

- UAT mode: mixed
- Why this mode is sufficient: S05 is a docs-plus-final-assembly slice, so the correct acceptance surface is a public docs build/truth check paired with the real live-Postgres S02/S03/S04 replay.

## Preconditions

- Run from the repo root with dependencies installed.
- PostgreSQL is available on `localhost:5432` for the delegated S02/S03/S04 verifiers.
- No concurrent VitePress build is running; execute the docs build and the S05 wrapper serially.
- `.tmp/m033-s05/verify/` may be absent or stale before the run; the wrapper should repopulate it.

## Smoke Test

1. Run `bash scripts/verify-m033-s05.sh`.
2. Confirm the phases print in order: `[docs-build]`, `[docs-truth]`, `[s02]`, `[s03]`, `[s04]`.
3. **Expected:** the command exits 0 and ends with `verify-m033-s05: ok`.

## Test Cases

### 1. Public database docs teach the shipped boundary

1. Open `website/docs/docs/databases/index.md`.
2. Confirm the page names the portable APIs `Expr.label`, `Expr.value`, `Expr.column`, `Expr.null`, `Expr.case_when`, `Expr.coalesce`, `Query.where_expr`, `Query.select_exprs`, `Repo.insert_expr`, `Repo.update_where_expr`, `Repo.insert_or_update_expr`, and `Migration.create_index(...)`.
3. Confirm the page marks PostgreSQL-only behavior under `Pg.*`, explicitly names `Repo.query_raw`, `Repo.execute_raw`, and `Migration.execute` as escape hatches, and says SQLite-specific extras are later work.
4. Confirm the page includes the canonical command `bash scripts/verify-m033-s05.sh`.
5. **Expected:** the public docs describe the real neutral-vs-PG-vs-raw contract and point readers at the canonical S05 replay.

### 2. Docs build passes in isolation

1. Run `npm --prefix website run build`.
2. **Expected:** the VitePress build completes successfully with no frontmatter or page-generation errors.

### 3. Canonical assembled acceptance replay stays green

1. Run `bash scripts/verify-m033-s05.sh`.
2. Wait for the wrapper to run the docs build, docs-truth sweep, then `bash scripts/verify-m033-s02.sh`, `bash scripts/verify-m033-s03.sh`, and `bash scripts/verify-m033-s04.sh` serially.
3. **Expected:** the wrapper exits 0, prints `verify-m033-s05: ok`, and does not parallelize the Postgres-backed phases.

### 4. Failure visibility artifacts are preserved

1. After a successful S05 run, inspect `.tmp/m033-s05/verify/`.
2. Confirm the directory contains `01-docs-build.log`, `02-docs-truth.log`, `03-verify-m033-s02.log`, `04-verify-m033-s03.log`, and `05-verify-m033-s04.log`.
3. Open one docs-phase log and one delegated-verifier log.
4. **Expected:** each phase has its own named log, so the first failing step would be obvious if the replay regressed.

## Edge Cases

### Concurrent VitePress builds

1. If `npm --prefix website run build` fails with `ERR_MODULE_NOT_FOUND` for generated `.md.js` files after overlapping docs runs, stop the overlapping build and rerun the required commands serially.
2. **Expected:** a serial rerun of `npm --prefix website run build` and then `bash scripts/verify-m033-s05.sh` passes, confirming the earlier failure was a shared `.vitepress/.temp` race rather than real docs drift.

### Honest leftover boundary

1. Re-read the escape-hatch section of `website/docs/docs/databases/index.md`.
2. Confirm the page presents a short named raw keep-list and does not claim zero raw SQL/DDL coverage.
3. **Expected:** the docs remain honest about current limits instead of marketing the portable surface as universal.

## Failure Signals

- `npm --prefix website run build` exits non-zero or reports frontmatter/VitePress render errors.
- `bash scripts/verify-m033-s05.sh` stops before `verify-m033-s05: ok`.
- `.tmp/m033-s05/verify/` is missing any named phase log after a run.
- The docs page no longer names the required portable APIs, `Pg.*` boundary, raw escape hatches, SQLite-later seam, or canonical S05 command.

## Requirements Proved By This UAT

- R038 — the public docs plus canonical serial replay prove that Mesher now uses stronger ORM/migration surfaces where honest coverage exists, while retaining only a short named raw SQL/DDL keep-list and explicit escape hatches.
- R040 — partially advanced: the docs truth gate enforces the portable-core vs explicit `Pg.*` vs SQLite-later seam, but does not runtime-prove SQLite extras.

## Not Proven By This UAT

- SQLite-specific runtime behavior or SQLite-only extras.
- Elimination of every raw SQL/DDL site; the slice explicitly preserves a short honest keep-list.

## Notes for Tester

Run the docs build and the S05 wrapper serially, not in parallel. If either the public contract or the owned raw-boundary ledger changes later, update `website/docs/docs/databases/index.md` and `scripts/verify-m033-s05.sh` together so the truth sweep stays aligned with the real proof surface.
