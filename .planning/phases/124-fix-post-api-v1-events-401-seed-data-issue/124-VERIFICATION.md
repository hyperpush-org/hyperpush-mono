---
phase: 124-fix-post-api-v1-events-401-seed-data-issue
verified: 2026-02-26T00:00:00Z
status: passed
score: 4/4 must-haves verified
re_verification: false
human_verification:
  - test: "Run meshc migrate up against a live PostgreSQL instance and then POST /api/v1/events with the default API key"
    expected: "HTTP 202 with body {\"status\":\"accepted\"}"
    why_human: "Cannot execute SQL migrations or live HTTP requests in a static analysis environment"
  - test: "Run meshc migrate up a second time against a database that already has the seed data applied"
    expected: "Command exits 0 with no errors and no duplicate rows in organizations/projects/api_keys"
    why_human: "Idempotency of ON CONFLICT DO NOTHING requires a live PostgreSQL instance to confirm"
  - test: "Run meshc migrate down and confirm api_keys, projects, and organizations rows are removed without FK violations"
    expected: "Three DELETE statements succeed in order; no constraint errors"
    why_human: "Requires a live database to exercise the FK constraint ordering"
---

# Phase 124: Fix POST /api/v1/events 401 Seed Data Issue — Verification Report

**Phase Goal:** After running `meshc migrate up`, developers can immediately test POST /api/v1/events with a known API key without any manual database setup.
**Verified:** 2026-02-26
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Running `meshc migrate up` applies both the schema migration and the seed migration without error | VERIFIED | Seed migration `20260226000000_seed_default_org.mpl` exists with valid `pub fn up(pool :: PoolHandle) -> Int!String` signature matching the runner's expected interface. Version `20260226000000 > 20260216120000` guarantees correct ordering. |
| 2 | After migration, POST /api/v1/events with `x-sentry-auth: mshr_devdefaultapikey000000000000000000000000000` returns 202, not 401 | VERIFIED | Seed migration inserts the exact key value `mshr_devdefaultapikey000000000000000000000000000` into `api_keys`. `auth.mpl:authenticate_request` calls `get_project_by_api_key(pool, key)`. `queries.mpl:get_project_by_api_key` JOINs `api_keys` on `key_value = ?` with `revoked_at IS NULL` and returns a `Project` on match. Since `revoked_at` is not set by the seed, auth will succeed, and the ingestion route will return 202. |
| 3 | Running `meshc migrate up` a second time does not fail or duplicate seed data | VERIFIED | All three INSERT statements use `ON CONFLICT ... DO NOTHING`: organizations uses `ON CONFLICT (slug)`, projects uses `ON CONFLICT (slug) WHERE slug IS NOT NULL` (correct partial-index form), api_keys uses `ON CONFLICT (key_value)`. Second run is a no-op. |
| 4 | Running `meshc migrate down` rolls back the seed migration cleanly (drops inserted rows without FK violations) | VERIFIED | `down()` deletes in reverse FK dependency order: api_keys first (references projects), then projects (references organizations), then organizations. No FK violations possible. |

**Score: 4/4 truths verified**

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `mesher/migrations/20260226000000_seed_default_org.mpl` | Idempotent seed migration with `up()` and `down()` functions | VERIFIED | File exists, 50 lines (exceeds min_lines: 40). Contains `pub fn up` and `pub fn down` with the correct `PoolHandle` signature. Contains 3 occurrences of `mshr_devdefaultapikey` (header comment + INSERT + DELETE). |

### Artifact Detail — Level 1 (Exists)

`mesher/migrations/20260226000000_seed_default_org.mpl` — exists, 50 lines.

### Artifact Detail — Level 2 (Substantive)

- Contains `pub fn up(pool :: PoolHandle) -> Int!String do` — line 11
- Contains `pub fn down(pool :: PoolHandle) -> Int!String do` — line 44
- Contains `ON CONFLICT (slug) DO NOTHING` for organizations — line 13
- Contains `ON CONFLICT (slug) WHERE slug IS NOT NULL DO NOTHING` for projects — line 25
- Contains `ON CONFLICT (key_value) DO NOTHING` for api_keys — line 33
- Contains `DELETE FROM api_keys` — line 46
- Contains `DELETE FROM projects` — line 47
- Contains `DELETE FROM organizations` — line 48
- Contains `Repo.query_raw` for SELECT statements (org_id retrieval, project_id retrieval)
- No TODOs, placeholders, or stub implementations

### Artifact Detail — Level 3 (Wired)

This is a standalone migration file consumed directly by the `meshc migrate` runner. It is not imported by application code. Wiring is via filename convention: the runner scans `mesher/migrations/*.mpl` ordered by timestamp prefix, and calls `up(pool)` or `down(pool)` from each file. The file is correctly named, exports the correct function signatures, and sits alongside the existing schema migration.

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `mesher/migrations/20260226000000_seed_default_org.mpl` | `api_keys` table | `Pool.execute INSERT` with `key_value = 'mshr_devdefaultapikey000000000000000000000000000'` | WIRED | Line 33: `INSERT INTO api_keys (project_id, key_value, label) VALUES ($1::uuid, 'mshr_devdefaultapikey000000000000000000000000000', 'dev-default') ON CONFLICT (key_value) DO NOTHING` |
| `mesher/ingestion/auth.mpl:authenticate_request` | `mesher/storage/queries.mpl:get_project_by_api_key` | `from Storage.Queries import get_project_by_api_key` + call on line 32 | WIRED | `auth.mpl` line 4 imports `get_project_by_api_key`; line 32 calls `get_project_by_api_key(pool, key)`. `queries.mpl` line 113 implements the JOIN on `api_keys.key_value = ?` with `revoked_at IS NULL`. |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| SEED-01 | 124-01-PLAN.md | Seed migration providing default org, project, and API key after `meshc migrate up` | SATISFIED | `mesher/migrations/20260226000000_seed_default_org.mpl` exists and implements idempotent insertion of all three entities with the documented API key. |

### Requirements Cross-Reference Note

`SEED-01` is declared in `124-01-PLAN.md` frontmatter under `requirements: [SEED-01]` and marked complete in the SUMMARY (`requirements-completed: [SEED-01]`). However, **SEED-01 does not appear in `.planning/REQUIREMENTS.md`** — the requirements document covers v12.0 requirements (PIPE-*, REGEX-*, STRG-*, SKILL-*, REPO-*, BENCH-*) and has no SEED section. The traceability table ends at Phase 123 with BENCH-06.

This means SEED-01 is a phase-local requirement ID defined in the plan itself, not registered in the central requirements document. This is an informational gap in the project's requirements traceability but does not affect whether the implementation satisfies the stated goal.

---

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `mesher/migrations/20260226000000_seed_default_org.mpl` | Header | Plan describes key suffix as "48 hex chars" but actual suffix is 43 chars and contains non-hex characters (`v`, `u`, `l`, `t`, `p`, `i`, `k`, `y`) | INFO | No functional impact. Authentication is an exact text match against the stored key_value. The key literal is identical in the plan specification, migration file, and documented curl command. |

No blockers. No stub implementations. No TODO/FIXME/placeholder comments.

---

## Human Verification Required

### 1. Live Migration Apply + Auth Test

**Test:** Start PostgreSQL, run `DATABASE_URL=postgres://... meshc migrate up`, then run:
```
curl -X POST http://localhost:8080/api/v1/events \
  -H "x-sentry-auth: mshr_devdefaultapikey000000000000000000000000000" \
  -H "Content-Type: application/json" \
  -d '{"message":"test error","level":"error"}'
```
**Expected:** HTTP 202, body `{"status":"accepted"}`
**Why human:** Cannot run migrations or make HTTP requests in static analysis.

### 2. Idempotency Verification

**Test:** Run `meshc migrate up` twice against the same database.
**Expected:** Second run exits 0 with no errors, no duplicate rows in any of the three tables.
**Why human:** ON CONFLICT logic requires a live PostgreSQL instance to confirm.

### 3. Rollback Verification

**Test:** After applying, run `meshc migrate down`.
**Expected:** api_keys row deleted first, then projects row, then organizations row; no FK violation errors.
**Why human:** FK constraint ordering must be validated against live PostgreSQL.

---

## Gaps Summary

No gaps. All four must-have truths are verified through static analysis. The migration file is complete, substantive, and correctly wired to the migration runner via filename convention. The auth path from `x-sentry-auth` header to `api_keys` table lookup is fully implemented. Three items require human verification with a live database but are not blocking — the implementation is correct by inspection.

---

_Verified: 2026-02-26_
_Verifier: Claude (gsd-verifier)_
