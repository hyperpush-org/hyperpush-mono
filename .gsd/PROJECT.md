# Project

## What This Is

Mesh is a programming language and backend application platform repository aimed at being trustworthy for real backend work, not just toy examples. The repo contains the compiler, runtime, formatter, LSP, REPL, package tooling, docs site, package registry, package website, and two dogfood applications: `reference-backend/` as the narrow proof surface and `mesher/` as the broader pressure test.

## Core Value

Dogfood friction should turn into honest platform improvements: when Mesh or its surrounding delivery/tooling/editor/package surfaces hit a real limitation, the repo should fix that limitation at the source and then prove the repaired path end to end instead of carrying permanent folklore or artifact-only confidence.

## Current State

Mesh already ships a broad backend-oriented stack:
- Rust workspace crates under `compiler/` for lexing, parsing, type checking, code generation, runtime, formatter, LSP, REPL, package tooling, and CLI commands
- native compilation to standalone binaries
- runtime support for actors, supervision, HTTP, WebSocket, JSON, database access, migrations, files, env, crypto, datetime, and collections
- dogfooded applications: `reference-backend/` and `mesher/`
- a real package registry service in `registry/`, a public packages website in `packages-website/`, and a VS Code extension in `tools/editors/vscode-mesh/`

Recent milestone state:
- M028 established the production-backend trust baseline around API + DB + migrations + jobs
- M029 completed the major formatter correctness and dogfood cleanup wave across `mesher/` and `reference-backend/`
- M031 fixed several real DX/compiler rough edges found through dogfooding and expanded the regression suite
- M032 retired stale Mesher limitation folklore, fixed real blockers in Mesh, and dogfooded those repairs back into `mesher/`
- M033 strengthened the neutral ORM/migration core, added explicit PostgreSQL extras, and left a clean path for later SQLite-specific work
- M034/S01 now proves the real registry publish/install path end to end: scoped installed packages resolve from their natural cache layout, `scripts/verify-m034-s01.sh` publishes and installs a real scoped package through the live registry path, and the publish/download/named-install/docs contracts are now mechanically checked instead of assumed.

The remaining near-term trust gap is no longer the backend core or the basic real-registry package path. It is wiring that truth into CI/release assembly, hardening installer and extension release lanes, and continuing the planned test-framework and editor-support maturation work.

## Architecture / Key Patterns

- Rust workspace under `compiler/` with separate crates for parser, type checker, codegen, runtime, formatter, LSP, CLI, REPL, package tooling, and CLI-facing package manager code
- backend-first proof surfaces through `reference-backend/` and `mesher/`
- release/deploy/package surfaces split across GitHub Actions, `registry/`, `packages-website/`, `website/`, install scripts, and `tools/editors/vscode-mesh/`
- proof-first dogfooding: reproduce a real app or delivery limitation, fix it at the right layer, then dogfood the repaired path back into the repo’s real workflows
- keep the default surface boring and composable; use explicit boundary markers when behavior is genuinely vendor-, editor-, or deployment-specific

## Capability Contract

See `.gsd/REQUIREMENTS.md` for the explicit capability contract, requirement status, and coverage mapping.

## Milestone Sequence

- [x] M028: Language Baseline Audit & Hardening — prove the first honest API + DB + migrations + jobs backend path
- [x] M029: Mesher & Reference-Backend Dogfood Completion — fix formatter corruption and complete the dogfood cleanup wave
- [x] M031: Language DX Audit & Rough Edge Fixes — retire real dogfood rough edges through compiler and parser fixes
- [x] M032: Mesher Limitation Truth & Mesh Dogfood Retirement — audit workaround folklore, fix real blockers in Mesh, and dogfood those repairs back into `mesher/`
- [x] M033: ORM Expressiveness & Schema Extras — strengthen the neutral data layer, add PG-first extras now, and leave a clean path for SQLite extras later
- [ ] M034: Delivery Truth & Public Release Confidence — harden CI/CD, prove the package manager end to end, and make the public release path trustworthy instead of artifact-only
- [ ] M035: Test Framework Hardening — get Mesh’s testing story ready to test `mesher` thoroughly during development
- [ ] M036: Editor Parity & Multi-Editor Support — make editor support match real Mesh syntax and give at least one non-VSCode editor a first-class path
- [ ] M037: Package Experience & Ecosystem Polish — improve the package manager experience, website-first, once the underlying trust path is proven
