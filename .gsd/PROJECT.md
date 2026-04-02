# Project

## What This Is

Mesh is a programming language and backend application platform repository aimed at being trustworthy for real backend and distributed-systems work, not just toy examples. The repo contains the compiler, runtime, formatter, LSP, REPL, package tooling, docs site, package registry, packages website, landing site, and dogfood applications used to pressure-test the language.

M047 is complete. The repo now has a truthful clustered-route wrapper implementation plus native and Docker proof that the Todo starter's selected read routes execute through explicit-count clustered route wrappers end to end. M048 is active: S01 closed the compiler/build/test entrypoint seam, S02 extended the same default-plus-override contract through `mesh-lsp`, `meshc lsp`, the repo-owned Neovim and VS Code hosts, and `meshpkg publish`, S03 added installer-backed `meshc update` / `meshpkg update` commands plus a retained staged-release acceptance rail, and S04 reset editor syntax plus init-time Mesh skills to the current `@cluster` / interpolation / clustered-runtime truth. S05 remains to assemble the milestone proof and reconcile the minimal public touchpoints.

The current wave is a public-surface reset aimed at new evaluators: configurable entrypoints, toolchain self-update, editor and init-skill parity, a cleaner scaffold/example story, evaluator-facing docs instead of a proof-maze, Mesher as the deeper real reference app, and deploy truth that matches what the site claims.

## Core Value

If Mesh claims it can cluster, route work, survive node loss, and report truthful runtime status, those claims must be proven through small docs-grade examples where the language/runtime owns the magic instead of the example app reimplementing distributed behavior — including the syntax users actually write.

The public Mesh story should stay honest: Mesh is a general-purpose language, but its strongest proof surface and clearest value are fault-tolerant distributed systems and backend workloads.

## Current State

Mesh already ships a broad backend-oriented stack:
- Rust workspace crates under `compiler/` for lexing, parsing, type checking, code generation, runtime, formatter, LSP, REPL, package tooling, and CLI commands
- native compilation to standalone binaries
- runtime support for actors, supervision, HTTP, WebSocket, JSON, database access, migrations, files, env, crypto, datetime, and collections
- a distributed runtime surface with node start/connect/list/monitor, remote spawn/send, continuity, authority, and clustered-app tooling
- dogfooded applications: `reference-backend/` as the narrow backend proof surface, `mesher/` as the broader pressure test, `tiny-cluster/` as the local route-free clustered proof, and `cluster-proof/` as the packaged route-free clustered proof
- a real package registry service in `registry/`, a public packages website in `packages-website/`, a docs site in `website/`, and a separate landing site in `mesher/landing/`
- editor surfaces including the VS Code extension and repo-owned Neovim pack

Recent distributed-runtime state:
- M039 proved automatic cluster formation, truthful membership, runtime-native internal balancing, and single-cluster degrade/rejoin on a narrow proof app
- M042 moved single-cluster keyed continuity into `mesh-rt` behind a Mesh-facing `Continuity` API
- M043 proved cross-cluster primary/standby continuity, bounded promotion, and packaged same-image failover/operator rails
- M044 productized clustered apps: manifest opt-in, runtime-owned declared-handler execution, built-in read-only operator/CLI surfaces, `meshc init --clustered`, bounded automatic promotion/recovery, and a rewritten `cluster-proof` on the public clustered-app contract
- M045 simplified the clustered example story around runtime-owned bootstrap, runtime-chosen remote execution, automatic failover, and scaffold-first docs
- M046 closed the route-free clustered proof wave: `tiny-cluster/`, rebuilt `cluster-proof/`, and `meshc init --clustered` now share one tiny `1 + 1` clustered-work contract, and the authoritative closeout rail is `bash scripts/verify-m046-s06.sh`
- M047 completed the public cutover to source-first `@cluster`, carried replication counts through runtime truth, shipped `HTTP.clustered(...)`, and updated the Todo scaffold, docs, and closeout rails around that shipped route wrapper

Public docs and repo teaching surfaces are still uneven in ways the next wave needs to fix:
- the default-plus-override `[package].entrypoint` contract now spans compiler build, test discovery, LSP, editor hosts, and `meshpkg publish`, and the reset wave now also includes explicit installer-backed `meshc update` / `meshpkg update` commands proven against a retained staged release server rail in `compiler/meshc/tests/e2e_m048_s03.rs`
- editor syntax and init-time Mesh skills now have retained parity rails for `@cluster`, both interpolation forms, and clustered-runtime teaching truth: `bash scripts/verify-m036-s01.sh`, `NEOVIM_BIN="${NEOVIM_BIN:-nvim}" bash scripts/verify-m036-s02.sh syntax`, `node --test scripts/tests/verify-m036-s02-contract.test.mjs`, and `node --test scripts/tests/verify-m048-s04-skill-contract.test.mjs`
- public docs still expose proof-heavy surfaces centered on `reference-backend/`, `tiny-cluster/`, and `cluster-proof/`
- the landing site still reflects stale product positioning rather than Mesh's actual language story
- `reference-backend/` still exists even though the next wave is expected to retire it in favor of `mesher/`

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
- [ ] M048: Entrypoint Flexibility & Tooling Truth Reset — make entrypoints configurable, add toolchain self-update, and align editors plus init-time skills with the current language/runtime contract
- [ ] M049: Scaffold & Example Reset — support SQLite or Postgres scaffolds, generate checked-in examples, and replace proof-app-shaped public teaching surfaces
- [ ] M050: Public Docs Truth Reset — make docs evaluator-facing, remove proof-maze public material, and re-test commands and code samples one by one
- [ ] M051: Mesher as the Living Reference App — retire `reference-backend/`, keep `mesher/` healthy, and modernize it as the deeper real reference app
- [ ] M052: Public Website & Packages Surface Reset — align landing, docs, and packages surfaces into one coherent public Mesh story
- [ ] M053: Deploy Truth for Scaffolds & Packages Surface — prove the public scaffold and packages surfaces through CI and real Fly-backed deployment behavior
- [ ] M054: Load Balancing Deep Dive & Runtime Follow-through — explain current balancing honestly and implement follow-through if the existing server-side story is not sufficient
- [ ] M035: Test Framework Hardening — get Mesh's testing story ready to test `mesher` thoroughly during development
- [ ] M037: Package Experience & Ecosystem Polish — improve the package manager experience, website-first, once the underlying trust path is proven
