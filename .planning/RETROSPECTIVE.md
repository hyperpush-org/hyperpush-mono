# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

---

## Milestone: v11.0 — Query Builder

**Shipped:** 2026-02-25
**Phases:** 11 (including 109.1) | **Plans:** 22 | **Sessions:** ~8 days

### What Was Built
- Advanced ORM query builder: NOT IN, BETWEEN, ILIKE, OR grouped conditions, raw SQL fragments with $N renumbering across all query positions (WHERE/SELECT/ORDER BY/GROUP BY)
- JOIN support (inner, left, aliased, multi-table) and full aggregation suite (count/sum/avg/min/max, group_by, having) — all with runtime SQLite E2E verification
- Upsert (INSERT ON CONFLICT DO UPDATE), RETURNING clause, subquery WHERE IN, plus critical compiler fixes: type checker arity bug and service loop type dispatch for Bool/Float/Struct params
- 49+ Mesher raw SQL queries rewritten to ORM across all 7 domains; 18 intentional ORM boundaries documented with rationale; zero unaccounted raw SQL
- Full E2E verification: Mesher compiled zero errors, all 8 HTTP API domains return 2xx, WebSocket 101, EventProcessor SIGSEGV resolved

### What Worked
- Decimal phase insertion (109.1) cleanly handled two blocking bugs mid-milestone without renumbering subsequent phases — the pattern is unambiguous and low-friction
- Incremental domain-by-domain Mesher rewrite (auth → issues → search/dashboard/alerts → retention) validated the ORM pattern before tackling more complex domains
- Explicit ORM boundary documentation (18 sites with rationale comments) was faster than trying to force-fit every query — honest scope beats over-engineering
- Phase 115 tracking-corrections pattern: a dedicated cleanup phase after verification to formally close audit gaps and canonicalize API style worked well
- Audit-then-complete workflow (two audit runs before archiving) caught 13 tracking gaps that would have silently entered the milestone record

### What Was Inefficient
- Phase 106 requirement tracking was incomplete at execution time — WHERE-01..06 and FRAG-01..04 were not marked in REQUIREMENTS.md during execution, requiring Phase 115 to close the gap retroactively
- The initial audit (gaps_found) required a second audit run after corrections; better to mark requirements during execution so first audit passes
- ROADMAP plan-level checkboxes accumulated cosmetic inconsistencies (some `[ ]` instead of `[x]` for completed plans) — minor but creates noise in audits
- Phase 109 API style (positional args) diverged from the roadmap description (keyword-option style) without an in-phase note; caught in audit, resolved in Phase 115

### Patterns Established
- **ORM boundary documentation pattern**: when a query cannot be expressed via ORM, add a comment explaining the specific SQL feature that prevents ORM use (arithmetic SET expressions, server-side JSONB, nested subqueries, DDL) — creates a clear inventory for future ORM extensions
- **Decimal phase insertion**: `109.1` for urgent mid-milestone bug fixes preserves ordering and avoids renumbering; INSERTED marker in roadmap
- **Three-phase verification sequence**: compile-zero-errors → startup/migration → HTTP+WS smoke test; each is a distinct plan
- **Requirement tracking discipline**: mark `requirements-completed` in SUMMARY frontmatter at execution time, not retroactively — prevents audit gaps

### Key Lessons
1. Mark requirements complete during execution in SUMMARY frontmatter — retroactive tracking (Phase 115) costs an extra phase
2. When an API style diverges from the roadmap spec during implementation, add an inline acceptance note to the roadmap immediately (not at audit time)
3. ORM boundary documentation is a feature, not a gap — explicitly catalogued raw SQL sites are better than hidden ones
4. Runtime SQLite E2E tests are high-value for query builder features: they catch SQL generation bugs that unit tests miss and create a regression baseline

### Cost Observations
- Model mix: quality profile throughout (primarily opus)
- Sessions: ~8 working days
- Notable: plan_check + verifier gates added confidence but extended execution time per phase; the trade-off was worthwhile for a large rewrite milestone

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Phases | Plans | Key Change |
|-----------|--------|-------|------------|
| v1.0 | 10 | 55 | Initial project — full compiler from scratch |
| v8.0 | 6 | 11 | Developer tooling — LSP, install scripts, VS Code |
| v9.0 | 9 | 38 | First large Mesh application (Mesher) — multi-domain |
| v10.0 | 8 | 25 | ORM library — schema DSL, repo pattern, migrations |
| v11.0 | 11 | 22 | ORM query builder + full application rewrite |

### Top Lessons (Cross-Milestone)

1. Runtime E2E tests (SQLite for DB, live HTTP smoke for services) are more valuable than static analysis for verifying query/protocol features
2. Incremental domain-by-domain rewrites (auth → issues → search → ...) are more reliable than big-bang rewrites — each domain validates the pattern before the next
3. Explicit audit + gap-closure phases (115 pattern) are worth the overhead for large milestones with many requirements
