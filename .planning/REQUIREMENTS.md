# Requirements: Mesh

**Defined:** 2026-02-17
**Core Value:** Expressive, readable concurrency -- writing concurrent programs should feel as natural and clean as writing sequential code, with the safety net of supervision and fault tolerance built into the language.

## v11.0 Requirements

Requirements for Query Builder milestone. Expand ORM with comprehensive query capabilities and rewrite Mesher to eliminate all raw SQL.

### JOINs

- [ ] **JOIN-01**: Query builder supports inner join with on-clause expression
- [ ] **JOIN-02**: Query builder supports left join with on-clause expression
- [ ] **JOIN-03**: Query builder supports multiple joins in a single query
- [ ] **JOIN-04**: JOIN results include columns from all joined tables

### Aggregations

- [ ] **AGG-01**: Query builder supports count() aggregation
- [ ] **AGG-02**: Query builder supports sum()/avg()/min()/max() aggregations
- [ ] **AGG-03**: Query builder supports group_by clause
- [ ] **AGG-04**: Query builder supports having clause with conditions

### Advanced WHERE

- [ ] **WHERE-01**: Query builder supports comparison operators (>, <, >=, <=, !=)
- [ ] **WHERE-02**: Query builder supports IN and NOT IN with value lists
- [ ] **WHERE-03**: Query builder supports IS NULL and IS NOT NULL
- [ ] **WHERE-04**: Query builder supports BETWEEN for range checks
- [ ] **WHERE-05**: Query builder supports LIKE and ILIKE for pattern matching
- [ ] **WHERE-06**: Query builder supports OR conditions and grouped conditions

### Upsert, RETURNING, Subqueries

- [ ] **UPS-01**: Repo supports upsert (INSERT ON CONFLICT DO UPDATE) with conflict target
- [ ] **UPS-02**: Repo insert/update/delete support RETURNING clause
- [ ] **UPS-03**: Query builder supports subqueries in WHERE clause

### Raw SQL Fragments

- [ ] **FRAG-01**: Query.fragment() embeds raw SQL with parameter binding in queries
- [ ] **FRAG-02**: Fragments work in WHERE, SELECT, ORDER BY, and GROUP BY positions
- [ ] **FRAG-03**: Fragments support PG functions (crypt, gen_random_bytes, date_trunc, random)
- [ ] **FRAG-04**: Fragments support JSONB operators and full-text search expressions

### Mesher Rewrite

- [ ] **REWR-01**: User/session/API-key queries rewritten with ORM (8 queries)
- [ ] **REWR-02**: Issue management queries rewritten with ORM + upserts (10 queries)
- [ ] **REWR-03**: Search/filtering queries rewritten with ORM + fragments (4 queries)
- [ ] **REWR-04**: Dashboard/analytics queries rewritten with ORM aggregations (7 queries)
- [ ] **REWR-05**: Alert system queries rewritten with ORM + fragments (12 queries)
- [ ] **REWR-06**: Retention/storage queries rewritten with ORM (6 queries)
- [ ] **REWR-07**: Event writer/extraction rewritten with ORM + fragments (2 queries)
- [ ] **REWR-08**: Zero Repo.query_raw/execute_raw in Mesher data queries

### Verification

- [ ] **VER-01**: Mesher compiles with zero errors
- [ ] **VER-02**: All HTTP API endpoints return correct responses
- [ ] **VER-03**: WebSocket endpoints function correctly

## v10.0/v10.1 Requirements (Complete)

All v10.0 ORM requirements shipped (50 requirements, phases 96-103). v10.1 stabilization fixed codegen ABI issues (phases 104-105.1).

## Future Requirements

### Query Builder Extensions

- **QEXT-01**: Right/full outer join support
- **QEXT-02**: Window functions (ROW_NUMBER, RANK, etc.)
- **QEXT-03**: Common table expressions (WITH / CTE)
- **QEXT-04**: UNION/INTERSECT/EXCEPT set operations

## Out of Scope

| Feature | Reason |
|---------|--------|
| DDL/partition management | Schema operations stay as raw SQL -- not data queries |
| System catalog queries (pg_inherits) | PostgreSQL internals, not application data |
| New Mesher features | Rewrite only -- no new functionality |
| Multi-database support | PostgreSQL-only for now |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| WHERE-01 | Phase 106 | Pending |
| WHERE-02 | Phase 106 | Pending |
| WHERE-03 | Phase 106 | Pending |
| WHERE-04 | Phase 106 | Pending |
| WHERE-05 | Phase 106 | Pending |
| WHERE-06 | Phase 106 | Pending |
| FRAG-01 | Phase 106 | Pending |
| FRAG-02 | Phase 106 | Pending |
| FRAG-03 | Phase 106 | Pending |
| FRAG-04 | Phase 106 | Pending |
| JOIN-01 | Phase 107 | Pending |
| JOIN-02 | Phase 107 | Pending |
| JOIN-03 | Phase 107 | Pending |
| JOIN-04 | Phase 107 | Pending |
| AGG-01 | Phase 108 | Pending |
| AGG-02 | Phase 108 | Pending |
| AGG-03 | Phase 108 | Pending |
| AGG-04 | Phase 108 | Pending |
| UPS-01 | Phase 109 | Pending |
| UPS-02 | Phase 109 | Pending |
| UPS-03 | Phase 109 | Pending |
| REWR-01 | Phase 110 | Pending |
| REWR-02 | Phase 111 | Pending |
| REWR-07 | Phase 111 | Pending |
| REWR-03 | Phase 112 | Pending |
| REWR-04 | Phase 112 | Pending |
| REWR-05 | Phase 112 | Pending |
| REWR-06 | Phase 113 | Pending |
| REWR-08 | Phase 113 | Pending |
| VER-01 | Phase 114 | Pending |
| VER-02 | Phase 114 | Pending |
| VER-03 | Phase 114 | Pending |

**Coverage:**
- v11.0 requirements: 32 total
- Mapped to phases: 32
- Unmapped: 0

---
*Requirements defined: 2026-02-17*
*Last updated: 2026-02-17 after roadmap creation (phases 106-114)*
