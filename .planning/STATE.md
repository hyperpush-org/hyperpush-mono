---
gsd_state_version: 1.0
milestone: v14.0
milestone_name: Ecosystem & Standard Library
status: planning
last_updated: "2026-02-28T00:00:00.000Z"
progress:
  total_phases: 0
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-28)

**Core value:** Expressive, readable concurrency -- writing concurrent programs should feel as natural and clean as writing sequential code, with the safety net of supervision and fault tolerance built into the language.
**Current focus:** v14.0 Ecosystem & Standard Library

## Current Position

Phase: Not started (defining requirements)
Plan: —
Status: Research complete — ready to define requirements
Last activity: 2026-02-28 — Research complete (4 agents + synthesizer); SUMMARY.md committed at 54d5a3ad

## Performance Metrics

**All-time Totals (through v12.0):**
- Plans completed: 343
- Phases completed: 125
- Milestones shipped: 22 (v1.0-v12.0)

**v13.0 plan (11 plans across 6 phases):**

| Phase | Plans | Status |
|-------|-------|--------|
| 126. Multi-line Pipe | 2 | Complete (2/2) |
| 127. Type Aliases | 3 | Complete (3/3) |
| 128. TryFrom/TryInto | 2 | Complete (2/2) |
| 129. Map.collect + Quality | 2 | Complete (2/2) |
| 130. Mesher Dogfooding | 1 | Complete (1/1) |
| 131. Documentation | 2 | Complete (2/2) |

**v13.0 Execution Metrics:**

| Phase | Plan | Duration | Tasks | Files |
|-------|------|----------|-------|-------|
| 126 | P01 | 4m 7s | 2 | 8 |
| 126 | P02 | 3m | 2 | 3 |
| 127 | P01 | 18m | 2 | 9 |
| 127 | P02 | 12m | 2 | 5 |
| 127 | P03 | 20m | 1 | 3 |
| 128 | P01 | 3m | 2 | 2 |
| 128 | P02 | 22m | 2 | 9 |
| 129 | P01 | 11m | 2 | 3 |
| 129 | P02 | 20m | 2 | 2 |
| 130 | P01 | 7m | 2 | 5 |
| 131 | P01 | 2m | 2 | 2 |
| 131 | P02 | 1m 2s | 2 | 1 |
| 132 | P01 | 2m | 2 | 3 |
| 132 | P02 | 9m 22s | 2 | 13 |
| 132 | P03 | 10m 25s | 3 | 10 |
| 133 | P01 | 92s | 2 | 2 |
| 133 | P02 | 3min | 2 | 3 |
| 134 | P01 | 79s | 2 | 3 |
| 134 | P02 | 71s | 2 | 2 |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [v13.0 Roadmap]: Phase 127 (Type Aliases) listed as independent of 126 — can run in parallel if desired
- [v13.0 Roadmap]: Phase 128 (TryFrom) depends on Phase 127 — type aliases may appear in TryFrom signatures
- [v13.0 Roadmap]: Phase 129 groups Map.collect fix (MAPCOL-01) with code quality (QUAL-01, QUAL-02) — small independent fixes bundled together
- [v13.0 Roadmap]: Phase 130 (Dogfooding) deferred until all compiler phases complete — prevents rework
- [v13.0 Roadmap]: Phase 131 (Docs) after dogfooding — examples sourced from verified Mesher patterns
- [Phase 126]: Made is_newline_insignificant pub(crate) rather than adding a new method — minimal change
- [Phase 126]: Named regression test e2e_pipe_126_regression (not e2e_pipe_regression_single_line) because e2e_pipe already exists
- [Phase 127-01]: ALIAS-04 validation skips generic aliases (type Pair<A,B>=...) since type vars aren't in registry
- [Phase 127-01]: target_type_name() returns None for complex types — only validates simple single-IDENT alias targets to avoid false positives
- [Phase 127]: Used single-file fallback form for E2E pub type alias test since compile_and_run writes one main.mpl file
- [Phase 127]: Made TypeRegistry::register_alias pub to allow pre-registration from infer_with_imports
- [Phase 127]: Added DOT to collect_annotation_tokens and IDENT.DOT.IDENT joining in parse_type_tokens to support qualified type annotations like Types.UserId
- [Phase 127]: Register imported aliases under qualified name (Types.UserId) as well as short name (UserId) during infer_with_imports pre-registration
- [Phase 127]: Use fn main() wrapper in cross-module fixtures — all compile_multifile_and_run tests require a main function
- [Phase 128-01]: No built-in TryFrom impls added — TryFrom is user-defined only (unlike From which ships Int->Float/String)
- [Phase 128-01]: TryInto return_type set to None in synthetic impl — actual Result<T,E> resolved per-impl from user body
- [Phase 128-02]: Struct boxing threshold changed from >8 to always-box — ptr slot in {i8,ptr} variant layout is always dereferenced, even 8-byte structs must be heap-allocated
- [Phase 128-02]: TryInto return type now mirrors TryFrom return type at synthesis time so type-checker accepts .try_into() calls
- [Phase 128-02]: impl method return type now uses resolve_type_annotation (handles generic types) over resolve_type_name (simple ident only)
- [Phase 129]: Fixed Map.collect string key dispatch for Iter.zip: extended pipe_chain_has_string_keys with Iter.zip detection (rhs_is_iter_zip + pipe_source_has_string_list) instead of result-type check, because HM let-generalization prevents K=String unification at collect-pipe time
- [Phase 129-02]: Passthrough middleware (next(request) body) requires :: Request annotation — without it, type variable gets generalized as forall T; codegen emits {} (empty struct) LLVM type → SIGBUS at runtime. Handler inference works when body uses Request.* accessors (constrains type before generalization).
- [Phase 130-01]: FromImportDecl handler in infer.rs didn't check mod_exports.type_aliases — importing a pub type alias by name caused E0034. Fixed by adding type_aliases check branch in the import name lookup chain.
- [Phase 130-01]: WS close callback unannotated code/reason parameters caused LLVM {} type mismatch (same root cause as Phase 129-02 passthrough middleware). Fixed with :: Int and :: String annotations.
- [Phase 131]: Type Aliases section placed after Generics and before Structs to match conceptual progression
- [Phase 131]: TryFrom/TryInto section placed immediately after From/Into as natural fallible extension
- [Phase 131]: Language Basics Multi-Line Pipes placed as H3 under Pipe Operator, Type Aliases as H2 before What's Next; cheatsheet entries added inline in existing blocks
- [Phase 132-02]: Two-level json lowering: lower_json_expr_inner returns Ptr (raw object), lower_json_expr wraps with mesh_json_encode for String output
- [Phase 132-02]: Json-typed variable nesting uses mesh_json_parse_raw to decode String back to raw *mut MeshJson pointer (avoids double-encoding)
- [Phase 132-02]: nil in json { } fields emits mesh_json_null() — detected via MirType::Unit check in lower_json_expr_inner
- [Phase 132-02]: mesh_json_parse_raw added to mesh-rt (extern C, no_mangle); panics on invalid JSON since codegen-produced strings are always valid
- [Phase 132-03]: Skip 'type' as json { } key: Mesh reserved keyword (TYPE_KW token), not IDENT; parser requires bare IDENT for json literal keys
- [Phase 132-03]: Skip pre-encoded JSONB field values in json { }: exception/stacktrace/condition_json etc. would double-encode as JSON strings
- [Phase 133]: Placed #regex-literals and #atoms before #strings in grammar patterns array so they take priority; pipe pattern updated to \|[0-9]*> covering both |> and |N>
- [Phase 134]: json { } is the idiomatic way to return JSON from HTTP handlers, replacing escaped string literals in all examples
- [Phase 134]: JSON Object Literals subsection placed first under ## JSON in web docs to front-load the preferred pattern
- [Phase 134]: json { } documented in strings sub-skill (not new sub-skill) — JSON literals are a string/serialization concern
- [Phase 134]: Heredoc Strings section gets explicit note pointing to json { } — prevents AI recommending heredoc for JSON
- [Phase 134]: Type serialization table included with all 7 Mesh types — AI needs precise mapping to generate correct json { } code

### Roadmap Evolution

- Phase 132 added: Improve language JSON handling with native object literal syntax instead of manual string concatenation
- Phase 133 added: Ensure the vscode extension is updated with changes from milestones 10, 11, 12, 13
- Phase 134 added: Add phase 132's change to the documentation site and update the appropriate skill(s) in tools/skill/mesh/skills/

### Pending Todos

- [ ] Complete `/gsd:new-milestone` — run in fresh context. Research done. Next: define requirements (step 9) and create roadmap (step 10).
  - Research is at `.planning/research/SUMMARY.md`
  - 6 suggested phases from SUMMARY.md: Encoding+Crypto / DateTime / HTTP Client / Testing Framework / Package Manifest+CLI / Registry Backend+Website
  - Scope confirmed: Full suite — stdlib (crypto/date/encoding), HTTP client (streaming/keep-alive/builder), Testing (runner/assert/mock), Package registry (CLI + hosted site)
  - Coverage reporting: defer to v14.1 (per research recommendation)

### Blockers/Concerns

None. v12.0 fully shipped. v13.0 roadmap created with 100% requirement coverage (17/17 mapped).

## Session Continuity

Last session: 2026-02-28
Stopped at: Completed 134-02-PLAN.md — HTTP skill SKILL.md and web/index.md updated with json { } as idiomatic JSON response pattern. Phase 134 complete (2/2 plans). All phases complete.
Resume file: None
