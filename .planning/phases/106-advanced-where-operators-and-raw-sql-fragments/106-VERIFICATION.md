---
phase: 106-advanced-where-operators-and-raw-sql-fragments
verified: 2026-02-17T20:46:11Z
status: passed
score: 13/13 must-haves verified
re_verification: false
---

# Phase 106: Advanced WHERE Operators and Raw SQL Fragments Verification Report

**Phase Goal:** Mesh programs can express rich query conditions -- comparisons, set membership, nullability, ranges, pattern matching, boolean logic -- and embed arbitrary PostgreSQL expressions (crypt, JSONB, FTS) via raw SQL fragments with parameter binding
**Verified:** 2026-02-17T20:46:11Z
**Status:** passed
**Re-verification:** No -- initial verification

## Goal Achievement

### Observable Truths

#### Plan 01: Advanced WHERE Operators

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `Query.where_not_in(q, :status, ["archived", "deleted"])` generates `WHERE status NOT IN ($1, $2)` | VERIFIED | `test_select_with_not_in` asserts exact SQL; `mesh_query_where_not_in` encodes `status NOT_IN:2`; repo.rs `NOT_IN:` branch generates `NOT IN ($1, $2)` |
| 2 | `Query.where_between(q, :age, "18", "65")` generates `WHERE age BETWEEN $1 AND $2` | VERIFIED | `test_select_with_between` asserts exact SQL; `mesh_query_where_between` encodes `age BETWEEN`; repo.rs BETWEEN branch generates `BETWEEN $1 AND $2` |
| 3 | `Query.where_op(q, :name, :ilike, "%alice%")` generates `WHERE name ILIKE $1` | VERIFIED | `"ilike" => "ILIKE"` in `atom_to_sql_op` (query.rs:99); `test_select_with_ilike` asserts exact SQL |
| 4 | `Query.where_or(q, [["status", "active"], ["level", "error"]])` generates `WHERE (status = $1 OR level = $2)` | VERIFIED | `test_select_with_or` asserts exact SQL; `mesh_query_where_or` encodes `OR:status,level:2`; repo.rs `OR:` branch generates `("status" = $1 OR "level" = $2)` |
| 5 | All new WHERE operators work in pipe chains with existing Query builder methods | VERIFIED | `e2e_query_builder_advanced_where_combined` chains `where`, `where_op`, `where_not_in`, `where_between`, `where_or`, `where_null`, `order_by`, `limit` and asserts `"ok\n"` |
| 6 | All parameter indices are correctly sequenced across mixed clause types | VERIFIED | `test_mixed_where_clauses` combines WHERE + NOT_IN:2 + BETWEEN + OR:2 and asserts exact SQL with $1 through $7 in correct order |

#### Plan 02: Fragment Renumbering and Raw ORDER BY/GROUP BY

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 7 | `Query.fragment(q, "crypt($1, gen_salt('bf'))", [password])` embeds SQL with $1 renumbered to correct parameter index | VERIFIED | `test_fragment_dollar_renumbering` asserts $1 becomes $3 after 2 WHERE params; `renumber_placeholders` helper scans char-by-char and adds `start_idx - 1` offset |
| 8 | Fragment $1-style placeholders are renumbered when prior WHERE clauses consume parameters | VERIFIED | Same as above plus `test_fragment_with_jsonb` asserts `metadata @> $2::jsonb` after 1 WHERE param |
| 9 | `Query.order_by_raw(q, "random()")` generates `ORDER BY random()` verbatim | VERIFIED | `test_order_by_raw` asserts exact SQL; ORDER BY builder strips `RAW:` prefix and emits verbatim |
| 10 | `Query.group_by_raw(q, "date_trunc('hour', received_at)")` generates `GROUP BY date_trunc('hour', received_at)` verbatim | VERIFIED | `test_group_by_raw` asserts exact SQL; GROUP BY builder strips `RAW:` prefix and emits verbatim |
| 11 | `Query.where_raw` combined with `Query.fragment` in the same query generates correct parameter numbering | VERIFIED | `test_mixed_fragments_and_where` asserts `WHERE "project_id" = $1 AND received_at > now() ... AND tags @> $2::jsonb` with correct sequencing |
| 12 | Fragments with PG functions (crypt, gen_random_bytes, date_trunc, random) produce syntactically correct SQL | VERIFIED | `test_fragment_with_pg_crypt` asserts `crypt($1, gen_salt('bf'))` passes through unchanged; `e2e_query_builder_fragment_crypt` confirms full compilation |
| 13 | Fragments with JSONB operators (`metadata @> $1::jsonb`, `tags ? $1`) produce correct SQL | VERIFIED | `test_fragment_with_jsonb` asserts `metadata @> $2::jsonb` with correct renumbering; `e2e_query_builder_jsonb_fragment` confirms full compilation |

**Score:** 13/13 truths verified

### Required Artifacts

#### Plan 01 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/mesh-rt/src/db/query.rs` | `mesh_query_where_not_in`, `mesh_query_where_between`, `mesh_query_where_or` extern C functions | VERIFIED | All three `#[no_mangle] pub extern "C"` functions present at lines 242-323; ILIKE added to `atom_to_sql_op` at line 99 |
| `crates/mesh-rt/src/db/repo.rs` | SQL generation for NOT IN, BETWEEN, OR conditions in `build_select_sql_from_parts` | VERIFIED | `NOT_IN:` branch at line 313, `BETWEEN` branch at line 332, `OR:` branch at line 259; all with correct `param_idx` sequencing |
| `crates/meshc/tests/e2e.rs` | E2E compilation tests for all new WHERE operators | VERIFIED | 5 E2E tests at lines 3919-3989: `where_not_in`, `where_between`, `where_ilike`, `where_or`, `advanced_where_combined` |

#### Plan 02 Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/mesh-rt/src/db/query.rs` | `mesh_query_order_by_raw`, `mesh_query_group_by_raw` extern C functions | VERIFIED | Both present at lines 395-482; encode with `RAW:` prefix into respective slots |
| `crates/mesh-rt/src/db/repo.rs` | Fixed $N renumbering in fragment and where_raw SQL builders; RAW: prefix for order_by and group_by | VERIFIED | `renumber_placeholders` helper at line 121; WHERE RAW: handler at line 277; ORDER BY RAW: handler at line 417; GROUP BY RAW: handler at line 378; fragment injection at line 402 |
| `crates/meshc/tests/e2e.rs` | E2E tests for fragments in all positions (WHERE, SELECT, ORDER BY, GROUP BY) | VERIFIED | 7 E2E tests at lines 3995-4093: `order_by_raw`, `group_by_raw`, `select_raw_with_group_by_raw`, `where_raw_dollar`, `fragment_crypt`, `jsonb_fragment`, `fragments_all_positions` |

### Key Link Verification

#### Plan 01 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/mesh-rt/src/db/query.rs` | `crates/mesh-rt/src/db/repo.rs` | WHERE clause encoding format (`NOT_IN:3`, `BETWEEN`, `OR:N`) | VERIFIED | query.rs encodes `"status NOT_IN:2"`, `"age BETWEEN"`, `"OR:status,level:2"`; repo.rs parses all three in `build_select_sql_from_parts` WHERE loop |
| `crates/mesh-codegen/src/mir/lower.rs` | `crates/mesh-rt/src/db/query.rs` | `map_builtin_name` mapping (`query_where_not_in -> mesh_query_where_not_in`) | VERIFIED | lower.rs lines 10508-10510 map all three; `known_functions` entries at lines 867-872 with correct type signatures |

#### Plan 02 Key Links

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/mesh-rt/src/db/repo.rs` (fragment injection) | `crates/mesh-rt/src/db/repo.rs` (where_raw handling) | Shared `renumber_placeholders` for both `?` and `$N` styles | VERIFIED | Single `renumber_placeholders` function called at line 279 (where_raw), line 403 (fragment injection), line 528 (count builder), line 641 (exists builder) |
| `crates/mesh-codegen/src/mir/lower.rs` | `crates/mesh-rt/src/db/query.rs` | `map_builtin_name` for `order_by_raw`, `group_by_raw` | VERIFIED | lower.rs lines 10523-10524; `known_functions` entries at lines 890-893 |

### Requirements Coverage

All 5 success criteria from the ROADMAP are satisfied:
- SC1: Comparison operators via `where_op` (existing + new `ilike`) -- SATISFIED
- SC2: IN, IS NULL, IS NOT NULL, BETWEEN, LIKE, ILIKE generate correct SQL -- SATISFIED
- SC3: OR conditions generate grouped parenthesized SQL -- SATISFIED
- SC4: Fragment `$1` placeholders renumber correctly -- SATISFIED
- SC5: Fragments work in WHERE, SELECT, ORDER BY, GROUP BY -- SATISFIED

### Pipeline Registration Coverage

All 7 required touch points registered for Plan 01 functions (where_not_in, where_between, where_or):
- Runtime function (`mesh_rt/src/db/query.rs`): VERIFIED
- SQL generation (`mesh_rt/src/db/repo.rs`): VERIFIED
- MIR `known_functions` (`mesh-codegen/src/mir/lower.rs` lines 867-872): VERIFIED
- MIR `map_builtin_name` (`mesh-codegen/src/mir/lower.rs` lines 10508-10510): VERIFIED
- LLVM intrinsics (`mesh-codegen/src/codegen/intrinsics.rs` lines 953-966): VERIFIED
- JIT symbols (`mesh-repl/src/jit.rs` lines 278-280): VERIFIED
- lib.rs re-exports (`mesh-rt/src/lib.rs` lines 56-58): VERIFIED
- Typechecker (`mesh-typeck/src/infer.rs` lines 1083-1097): VERIFIED (auto-fixed deviation)

All 7 touch points registered for Plan 02 functions (order_by_raw, group_by_raw):
- Runtime, SQL gen, MIR known_functions, MIR map_builtin_name, LLVM intrinsics, JIT symbols, lib.rs, typechecker: all VERIFIED

### Anti-Patterns Found

None. Scanned `query.rs`, `repo.rs`, and `lower.rs` for TODO/FIXME/placeholder patterns. No stub implementations detected. All extern C functions have substantive implementations. All SQL generation branches produce real SQL with correct parameter indexing.

### Git Commit Verification

All 4 task commits confirmed in git history:
- `305f11ce` feat(106-01): add NOT IN, BETWEEN, ILIKE, OR runtime functions and SQL generation
- `bd00f8ac` feat(106-01): register advanced WHERE operators in MIR/codegen/JIT/typeck with E2E tests
- `90d67808` feat(106-02): fix $N renumbering and add ORDER BY/GROUP BY raw support
- `c7986a56` feat(106-02): register raw ORDER BY/GROUP BY in pipeline and add E2E tests

### Test Coverage

| Test Suite | Count | Location |
|-----------|-------|----------|
| Unit tests (NOT IN, BETWEEN, OR, ILIKE, mixed $N sequencing) | 5 | `repo.rs` lines 2124-2207 |
| Unit tests ($N renumbering, ORDER BY/GROUP BY raw, crypt, JSONB, mixed) | 7+ | `repo.rs` lines 2287-2400 |
| E2E tests (plan 01 WHERE operators) | 5 | `e2e.rs` lines 3919-3989 |
| E2E tests (plan 02 fragments + raw clauses) | 7 | `e2e.rs` lines 3995-4093 |

### Human Verification Required

None. All goal-relevant behaviors are verified through unit tests with exact SQL assertions and E2E compilation tests. The phase does not involve visual components, real-time behavior, or external service integration.

## Gaps Summary

No gaps. All 13 must-have truths are verified against the actual codebase. The phase goal -- rich WHERE conditions and raw SQL fragment embedding -- is achieved end-to-end from Mesh source syntax through MIR lowering, LLVM codegen, JIT registration, and SQL generation.

---

_Verified: 2026-02-17T20:46:11Z_
_Verifier: Claude (gsd-verifier)_
