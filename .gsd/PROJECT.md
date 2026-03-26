# Project

## What This Is

Mesh is a programming language and backend application platform repository aimed at being trustworthy for real backend work, not just toy examples. The repo contains the compiler, runtime, formatter, LSP, REPL, package tooling, docs site, and two dogfood applications: `reference-backend/` as the narrow proof surface and `mesher/` as the broader pressure test.

## Core Value

Dogfood friction should turn into honest platform improvements: when Mesh or its data layer hits a real backend limitation, the repo should fix that limitation in Mesh and then use the repaired path in the app instead of carrying permanent folklore workarounds.

## Current State

Mesh already ships a broad backend-oriented stack:
- Rust workspace crates under `compiler/` for lexing, parsing, type checking, code generation, runtime, formatter, LSP, REPL, package tooling, and CLI commands
- native compilation to standalone binaries
- runtime support for actors, supervision, HTTP, WebSocket, JSON, database access, migrations, files, env, crypto, datetime, and collections
- dogfooded applications: `reference-backend/` and `mesher/`

Recent milestone state:
- M028 established the production-backend trust baseline around API + DB + migrations + jobs
- M029 completed the major formatter correctness and dogfood cleanup wave across `mesher/` and `reference-backend/`
- M031 fixed several real DX/compiler rough edges found through dogfooding and expanded the regression suite
- M032/S02 retired the unconstrained inferred-export blocker by threading concrete call-site signatures into MIR lowering, replaying `xmod_identity` as a success path, and dogfooding the repaired export via `mesher/storage/writer.mpl`
- M032/S03 retired the stale request/handler/control-flow folklore in the audited `mesher/` modules by dogfooding direct `Request.query(...)`, inline service-call `case`, and inline cast-handler `if/else`, while preserving the real route-closure, nested-`&&`, and timer keep-sites
- M032/S04 retired the stale module-boundary `from_json` folklore in Mesher's event ingestion/storage comments, kept the real PostgreSQL JSONB/ORM keep-sites explicit, and revalidated the supported cross-module `from_json` path plus Mesher fmt/build on the cleaned codebase
- M032/S05 replayed the full Mesher proof matrix, closed the supported-now versus retained-limit ledger, and left a short file-backed keep-list for the remaining Mesh and data-layer pressure sites
- M032/S06 backfilled the missing S01 acceptance artifact, reran the live S01 proof bundle with non-zero test-count guards, and closed the last milestone evidence gap so M032 now seals cleanly
- M032 is now fully closed through `.gsd/milestones/M032/M032-SUMMARY.md`, which records the compiler repair, the Mesher dogfood cleanup, and the three-bucket handoff into M033 (supported-now proof, still-real Mesh keep-sites, and real data-layer follow-on work)
- M033/S01 is now complete: Mesh ships the neutral expression builder and expression-aware Query/Repo select/update/upsert surface, Mesher’s S01-owned write paths run on that core, the live ingest/rate-limit/writer blockers are retired, and `bash scripts/verify-m033-s01.sh` closes green against the Postgres-backed acceptance suite
- M033/S02 is now complete: Mesh ships explicit PG helper usage on the live Mesher auth/search/JSONB/alert paths, the S02 proof bundle (`compiler/meshc/tests/e2e_m033_s02.rs`) passes against live Postgres, and `bash scripts/verify-m033-s02.sh` now enforces the owned keep-list plus the named S03 `extract_event_fields` raw boundary.
- M033/S03 is now complete: Mesher's read-side query surface now covers the honest scalar/count/join/list/aggregation families plus the slice-owned hard read proofs through `compiler/meshc/tests/e2e_m033_s03.rs`, while `bash scripts/verify-m033-s03.sh` enforces the short named raw-read keep-list instead of allowing read-side SQL drift to hide again.
- M033/S04 is now complete: Mesh ships honest neutral index-name/order migration support plus explicit `Pg` schema helpers for extensions, partitioned parents, GIN/opclass indexes, and runtime partition lifecycle; Mesher's initial migration, startup partition bootstrap, and retention cleanup now use those helpers, and `compiler/meshc/tests/e2e_m033_s04.rs` plus `bash scripts/verify-m033-s04.sh` prove the result against live Postgres catalogs and startup logs.
- M033/S05 is now complete: the public Mesh database docs now teach the real neutral `Expr` / `Query` / `Repo` / `Migration.create_index(...)` core, explicit PostgreSQL-only `Pg.*` extras, the short named raw escape-hatch boundary, and the SQLite-later seam; `bash scripts/verify-m033-s05.sh` replays the docs build, docs-truth sweep, and the S02/S03/S04 live-Postgres verifiers serially with named logs under `.tmp/m033-s05/verify/`.

M033 is now slice-complete and ready for milestone closeout / roadmap reassessment.

## Architecture / Key Patterns

- Rust workspace under `compiler/` with separate crates for parser, type checker, codegen, runtime, formatter, LSP, CLI, REPL, and package tooling
- backend-first proof surfaces through `reference-backend/` and `mesher/`
- Mesh data access built around `Repo`, `Query`, and `Migration` runtime surfaces
- proof-first dogfooding: reproduce a real app limitation, fix Mesh at the source, then dogfood the repaired path back into the app
- keep the default surface boring and composable; use database-specific extras explicitly when the underlying behavior is genuinely vendor-specific

## Capability Contract

See `.gsd/REQUIREMENTS.md` for the explicit capability contract, requirement status, and coverage mapping.

## Milestone Sequence

- [x] M028: Language Baseline Audit & Hardening — prove the first honest API + DB + migrations + jobs backend path
- [x] M029: Mesher & Reference-Backend Dogfood Completion — fix formatter corruption and complete the dogfood cleanup wave
- [ ] M030: Tooling & Package Trust — make package, dependency, and daily-driver tooling flow credible for backend work
- [x] M031: Language DX Audit & Rough Edge Fixes — retire real dogfood rough edges through compiler and parser fixes
- [x] M032: Mesher Limitation Truth & Mesh Dogfood Retirement — audit workaround folklore, fix real blockers in Mesh, and dogfood those repairs back into `mesher/`
- [ ] M033: ORM Expressiveness & Schema Extras — strengthen the neutral data layer, add PG-first extras now, and leave a clean path for SQLite extras later
