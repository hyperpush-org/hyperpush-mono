# Project

## What This Is

Mesh is a programming language and backend application platform repository aimed at being trustworthy for real backend and distributed-systems work, not just toy examples. The repo contains the compiler, runtime, formatter, LSP, REPL, package tooling, docs site, package registry, packages website, landing site, and dogfood applications used to pressure-test the language.

M048 is complete. The repo now ships a default-plus-override executable-entry contract across compiler build/test, `mesh-lsp`, editor hosts, and `meshpkg publish`; installer-backed `meshc update` / `meshpkg update`; truthful editor grammar and init-time Mesh skill guidance for `@cluster` plus both interpolation forms; and one retained closeout rail (`bash scripts/verify-m048-s05.sh`) plus bounded public docs that keep first-contact tooling claims honest.

M049 is now complete. The repo no longer teaches `todo-api` onboarding through repo-root proof apps: `meshc init --template todo-api --db postgres <name>` emits the migration-first shared/deployable starter, `meshc init --template todo-api --db sqlite <name>` emits the explicit local single-node starter, generator-owned `examples/todo-postgres` and `examples/todo-sqlite` mirror scaffold output mechanically, repo-root `tiny-cluster/` and `cluster-proof/` have been retired in favor of fixture-backed retained proofs under `scripts/fixtures/clustered/`, and one assembled closeout rail (`bash scripts/verify-m049-s05.sh`) now replays the dual-db scaffold, example parity, public-surface retirement, and retained M039/M045/M047/M048 guardrails into a single retained bundle.

The current wave has closed the evaluator-facing public-docs reset and the reference-app retirement wave: M050 and M051 are complete. Mesher is now the maintained deeper reference app, the repo-root `reference-backend/` compatibility tree is gone, retained backend-only proof lives under `scripts/fixtures/backend/reference-backend/`, and `bash scripts/verify-m051-s05.sh` is the terminal post-deletion acceptance rail. Later milestones still align landing/packages/deploy/load-balancing surfaces with that same truthful story.

M049/S01-S05 are complete. `meshc init --template todo-api --db postgres <name>` emits the migration-first shared/deployable starter from S01, `meshc init --template todo-api --db sqlite <name>` emits the explicit local single-node starter from S02, and S03 checks in generator-owned `/examples/todo-postgres` and `/examples/todo-sqlite` as public `meshc init` output instead of hand-maintained showcase trees. S04 retired the repo-root `tiny-cluster/` and `cluster-proof/` onboarding surfaces entirely, moved those proofs under `scripts/fixtures/clustered/`, repointed retained Rust and shell rails to shared fixture helpers, and made the public clustered story scaffold/examples-first while keeping the lower-level proof fixtures available only through retained verifier/docs surfaces. S05 closes that reset with one named assembled verifier: `bash scripts/verify-m049-s05.sh` replays the dual-db scaffold rails, direct `/examples` parity, retained M039/M045/M047 clustered proof wrappers, and the retained M048 tooling rail, then publishes one retained bundle under `.tmp/m049-s05/verify/retained-proof-bundle/`. The SQLite path still passes generator, tooling, and live runtime rails; `/health` reports local SQLite state without clustered metadata; restart persistence is proven against the retained `todo.sqlite3`; the old clustered SQLite Todo contract now lives behind the committed M047 fixture plus retained verification bundles instead of behind the public scaffold; and the assembled S05 bundle now retains fresh `m049-s01`, `m049-s02`, and `m049-s03` artifact buckets alongside the delegated historical verify trees.

M050 is complete. S01 reset the docs graph so `Getting Started` and `Clustered Example` are the only primary first-contact entries, S02 rewrote README / Getting Started / Clustered Example / Tooling around the explicit clustered-vs-SQLite-vs-Postgres starter chooser and the assembled first-contact docs rail, and S03 rebalanced `Distributed Actors`, `Distributed Proof`, and `Production Backend Proof` into honest public-secondary surfaces. The public docs path is now backed by `bash scripts/verify-m050-s01.sh`, `bash scripts/verify-m050-s02.sh`, `bash scripts/verify-m050-s03.sh`, and the assembled `bash scripts/verify-m049-s05.sh` preflights.

M051 is complete. S01 refit Mesher onto the current bootstrap/runtime contract with a maintainer runbook and live Postgres rail, S02 extracted backend-only proof into retained fixtures plus `bash scripts/verify-m051-s02.sh`, S03 moved tooling/editor/LSP/formatter rails onto the bounded retained backend fixture, S04 retargeted public docs, scaffold output, and clustering skills to the examples-first story while keeping Mesher maintainer-facing, and S05 deleted the repo-root `reference-backend/` tree while closing the post-deletion acceptance rail with `bash scripts/verify-m051-s05.sh`.

## Core Value

If Mesh claims it can cluster, route work, survive node loss, and report truthful runtime status, those claims must be proven through small docs-grade examples where the language/runtime owns the magic instead of the example app reimplementing distributed behavior — including the syntax users actually write.

The public Mesh story should stay honest: Mesh is a general-purpose language, but its strongest proof surface and clearest value are fault-tolerant distributed systems and backend workloads.

## Current State

Mesh already ships a broad backend-oriented stack:
- Rust workspace crates under `compiler/` for lexing, parsing, type checking, code generation, runtime, formatter, LSP, REPL, package tooling, and CLI commands
- native compilation to standalone binaries
- runtime support for actors, supervision, HTTP, WebSocket, JSON, database access, migrations, files, env, crypto, datetime, and collections
- a distributed runtime surface with node start/connect/list/monitor, remote spawn/send, continuity, authority, and clustered-app tooling
- dogfooded applications: `mesher/` as the maintained deeper reference app, retained backend-only proof under `scripts/fixtures/backend/reference-backend/`, and retained clustered proof fixtures under `scripts/fixtures/clustered/tiny-cluster` plus `scripts/fixtures/clustered/cluster-proof`
- a real package registry service in `registry/`, a public packages website in `packages-website/`, a docs site in `website/`, and a separate landing site in `mesher/landing/`
- editor surfaces including the VS Code extension and repo-owned Neovim pack

Recent distributed-runtime state:
- M039 proved automatic cluster formation, truthful membership, runtime-native internal balancing, and single-cluster degrade/rejoin on a narrow proof app
- M042 moved single-cluster keyed continuity into `mesh-rt` behind a Mesh-facing `Continuity` API
- M043 proved cross-cluster primary/standby continuity, bounded promotion, and packaged same-image failover/operator rails
- M044 productized clustered apps: manifest opt-in, runtime-owned declared-handler execution, built-in read-only operator/CLI surfaces, `meshc init --clustered`, bounded automatic promotion/recovery, and a rewritten `cluster-proof` on the public clustered-app contract
- M045 simplified the clustered example story around runtime-owned bootstrap, runtime-chosen remote execution, automatic failover, and scaffold-first docs
- M046 closed the route-free clustered proof wave: `meshc init --clustered` plus the internal `scripts/fixtures/clustered/tiny-cluster` and `scripts/fixtures/clustered/cluster-proof` proofs now share one tiny `1 + 1` clustered-work contract, and the authoritative closeout rail is `bash scripts/verify-m046-s06.sh`
- M047 completed the public cutover to source-first `@cluster`, carried replication counts through runtime truth, shipped `HTTP.clustered(...)`, and updated the Todo scaffold, docs, and closeout rails around that shipped route wrapper

Public docs and repo teaching surfaces are still uneven in ways later milestones need to fix, even though M048-M050 have now made the first-contact and public-secondary docs story truthful:
- the default-plus-override `[package].entrypoint` contract now spans compiler build, test discovery, LSP, editor hosts, and `meshpkg publish`; first-contact docs also point at the retained `bash scripts/verify-m048-s05.sh` closeout rail and keep `main.mpl` as the simple default while documenting override entries such as `lib/start.mpl`
- the reset wave now includes explicit installer-backed `meshc update` / `meshpkg update` commands, bounded VS Code same-file-definition wording, manifest-first editor proof, and retained parity rails for `@cluster`, both interpolation forms, and clustered-runtime teaching truth: `bash scripts/verify-m036-s01.sh`, `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh syntax`, `node --test scripts/tests/verify-m036-s02-contract.test.mjs`, `node --test scripts/tests/verify-m048-s04-skill-contract.test.mjs`, and the assembled `bash scripts/verify-m048-s05.sh`
- M050 and M051 are complete: the public docs graph, first-contact pages, secondary proof surfaces, retained backend fixture, and post-deletion closeout rail now all route readers through scaffold/examples-first material while keeping Mesher maintainer-facing and the retired backend proof behind retained verifiers (`bash scripts/verify-m050-s01.sh`, `bash scripts/verify-m050-s02.sh`, `bash scripts/verify-m050-s03.sh`, `bash scripts/verify-m051-s02.sh`, and `bash scripts/verify-m051-s05.sh`)
- the landing site still reflects stale product positioning rather than Mesh's actual language story

## Architecture / Key Patterns

- Rust workspace under `compiler/` with separate crates for parser, type checker, codegen, runtime, formatter, LSP, CLI, REPL, package tooling, and package manager code
- backend-first proof surfaces through narrow reference apps and shell verifiers, not marketing-only examples
- proof-first dogfooding: reproduce a real runtime/platform limitation, fix it at the correct layer, then prove the repaired path end to end
- explicit honesty boundaries when behavior is genuinely environment-specific; avoid claiming portability or automation that the runtime does not really own
- assembled closeout verifiers own a fresh `.tmp/<slice>/verify` bundle and retain delegated subrails by copying their verify trees plus bundle pointers, rather than sharing or mutating lower-level `.tmp/.../verify` directories directly
- current clustered runtime surface lives primarily in `compiler/mesh-rt/src/dist/`, `compiler/mesh-codegen/`, `compiler/mesh-typeck/`, and `compiler/meshc/`, with user-facing docs in `website/docs/docs/distributed/` and scaffold generation in `compiler/mesh-pkg/src/scaffold.rs`
- clustered HTTP routes now reuse the same declared-handler seam as ordinary clustered work: compiler lowering rewrites `HTTP.clustered(...)` to deterministic `__declared_route_<runtime_name>` bare shims, router registration reverse-maps those shims onto declared-handler runtime metadata, and continuity/operator views stay keyed by the real handler runtime name rather than the shim symbol
- for the next wave, public evaluator-facing surfaces should stay simpler than internal proof rails: scaffold/examples first, Mesher as the deeper real app, and repo verifier detail kept out of the primary docs story

## Capability Contract

See `.gsd/REQUIREMENTS.md` for the explicit capability contract, requirement status, and coverage mapping.

## Milestone Sequence

- [x] M028: Language Baseline Audit & Hardening — prove the first honest API + DB + migrations + jobs backend path
- [x] M029: Mesher & Reference-Backend Dogfood Completion — fix formatter corruption and complete the dogfood cleanup wave
- [x] M031: Language DX Audit & Rough Edge Fixes — retire real dogfood rough edges through compiler and parser fixes
- [x] M032: Mesher Limitation Truth & Mesh Dogfood Retirement — audit workaround folklore, fix real blockers in Mesh, and dogfood those repairs back into `mesher/`
- [x] M033: ORM Expressiveness & Schema Extras — strengthen the neutral data layer, add PG-first extras now, and leave a clean path for SQLite extras later
- [x] M034: Delivery Truth & Public Release Confidence — harden CI/CD, prove the package manager end to end, and make the public release path trustworthy instead of artifact-only
- [x] M036: Editor Parity & Multi-Editor Support — make editor support match real Mesh syntax and give at least one non-VSCode editor a first-class path
- [x] M038: Fix Windows MSVC Build — repair the hosted Windows release lane so the shipped compiler path is trustworthy
- [x] M039: Auto-Discovery & Native Cluster Balancing — prove discovery, truthful membership, runtime-native internal balancing, and single-cluster failure/rejoin on a narrow proof app
- [x] M042: Runtime-Native Distributed Continuity Core — move single-cluster distribution, replication, and keyed continuity into `mesh-rt` behind a simple Mesh-facing API
- [x] M043: Runtime-Native Cross-Cluster Disaster Continuity — extend the same runtime-owned continuity model across primary/standby clusters
- [x] M044: First-Class Clustered Apps & Bounded Auto-Promotion — turn runtime continuity/failover into the default productized clustered-app model for ordinary Mesh services
- [x] M045: Language-Owned Clustered Example Simplification — make the primary clustered example tiny, docs-grade, and fully language/runtime-owned instead of proof-app-shaped
- [x] M046: Language-Owned Tiny Cluster Proofs — make clustered work auto-triggered, decorator-declarable, route-free, and equally proven through `meshc init --clustered`, `tiny-cluster/`, and rebuilt `cluster-proof/`
- [x] M047: Cluster Declaration Reset & Clustered Route Ergonomics — replace `clustered(work)` with `@cluster`, reset canonical examples/scaffolds to ordinary `@cluster` function names, continue the clustered-route wrapper work honestly, and ship a clear SQLite Todo scaffold with a complete Dockerfile that makes clustering obvious without looking like a proof app
- [x] M048: Entrypoint Flexibility & Tooling Truth Reset — make entrypoints configurable, add toolchain self-update, and align editors plus init-time skills with the current language/runtime contract
- [x] M049: Scaffold & Example Reset — support SQLite-local and Postgres-clustered scaffolds, generate checked-in examples, and replace proof-app-shaped public teaching surfaces
- [x] M050: Public Docs Truth Reset — make docs evaluator-facing, remove proof-maze public material, and re-test commands and code samples one by one
- [x] M051: Mesher as the Living Reference App — complete; `reference-backend/` is retired, Mesher is the maintained deeper reference app, and `bash scripts/verify-m051-s05.sh` is the post-deletion acceptance rail
- [ ] M053: Deploy Truth for Scaffolds & Packages Surface — prove the Postgres starter and packages surfaces through CI and real deployment evidence while keeping the public contract platform-agnostic
- [ ] M054: Load Balancing Deep Dive & Runtime Follow-through — explain current balancing honestly and implement follow-through if the existing server-side story is not sufficient
- [ ] M035: Test Framework Hardening — get Mesh's testing story ready to test `mesher` thoroughly during development
- [ ] M037: Package Experience & Ecosystem Polish — improve the package manager experience, website-first, once the underlying trust path is proven
