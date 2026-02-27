---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: Language Ergonomics & Open Source Readiness
status: unknown
last_updated: "2026-02-27T17:29:31.620Z"
progress:
  total_phases: 131
  completed_phases: 131
  total_plans: 340
  completed_plans: 340
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-25)

**Core value:** Expressive, readable concurrency -- writing concurrent programs should feel as natural and clean as writing sequential code, with the safety net of supervision and fault tolerance built into the language.
**Current focus:** Phase 116 — Slot Pipe Operator (first phase of v12.0)

## Current Position

Phase: 123 of 123 (Phase 123: Performance Benchmarks — COMPLETE)
Plan: 03 of 03 COMPLETE (all 5 tasks done)
Status: Phase 123 fully complete — RESULTS.md, METHODOLOGY.md, throughput.svg, README.md Performance section committed. Machines destroyed.
Last activity: 2026-02-26 - Completed quick task 6: write an article about the benchmarks in MD format

Progress: [█░░░░░░░░░] 5% (v12.0)

## Performance Metrics

**All-time Totals:**
- Plans completed: 320
- Phases completed: 115+
- Milestones shipped: 21 (v1.0-v11.0)
- Lines of Rust: ~168,500
- Lines of website: ~5,500
- Lines of Mesh: ~7,700
- Timeline: 20 days (2026-02-05 -> 2026-02-25)

| Phase | Plan | Duration | Tasks | Files |
|-------|------|----------|-------|-------|
| 113   | 01   | 5min     | 2     | 1     |
| 114   | 01   | 30min    | 2     | 2     |
| 114   | 02   | 15min    | 1     | 1     |
| 115   | 01   | 3min     | 2     | 3     |
| 115   | 02   | 3min     | 2     | 2     |
| 116   | 01   | 4min     | 2     | 7     |
| 116   | 02   | 8min     | 2     | 6     |
| 117   | 01   | 8min     | 2     | 3     |
| 117   | 02   | 8min     | 2     | 4     |
| 118   | 01   | 10min    | 2     | 6     |
| 118   | 02   | 13min    | 2     | 6     |
| 119   | 01   | 6min     | 2     | 8     |
| 119   | 02   | 10min    | 2     | 8     |
| 119   | 03   | 13min    | 2     | 10    |
| 120   | 01   | 4min     | 2     | 6     |
| Phase 120 P02 | 70min | 2 tasks | 3 files |
| Phase 121 P01 | 3min | 3 tasks | 3 files |
| Phase 121 P02 | 2min | 3 tasks | 3 files |
| Phase 121 P03 | 2min | 3 tasks | 3 files |
| Phase 121 P04 | 2min | 3 tasks | 3 files |
| Phase 122 P01 | 3min | 2 tasks | 15 files |
| Phase 122 P02 | 15min | 3 tasks | 1 files |
| Phase 123 P02 | 2min | 2 tasks | 5 files |
| Phase 123 P01 | 5min | 2 tasks | 3 files |
| Phase 123 P03 | 3 | 3 tasks | 5 files |
| Phase 124 P01 | 2min | 2 tasks | 1 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [Phase 115]: Documentation-only gap closure: Phase 106/109 implementations were correct, only tracking records were missing
- [Phase 115]: Phase 109 positional API (insert_or_update, delete_where_returning, where_sub) accepted as canonical v11.0 API
- [Phase 115]: get_project_id_by_key and get_user_orgs removed from queries.mpl (zero import sites)
- [v12.0 Roadmap]: Phase 119 (Regex) depends only on Phase 115 -- may run in parallel with 117-118 if desired
- [v12.0 Roadmap]: Phase 121 (Agent Skill) depends only on Phase 115 -- no code changes, can run at any point
- [v12.0 Roadmap]: PIPE-05 and STRG-06 bundled into Phase 120 (Mesher Dogfooding) after all compiler work done
- [v12.0 Roadmap]: REPO (Phase 122) scheduled after Mesher dogfooding -- disruptive restructure deferred until language features stable
- [v12.0 Roadmap]: BENCH (Phase 123) scheduled last -- depends on repo being stable for benchmark code commit location
- [Phase 116-01]: |0> and |1> emit TokenKind::Error at lex time (hard error by design, not recoverable parse error)
- [Phase 116-01]: SlotPipe uses same Pratt binding power (3, 4) as Pipe -- chain with equal precedence
- [Phase 116-01]: todo!() placeholders added to mesh-typeck and mesh-codegen to unblock builds until Plan 02
- [Phase 116-02]: Slot pipe uses insertion semantics — x |2> f(a,b,c) = f(a,x,b,c); conflict check removed, arity unification handles mismatches
- [Phase 116-02]: SlotPositionConflict error variant exists in enum but not emitted in normal insertion; SlotPipeOutOfRange emitted when slot > known arity
- [Phase 117]: Both ${ and #{ emit identical InterpolationStart tokens — parser/codegen require zero changes, only lexer updated
- [Phase 117]: apply_heredoc_content() processes each STRING_CONTENT segment independently to handle interpolation boundaries; mid-line content after #{} is left untouched
- [Phase 117]: into_token() used instead of as_token() in iterator chains to avoid Rust E0515 borrow errors
- [Phase 118]: Old bare env_get (Option-returning) removed entirely from builtins.rs; env_get now routes to 2-arg mesh_env_get_with_default
- [Phase 118]: env_get_int silently returns default on any parse failure (non-numeric, overflow) — no stderr warning required
- [Phase 118]: env_args type signature upgraded to Ty::list(Ty::string()) in builtins.rs
- [Phase 118]: stdlib_modules() Env entry in infer.rs updated to 2-arg get, get_int, args signatures (was stale 1-arg Option-returning)
- [Phase 118]: get_env_or_default helper removed from mesher/main.mpl; all callsites use direct Env.get(key, default)
- [Phase 119]: RegexExpr.pattern()/flags() parse CST source text (not TokenKind payload) since SyntaxToken only stores text spans
- [Phase 119]: Flags bitmask i=1, m=2, s=4; only i/m/s valid -- other letters produce lexer Error token
- [Phase 119]: mesh_regex_from_literal call site wired in Plan 01; runtime symbol added in Plan 02
- [Phase 119]: Bool return for mesh_regex_match uses i8 (matches mesh_string_contains convention)
- [Phase 119]: No bare regex 'replace'/'split' in map_builtin_name (would conflict with string variants); module-qualified regex_replace/regex_split unambiguous
- [Phase 119]: Regex.is_match used instead of Regex.match: 'match' is a Mesh keyword causing parse errors
- [Phase 119]: Ty::Con(Regex) maps to MirType::Ptr in types.rs resolve_con: opaque heap pointer, prevents LLVM opaque struct failures
- [Phase 119]: Regex added to STDLIB_MODULE_NAMES in infer.rs: required for Regex.compile/is_match/etc to route through module call path
- [Phase 120]: Slot pipe applied only in fingerprint.mpl where genuine argument threading exists; other API files already use idiomatic patterns
- [Phase 120]: Heredocs used for JSON builders with 2+ embedded double quotes; simple error strings use regular #{} without heredoc
- [Phase 120]: HTTP test assertions fixed: unescape_string() in MIR lowerer correctly processes backslash escapes; test comments claiming otherwise were stale since Phase 117
- [Phase 121]: Skill lives at skill/mesh/ inside snow repo with SKILL.md format (frontmatter + numbered rules + code examples from tests/e2e/)
- [Phase 121-02]: Sub-skills use cross-references via 'See also: skills/<name>' pattern for related concept linking
- [Phase 121]: Actors sub-skill includes self() self-messaging pattern for actor loops (from tce_actor_loop.mpl)
- [Phase 121]: Supervisors sub-skill cross-references actors sub-skill — supervisors manage actors
- [Phase 121]: Collections sub-skill: module-qualified (List.map) preferred over global bare forms; Iter pipeline lazy evaluation documented as key performance distinction
- [Phase 121]: strings sub-skill covers #{} and ${} interpolation (both syntaxes), heredocs, 11 String functions, Env.get/get_int, and full Regex API with literals and runtime compile
- [Phase 121]: http sub-skill documents router rebind convention (let r = HTTP.use(r, ...)) and covers routing/middleware/client/WebSocket/crash isolation
- [Phase 121]: database sub-skill covers Sqlite/PostgreSQL raw APIs, deriving(Row) ORM, upserts/RETURNING/subqueries, JOINs, and gotchas (execute vs query for RETURNING)
- [Phase 122]: Root package.json deleted (not moved to website/) because website/ already has its own real package.json; root was dev-only concurrently wrapper
- [Phase 122]: crates/ flattened directly to compiler/ with no intermediate directory — all 11 crates are direct children of compiler/
- [Phase 122]: Docker/Mesher E2E test skipped in 122-02 — container crashed; user approved and confirmed E2E works
- [Phase 123]: Port assignments: Mesh=3000, Go=3001, Rust=3002, Elixir=3003 — consistent across all benchmark plans
- [Phase 123]: Port 3000 for Mesh benchmark server, port 3001 for Go benchmark server (hardcoded, wrk runner script uses these)
- [Phase 123]: Go benchmark uses GOMAXPROCS(NumCPU()) at startup for fair multi-core CPU utilization comparison
- [Phase 123]: Mesh compiler built from compiler/ Rust workspace in Dockerfile (mesher/ contains only macOS arm64 binary)
- [Phase 123]: Internal DNS hostname (bench-servers.vm.bench-mesh.internal) recommended over raw IPv6 to avoid bracket notation issues
- [Phase 123]: RSS logged as CSV to stdout (RSS,lang,epoch,kB) every 2s; extracted via fly logs | grep '^RSS,'
- [Phase 124]: projects.slug ON CONFLICT requires WHERE slug IS NOT NULL predicate to match partial index (bare ON CONFLICT (slug) causes PostgreSQL runtime error)
- [Phase 124]: Seed migration uses Pool.execute for INSERT/DELETE and Repo.query_raw for SELECT, matching existing schema migration pattern

### Roadmap Evolution

- v12.0 roadmap created 2026-02-25: 8 phases (116-123), 33 requirements mapped, 100% coverage
- Phase ordering: compiler features first (116-119), then dogfooding (120), then skill (121), then repo (122), then benchmarks (123)
- Phase 124 added: Fix POST /api/v1/events 401 seed data issue

### Pending Todos

None.

### Blockers/Concerns

None. v11.0 fully shipped and verified. Zero known compiler correctness issues.

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 6 | write an article about the benchmarks in MD format | 2026-02-26 | a125e3f0 | [6-write-an-article-about-the-benchmarks-in](./quick/6-write-an-article-about-the-benchmarks-in/) |
| 7 | get isolated peak throughput numbers by running each server alone | 2026-02-26 | 30cc9dc8 | [7-get-isolated-peak-throughput-numbers-by-](./quick/7-get-isolated-peak-throughput-numbers-by-/) |

## Session Continuity

Last session: 2026-02-27
Stopped at: Isolated benchmark COMPLETE. All results in hand. Need to fill RESULTS.md and ARTICLE.md, commit, close quick-7, destroy fly machines, then plan Phase 124.
Resume file: None

ISOLATED BENCHMARK RESULTS (machine 48e693ec054208, now complete):

/text endpoint (runs 2–5 avg):
  Mesh:   29,108 req/s  p50=2.77ms  p99=16.94ms
  Go:     30,306 req/s  p50=2.95ms  p99=8.51ms
  Rust:   46,244 req/s  p50=2.06ms  p99=4.55ms
  Elixir: 12,441 req/s  p50=6.74ms  p99=25.14ms

/json endpoint (runs 2–5 avg):
  Mesh:   28,955 req/s  p50=2.84ms  p99=16.19ms
  Go:     29,934 req/s  p50=2.97ms  p99=8.40ms
  Rust:   46,234 req/s  p50=2.08ms  p99=4.77ms
  Elixir: 12,733 req/s  p50=7.15ms  p99=23.41ms

Run 1 (excluded) values:
  Mesh /text: 28,681  Mesh /json: 28,562
  Go /text:   30,270  Go /json:   30,690
  Rust /text: 45,584  Rust /json: 46,672
  Elixir /text: 12,583  Elixir /json: 13,391

Peak RSS: N/A (fly logs --no-tail too fast to catch RSS lines)

Co-located vs Isolated deltas:
  Mesh /text:   19,718 → 29,108  (+47%)
  Mesh /json:   20,483 → 28,955  (+41%)
  Go /text:     26,278 → 30,306  (+15%)
  Go /json:     26,175 → 29,934  (+14%)
  Rust /text:   27,133 → 46,244  (+70%)
  Rust /json:   28,563 → 46,234  (+62%)
  Elixir /text: 11,842 → 12,441  (+5%)
  Elixir /json: 11,481 → 12,733  (+11%)

Next action: /clear then fill RESULTS.md isolated tables + update ARTICLE.md + commit + destroy fly machines + close quick-7 + plan Phase 124
