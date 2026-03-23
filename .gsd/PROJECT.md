# Project

## What This Is

Mesh is a programming language and application platform repository focused on becoming a production-trustworthy general-purpose language with a lean toward server/backend code. It already contains the compiler, runtime, standard library, formatter, LSP, REPL, package tooling, registry, docs site, benchmarks, and dogfooded applications. The current priority is not to keep adding features blindly; it is to make Mesh trustworthy for real backend use.

## Core Value

Mesh should be something you can trust for a real production app backend in any capacity, starting with an honest API + DB + migrations + background jobs path that feels as easy to deploy as a Go binary.

## Current State

The repository already ships a broad backend-oriented language platform:
- Rust workspace crates for lexing, parsing, type checking, code generation, runtime, formatter, LSP, REPL, package resolution, and CLI tooling
- native LLVM code generation to standalone binaries
- runtime support for actors, supervision, HTTP, WebSocket, JSON, database access, migrations, files, env, crypto, datetime, and collections
- package and registry infrastructure plus a docs/website surface
- dogfooded backend applications and benchmarks inside the repo

The current gap is not feature count. The current gap is trust. M028/S01-S04 now close the first four concrete proof gaps: the repo contains a working `reference-backend/` package that builds, starts, migrates, serves HTTP, persists jobs in Postgres, and lets a timer-driven worker move a durable row from `pending` to `processed`; the compiler-facing Rust harness proves migration truth, HTTP/DB/health job-state agreement, and two-instance exact-once processing without benign claim contention inflating worker failure signals; the same backend now has trustworthy formatter, test-runner, LSP, and doc/editor proof surfaces instead of toy-only tooling claims; and the package now ships an artifact-first boring deployment path with a staged native binary, checked-in deploy SQL, thin runtime scripts, staged-bundle smoke proof outside the repo root, and operator-facing docs for build-host versus runtime-host responsibilities. The remaining gaps are the follow-on trust layers: supervision/recovery hardening and final docs/examples promotion.

## Architecture / Key Patterns

- Rust workspace under `compiler/` with distinct crates for lexer, parser, type checker, codegen, runtime, formatter, LSP, REPL, package tooling, and CLI
- native-binary compilation via LLVM rather than a VM runtime requirement
- runtime centered on actors, supervision, HTTP, WebSocket, DB, migrations, and other backend primitives
- dogfooding through repo-local applications such as `mesher/` and benchmark fixtures
- proof-first rule for this phase: if a baseline/backend trust gap is exposed, fix Mesh at the source and prove it through a real backend workflow rather than a toy-only demo

## Capability Contract

See `.gsd/REQUIREMENTS.md` for the explicit capability contract, requirement status, and coverage mapping.

## Milestone Sequence

- [x] M028: Language Baseline Audit & Hardening — define the production-ready backend baseline and prove the canonical backend path honestly
- [ ] M029: Backend Ergonomics — improve the language/runtime/DX where real backend pressure exposes friction
- [ ] M030: Tooling & Package Trust — make fmt/LSP/tests/coverage/dependency flow credible for daily backend work
- [ ] M031: Production Backend Maturity — extend proof to long-running services, realtime, and distributed backends credibly
